use identus_apollo::hex::HexStr;
use identus_did_prism::did::operation::Service;
use identus_did_prism::did::{DidState, PrismDid, StorageState};
use identus_did_prism::dlt::{NetworkIdentifier, OperationMetadata};
use identus_did_prism::prelude::SignedPrismOperation;
use identus_did_prism::protocol::error::ProcessError;
use maud::{Markup, html};

use super::{diagnostics, evidence, identity, keys, operations};
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
    // Capture Debug payloads before evidence hashing can update protobuf size caches.
    // Unpublished resolution contributes synthetic metadata, not ledger history.
    let operation_history = match &did_state {
        Ok((_, state)) if !state.is_published => operations::history(&[]),
        _ => operations::history(&did_debug),
    };
    let body = html! {
        (search_box(Some(did_str)))
        div class="prism-resolver-result" {
            @match did_state.as_ref() {
                Err(e) => {
                    (resolution_error_body(e))
                    (&operation_history)
                }
                Ok((did, state)) => {
                    @let ledger = evidence::ledger(state, &did_debug);
                    div class="prism-resolution-grid" {
                        div class="prism-dossier" {
                            (identity::summary(did, state, if state.is_published { did_debug.len() } else { 0 }))
                            (did_document_body(state))
                            (&operation_history)
                        }
                        aside class="prism-evidence-rail" aria-label="Identity evidence and API" {
                            (ledger)
                            (identity::api(did))
                        }
                    }
                }
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
    let message = diagnostics::resolution(error);
    let error_lines = message.split("\n").map(|s| html! { (s) br; }).collect::<Vec<_>>();
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

fn did_document_body(state: &DidState) -> Markup {
    html! {
        div class="prism-state-sections" {
            h2 class="sr-only" { "DID state" }
            (keys::public_keys(&state.public_keys))
            (context_card(&state.context))
            (service_card(&state.services))
            (storage_card(&state.storage))
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
