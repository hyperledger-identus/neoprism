use identus_did_prism::did::CanonicalPrismDid;
use identus_did_prism::dlt::{DltCursor, NetworkIdentifier};
use identus_did_prism::utils::paging::Paginated;
use maud::{Markup, html};

use crate::http::features::ui_explorer::models::PageQuery;
use crate::http::features::ui_resolver::models::DidQuery;
use crate::http::{components, urls};

pub fn index(
    network: Option<NetworkIdentifier>,
    cursor: Option<DltCursor>,
    dids: Paginated<CanonicalPrismDid>,
) -> Markup {
    let body = html! {
        (dlt_cursor_card(cursor))
        (did_list(dids))
    };
    components::page_layout("Explorer", network, body)
}

pub fn did_list(dids: Paginated<CanonicalPrismDid>) -> Markup {
    let did_elems = dids.items.iter().map(|did| {
        let uri = urls::Resolver::new_uri(Some(DidQuery {
            did: Some(did.to_string()),
        }));
        html! {
            div  class="text-center text-sm w-full truncate" {
                a class="link link-hover font-mono" href=(uri) { (did.to_string()) }
            }
        }
    });
    let pagination_items = (0..dids.total_pages())
        .map(|i| {
            // Only first, last and N pages before and after the current page
            let should_display = |i: u32| i == 0 || i == (dids.total_pages() - 1) || i.abs_diff(dids.current_page) <= 2;
            (i, should_display(i), should_display(i + 1))
        })
        .filter_map(
            |(i, should_display, should_display_next)| match (should_display, should_display_next) {
                (true, _) => Some((i, false)),
                (false, true) => Some((i, true)),
                (false, false) => None,
            },
        )
        .map(|(i, is_3dots)| {
            let is_active = i == dids.current_page;
            let classes = if is_active {
                "join-item btn btn-active".to_string()
            } else {
                "join-item btn".to_string()
            };
            if is_3dots {
                html! {
                    a class=(classes) { "..." }
                }
            } else {
                let userfacing_page = i + 1;
                let goto_uri = urls::Explorer::new_uri(Some(PageQuery {
                    page: Some(userfacing_page),
                }));
                html! { a href=(goto_uri) class=(classes) { (userfacing_page) } }
            }
        });
    let pagination = html! {
        div class="join my-2" {
            @for i in pagination_items { (i) }
        }
    };
    let hx_url = urls::ExplorerDidList::new_uri(Some(PageQuery {
        page: Some(dids.current_page + 1),
    }));
    html! {
        div
            class="flex flex-col items-center w-full mx-2"
            id="did-list"
            hx-get=(hx_url)
            hx-trigger="load delay:5s"
            hx-swap="outerHTML"
        {
            (pagination)
            @for elem in did_elems { (elem) }
            (pagination)
        }
    }
}

pub use super::cursor::dlt_cursor_card;
