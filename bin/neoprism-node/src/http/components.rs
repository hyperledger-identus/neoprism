use identus_did_prism::dlt::NetworkIdentifier;
use maud::{DOCTYPE, Markup, html};

use crate::VERSION;
use crate::http::urls;

/// Shared Prism chrome for the indexer UI. Submitter-only mode does not mount these pages.
pub fn page_layout(title: &str, network: Option<NetworkIdentifier>, body: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" data-theme="light" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                meta name="color-scheme" content="light";
                title { (title) " · NeoPRISM" }
                link rel="stylesheet" href=(urls::AssetStyleSheet::new_uri());
                link rel="stylesheet" href="/assets/prism.css";
                script defer src="/assets/prism.js" {}
                script defer src="/assets/copy.js" {}
                script defer src="/assets/cursor.js" {}
                script
                    src="https://unpkg.com/htmx.org@2.0.4"
                    integrity="sha384-HGfztofotfshcF7+8n44JQL2oJmowVChPTg48S+jvZoztPfvwD79OC/LTtG6dMp+"
                    crossorigin="anonymous"
                    {}
            }
            body class="prism" {
                a class="skip-link" href="#main" { "Skip to content" }
                div class="prism-shell" {
                    aside class="prism-sidebar" aria-label="Application" {
                        (brand(true))
                        (navigation(title, "Primary"))
                        div class="prism-deployment" { (deployment(network)) }
                    }
                    header class="prism-mobile-header" {
                        (brand(false))
                        details id="mobile-menu" class="prism-mobile-menu" {
                            summary id="menu-toggle" aria-controls="mobile-destinations" {
                                (icon("menu"))
                                span class="sr-only" { "Menu" }
                            }
                            div id="mobile-destinations" class="prism-mobile-destinations" {
                                (navigation(title, "Mobile"))
                                (deployment(network))
                            }
                        }
                    }
                    main id="main" class="prism-workspace" tabindex="-1" {
                        (body)
                    }
                }
            }
        }
    }
}

fn brand(desktop: bool) -> Markup {
    html! {
        a class="prism-brand" href=(urls::Home::new_uri()) aria-label="NeoPRISM home" {
            img src="/assets/neoprism.svg" width="24" height="24" alt="";
            span {
                strong { "NeoPRISM" }
                @if desktop { small { "Hyperledger Identus" } }
            }
        }
    }
}

fn navigation(title: &str, label: &str) -> Markup {
    html! {
        nav aria-label=(label) class="prism-navigation" {
            @for (destination, href, glyph) in [
                ("Resolver", urls::Resolver::new_uri(None).to_string(), "resolver"),
                ("Explorer", urls::Explorer::new_uri(None).to_string(), "explorer"),
                ("API Docs", urls::OpenApi::new_uri().to_string(), "book"),
            ] {
                a href=(href) aria-current=[(title == destination).then_some("page")] {
                    (icon(glyph)) (destination)
                }
            }
        }
    }
}

fn deployment(network: Option<NetworkIdentifier>) -> Markup {
    html! {
        div class="prism-metadata" {
            @match network {
                Some(network) => {
                    p class="prism-network" {
                        @match network {
                            NetworkIdentifier::Mainnet => "Mainnet",
                            NetworkIdentifier::Preprod => "Preprod",
                            NetworkIdentifier::Preview => "Preview",
                            NetworkIdentifier::Custom => "Custom",
                        }
                    }
                    p { "Configured network" }
                }
                None => p { "Network not configured" },
            }
            p class="prism-version" { "v" (VERSION) }
        }
        a class="prism-github" href="https://github.com/hyperledger-identus/neoprism" target="_blank" rel="noopener noreferrer" {
            (icon("github")) "View on GitHub"
        }
    }
}

pub fn icon(name: &str) -> Markup {
    let path = match name {
        "menu" => "M4 5h16M4 12h16M4 19h16",
        "copy" => "M9 9h12v12H9V9ZM15 5V3H3v12h2",
        "chevron-right" => "m9 5 7 7-7 7",
        "resolver" => {
            "M8 3H5a2 2 0 0 0-2 2v3m13-5h3a2 2 0 0 1 2 2v3M3 16v3a2 2 0 0 0 2 2h3m8 0h3a2 2 0 0 0 2-2v-3M9 12h6"
        }
        "explorer" => "m16 8-3 5-5 3 3-5 5-3ZM22 12a10 10 0 1 1-20 0 10 10 0 0 1 20 0Z",
        "book" => "M12 5c-3-2-6-2-10-1v16c4-1 7-1 10 1 3-2 6-2 10-1V4c-4-1-7-1-10 1Zm0 0v16",
        "github" => {
            "M9 19c-4 1-4-2-6-2m12 5v-4c0-1-.3-2-1-2 4 0 7-2 7-6 0-2-1-3-2-4 0-1 0-3-.5-4-2 0-3 1-4 1a15 15 0 0 0-5 0c-1 0-2-1-4-1-.5 1-.5 3-.5 4-1 1-2 2-2 4 0 4 3 6 7 6-1 0-1 1-1 2v4"
        }
        _ => "m21 21-5-5M18 10a8 8 0 1 1-16 0 8 8 0 0 1 16 0Z",
    };
    html! {
        svg class="prism-icon" aria-hidden="true" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" {
            path d=(path) {}
        }
    }
}
