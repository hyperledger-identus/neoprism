use identus_did_prism::did::CanonicalPrismDid;
use identus_did_prism::utils::paging::Paginated;
use maud::{Markup, html};

use super::models::PageQuery;
use crate::http::features::ui_resolver::models::DidQuery;
use crate::http::{components, urls};

pub fn render(dids: Paginated<CanonicalPrismDid>) -> Markup {
    let page = dids.current_page + 1;
    if dids.items.is_empty() {
        return resource(
            page,
            "received",
            html! {
                    div class="prism-panel prism-list-state" {
                        @if dids.total_items == 0 {
                            h2 { "No indexed DIDs on this node" }
                            a class="prism-button prism-secondary" href=(urls::Resolver::new_uri(None)) { "Resolve a known DID" }
                        } @else {
                            h2 { "Page " (page) " is beyond the last page" }
                            p { "There are " (dids.total_pages()) " pages of indexed DIDs. Return to the beginning to browse them." }
                            a class="prism-button prism-secondary" href=(urls::Explorer::new_uri(Some(PageQuery { page: Some(1) }))) { "Go to first page" }
                        }
                    }
            },
        );
    }
    resource(
        page,
        "received",
        html! {
                (pagination(&dids, "Top"))
                div class="prism-panel prism-indexed-dids" {
                    ul class="prism-did-rows" {
                        @for did in &dids.items {
                            @let value = did.to_string();
                            @let uri = urls::Resolver::new_uri(Some(DidQuery { did: Some(value.clone()) }));
                            li class="prism-did-row" data-refresh-key=(value) {
                                a class="prism-did-link" href=(uri) aria-label=(format!("Resolve {value}")) {
                                    span class="prism-did-full" { (value) }
                                    span class="prism-did-short" aria-hidden="true" {
                                        span class="prism-did-prefix" { (&value[..15]) }
                                        span class="prism-did-tail" { "…" (&value[value.len()-8..]) }
                                    }
                                    (components::icon("chevron-right"))
                                }
                                button type="button" class="prism-copy" data-copy-value=(value) aria-label=(format!("Copy {value}")) {
                                    (components::icon("copy"))
                                }
                            }
                        }
                    }
                    p class="prism-list-count" {
                        (dids.items.len()) " " (if dids.items.len() == 1 { "DID" } else { "DIDs" })
                        @if dids.total_pages() > 1 { " on this page" }
                    }
                }
                (pagination(&dids, "Bottom"))
        },
    )
}

fn resource(page: u32, state: &str, content: Markup) -> Markup {
    html! {
        section id="did-list" class="prism-explorer-list" aria-labelledby="indexed-dids-title"
            data-refresh-url=(urls::ExplorerDidList::new_uri(Some(PageQuery { page: Some(page) })))
            data-refresh-interval="5000" data-refresh-resource="list" data-refresh-state=(state) {
            header class="prism-refresh-feedback" {
                h2 id="indexed-dids-title" { "Indexed DIDs" }
                p role="status" data-refresh-status hidden[state != "failed"] {
                    @if state == "failed" { "Couldn’t load indexed DIDs" }
                }
                p class="prism-refresh-deferred" data-refresh-deferred hidden {}
                a class="prism-button prism-secondary prism-refresh-action" data-refresh-action
                    href=(urls::Explorer::new_uri(Some(PageQuery { page: Some(page) })))
                    aria-label=(if state == "failed" { "Retry indexed DIDs" } else { "Refresh indexed DIDs" }) {
                    span data-refresh-label {
                        @if state == "failed" { "Retry" }
                        @else { "Refresh" }
                    }
                    span class="prism-refresh-icon" aria-hidden="true" { (components::icon("refresh")) }
                }
            }
            div data-refresh-content { (content) }
            details id="list-delivery" class="prism-delivery-details" {
                summary aria-label="Indexed DIDs technical details" { "Technical details" }
                p class="prism-sync-receipt" data-refresh-receipt {
                    @if state == "failed" { "No indexed DID response received. " }
                    @else { "Indexed DID response supplied with this response. " }
                    "Browser checks every 5 seconds while visible when scripts are enabled."
                }
                p { "Newest first-indexed slot first · DID order breaks ties" }
                p { "Refresh keeps the selected page. Copy uses the complete DID." }
            }
        }
    }
}

