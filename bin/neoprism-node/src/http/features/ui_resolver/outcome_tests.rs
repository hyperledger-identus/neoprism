//! Test-only rendering fixtures; never mounted by the application router.
use super::*;
use crate::app::service::error::{InvalidDid, ResolutionError};

#[test]
fn outcome_publication_and_deactivation_use_state_not_input_syntax_and_retain_real_history() {
    for (name, published, deactivated) in [
        ("local", false, false),
        ("published", true, false),
        ("deactivated", true, true),
    ] {
        let (did, mut state) = local_identity();
        state.is_published = published;
        if deactivated {
            state.public_keys.clear();
        }
        let body = views::resolve(
            None,
            LONG_FORM,
            Ok((did, state)),
            vec![history_entry(7, "actual-record")],
        )
        .into_string();
        assert!(body.contains(&format!("value=\"{LONG_FORM}\"")));
        match name {
            "local" => {
                assert!(body.contains("Locally derived"));
                assert!(body.contains("No ledger confirmation"));
                assert!(!body.contains("actual-record"));
                assert!(!body.contains("2025-03-07"));
            }
            "published" => {
                assert!(body.contains("Indexed state available"));
                assert!(!body.contains("Locally derived"));
                assert!(body.contains("actual-record"));
            }
            _ => {
                assert!(body.contains("Deactivated"));
                assert!(body.contains("No public keys"));
                assert!(body.contains("actual-record"));
            }
        }
    }
}

#[tokio::test]
async fn native_http_get_distinguishes_real_empty_storage_from_failed_retrieval() {
    use axum::http::StatusCode;
    use node_storage::SqliteDb;

    use crate::IndexerUiState;
    use crate::app::service::PrismDidService;
    use crate::http::features::ui_resolver::models::DidQuery;
    use crate::http::urls;

    let input = format!("did:prism:{}", "ab".repeat(32));
    let uri = urls::Resolver::new_uri(Some(DidQuery {
        did: Some(input.clone()),
    }))
    .to_string();
    for migrated in [false, true] {
        let db = SqliteDb::connect("sqlite::memory:").await.unwrap();
        if migrated {
            db.migrate().await.unwrap();
        }
        let app = super::super::router().with_state(IndexerUiState {
            prism_did_service: PrismDidService::new(Arc::new(db)),
            dlt_source: None,
        });
        let (status, body) = crate::http::tests::get(app, &uri).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains(&format!("value=\"{input}\"")));
        if migrated {
            assert!(body.contains("No indexed state on this node"));
            assert!(body.contains("No indexed operations"));
            assert!(!body.contains("Couldn’t retrieve identity state"));
        } else {
            assert!(body.contains("Couldn’t retrieve identity state"));
            assert!(body.contains("No operation history was retrieved"));
            assert!(!body.contains("No indexed operations"));
            assert!(!body.contains("no such table"));
            assert!(!body.contains("raw_operation"));
        }
    }
}

#[test]
#[ignore = "exports test-only Resolver outcome fixtures for the browser seam"]
fn export_outcome_browser_fixtures() {
    use identus_did_prism::did::LongFormPrismDid;
    use identus_did_prism::protocol::resolver::resolve_published;

    let dir = std::env::var("PRISM_OUTCOME_FIXTURES").expect("set PRISM_OUTCOME_FIXTURES outside the checkout");
    let dir = std::path::Path::new(&dir);
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("entry.html"), views::index(None).into_string()).unwrap();
    let mut inputs = serde_json::Map::new();
    let absent = format!("did:prism:{}", "ab".repeat(32));
    let invalid = format!("did:prism:<invalid>&\"{}", "retain-original-".repeat(100));
    let invalid_error = invalid.parse::<PrismDid>().unwrap_err();
    let (did, _) = local_identity();
    let PrismDid::LongForm(long) = did else { unreachable!() };
    let mut invalid_operation = long.operation().unwrap();
    let Some(identus_did_prism::proto::prism::prism_operation::Operation::CreateDid(create)) =
        invalid_operation.operation.as_mut()
    else {
        unreachable!()
    };
    create.did_data.as_mut().unwrap().public_keys.clear();
    let invalid_initial = LongFormPrismDid::from_operation(&invalid_operation)
        .unwrap()
        .to_string();
    let invalid_initial_error = resolve_unpublished(invalid_operation).unwrap_err();
    let (unusable_state, unusable_history) = resolve_published(vec![(
        history_entry(7, "rejected-original").0,
        history_entry(7, "rejected-original").1,
    )]);
    assert!(unusable_state.is_none());
    assert!(unusable_history.iter().all(|(_, _, e)| e.is_some()));
    for (name, input, result, history) in [
        (
            "invalid",
            invalid.clone(),
            Err(ResolutionError::InvalidDid {
                source: InvalidDid::InvalidPrismDid { source: invalid_error },
            }),
            vec![],
        ),
        (
            "initial",
            invalid_initial,
            Err(ResolutionError::InvalidDid {
                source: InvalidDid::ProcessStateFailed {
                    source: invalid_initial_error,
                },
            }),
            vec![],
        ),
        ("absent", absent, Err(ResolutionError::NotFound), vec![]),
        (
            "unusable",
            LONG_FORM.to_string(),
            Err(ResolutionError::NotFound),
            unusable_history,
        ),
        (
            "internal",
            LONG_FORM.to_string(),
            Err(ResolutionError::InternalError {
                source: anyhow::anyhow!("secret-password /private/db"),
            }),
            vec![],
        ),
    ] {
        inputs.insert(name.into(), input.clone().into());
        std::fs::write(
            dir.join(format!("{name}.html")),
            views::resolve(None, &input, result, history).into_string(),
        )
        .unwrap();
    }
    for (name, published, deactivated) in [
        ("local", false, false),
        ("published", true, false),
        ("deactivated", true, true),
    ] {
        let (did, mut state) = local_identity();
        state.is_published = published;
        if deactivated {
            state.public_keys.clear();
        }
        // Publication is a backend-state flag, never inferred from long-form syntax.
        let input = LONG_FORM.to_string();
        inputs.insert(name.into(), input.clone().into());
        let history = if published {
            vec![history_entry(7, "retained-original")]
        } else {
            vec![]
        };
        std::fs::write(
            dir.join(format!("{name}.html")),
            views::resolve(None, &input, Ok((did, state)), history).into_string(),
        )
        .unwrap();
    }
    std::fs::write(dir.join("inputs.json"), serde_json::to_string(&inputs).unwrap()).unwrap();
}
