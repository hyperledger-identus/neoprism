use identus_did_prism::did::{DidState, PrismDid, PrismDidOps};
use maud::{Markup, html};

use crate::http::urls;

pub(super) fn copy(label: &str, value: &str) -> Markup {
    // HTML parsing normalizes literal CR/CRLF and replaces NUL. JSON escapes
    // those code points before Maud performs the usual attribute escaping.
    let encoded = value.contains(['\r', '\0']);
    let transport = if encoded {
        serde_json::to_string(value).expect("strings can be serialized")
    } else {
        value.to_owned()
    };
    html! {
        button type="button" class="prism-copy prism-button prism-secondary"
            data-copy-value=(transport) data-copy-encoding=[encoded.then_some("json")] aria-label=(format!("Copy {label}")) {
            @if label == "DID" { "Copy DID" }
            @else {
                svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true" {
                    rect x="9" y="9" width="12" height="12" rx="2" {}
                    path d="M15 9V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h4" {}
                }
            }
        }
    }
}

pub(super) fn summary(did: &PrismDid, state: &DidState, operation_count: usize) -> Markup {
    let did = did.to_did().to_string();
    html! {
        section class="prism-panel prism-identity" aria-labelledby="identity-title" {
            h2 id="identity-title" { "Identity" }
            @if state.is_published && state.is_deactivated() {
                p class="prism-identity-status prism-deactivated" { "Deactivated" }
                p class="prism-muted" { "This indexed identity is deactivated. Its operation history remains available." }
            } @else if state.is_published {
                p class="prism-identity-status" { "Indexed state available" }
            } @else {
                p class="prism-identity-status" { "Locally derived" }
                p class="prism-muted" { "No ledger confirmation. This identity is derived locally from its initial state." }
            }
            code class="prism-identity-did" { (did) }
            div class="prism-identity-actions" {
                (copy("DID", &did))
                (api_link(&did))
            }
            p class="prism-identity-counts" {
                @for (index, (count, singular, plural)) in [
                    (state.public_keys.len(), "public key", "public keys"),
                    (state.services.len(), "service", "services"),
                    (state.context.len(), "context", "contexts"),
                    (state.storage.len(), "VDR entry", "VDR entries"),
                    (operation_count, "operation", "operations"),
                ].iter().enumerate() {
                    @if index > 0 { " · " }
                    (count) " " (if *count == 1 { singular } else { plural })
                }
            }
        }
    }
}

fn api_link(did: &str) -> Markup {
    html! {
        a class="prism-button prism-secondary" href=(urls::ApiDid::new_uri(did.to_owned()))
            target="_blank" rel="noopener noreferrer" { "Open Resolver API" }
    }
}

pub(super) fn api(did: &PrismDid) -> Markup {
    html! {
        section class="prism-panel prism-api" aria-labelledby="resolver-api-title" {
            h2 id="resolver-api-title" { "Resolver API" }
            p { "Indexed state is not a DID document response. Open the API to request one separately in a new tab." }
            (api_link(&did.to_did().to_string()))
        }
    }
}
