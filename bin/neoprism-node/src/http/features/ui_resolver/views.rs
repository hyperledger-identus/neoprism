use std::error::Report;

use identus_apollo::hex::HexStr;
use identus_apollo::jwk::EncodeJwk;
use identus_did_core::Did;
use identus_did_prism::did::operation::{self, PublicKey, Service};
use identus_did_prism::did::{DidState, PrismDid, PrismDidOps, StorageState};
use identus_did_prism::dlt::{NetworkIdentifier, OperationMetadata};
use identus_did_prism::prelude::SignedPrismOperation;
use identus_did_prism::protocol::error::ProcessError;
use maud::{Markup, html};

use crate::app::service::error::ResolutionError;
use crate::http::{components, urls};

pub fn index(network: Option<NetworkIdentifier>) -> Markup {
    let body = html! {
        (search_box(None))
        section class="prism-entry prism-panel" aria-labelledby="entry-title" {
            h2 id="entry-title" { "Start with an identity" }
            p class="prism-entry-intro" { "Resolve a PRISM DID to inspect its keys, services, data and operation history." }
            p { "Use a canonical DID or a long-form DID. Resolution is read-only; you don’t need a wallet." }
            code class="prism-format" { "did:prism:<64-character hexadecimal suffix>" }
            a class="prism-button prism-secondary" href=(urls::Explorer::new_uri(None)) { "Browse indexed DIDs" }
            p class="prism-entry-note" { "Looking for an unpublished identity? Paste its full long-form DID, including the encoded initial state." }
        }
    };
    components::page_layout("Resolver", network, body)
}

pub fn resolve(
    network: Option<NetworkIdentifier>,
    did_str: &str,
    did_state: Result<(PrismDid, DidState), ResolutionError>,
    did_debug: Vec<(OperationMetadata, SignedPrismOperation, Option<ProcessError>)>,
) -> Markup {
    let resolution_body = match did_state.as_ref() {
        Err(e) => resolution_error_body(e),
        Ok((did, state)) => did_document_body(&did.to_did(), state),
    };
    let body = html! {
        (search_box(Some(did_str)))
        div class="flex flex-row w-full justify-center" {
            div class="flex flex-col w-full max-w-4xl items-center" {
                (resolution_body)
                (did_debug_body(did_debug))
            }
        }
    };
    components::page_layout("Resolver", network, body)
}

fn search_box(did: Option<&str>) -> Markup {
    html! {
        header class="prism-page-header" {
            h1 { "Identity resolver" }
            p { "A clearer view of your decentralized identity." }
        }
        form method="GET" action=(urls::Resolver::new_uri(None)) class="prism-resolve-form" {
            div class="prism-field" {
                label for="did" { "PRISM DID" }
                div class="prism-did-input" {
                    (components::icon("search"))
                    input type="text" id="did" name="did" placeholder="Enter PRISM DID"
                        value=[did] aria-describedby="did-help" autocomplete="off" autocapitalize="none"
                        spellcheck="false" required;
                }
            }
            button type="submit" class="prism-button prism-primary" { "Resolve" }
            p id="did-help" class="sr-only" { "Enter a canonical PRISM DID or the complete long-form DID, including its encoded initial state." }
            span class="sr-only" role="status" aria-live="polite" id="resolution-status" {}
        }
    }
}

fn resolution_error_body(error: &ResolutionError) -> Markup {
    let error_lines = Report::new(error)
        .pretty(true)
        .to_string()
        .split("\n")
        .map(|s| html! { (s) br; })
        .collect::<Vec<_>>();
    html! {
        div class="flex justify-center w-full" {
            div class="w-full m-4 space-y-4" {
                p class="text-2xl font-bold" { "Resolution error" }
                div class="card bg-base-200 border border-gray-700 font-mono text-sm p-3" {
                    @for line in error_lines { (line) }
                }
            }
        }
    }
}

fn did_document_body(did: &Did, state: &DidState) -> Markup {
    let contexts = state.context.as_slice();
    let public_keys = state.public_keys.as_slice();
    let services = state.services.as_slice();
    let did_doc_url = urls::ApiDid::new_uri(did.to_string());
    let storages = &state.storage;
    html! {
        div class="flex justify-center w-full" {
            div class="w-full m-4 space-y-4" {
                p class="text-2xl font-bold" { "DID state" }
                a class="btn btn-xs btn-outline" href=(did_doc_url) target="_blank" { "Resolver API" }
                (context_card(contexts))
                (public_key_card(public_keys))
                (service_card(services))
                (storage_card(storages))
            }
        }
    }
}

fn context_card(context: &[String]) -> Markup {
    html! {
        div class="card bg-base-200 border border-gray-700" {
            div class="card-body" {
                h2 class="card-title" { "@context" }
                @if context.is_empty() {
                    p class="text-neutral-content" { "Empty" }
                }
                ul class="list-disc list-inside" {
                    @for ctx in context {
                        li { (ctx) }
                    }
                }
            }
        }
    }
}

