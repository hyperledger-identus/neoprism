use std::path::Path;
use std::sync::Arc;

use axum::Router;
use node_storage::SqliteDb;

use super::tests::get;
use crate::app::service::PrismDidService;
use crate::{IndexerUiState, RunMode};

// Deterministic storage-boundary fixture, never linked from production routes.
async fn fixture(count: u32, migrated: bool) -> Router {
    let db = SqliteDb::connect("sqlite::memory:").await.unwrap();
    if migrated {
        db.migrate().await.unwrap();
        for n in 1..=count {
            // Highest DID has newest first slot; all others share a slot to exercise DID tie-breaks.
            let slot = if n == count { 20 } else { 10 };
            let id = n.to_be_bytes().to_vec();
            sqlx::query("INSERT INTO raw_operation (id, signed_operation_data, slot, block_number, cbt, absn, osn, is_indexed, tx_hash, operation_id) VALUES (?, X'', ?, ?, '2025-01-01T00:00:00Z', 0, 0, 1, zeroblob(32), zeroblob(32))")
                .bind(&id).bind(slot).bind(n).execute(&db.pool).await.unwrap();
            let mut suffix = vec![0u8; 28];
            suffix.extend_from_slice(&n.to_be_bytes());
            sqlx::query("INSERT INTO indexed_ssi_operation (raw_operation_id, did, indexed_at) VALUES (?, ?, '2025-01-01T00:00:00Z')")
                .bind(&id).bind(suffix).execute(&db.pool).await.unwrap();
        }
    }
    super::router(
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/ui-assets")),
        RunMode::Indexer,
        8080,
        None,
    )
    .indexer_ui_router
    .with_state(IndexerUiState {
        prism_did_service: PrismDidService::new(Arc::new(db)),
        dlt_source: None,
    })
}

#[tokio::test]
async fn explorer_empty_and_beyond_last_page_offer_truthful_recovery() {
    let (_, empty) = get(fixture(0, true).await, "/explorer").await;
    assert!(empty.contains("No indexed DIDs yet"));
    assert!(empty.contains("Resolve a known DID"));
    assert!(!empty.contains("Page 1 of 0"));
    let (_, beyond) = get(fixture(103, true).await, "/explorer?page=3").await;
    assert!(beyond.contains("Page 3 is beyond the last page"));
    assert!(beyond.contains("Go to first page"));
    assert!(beyond.contains("hx-get=\"/explorer/did-list?page=3\""));
    assert!(!beyond.contains("No indexed DIDs yet"));
    let (status, far_beyond) = get(fixture(1, true).await, "/explorer?page=4294967295").await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert!(far_beyond.contains("Page 4294967295 is beyond the last page"));
}

#[tokio::test]
async fn explorer_retrieval_failure_is_generic_recoverable_and_not_empty() {
    for uri in ["/explorer?page=2", "/explorer/did-list?page=2"] {
        let (status, body) = get(fixture(0, false).await, uri).await;
        assert_eq!(status, axum::http::StatusCode::SERVICE_UNAVAILABLE);
        assert!(body.contains("Couldn’t load indexed DIDs"));
        assert!(body.contains("href=\"/explorer?page=2\""));
        assert!(body.contains("Try again"));
        assert!(body.contains("Resolve a known DID"));
        assert!(!body.contains("No indexed DIDs yet"));
        assert!(!body.contains("no such table"));
        assert!(!body.contains("did_stats"));
    }
}

#[tokio::test]
async fn explorer_invalid_queries_offer_first_page_without_internal_details() {
    let app = fixture(103, true).await;
    for page in ["nope", "-1", "4294967296", "1&page=2"] {
        let (status, body) = get(app.clone(), &format!("/explorer?page={page}")).await;
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
        assert!(body.contains("Choose a valid page"));
        assert!(body.contains("href=\"/explorer?page=1\""));
        assert!(body.contains("prism-shell"));
        assert!(!body.contains("Failed to deserialize"));
    }
}

#[tokio::test]
async fn explorer_final_partial_page_and_zero_normalization_preserve_get_and_refresh_query() {
    let app = fixture(103, true).await;
    let (_, body) = get(app.clone(), "/explorer?page=2").await;
    assert_eq!(body.matches("data-copy-value=").count(), 3);
    let expected = [100, 101, 102].map(|n| format!("data-copy-value=\"did:prism:{n:064x}\""));
    assert!(body.find(&expected[0]).unwrap() < body.find(&expected[1]).unwrap());
    assert!(body.find(&expected[1]).unwrap() < body.find(&expected[2]).unwrap());
    assert!(body.contains("hx-get=\"/explorer/did-list?page=2\""));
    assert!(body.contains("hx-trigger=\"load delay:5s\""));
    assert_eq!(body.matches("Page 2 of 2 · 100 DIDs per page").count(), 2);
    assert!(body.contains("href=\"/explorer?page=1\""));
    assert!(!body.contains("href=\"/explorer?page=3\""));
    let (_, zero) = get(app.clone(), "/explorer?page=0").await;
    assert_eq!(zero.matches("data-copy-value=").count(), 100);
    assert!(zero.contains("hx-get=\"/explorer/did-list?page=1\""));
    let (_, fragment) = get(app, "/explorer/did-list?page=2").await;
    assert_eq!(fragment.matches("data-copy-value=").count(), 3);
    assert!(!fragment.contains("prism-shell"));
    assert!(fragment.contains("hx-get=\"/explorer/did-list?page=2\""));
}

// Explicit opt-in export of the HTTP seam's synthetic responses for browser checks.
#[tokio::test]
#[ignore = "requires PRISM_FIXTURE_DIR, used by tools/ui-tests/prism_explorer.py"]
async fn export_explorer_browser_fixtures() {
    let dir = std::path::PathBuf::from(std::env::var("PRISM_FIXTURE_DIR").expect("set PRISM_FIXTURE_DIR"));
    std::fs::create_dir_all(&dir).unwrap();
    let populated = fixture(1103, true).await;
    for (name, app, uri) in [
        ("full", populated.clone(), "/explorer?page=3"),
        ("partial", populated.clone(), "/explorer?page=12"),
        ("beyond", populated, "/explorer?page=13"),
        ("empty", fixture(0, true).await, "/explorer"),
        ("failure", fixture(0, false).await, "/explorer?page=3"),
        ("invalid", fixture(0, true).await, "/explorer?page=wrong"),
    ] {
        let (_, body) = get(app, uri).await;
        std::fs::write(dir.join(format!("{name}.html")), body).unwrap();
    }
}

#[tokio::test]
async fn explorer_full_page_has_ordered_complete_independent_resolve_and_copy_actions() {
    let (_, body) = get(fixture(103, true).await, "/explorer?page=1").await;
    assert_eq!(body.matches("data-copy-value=").count(), 100);
    let first = format!("did:prism:{:064x}", 103);
    let second = format!("did:prism:{:064x}", 1);
    assert!(body.find(&first).unwrap() < body.find(&second).unwrap());
    assert!(body.contains(&format!("data-copy-value=\"{first}\"")));
    assert!(body.contains(&format!("href=\"/resolver?did=did%3Aprism%3A{:064x}\"", 103)));
    assert!(body.contains("</a><button type=\"button\""));
    assert_eq!(body.matches("Page 1 of 2 · 100 DIDs per page").count(), 2);
    assert!(body.contains("Newest first-indexed slot first · DID order breaks ties"));
    assert!(!body.contains("excerpt"));
}
