//! Resolver presentation outcomes; retrieval and protocol semantics stay in the service.
use maud::{Markup, html};

use super::diagnostics;
use super::models::DidQuery;
use crate::app::service::error::{InvalidDid, ResolutionError};
use crate::http::urls;

pub(super) fn failure(error: &ResolutionError, input: &str, has_history: bool) -> Markup {
    let diagnostic = diagnostics::resolution(error);
    let (title, explanation) = match error {
        ResolutionError::NotFound if has_history => (
            "Stored operations cannot initialize DID state",
            "Stored operations were retrieved, but they could not initialize a valid DID state. This is not a server interruption. Inspect the rejected operations and safe domain diagnostics below, or check the DID and configured network.",
        ),
        ResolutionError::NotFound => (
            "No indexed state on this node",
            "No stored operations were found on this node. This does not prove that the DID does not exist on-chain. Check the DID and configured network; a recent operation may not be indexed yet.",
        ),
        ResolutionError::InternalError { .. } => (
            "Couldn’t retrieve identity state",
            "This node could not retrieve stored operations. Your input is preserved. Retry resolution or browse Explorer; no claim about the identity or its history can be made from this request.",
        ),
        ResolutionError::InvalidDid {
            source: InvalidDid::ProcessStateFailed { .. },
        } => (
            "Check the encoded initial state",
            "Your complete input is preserved. The long-form DID’s initial operation could not initialize valid state. Check the original long-form DID, including its encoded initial state, and resolve again.",
        ),
        ResolutionError::InvalidDid { .. } | ResolutionError::MethodNotSupported => (
            "Check the DID format",
            "Your complete input is preserved. Enter did:prism: followed by a 64-character hexadecimal suffix, or a complete valid long-form DID. Correct the value and resolve again.",
        ),
    };
    html! {
        section class="prism-panel prism-outcome" aria-labelledby="resolution-outcome-title" {
            h2 id="resolution-outcome-title" { (title) }
            p { (explanation) }
            div class="prism-outcome-actions" {
                a class="prism-button prism-secondary" href="#did" { "Correct DID" }
                a data-resolver-retry class="prism-button prism-secondary" href=(urls::Resolver::new_uri(Some(DidQuery { did: Some(input.to_owned()) }))) { @if matches!(error, ResolutionError::InternalError { .. }) { "Retry resolution" } @else { "Try again" } }
                a class="prism-button prism-secondary" href=(urls::Explorer::new_uri(None)) { "Browse Explorer" }
            }
            details class="prism-outcome-diagnostics" {
                summary { "Safe resolution diagnostics" }
                pre { code { (diagnostic) } }
            }
        }
    }
}
