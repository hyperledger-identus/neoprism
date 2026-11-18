//! Rendered Resolver seam. Fixtures are test-only; no production routes or samples.
use std::rc::Rc;

use identus_apollo::hash::Sha256Digest;
use identus_did_prism::did::operation::{StatusListData, StorageData};
use identus_did_prism::did::{DidState, PrismDid, StorageState};

use super::views;
use crate::http::test_support::{LONG_FORM, local_identity as fixture, section};

fn render(did: PrismDid, state: DidState) -> String {
    views::resolve(None, LONG_FORM, Ok((did, state)), vec![]).into_string()
}

fn entry(initial: u8, last: u8, data: StorageData) -> StorageState {
    StorageState {
        init_operation_hash: Rc::new(Sha256Digest::from_bytes(&[initial; 32]).unwrap()),
        last_operation_hash: Rc::new(Sha256Digest::from_bytes(&[last; 32]).unwrap()),
        data: Rc::new(data),
    }
}

#[test]
fn vdr_bytes_keep_complete_hashes_hex_data_and_captured_raw_in_a_native_disclosure() {
    let (did, mut state) = fixture();
    state.storage = vec![entry(0x12, 0xab, StorageData::Bytes(vec![0, 10, 128, 255]))];
    let body = render(did, state);
    let section = section(&body, "vdr-entries");
    for expected in [
        "Initial operation hash",
        "Last operation hash",
        "Bytes",
        "Bytes · hex",
        "data-copy-value=\"1212121212121212121212121212121212121212121212121212121212121212\"",
        "data-copy-value=\"abababababababababababababababababababababababababababababababab\"",
        "data-copy-value=\"000a80ff\"",
        "<details",
        "<summary",
        "Raw data · entry 1",
        "Captured Debug representation",
        "<pre",
        "Bytes([0, 10, 128, 255])",
        "data-copy-value=\"Bytes([0, 10, 128, 255])\"",
        "aria-label=\"Copy VDR entry 1 raw data\"",
    ] {
        assert!(section.contains(expected), "missing {expected}");
    }
}

#[test]
fn vdr_status_list_keeps_numeric_state_name_and_detail_without_interpretation() {
    let (did, mut state) = fixture();
    state.storage = vec![entry(
        3,
        4,
        StorageData::StatusList(StatusListData {
            state: -42,
            name: "Credential \"résumé\" <list>".into(),
            detail: "https://issuer.example/status?a=1&b=2\nsecond line".into(),
        }),
    )];
    let body = render(did, state);
    let section = section(&body, "vdr-entries");
    for expected in [
        "StatusList",
        "<dt>State</dt>",
        "<dt>Name</dt>",
        "<dt>Detail</dt>",
        "data-copy-value=\"-42\"",
        "data-copy-value=\"Credential &quot;résumé&quot; &lt;list&gt;\"",
        "data-copy-value=\"https://issuer.example/status?a=1&amp;b=2\nsecond line\"",
        "Numeric state is shown without interpretation.",
        "StatusList(StatusListData { state: -42, name:",
    ] {
        assert!(section.contains(expected), "missing {expected}");
    }
    for invented in ["Active", "Revoked", "Healthy", "Valid status"] {
        assert!(!section.contains(invented), "invented meaning: {invented}");
    }
}

#[test]
fn vdr_ipfs_is_complete_selectable_text_not_an_automatic_external_request() {
    let (did, mut state) = fixture();
    state.storage = vec![entry(
        5,
        6,
        StorageData::Ipfs("bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi".into()),
    )];
    let body = render(did, state);
    let section = section(&body, "vdr-entries");
    assert!(section.contains("<dt>IPFS value</dt>"));
    assert!(section.contains("data-copy-value=\"bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi\""));
    assert!(section.contains("Ipfs(&quot;bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi&quot;)"));
    assert!(!section.contains("href="));
    assert!(!section.contains("src="));
}

#[test]
fn vdr_empty_typed_values_are_explicit_but_copy_the_actual_empty_string() {
    let (did, mut state) = fixture();
    state.storage = vec![
        entry(
            3,
            3,
            StorageData::StatusList(StatusListData {
                state: i64::MIN,
                name: String::new(),
                detail: String::new(),
            }),
        ),
        entry(1, 1, StorageData::Bytes(vec![])),
        entry(2, 2, StorageData::Ipfs(String::new())),
    ];
    let body = render(did, state);
    let section = section(&body, "vdr-entries");
    assert_eq!(section.matches(">Empty</span>").count(), 4);
    assert_eq!(section.matches("data-copy-value=\"\"").count(), 4);
    assert!(section.contains("data-copy-value=\"-9223372036854775808\""));
    assert!(section.find("Bytes · hex").unwrap() < section.find("IPFS value").unwrap());
    assert!(section.find("IPFS value").unwrap() < section.find("StatusList").unwrap());
}

#[tokio::test]
async fn vdr_empty_is_delivered_by_the_real_get_route_without_fixture_endpoints() {
    use std::sync::Arc;

    use axum::http::StatusCode;
    use node_storage::SqliteDb;

    use crate::IndexerUiState;
    use crate::app::service::PrismDidService;
    use crate::http::tests::get;

    let db = SqliteDb::connect("sqlite::memory:").await.unwrap();
    db.migrate().await.unwrap();
    let app = super::router().with_state(IndexerUiState {
        prism_did_service: PrismDidService::new(Arc::new(db)),
        dlt_source: None,
    });
    let (status, body) = get(app.clone(), &format!("/resolver?did={LONG_FORM}")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("No VDR entries"));
    assert!(body.contains("aria-labelledby=\"vdr-title\""));
    assert!(body.contains("/assets/copy.js"));
    let (status, _) = get(app, "/vdr.html").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[test]
#[ignore = "exports test-only rendered fixtures for the VDR browser suite"]
fn export_vdr_browser_fixtures() {
    let dir = std::env::var("PRISM_VDR_FIXTURES").expect("set PRISM_VDR_FIXTURES outside the checkout");
    std::fs::create_dir_all(&dir).unwrap();
    for name in ["vdr", "vdr-long", "vdr-empty"] {
        let (did, mut state) = fixture();
        if name != "vdr-empty" {
            let long = name == "vdr-long";
            state.storage = vec![
                entry(
                    0x56,
                    0xef,
                    StorageData::Ipfs(if long {
                        format!(
                            "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi/{}",
                            "credential-data".repeat(128)
                        )
                    } else {
                        "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi".into()
                    }),
                ),
                entry(
                    0x34,
                    0xcd,
                    StorageData::StatusList(StatusListData {
                        state: -42,
                        name: "Credential \"résumé\" <list>".into(),
                        detail: if long {
                            format!("{}\nsecond line & <end>", "https://issuer.example/status/".repeat(128))
                        } else {
                            "https://issuer.example/status?a=1&b=2\nsecond line".into()
                        },
                    }),
                ),
                entry(
                    0x12,
                    0xab,
                    StorageData::Bytes(if long {
                        [0, 10, 128, 255].repeat(128)
                    } else {
                        vec![0, 10, 128, 255]
                    }),
                ),
            ];
        }
        std::fs::write(
            std::path::Path::new(&dir).join(format!("{name}.html")),
            render(did, state),
        )
        .unwrap();
    }
}

#[test]
fn vdr_empty_is_explicit_compact_and_has_no_empty_disclosures() {
    let (did, state) = fixture();
    let body = render(did, state);
    let section = section(&body, "vdr-entries");
    assert!(section.contains("No VDR entries"));
    assert!(!section.contains("<details"));
    assert!(!section.contains("data-copy-value"));
}