fn public_key_card(public_keys: &[PublicKey]) -> Markup {
    let mut sorted_pks = public_keys.to_vec();
    sorted_pks.sort_by_key(|i| i.id.to_string());

    let pk_elems = sorted_pks
        .iter()
        .map(|pk| {
            let jwk = match &pk.data {
                operation::PublicKeyData::Master { data } => data.encode_jwk(),
                operation::PublicKeyData::Vdr { data } => data.encode_jwk(),
                operation::PublicKeyData::Other { data, .. } => data.encode_jwk(),
            };
            let key_id = pk.id.to_string();
            let key_usage = format!("{:?}", pk.data.usage());
            let curve = jwk.crv;
            let encoded_x = jwk.x.map(|i| i.to_string()).unwrap_or_default();
            let encoded_y = jwk.y.map(|i| i.to_string()).unwrap_or_default();
            html! {
                li class="border p-2 rounded-md border-gray-700 wrap-anywhere" {
                    strong { "ID: " } (key_id)
                    br;
                    strong { "Usage: " } (key_usage)
                    br;
                    strong { "Curve: " } (curve)
                    br;
                    strong { "X: " } (encoded_x)
                    br;
                    strong { "Y: " } (encoded_y)
                }
            }
        })
        .collect::<Vec<_>>();

    html! {
        div class="card bg-base-200 border border-gray-700" {
            div class="card-body" {
                h2 class="card-title" { "Public Keys" }
                @if pk_elems.is_empty() {
                    p class="text-neutral-content" { "Empty" }
                }
                ul class="space-y-2" {
                    @for elem in pk_elems { (elem) }
                }
            }
        }
    }
}

fn service_card(services: &[Service]) -> Markup {
    let svc_elems = services
        .iter()
        .map(|svc| {
            let svc_id = &svc.id;
            let svc_ty = format!("{:?}", svc.r#type);
            let svc_ep = format!("{:?}", svc.service_endpoint);
            html! {
                li class="border p-2 rounded-md border-gray-700 wrap-anywhere" {
                    strong { "ID: " } (svc_id)
                    br;
                    strong { "Type: " } span class="font-mono" { (svc_ty) }
                    br;
                    strong { "Endpoint: " } span class="font-mono" { (svc_ep) }
                }
            }
        })
        .collect::<Vec<_>>();

    html! {
        div class="card bg-base-200 border border-gray-700" {
            div class="card-body" {
                h2 class="card-title" { "Services" }
                @if svc_elems.is_empty() {
                    p class="text-neutral-content" { "Empty" }
                }
                ul class="space-y-2" {
                    @for elem in svc_elems { (elem) }
                }
            }
        }
    }
}

fn storage_card(storages: &[StorageState]) -> Markup {
    let mut sorted_storages = storages.to_vec();
    sorted_storages.sort_by_key(|s| s.init_operation_hash.to_vec());

    let storage_elems = sorted_storages
        .iter()
        .map(|s| {
            let init_hash_hex = HexStr::from(s.init_operation_hash.as_bytes()).to_string();
            let last_hash_hex = HexStr::from(s.last_operation_hash.as_bytes()).to_string();
            let data = format!("{:?}", s.data);
            html! {
                li class="border p-2 rounded-md border-gray-700 wrap-anywhere" {
                    strong { "Init operation hash: " } (init_hash_hex)
                    br;
                    strong { "Last operation hash: " } (last_hash_hex)
                    br;
                    strong { "Data: " }
                    br;
                    div class="bg-base-300 font-mono text-sm text-neutral-content p-3" {
                        (data)
                    }
                }
            }
        })
        .collect::<Vec<_>>();

    html! {
        div class="card bg-base-200 border border-gray-700" {
            div class="card-body" {
                h2 class="card-title" { "VDR entries" }
                @if storage_elems.is_empty() {
                    p class="text-neutral-content" { "Empty" }
                }
                ul class="space-y-2" {
                    @for elem in storage_elems { (elem) }
                }
            }
        }
    }
}

fn did_debug_body(did_debug: Vec<(OperationMetadata, SignedPrismOperation, Option<ProcessError>)>) -> Markup {
    let op_elems = did_debug
        .iter()
        .map(|(metadata, signed_op, error)| {
            let block_time = metadata.block_metadata.cbt.to_rfc3339();
            let operation_id = signed_op.operation_id();
            let operation_payload = format!("{signed_op:?}");
            let error_lines = error
                .as_ref()
                .map(|e| Report::new(e).pretty(true).to_string())
                .unwrap_or_else(|| "-".to_string())
                .split("\n")
                .map(|s| html! { (s) br; })
                .collect::<Vec<_>>();
            html! {
                li class="border p-2 rounded-md bg-base-200 border-gray-700 wrap-anywhere" {
                    strong { "Block time: " } (block_time)
                    br;
                    strong { "Slot no: " } (metadata.block_metadata.slot_number)
                    br;
                    strong { "Block no: " } (metadata.block_metadata.block_number)
                    br;
                    strong { "Block seq no: " } (metadata.block_metadata.absn)
                    br;
                    strong { "Transaction ID: " } (metadata.block_metadata.tx_id)
                    br;
                    strong { "Operation ID: " } (operation_id)
                    br;
                    strong { "Operation seq no: " } (metadata.osn)
                    br;
                    strong { "Operation payload: " }
                    br;
                    div class="bg-base-300 font-mono text-sm text-neutral-content p-3" {
                        (operation_payload)
                    }
                    strong { "Error: " }
                    br;
                    div class="bg-base-300 font-mono text-sm text-neutral-content p-3" {
                        @for line in error_lines { (line) }
                    }
                }
            }
        })
        .collect::<Vec<_>>();

    html! {
        div class="flex justify-center w-full" {
            div class="w-full m-4 space-y-4" {
                p class="text-2xl font-bold" { "Operation debug" }
                ul class="space-y-2" {
                    @for elem in op_elems { (elem) }
                }
            }
        }
    }
}
