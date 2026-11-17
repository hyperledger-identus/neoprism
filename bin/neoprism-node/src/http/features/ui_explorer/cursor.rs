use chrono::SecondsFormat;
use identus_apollo::hex::HexStr;
use identus_did_prism::dlt::DltCursor;
use maud::{Markup, html};

use crate::http::urls;

/// The watch snapshot is evidence of a reported DLT position, not source liveness.
pub fn dlt_cursor_card(cursor: Option<DltCursor>) -> Markup {
    let present = cursor.is_some();
    html! {
        section id="dlt-cursor-card" class="prism-panel prism-sync"
            aria-labelledby="sync-heading" data-cursor-present=(cursor.is_some())
            data-refresh-url=(urls::ExplorerDltCursor::new_uri())
            data-refresh-interval="2000" data-refresh-resource="cursor" data-refresh-state="received" {
            header class="prism-sync-heading" {
                h2 id="sync-heading" { "Sync status" }
                p class="prism-sync-status" role="status" data-cursor-status {
                    @if cursor.is_some() { "Cursor snapshot supplied" }
                    @else { "Waiting for sync information" }
                }
            }
            div data-refresh-content {
            @if let Some(cursor) = cursor {
                @let slot = cursor.slot.to_string();
                @let formatted_slot = slot.chars().enumerate().fold(String::new(), |mut text, (i, ch)| {
                    if i > 0 && (slot.len() - i).is_multiple_of(3) { text.push(','); }
                    text.push(ch);
                    text
                });
                @let hash = HexStr::from(&cursor.block_hash).to_string();
                @let timestamp = cursor.cbt.map(|time| time.to_rfc3339_opts(SecondsFormat::AutoSi, true));
                dl class="prism-sync-summary" {
                    div {
                        dt { "Reported slot" }
                        dd class="prism-sync-slot" { (formatted_slot) }
                    }
                    div {
                        dt { "Block time · UTC" }
                        dd {
                            @if let Some(time) = cursor.cbt {
                                time datetime=(timestamp.as_deref().unwrap_or_default()) {
                                    (time.format("%d %b %Y, %H:%M:%S UTC"))
                                }
                            } @else { "Not provided" }
                        }
                    }
                }
                p class="prism-sync-distance" { "Distance to chain tip unknown" }
                details id="cursor-technical" class="prism-sync-details" {
                    summary aria-controls="cursor-technical-values" { "Technical details" }
                    div id="cursor-technical-values" {
                        dl class="prism-sync-raw" {
                            (technical_value("Reported slot · raw", "reported slot", Some(&slot)))
                            (technical_value("Block hash", "block hash", Some(&hash)))
                            (technical_value("Block time · raw UTC", "block timestamp", timestamp.as_deref()))
                        }
                        p class="prism-sync-explanation" {
                            "This is the source-reported DLT cursor, not the last indexed PRISM operation. "
                            "Browser receipt time is separate from block time. "
                            "Receiving a snapshot does not prove cursor advancement, upstream health or chain synchronization."
                        }
                    }
                }
            } @else {
                p class="prism-sync-distance" { "Distance to chain tip unknown" }
            }
            }
            p class="prism-sync-receipt" data-cursor-receipt data-refresh-receipt {
                @if present { "Snapshot supplied with this response. " }
                @else { "No cursor was supplied with this response. " }
                "Browser checks every 2 seconds while visible when scripts are enabled."
            }
            p class="prism-refresh-deferred" data-refresh-deferred hidden {}
            a class="prism-button prism-secondary prism-refresh-action" href="" data-refresh-action { "Refresh sync information" }
        }
    }
}

fn technical_value(label: &str, copy_name: &str, value: Option<&str>) -> Markup {
    html! {
        div {
            dt { (label) }
            dd {
                div class="prism-sync-value" {
                    @if let Some(value) = value { code { (value) } }
                    @else { "Not provided" }
                }
                @if let Some(value) = value {
                    button type="button" class="prism-copy" data-copy-value=(value)
                        aria-label=(format!("Copy {copy_name}")) { "Copy" }
                }
            }
        }
    }
}
