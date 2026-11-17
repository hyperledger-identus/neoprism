use std::path::Path;
use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use identus_did_prism::dlt::{DltCursor, NetworkIdentifier};
use node_storage::SqliteDb;
use tower::ServiceExt;

use crate::app::service::PrismDidService;
use crate::{DltSourceState, IndexerUiState, RunMode};

async fn cursor_response(cursor: Option<DltCursor>, uri: &str) -> String {
    let routers = crate::http::router(
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/ui-assets")),
        RunMode::Indexer,
        18779,
        None,
    );
    let db = SqliteDb::connect("sqlite::memory:").await.unwrap();
    db.migrate().await.unwrap();
    let app = routers.indexer_ui_router.with_state(IndexerUiState {
        prism_did_service: PrismDidService::new(Arc::new(db)),
        dlt_source: Some(DltSourceState {
            network: NetworkIdentifier::Custom,
            cursor_rx: tokio::sync::watch::channel(cursor).1,
        }),
    });
    let response = app
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

fn supplied_cursor() -> DltCursor {
    DltCursor {
        slot: 1042816,
        block_hash: vec![0xab; 32],
        cbt: Some("2026-11-17T14:51:00.123456789Z".parse().unwrap()),
        blockfrost_page: None,
    }
}

#[tokio::test]
async fn supplied_cursor_exposes_summary_and_exact_native_technical_evidence() {
    let body = cursor_response(Some(supplied_cursor()), "/explorer/dlt-cursor").await;
    for expected in [
        "Sync status",
        "Reported slot",
        "1,042,816",
        "Block time · UTC",
        "17 Nov 2026, 14:51:00 UTC",
        "Distance to chain tip unknown",
        "<details",
        "Technical details",
        "Reported slot · raw",
        "data-copy-value=\"1042816\"",
        "data-copy-value=\"abababababababababababababababababababababababababababababababab\"",
        "data-copy-value=\"2026-11-17T14:51:00.123456789Z\"",
        "aria-label=\"Copy block hash\"",
        "source-reported DLT cursor, not the last indexed PRISM operation",
        "Browser receipt time is separate from block time",
        "does not prove cursor advancement, upstream health or chain synchronization",
    ] {
        assert!(body.contains(expected), "missing {expected}");
    }
    assert!(!body.contains("min-w-screen"));
}

#[tokio::test]
async fn missing_timestamp_and_long_values_remain_exact_and_never_use_receipt_time() {
    let mut cursor = supplied_cursor();
    cursor.slot = u64::MAX;
    cursor.block_hash = vec![0xab; 512];
    cursor.cbt = None;
    let body = cursor_response(Some(cursor), "/explorer/dlt-cursor").await;
    assert_eq!(body.matches("Not provided").count(), 2);
    assert!(body.contains("18,446,744,073,709,551,615"));
    assert!(body.contains("data-copy-value=\"18446744073709551615\""));
    assert!(body.contains(&format!("data-copy-value=\"{}\"", "ab".repeat(512))));
    assert!(!body.contains("Copy block timestamp"));
    assert!(!body.contains("<time"));
}

// Deterministic fixtures come from real HTTP route responses, never production sample data.
#[tokio::test]
async fn export_cursor_browser_fixtures() {
    let Ok(directory) = std::env::var("PRISM_CURSOR_FIXTURES") else {
        return;
    };
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        Path::new(&directory).join("empty-list.fragment"),
        cursor_response(None, "/explorer/did-list?page=1").await,
    )
    .unwrap();
    let mut missing_timestamp = supplied_cursor();
    missing_timestamp.cbt = None;
    let mut long = missing_timestamp.clone();
    long.slot = u64::MAX;
    long.block_hash = vec![0xab; 512];
    for (name, cursor) in [
        ("supplied", Some(supplied_cursor())),
        ("waiting", None),
        ("missing-timestamp", Some(missing_timestamp)),
        ("long", Some(long)),
    ] {
        for (suffix, uri) in [("html", "/explorer"), ("fragment", "/explorer/dlt-cursor")] {
            std::fs::write(
                Path::new(&directory).join(format!("{name}.{suffix}")),
                cursor_response(cursor.clone(), uri).await,
            )
            .unwrap();
        }
    }
}

#[tokio::test]
async fn missing_cursor_waits_without_claiming_source_or_chain_health() {
    for uri in ["/explorer", "/explorer/dlt-cursor"] {
        let body = cursor_response(None, uri).await;
        for expected in [
            "Sync status",
            "Waiting for sync information",
            "Distance to chain tip unknown",
            "data-refresh-interval=\"2000\"",
        ] {
            assert!(body.contains(expected), "missing {expected}");
        }
        for absent in [
            "Sync not enabled",
            "Fully synced",
            "Technical details",
            "Current sync slot",
        ] {
            assert!(!body.contains(absent), "unexpected {absent}");
        }
    }
}
