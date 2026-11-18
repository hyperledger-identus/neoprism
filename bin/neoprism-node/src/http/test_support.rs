//! Shared fixtures and HTML assertions for the HTTP/rendered seam only.
use identus_did_prism::did::{DidState, PrismDid};
use identus_did_prism::protocol::resolver::resolve_unpublished;

// Published documentation's deterministic identity; never production sample data.
pub(super) const LONG_FORM: &str = "did:prism:9b5118411248d9663b6ab15128fba8106511230ff654e7514cdcc4ce919bde9b:Cj8KPRI7CgdtYXN0ZXIwEAFKLgoJc2VjcDI1NmsxEiEDHpf-yhIns-LP3tLvA8icC5FJ1ZlBwbllPtIdNZ3q0jU";

pub(super) fn local_identity() -> (PrismDid, DidState) {
    let did: PrismDid = LONG_FORM.parse().expect("valid deterministic long-form DID");
    let PrismDid::LongForm(long) = &did else { unreachable!() };
    let state = resolve_unpublished(long.operation().unwrap()).expect("valid initial state");
    (did, state)
}

pub(super) fn section<'a>(body: &'a str, id: &str) -> &'a str {
    let marker = format!("id=\"{id}\"");
    let (_, content) = body
        .split_once(&marker)
        .unwrap_or_else(|| panic!("missing section {id}"));
    content
        .split_once("</section>")
        .unwrap_or_else(|| panic!("unclosed section {id}"))
        .0
}
