use identus_did_prism::dlt::NetworkIdentifier;
use maud::{DOCTYPE, Markup, html};

use crate::VERSION;
use crate::http::urls;

pub fn page_layout(title: &str, network: Option<NetworkIdentifier>, body: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "NeoPRISM UI" }
                link rel="stylesheet" href=(urls::AssetStyleSheet::new_uri());
                script
                    src="https://unpkg.com/htmx.org@2.0.4"
                    integrity="sha384-HGfztofotfshcF7+8n44JQL2oJmowVChPTg48S+jvZoztPfvwD79OC/LTtG6dMp+"
                    crossorigin="anonymous"
                    {}
            }
            body class="bg-base-100 flex flex-col min-h-screen" {
                (navbar(title, network))
                div class="flex-grow" {
                    (body)
                }
                (footer())
            }
        }
    }
}

fn navbar(title: &str, network: Option<NetworkIdentifier>) -> Markup {
    html! {
        nav class="navbar bg-base-200" {
            div class="navbar-start" {
                div class="dropdown" {
                    label class="btn btn-ghost" tabindex="0" {
                        svg class="h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor" {
                            path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16" {
                            }
                        }
                        span { "Menu" }
                    }
                    ul class="menu menu-sm dropdown-content mt-3 z-[1] p-2 shadow bg-base-200 rounded-box w-36 border" tabindex="0" {
                        li { a class="btn btn-ghost" href=(urls::Resolver::new_uri(None)) { "Resolver" } }
                        li { a class="btn btn-ghost" href=(urls::Explorer::new_uri(None)) { "Explorer" } }
                        li { a class="btn btn-ghost" href=(urls::OpenApi::new_uri()) { "API Docs" } }
                    }
                }
            }
            div class="navbar-center" {
                p class="text-xl font-bold" { (title) }
            }
            div class="navbar-end" {
                div class="mr-4" {
                    @match network {
                        Some(network) => span class="text-sm text-success" { (network) },
                        None => span class="text-sm text-warning" { "disconnected" },
                    }

                    div class="text-right text-xs text-base-content/50" { (format!("(v{})", VERSION)) }
                }
            }
        }
    }
}

fn footer() -> Markup {
    html! {
        footer class="footer footer-center p-4 bg-base-200 text-base-content" {
            div {
                a
                    href="https://github.com/hyperledger-identus/neoprism"
                    target="_blank"
                    rel="noopener noreferrer"
                    class="link link-hover flex items-center gap-2"
                    aria-label="GitHub Repository"
                {
                    svg class="h-5 w-5 fill-current" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" {
                        path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" {}
                    }
                    span { "View on GitHub" }
                }
            }
        }
    }
}