pub fn invalid_page() -> Markup {
    html! {
        section id="did-list" class="prism-explorer-list" aria-label="Indexed identities" {
            div class="prism-panel prism-list-state" {
                h2 { "Choose a valid page" }
                p { "Use a whole page number starting at 1." }
                a class="prism-button prism-secondary" href=(urls::Explorer::new_uri(Some(PageQuery { page: Some(1) }))) { "Go to first page" }
            }
        }
    }
}

pub fn unavailable(page: u32) -> Markup {
    resource(
        page,
        "failed",
        html! {
                div class="prism-panel prism-list-state" {
                    h2 { "Couldn’t load indexed DIDs" }
                    p { "Indexed identities are temporarily unavailable. Try again or resolve a known DID." }
                    a class="prism-button prism-secondary" href=(urls::Explorer::new_uri(Some(PageQuery { page: Some(page) }))) { "Try again" }
                    a class="prism-button prism-secondary" href=(urls::Resolver::new_uri(None)) { "Resolve a known DID" }
                }
        },
    )
}

fn page_link(page: u32, current: u32, class: &str, label: &str) -> Markup {
    html! {
        a class=(class) href=(urls::Explorer::new_uri(Some(PageQuery { page: Some(page) })))
            aria-current=[(page == current).then_some("page")]
            aria-label=[matches!(label, "Previous" | "Next").then_some(label)] { (page_label(label)) }
    }
}

fn page_label(label: &str) -> Markup {
    html! {
        @if matches!(label, "Previous" | "Next") {
            span class="prism-page-direction-label" { (label) }
            span class="prism-page-direction-icon" aria-hidden="true" {
                (components::icon(if label == "Previous" { "chevron-left" } else { "chevron-right" }))
            }
        } @else { (label) }
    }
}

fn pagination(dids: &Paginated<CanonicalPrismDid>, position: &str) -> Markup {
    if dids.total_pages() <= 1 {
        return html! {};
    }
    let current = dids.current_page + 1;
    let total = dids.total_pages();
    // Bounded work even for a very large dataset. Mobile shows first/current/last.
    let mut pages = vec![1, total];
    pages.extend(current.saturating_sub(2).max(1)..=current.saturating_add(2).min(total));
    pages.sort_unstable();
    pages.dedup();
    let mut mobile_pages = vec![1, current, total];
    mobile_pages.sort_unstable();
    mobile_pages.dedup();
    html! {
        nav class="prism-pagination" aria-label=(format!("{position} DID pages")) {
            p { "Page " (current) " of " (total) " · " (dids.page_size) " DIDs per page" }
            div class="prism-page-controls" {
                div class="prism-page-numbers prism-page-desktop" {
                    @for (i, page) in pages.iter().enumerate() {
                        @if i > 0 && *page > pages[i - 1] + 1 {
                            span class="prism-page-gap" aria-hidden="true" { "…" }
                        }
                        (page_link(*page, current,
                            if *page == 1 || *page == total || *page == current { "prism-page-number" } else { "prism-page-number prism-page-neighbor" },
                            &page.to_string()))
                    }
                }
                div class="prism-page-numbers prism-page-mobile" {
                    @for (i, page) in mobile_pages.iter().enumerate() {
                        @if i > 0 && *page > mobile_pages[i - 1] + 1 {
                            span class="prism-page-gap" aria-hidden="true" { "…" }
                        }
                        (page_link(*page, current, "prism-page-number", &page.to_string()))
                    }
                }
                div class="prism-page-previous" {
                    @if current > 1 { (page_link(current - 1, current, "prism-page-direction", "Previous")) }
                    @else { span class="prism-page-direction" aria-disabled="true" aria-label="Previous" { (page_label("Previous")) } }
                }
                div class="prism-page-next" {
                    @if current < total { (page_link(current + 1, current, "prism-page-direction", "Next")) }
                    @else { span class="prism-page-direction" aria-disabled="true" aria-label="Next" { (page_label("Next")) } }
                }
            }
        }
    }
}
