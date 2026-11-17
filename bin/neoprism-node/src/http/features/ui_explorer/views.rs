use identus_did_prism::dlt::{DltCursor, NetworkIdentifier};
use maud::{Markup, html};

use crate::http::components;

pub fn index(network: Option<NetworkIdentifier>, cursor: Option<DltCursor>, list: Markup) -> Markup {
    let body = html! {
        header class="prism-page-header" {
            h1 { "Identity explorer" }
            p { "Browse identities known to this node." }
        }
        (dlt_cursor_card(cursor))
        (list)
    };
    components::page_layout("Explorer", network, body)
}

pub use super::cursor::dlt_cursor_card;
