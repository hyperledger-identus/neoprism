use identus_apollo::hex::HexStr;
use identus_did_prism::did::StorageState;
use identus_did_prism::did::operation::StorageData;
use maud::{Markup, html};

use super::identity::copy;

pub(super) fn entries(storages: &[StorageState]) -> Markup {
    let mut sorted = storages.iter().collect::<Vec<_>>();
    sorted.sort_by_key(|s| s.init_operation_hash.to_vec());
    html! {
        section id="vdr-entries" class="prism-panel prism-vdr" aria-labelledby="vdr-title" {
            div class="prism-section-heading" {
                h2 id="vdr-title" { "VDR entries" }
                span class="prism-count" { (storages.len()) }
            }
            @if sorted.is_empty() {
                p class="prism-muted" { "No VDR entries" }
            } @else {
                p class="prism-muted" { "Verifiable data registry entries attach data to this identity." }
                ul class="prism-vdr-list" {
                    @for (index, storage) in sorted.iter().enumerate() {
                        @let number = index + 1;
                        @let initial = HexStr::from(storage.init_operation_hash.as_bytes()).to_string();
                        @let last = HexStr::from(storage.last_operation_hash.as_bytes()).to_string();
                        @let raw = format!("{:?}", storage.data);
                        li class="prism-vdr-entry" aria-label=(format!("VDR entry {number}")) {
                            dl {
                                (field(number, "Initial operation hash", &initial))
                                (field(number, "Last operation hash", &last))
                            }
                            @match storage.data.as_ref() {
                                StorageData::Bytes(bytes) => {
                                    p class="prism-vdr-type" { "Bytes" }
                                    dl { (field(number, "Bytes · hex", &HexStr::from(bytes.as_slice()).to_string())) }
                                }
                                StorageData::StatusList(status) => {
                                    p class="prism-vdr-type" { "StatusList" }
                                    p class="prism-muted" { "Numeric state is shown without interpretation." }
                                    dl {
                                        (field(number, "State", &status.state.to_string()))
                                        (field(number, "Name", &status.name))
                                        (field(number, "Detail", &status.detail))
                                    }
                                }
                                StorageData::Ipfs(value) => {
                                    p class="prism-vdr-type" { "IPFS" }
                                    dl { (field(number, "IPFS value", value)) }
                                }
                            }
                            details class="prism-vdr-raw" {
                                summary { (format!("Raw data · entry {number}")) }
                                p class="prism-muted" { "Captured Debug representation of storage data; not original wire bytes." }
                                pre { code { (raw) } }
                                div class="prism-vdr-raw-actions" { (copy(&format!("VDR entry {number} raw data"), &raw)) }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn field(number: usize, label: &str, value: &str) -> Markup {
    html! {
        div class="prism-data-field" {
            dt { (label) }
            dd {
                @if value.is_empty() {
                    span class="prism-vdr-empty prism-muted" { "Empty" }
                } @else {
                    code { (value) }
                }
                (copy(&format!("VDR entry {number} {label}"), value))
            }
        }
    }
}
