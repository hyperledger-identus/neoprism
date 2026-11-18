use identus_did_prism::did::{DidState, PrismDid};
use identus_did_prism::dlt::{NetworkIdentifier, OperationMetadata};
use identus_did_prism::prelude::SignedPrismOperation;
use identus_did_prism::protocol::error::ProcessError;
use maud::{Markup, html};

use super::{contexts_services, evidence, identity, keys, operations, outcomes, vdr};
use crate::app::service::error::ResolutionError;
use crate::http::{components, urls};

pub fn index(network: Option<NetworkIdentifier>) -> Markup {
    let body = html! {
        (search_box(None, false))
        div id="resolution-result" class="prism-resolver-result" {
            section class="prism-entry prism-panel" aria-labelledby="entry-title" {
                h2 id="entry-title" { "Start with an identity" }
                p class="prism-entry-intro" { "Resolve a PRISM DID to inspect its keys, services, data and operation history." }
                p { "Use a canonical DID or a long-form DID. Resolution is read-only; you don’t need a wallet." }
                code class="prism-format" { "did:prism:<64-character hexadecimal suffix>" }
                a class="prism-button prism-secondary" href=(urls::Explorer::new_uri(None)) { "Browse indexed DIDs" }
                p class="prism-entry-note" { "Looking for an unpublished identity? Paste its full long-form DID, including the encoded initial state." }
            }
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
        Err(ResolutionError::InternalError { .. }) => html! {
            section id="operation-history" class="prism-panel prism-outcome" aria-labelledby="history-unavailable-title" {
                h2 id="history-unavailable-title" { "Operation history" }
                p { "No operation history was retrieved. Stored operations may still exist; try again when retrieval is available." }
            }
        },
        Ok((_, state)) if !state.is_published => operations::history(&[]),
        _ => operations::history(&did_debug),
    };
    let body = html! {
        (search_box(Some(did_str), matches!(&did_state, Err(ResolutionError::InvalidDid { .. } | ResolutionError::MethodNotSupported))))
        div id="resolution-result" class="prism-resolver-result" data-resolved-input=(did_str) {
            @match did_state.as_ref() {
                Err(e) => {
                    (outcomes::failure(e, did_str, !did_debug.is_empty()))
                    (&operation_history)
                }
                Ok((did, state)) => {
                    @let ledger = evidence::ledger(state, &did_debug);
                    div class="prism-resolution-grid" {
                        div class="prism-dossier" {
                            (identity::summary(did, state, if state.is_published { did_debug.len() } else { 0 }))
                            (did_document_body(state))
                            (&operation_history)
                            a class="prism-return-link" href="#identity-summary" { "Back to identity" }
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

fn search_box(did: Option<&str>, invalid: bool) -> Markup {
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
                        value=[did] aria-invalid=[invalid.then_some("true")] aria-describedby="did-help" autocomplete="off" autocapitalize="none"
                        spellcheck="false" required;
                }
            }
            button type="submit" class="prism-button prism-primary" { "Resolve" }
            p id="did-help" class="sr-only" { "Enter a canonical PRISM DID or the complete long-form DID, including its encoded initial state." }
        }
        p class="prism-resolution-status" role="status" aria-live="polite" id="resolution-status" {}
        section id="resolution-recovery" class="prism-panel prism-outcome prism-request-failure" hidden aria-labelledby="request-failure-title" {
            h2 id="request-failure-title" { "Couldn’t load this resolution" }
            p { "No new result was received. Your complete input is preserved. Retry the request, correct the DID or browse Explorer." }
            p id="resolution-previous" hidden { "Previous result · not refreshed. Any identity state or history below belongs to the previous request, not this failed attempt." }
            div class="prism-outcome-actions" {
                a id="resolution-retry" data-resolver-retry class="prism-button prism-secondary" href=(urls::Resolver::new_uri(None)) { "Retry resolution" }
                a class="prism-button prism-secondary" href="#did" { "Correct DID" }
                a class="prism-button prism-secondary" href=(urls::Explorer::new_uri(None)) { "Browse Explorer" }
            }
        }
        div id="resolution-pending" class="prism-panel prism-resolution-pending" hidden aria-hidden="true" {
            div {} div {} div {} div {}
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
