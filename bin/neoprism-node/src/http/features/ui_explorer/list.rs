use identus_did_prism::did::CanonicalPrismDid;
use identus_did_prism::utils::paging::Paginated;
use maud::{Markup, html};

use super::models::PageQuery;
use crate::http::features::ui_resolver::models::DidQuery;
use crate::http::{components, urls};

pub fn render(dids: Paginated<CanonicalPrismDid>) -> Markup {
    let page = dids.current_page + 1;
    let hx_url = urls::ExplorerDidList::new_uri(Some(PageQuery { page: Some(page) }));
    if dids.items.is_empty() {
        return html! {
            section id="did-list" class="prism-explorer-list"
                hx-get=(hx_url) hx-trigger="load delay:5s" hx-swap="outerHTML" aria-label="Indexed identities" {
                div class="prism-panel prism-list-state" {
                    @if dids.total_items == 0 {
                        h2 { "No indexed DIDs yet" }
                        p { "This node has no indexed identities to show. You can still resolve a known DID." }
                        a class="prism-button prism-secondary" href=(urls::Resolver::new_uri(None)) { "Resolve a known DID" }
                    } @else {
                        h2 { "Page " (page) " is beyond the last page" }
                        p { "There are " (dids.total_pages()) " pages of indexed DIDs. Return to the beginning to browse them." }
                        a class="prism-button prism-secondary" href=(urls::Explorer::new_uri(Some(PageQuery { page: Some(1) }))) { "Go to first page" }
                    }
                }
            }
        };
    }
    html! {
        section id="did-list" class="prism-explorer-list"
            hx-get=(hx_url) hx-trigger="load delay:5s" hx-swap="outerHTML" aria-label="Indexed identities" {
            (pagination(&dids, "Top"))
            div class="prism-panel prism-indexed-dids" {
                h2 { "Indexed DIDs" }
                p class="prism-list-order" { "Newest first-indexed slot first · DID order breaks ties" }
                ul class="prism-did-rows" {
                    @for did in &dids.items {
                        @let value = did.to_string();
                        @let uri = urls::Resolver::new_uri(Some(DidQuery { did: Some(value.clone()) }));
                        li class="prism-did-row" {
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
                p class="prism-list-count" { (dids.items.len()) " items on this page" }
            }
            (pagination(&dids, "Bottom"))
            p class="prism-list-note" { "DID list refreshes every 5 seconds without changing your page. Copy always uses the full DID." }
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
    html! {
        section id="did-list" class="prism-explorer-list" aria-label="Indexed identities" {
            div class="prism-panel prism-list-state" {
                h2 { "Couldn’t load indexed DIDs" }
                p { "Indexed identities are temporarily unavailable. Try again or resolve a known DID." }
                a class="prism-button prism-secondary" href=(urls::Explorer::new_uri(Some(PageQuery { page: Some(page) }))) { "Try again" }
                a class="prism-button prism-secondary" href=(urls::Resolver::new_uri(None)) { "Resolve a known DID" }
            }
        }
    }
}

fn page_link(page: u32, current: u32, class: &str, label: &str) -> Markup {
    html! {
        a class=(class) href=(urls::Explorer::new_uri(Some(PageQuery { page: Some(page) })))
            aria-current=[(page == current).then_some("page")] { (label) }
    }
}

fn pagination(dids: &Paginated<CanonicalPrismDid>, position: &str) -> Markup {
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
                    @else { span class="prism-page-direction" aria-disabled="true" { "Previous" } }
                }
                div class="prism-page-next" {
                    @if current < total { (page_link(current + 1, current, "prism-page-direction", "Next")) }
                    @else { span class="prism-page-direction" aria-disabled="true" { "Next" } }
                }
            }
        }
    }
}
