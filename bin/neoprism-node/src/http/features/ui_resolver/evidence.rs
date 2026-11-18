use identus_did_prism::did::DidState;
use identus_did_prism::dlt::OperationMetadata;
use identus_did_prism::prelude::SignedPrismOperation;
use identus_did_prism::protocol::error::ProcessError;
use maud::{Markup, html};

use super::identity::copy;
use crate::http::components::disclosure_icon;

pub(super) fn ledger(
    state: &DidState,
    history: &[(OperationMetadata, SignedPrismOperation, Option<ProcessError>)],
) -> Markup {
    // Publication comes from resolution, never from canonical/long-form syntax.
    // Unpublished derivations have no ledger evidence, even if synthetic metadata is supplied.
    let selected = state
        .is_published
        .then(|| {
            history
                .iter()
                .filter(|(_, _, error)| error.is_none())
                .max_by(|(a, _, _), (b, _, _)| OperationMetadata::compare_time_asc(a, b))
        })
        .flatten();
    html! {
        details id="ledger-evidence" class="prism-panel prism-evidence" {
            summary {
                h2 { "Ledger evidence" }
                (disclosure_icon())
            }
            div class="prism-evidence-body" {
                @match selected {
                    Some((metadata, operation, _)) => {
                        p class="prism-evidence-source" { "Latest applied operation" }
                        p class="prism-muted" { "Evidence from this identity’s indexed history, not the chain tip or a freshness guarantee." }
                        dl {
                            (field("Block", &metadata.block_metadata.block_number.to_string()))
                            (field("Slot", &metadata.block_metadata.slot_number.to_string()))
                            (field("Signer", &operation.signed_with))
                            (field("Transaction ID", &metadata.block_metadata.tx_id.to_string()))
                            (field("Operation ID", &operation.operation_id().to_string()))
                            (field("Block time · UTC", &metadata.block_metadata.cbt.to_rfc3339()))
                        }
                    }
                    None => p class="prism-muted" {
                        @if state.is_published { "No applied ledger evidence available." }
                        @else { "No ledger confirmation. This identity was derived locally." }
                    },
                }
            }
        }
    }
}

fn field(label: &str, value: &str) -> Markup {
    html! {
        div class="prism-data-field" {
            dt { (label) }
            dd {
                code { (value) }
                (copy(label, value))
            }
        }
    }
}
