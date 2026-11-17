use std::path::Path;
use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use identus_did_prism::dlt::NetworkIdentifier;
use node_storage::SqliteDb;
use tower::ServiceExt;

use super::router;
use crate::app::service::PrismDidService;
use crate::{AppState, DltSourceState, IndexerUiState, RunMode, VERSION};

async fn app(mode: RunMode, network: Option<NetworkIdentifier>) -> Router {
    let routers = router(
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/ui-assets")),
        mode,
        8080,
        None,
    );
    let app = routers.app_router.with_state(AppState { run_mode: mode });
    if matches!(mode, RunMode::Submitter) {
        return app;
    }
    let db = SqliteDb::connect("sqlite::memory:").await.unwrap();
    db.migrate().await.unwrap();
    app.merge(routers.indexer_ui_router.with_state(IndexerUiState {
        prism_did_service: PrismDidService::new(Arc::new(db)),
        dlt_source: network.map(|network| DltSourceState {
            network,
            cursor_rx: tokio::sync::watch::channel(None).1,
        }),
    }))
}

pub(super) async fn get(app: Router, uri: &str) -> (StatusCode, String) {
    let response = app
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

#[tokio::test]
async fn prism_entry_offers_labelled_native_get_resolution_and_runtime_navigation() {
    let (status, body) = get(
        app(RunMode::Indexer, Some(NetworkIdentifier::Custom)).await,
        "/resolver",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    for expected in [
        "<main",
        "<h1",
        "Identity resolver",
        "Start with an identity",
        "method=\"GET\"",
        "action=\"/resolver\"",
        "name=\"did\"",
        "required",
        "<label for=\"did\"",
        "aria-describedby=\"did-help\"",
        "href=\"/resolver\"",
        "href=\"/explorer\"",
        "href=\"/openapi\"",
        "aria-current=\"page\"",
        "Configured network",
        "Custom",
        "https://github.com/hyperledger-identus/neoprism",
        "/assets/neoprism.svg",
        "<script defer src=\"/assets/copy.js\"></script>",
    ] {
        assert!(body.contains(expected), "missing {expected}");
    }
    assert!(body.contains(&format!("v{VERSION}")));
    assert!(!body.contains("Design mockup"));
    assert!(!body.contains("Sync lag"));
}

#[tokio::test]
async fn root_redirects_and_destination_availability_follow_node_mode() {
    for mode in [RunMode::Indexer, RunMode::Standalone, RunMode::Submitter] {
        let app = app(mode, None).await;
        let response = app
            .clone()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
        let submitter = matches!(mode, RunMode::Submitter);
        assert_eq!(
            response.headers()["location"],
            if submitter { "/openapi" } else { "/resolver" }
        );
        for destination in ["/resolver", "/explorer"] {
            assert_eq!(
                get(app.clone(), destination).await.0,
                if submitter {
                    StatusCode::NOT_FOUND
                } else {
                    StatusCode::OK
                }
            );
        }
        let (status, docs) = get(app, "/openapi").await;
        assert_eq!(status, StatusCode::OK);
        assert!(docs.contains("scalar"));
        assert!(!docs.contains("prism-shell"));
    }
}

#[tokio::test]
async fn configured_network_is_not_replaced_with_sample_telemetry() {
    for (network, label) in [
        (Some(NetworkIdentifier::Mainnet), "Mainnet"),
        (Some(NetworkIdentifier::Preprod), "Preprod"),
        (Some(NetworkIdentifier::Preview), "Preview"),
        (None, "Network not configured"),
    ] {
        let (_, body) = get(app(RunMode::Indexer, network).await, "/resolver").await;
        assert!(body.contains(label));
        assert!(!body.contains("Custom testnet"));
        assert!(!body.contains("Sync lag"));
    }
}

#[tokio::test]
async fn resolver_get_retains_escaped_input_and_hands_long_form_to_existing_resolver() {
    let app = app(RunMode::Standalone, None).await;
    let (_, invalid) = get(app.clone(), "/resolver?did=%22%3E%3Cscript%3E").await;
    assert!(invalid.contains("value=\"&quot;&gt;&lt;script&gt;\""));
    assert!(!invalid.contains("<script>"));
    assert!(invalid.contains("Check the DID format"));
    assert!(invalid.contains("aria-invalid=\"true\""));

    // Published documentation's deterministic long-form DID; never emitted as entry sample data.
    const LONG_FORM: &str = "did:prism:9b5118411248d9663b6ab15128fba8106511230ff654e7514cdcc4ce919bde9b:Cj8KPRI7CgdtYXN0ZXIwEAFKLgoJc2VjcDI1NmsxEiEDHpf-yhIns-LP3tLvA8icC5FJ1ZlBwbllPtIdNZ3q0jU";
    let (status, body) = get(app, &format!("/resolver?did={LONG_FORM}")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains(&format!("value=\"{LONG_FORM}\"")));
    assert!(body.contains("DID state"));
    assert!(body.contains("master0"));
    assert!(!body.contains("Resolution error"));
}

#[tokio::test]
async fn static_assets_are_served_locally_with_browser_content_types() {
    let app = app(RunMode::Indexer, None).await;
    for (path, content_type) in [
        ("/assets/neoprism.svg", "image/svg+xml"),
        ("/assets/prism.css", "text/css"),
        ("/assets/prism.js", "text/javascript"),
        ("/assets/resolver.js", "text/javascript"),
        ("/assets/copy.js", "text/javascript"),
        ("/assets/explorer.js", "text/javascript"),
        ("/assets/fonts/Poppins-Regular.ttf", "font/ttf"),
        ("/assets/fonts/Poppins-Medium.ttf", "font/ttf"),
        ("/assets/fonts/Poppins-SemiBold.ttf", "font/ttf"),
        ("/assets/fonts/JetBrainsMono-Regular.ttf", "font/ttf"),
        ("/assets/fonts/JetBrainsMono-Medium.ttf", "font/ttf"),
        ("/assets/licenses/Poppins-OFL.txt", "text/plain"),
        ("/assets/licenses/JetBrainsMono-OFL.txt", "text/plain"),
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        assert!(
            response.headers()["content-type"]
                .to_str()
                .unwrap()
                .starts_with(content_type),
            "{path}"
        );
        assert!(!to_bytes(response.into_body(), usize::MAX).await.unwrap().is_empty());
    }
}
