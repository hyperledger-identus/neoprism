//! Operation history is evidence captured during resolution, not a success-only view.
use identus_did_prism::dlt::OperationMetadata;
use identus_did_prism::prelude::SignedPrismOperation;
use identus_did_prism::proto::prism::prism_operation::Operation;
use identus_did_prism::protocol::error::ProcessError;
use maud::{Markup, html};

use super::{diagnostics, identity};

pub(super) type HistoryEntry = (OperationMetadata, SignedPrismOperation, Option<ProcessError>);

pub(super) fn history(entries: &[HistoryEntry]) -> Markup {
    html! {
        section id="operation-history" tabindex="-1" class="prism-panel prism-operations" aria-labelledby="operations-title" {
            div class="prism-section-heading" {
                h2 id="operations-title" { "Operation history" }
                span class="prism-count" { (entries.len()) }
            }
            @if entries.is_empty() {
                p class="prism-muted" { "No indexed operations" }
            } @else {
                p class="prism-muted" { "Captured ledger operations, including rejected operations. Processing outcomes do not guarantee a usable final identity state." }
                ol class="prism-operation-list" {
                    @for (index, (metadata, signed, error)) in entries.iter().enumerate() {
                        li {
                            (operation(index + 1, metadata, signed, error.as_ref()))
                        }
                    }
                }
            }
        }
    }
}

fn operation(
    index: usize,
    metadata: &OperationMetadata,
    signed: &SignedPrismOperation,
    error: Option<&ProcessError>,
) -> Markup {
    let outcome = if error.is_some() { "Rejected" } else { "Applied" };
    // Capture once; both inspection and copying consume precisely the same Debug string.
    // This is the full signed message, including signature, operation and protobuf fields.
    let payload = format!("{signed:?}");
    let block = &metadata.block_metadata;
    html! {
        details class="prism-operation" {
            summary {
                span class="prism-operation-label" {
                    span class="prism-operation-kind" { (operation_kind(signed)) }
                    span class="prism-muted" {
                        "Operation " (index) " · "
                        time datetime=(block.cbt.to_rfc3339()) { (block.cbt.format("%d %b %Y, %H:%M UTC")) }
                    }
                }
                span class=(if error.is_some() { "prism-operation-outcome prism-operation-rejected" } else { "prism-operation-outcome" }) { (outcome) }
            }
            dl class="prism-operation-fields" {
                (field(index, "Signer", &signed.signed_with))
                (field(index, "Block time · UTC", &block.cbt.to_rfc3339()))
                (field(index, "Slot", &block.slot_number.to_string()))
                (field(index, "Block", &block.block_number.to_string()))
                (field(index, "Block sequence · absn", &block.absn.to_string()))
                (field(index, "Operation sequence · osn", &metadata.osn.to_string()))
                (field(index, "Transaction ID", &block.tx_id.to_string()))
                (field(index, "Operation ID", &signed.operation_id().to_string()))
            }
            details class="prism-operation-inspector" {
                summary { "Processing outcome · " (outcome) }
                div class="prism-operation-processing" {
                    @if error.is_some() {
                        p { "Rejected during processing; not applied to DID state. Inspect the safe domain diagnostics and the signed payload." }
                    } @else {
                        p { "Applied to DID state during processing. No domain rejection was reported." }
                    }
                }
            }
            @if let Some(error) = error {
                details class="prism-operation-inspector prism-operation-domain" {
                    summary { "Safe domain diagnostics" }
                    div class="prism-operation-diagnostic" {
                        p { (diagnostics::process(error)) }
                    }
                }
            }
            details class="prism-operation-inspector prism-operation-payload" {
                summary { "Complete signed payload · Debug" }
                p class="prism-muted" { "Complete captured Debug representation of the signed operation, not authored wire bytes or JSON." }
                div class="prism-operation-copy" {
                    (identity::copy(&format!("operation {index} complete signed payload"), &payload))
                }
                pre tabindex="0" aria-label=(format!("Operation {index} complete signed payload")) { code { (&payload) } }
                a class="prism-return-link" href="#operation-history" { "Back to operation history" }
            }
        }
    }
}

fn operation_kind(signed: &SignedPrismOperation) -> &'static str {
    match signed
        .operation
        .as_ref()
        .and_then(|operation| operation.operation.as_ref())
    {
        Some(Operation::CreateDid(_)) => "Create DID",
        Some(Operation::UpdateDid(_)) => "Update DID",
        Some(Operation::DeactivateDid(_)) => "Deactivate DID",
        Some(Operation::ProtocolVersionUpdate(_)) => "Update protocol version",
        Some(Operation::CreateStorageEntry(_)) => "Create VDR entry",
        Some(Operation::UpdateStorageEntry(_)) => "Update VDR entry",
        Some(Operation::DeactivateStorageEntry(_)) => "Deactivate VDR entry",
        _ => "Unknown operation",
    }
}

fn field(index: usize, label: &str, value: &str) -> Markup {
    html! {
        div class="prism-data-field" {
            dt { (label) }
            dd {
                code { (value) }
                (identity::copy(&format!("operation {index} {label}"), value))
            }
        }
    }
}
