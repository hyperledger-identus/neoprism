use std::error::Report;

use identus_did_prism::did::{DidState, PrismDid};
use identus_did_prism::dlt::{NetworkIdentifier, OperationMetadata};
use identus_did_prism::prelude::SignedPrismOperation;
use identus_did_prism::protocol::error::ProcessError;
use maud::{Markup, html};

use super::{contexts_services, evidence, identity, keys, vdr};
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
    let body = html! {
        (search_box(Some(did_str)))
        div class="prism-resolver-result" {
            @match did_state.as_ref() {
                Err(e) => {
                    (resolution_error_body(e))
                    (did_debug_body(did_debug))
                }
                Ok((did, state)) => {
                    @let ledger = evidence::ledger(state, &did_debug);
                    div class="prism-resolution-grid" {
                        div class="prism-dossier" {
                            (identity::summary(did, state, if state.is_published { did_debug.len() } else { 0 }))
                            (did_document_body(state))
                            @if state.is_published {
                                (did_debug_body(did_debug))
                            } @else {
                                (did_debug_body(vec![]))
                            }
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

fn did_document_body(state: &DidState) -> Markup {
    html! {
        div class="prism-state-sections" {
            h2 class="sr-only" { "DID state" }
            (keys::public_keys(&state.public_keys))
            (contexts_services::contexts(&state.context))
            (contexts_services::services(&state.services))
            (vdr::entries(&state.storage))
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
                @if op_elems.is_empty() {
                    p { "No indexed operations" }
                }
                ul class="space-y-2" {
                    @for elem in op_elems { (elem) }
                }
            }
        }
    }
}
