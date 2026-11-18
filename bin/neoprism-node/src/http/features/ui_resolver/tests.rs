use std::io::Write;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use identus_did_prism::did::PrismDid;
use identus_did_prism::did::operation::{OperationParameters, PublicKey, Service};
use identus_did_prism::dlt::{BlockMetadata, OperationMetadata, TxId};
use identus_did_prism::prelude::SignedPrismOperation;
use identus_did_prism::proto::prism_ssi::public_key::Key_data;
use identus_did_prism::proto::prism_ssi::{self, KeyUsage};
use identus_did_prism::protocol::error::ProcessError;

#[path = "outcome_tests.rs"]
mod outcome_tests;

type History = Vec<(OperationMetadata, SignedPrismOperation, Option<ProcessError>)>;

#[derive(Clone)]
struct CapturedLog(Arc<Mutex<Vec<u8>>>);

impl Write for CapturedLog {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn html_success_response_masks_internal_causes_and_logs_the_full_chain() {
    use axum::body::to_bytes;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    use crate::app::service::error::ResolutionError;

    let log = CapturedLog(Arc::new(Mutex::new(Vec::new())));
    let writer = log.clone();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_writer(move || writer.clone())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    let error = anyhow::anyhow!("postgres://secret-user:secret-password@private-db/path")
        .context("filesystem /private/storage/resolver.db failed");
    let response = views::resolve(
        None,
        "did:prism:input",
        Err(ResolutionError::InternalError { source: error }),
        vec![],
    )
    .into_response();
    assert_eq!(response.status(), StatusCode::OK);
    let body = String::from_utf8(to_bytes(response.into_body(), usize::MAX).await.unwrap().to_vec()).unwrap();
    assert!(body.contains("unexpected server error"));
    for secret in ["secret-password", "private-db", "/private/storage", "postgres://"] {
        assert!(!body.contains(secret), "internal cause leaked: {secret}");
    }
    let logged = String::from_utf8(log.0.lock().unwrap().clone()).unwrap();
    assert!(logged.contains("filesystem /private/storage/resolver.db failed"));
    assert!(logged.contains("postgres://secret-user:secret-password@private-db/path"));
}

#[test]
fn node_local_absence_preserves_input_and_offers_bookmarkable_recovery() {
    use crate::app::service::error::ResolutionError;

    let input = format!("did:prism:{}", "ab".repeat(32));
    let body = views::resolve(None, &input, Err(ResolutionError::NotFound), vec![]).into_string();
    for expected in [
        "No indexed state on this node",
        "No stored operations were found on this node",
        "does not prove that the DID does not exist on-chain",
        "Try again",
        "Correct DID",
        "Browse Explorer",
        "href=\"#did\"",
        "No indexed operations",
        &format!("value=\"{input}\""),
        &format!("href=\"/resolver?did=did%3Aprism%3A{}\"", "ab".repeat(32)),
    ] {
        assert!(body.contains(expected), "missing recovery: {expected}");
    }
    assert!(!body.contains("server interruption"));
}

#[test]
fn stored_operations_without_initial_state_are_not_an_infrastructure_failure() {
    use crate::app::service::error::ResolutionError;

    let mut entry = history_entry(8, "retained-rejection");
    entry.2 = Some(ProcessError::DidStateInitFromNonCreateOperation);
    let body = views::resolve(None, LONG_FORM, Err(ResolutionError::NotFound), vec![entry]).into_string();
    for expected in [
        "Stored operations cannot initialize DID state",
        "not a server interruption",
        "Inspect the rejected operations",
        "retained-rejection",
        "Rejected",
        "Safe domain diagnostics",
    ] {
        assert!(body.contains(expected), "missing unusable outcome: {expected}");
    }
    assert!(!body.contains("No stored operations were found"));
    assert!(!body.contains("Couldn’t retrieve identity state"));
}

#[test]
fn retrieval_failure_does_not_claim_empty_storage_or_fabricate_operations() {
    use crate::app::service::error::ResolutionError;

    let body = views::resolve(
        None,
        LONG_FORM,
        Err(ResolutionError::InternalError {
            source: anyhow::anyhow!("secret-password /private/db"),
        }),
        vec![],
    )
    .into_string();
    for expected in [
        "Couldn’t retrieve identity state",
        "could not retrieve stored operations",
        "No operation history was retrieved",
        "Retry resolution",
        "unexpected server error",
    ] {
        assert!(body.contains(expected), "missing retrieval recovery: {expected}");
    }
    for absent in [
        "secret-password",
        "/private/db",
        "No indexed operations",
        "No stored operations were found",
        "class=\"prism-operation\"",
    ] {
        assert!(!body.contains(absent), "incorrect retrieval claim: {absent}");
    }
}

#[test]
fn invalid_input_and_initial_state_have_safe_actionable_diagnostics_and_retained_values() {
    use crate::app::service::error::{InvalidDid, ResolutionError};

    let malformed = "  did:prism:<bad>&\"entire input  ";
    let cases = [
        (
            malformed,
            InvalidDid::InvalidPrismDid {
                source: malformed.parse::<PrismDid>().unwrap_err(),
            },
            "Check the DID format",
        ),
        (
            LONG_FORM,
            InvalidDid::ProcessStateFailed {
                source: ProcessError::DidStateInitFromNonCreateOperation,
            },
            "Check the encoded initial state",
        ),
    ];
    for (input, source, title) in cases {
        let body = views::resolve(None, input, Err(ResolutionError::InvalidDid { source }), vec![]).into_string();
        let escaped = maud::html! { (input) }.into_string();
        for expected in [
            title,
            "Your complete input is preserved",
            "Correct DID",
            "Browse Explorer",
            "Safe resolution diagnostics",
            "aria-invalid=\"true\"",
            &format!("value=\"{escaped}\""),
        ] {
            assert!(body.contains(expected), "missing invalid recovery: {expected}");
        }
        assert!(!body.contains("Couldn’t retrieve identity state"));
    }
}

fn history_entry(block: u64, signer: &str) -> (OperationMetadata, SignedPrismOperation, Option<ProcessError>) {
    let (did, _) = local_identity();
    let PrismDid::LongForm(long) = did else { unreachable!() };
    (
        OperationMetadata {
            block_metadata: BlockMetadata {
                block_number: block.into(),
                slot_number: (block * 12).into(),
                absn: 2,
                cbt: "2025-03-07T12:34:56Z".parse().unwrap(),
                tx_id: TxId::from_bytes(&[block as u8; 32]).unwrap(),
            },
            osn: 3,
        },
        SignedPrismOperation {
            signed_with: signer.into(),
            signature: vec![block as u8],
            operation: Some(long.operation().unwrap()).into(),
            ..Default::default()
        },
        None,
    )
}

#[test]
fn nested_size_diagnostics_keep_domain_limits_without_type_or_source_locations() {
    use identus_did_prism::did::operation::PublicKeyId;

    use crate::app::service::error::{InvalidDid, ResolutionError};

    let invalid_signer = || ProcessError::SignedPrismOperationInvalidSignedWith {
        source: PublicKeyId::parse("overlong-key", 3).unwrap_err(),
    };
    let mut rejected = history_entry(6, "overlong-key");
    rejected.2 = Some(invalid_signer());
    let body = views::resolve(
        None,
        "did:prism:input",
        Err(ResolutionError::InvalidDid {
            source: InvalidDid::ProcessStateFailed {
                source: invalid_signer(),
            },
        }),
        vec![rejected],
    )
    .into_string();
    for expected in ["public key id is too long", "input size has a limit of 3 but got 12"] {
        assert!(
            body.matches(expected).count() >= 2,
            "missing safe input/rejection detail: {expected}"
        );
    }
    for absent in ["for type", "identus_did_prism::", "ssi.rs", "[at ", "lib/did-prism/src"] {
        assert!(!body.contains(absent), "implementation detail leaked: {absent}");
    }
}

#[test]
fn operations_remain_inspectable_with_all_metadata_payloads_and_outcomes_when_final_state_is_unusable() {
    use crate::app::service::error::ResolutionError;

    let mut history = vec![
        history_entry(7, "master<&\"-signer"),
        history_entry(9, "rejected-signer"),
    ];
    history[1].2 = Some(ProcessError::DidStateInitFromNonCreateOperation);
    let payload = format!("{:?}", history[0].1);
    // Protobuf serialization caches sizes through interior mutability; derive the
    // expected ID on a clone so it does not change our captured input fixture.
    let operation_id = history[0].1.clone().operation_id().to_string();
    let body = views::resolve(None, "did:prism:input", Err(ResolutionError::NotFound), history).into_string();
    let operations = body
        .split("id=\"operation-history\"")
        .nth(1)
        .expect("operation history independent of final state");
    for expected in [
        "Operation history",
        "Signer",
        "master&lt;&amp;&quot;-signer",
        "Block time · UTC",
        "2025-03-07T12:34:56+00:00",
        "Slot",
        "Block",
        "Block sequence · absn",
        "Operation sequence · osn",
        "Transaction ID",
        "Operation ID",
        "data-copy-value=\"7\"",
        "data-copy-value=\"84\"",
        "data-copy-value=\"2\"",
        "data-copy-value=\"3\"",
        "0707070707070707070707070707070707070707070707070707070707070707",
        &operation_id,
        "Applied",
        "Rejected",
        "Processing outcome",
        "Safe domain diagnostics",
        "Complete signed payload · Debug",
        "did state initialization requires operation to be CreateOperation",
        "<details",
        "<summary",
    ] {
        assert!(operations.contains(expected), "missing operation detail: {expected}");
    }
    let escaped_payload = maud::html! { (&payload) }.into_string();
    assert!(operations.contains(&format!("data-copy-value=\"{escaped_payload}\"")));
    assert!(operations.contains(&format!("<code>{escaped_payload}</code>")));
    assert!(payload.contains("signature: [7]"));
    assert!(payload.contains("CompressedECKeyData"));
    assert!(!body.contains("Indexed state available"));
    assert!(operations.contains("<summary>Safe domain diagnostics</summary>"));
}

#[test]
fn safe_domain_diagnostics_keep_rejection_context_but_stop_at_crypto_and_protobuf_leaves() {
    use identus_did_prism::did::error::{CreateDidOperationError, DidSyntaxError, Error, PublicKeyError, ServiceError};
    use identus_did_prism::did::operation::{PublicKeyId, ServiceId};
    use identus_did_prism::proto::MessageExt;
    use identus_did_prism::protocol::error::DidStateConflictError;

    use crate::app::service::error::{InvalidDid, ResolutionError};

    let cases = [
        (
            ProcessError::DidOperationInvalid {
                source: Error::CreateDidOperation {
                    source: CreateDidOperationError::InvalidPublicKey {
                        source: PublicKeyError::InvalidKeyData {
                            id: PublicKeyId::parse("authentication-key", 50).unwrap(),
                            source: identus_apollo::crypto::Error::KeyParsing {
                                source: Box::new(std::io::Error::other("crypto-leaf-secret /private/implementation")),
                            },
                        },
                    },
                },
            },
            "failed to parse key data into public key for id authentication-key",
        ),
        (
            ProcessError::DidOperationInvalid {
                source: Error::CreateDidOperation {
                    source: CreateDidOperationError::InvalidService {
                        source: ServiceError::InvalidServiceId {
                            id: "overlong-service".into(),
                            source: ServiceId::parse("overlong-service", 4).unwrap_err(),
                        },
                    },
                },
            },
            "service id is too long: input size has a limit of 4 but got 16",
        ),
        (
            ProcessError::DidStateConflict {
                source: DidStateConflictError::AfterUpdatePublicKeyExceedLimit { limit: 50, actual: 51 },
            },
            "did state has 51 public keys which exceed limit 50",
        ),
    ];
    for (error, expected) in cases {
        let mut entry = history_entry(8, "master0");
        entry.2 = Some(error);
        let body = views::resolve(None, LONG_FORM, Err(ResolutionError::NotFound), vec![entry]).into_string();
        assert!(body.contains(expected), "missing safe domain detail: {expected}");
        for absent in [
            "crypto-leaf-secret",
            "/private/implementation",
            "for type",
            "[at ",
            "ssi.rs",
        ] {
            assert!(!body.contains(absent), "unsafe nested leaf: {absent}");
        }
    }
    let protobuf_error = identus_did_prism::prelude::PrismOperation::decode(&[0x80]).unwrap_err();
    let raw_leaf = protobuf_error.to_string();
    let body = views::resolve(
        None,
        "submitted-input",
        Err(ResolutionError::InvalidDid {
            source: InvalidDid::InvalidPrismDid {
                source: Error::InvalidDidSyntax {
                    source: DidSyntaxError::DidEncodedStateInvalidProto {
                        source: protobuf_error,
                        did: "submitted-input".into(),
                    },
                },
            },
        }),
        vec![],
    )
    .into_string();
    assert!(body.contains("failed to decode did suffix submitted-input into protobuf message"));
    assert!(!body.contains(&raw_leaf));
}

#[test]
fn operation_summaries_identify_each_proto_kind_without_opening_payloads() {
    use identus_did_prism::proto::prism::prism_operation::Operation;

    let cases = [
        (Some(Operation::CreateDid(Default::default())), "Create DID"),
        (Some(Operation::UpdateDid(Default::default())), "Update DID"),
        (Some(Operation::DeactivateDid(Default::default())), "Deactivate DID"),
        (
            Some(Operation::ProtocolVersionUpdate(Default::default())),
            "Update protocol version",
        ),
        (
            Some(Operation::CreateStorageEntry(Default::default())),
            "Create VDR entry",
        ),
        (
            Some(Operation::UpdateStorageEntry(Default::default())),
            "Update VDR entry",
        ),
        (
            Some(Operation::DeactivateStorageEntry(Default::default())),
            "Deactivate VDR entry",
        ),
        (None, "Unknown operation"),
    ];
    for (operation, label) in cases {
        let mut entry = history_entry(7, "master-0");
        entry.1.operation.as_mut().unwrap().operation = operation;
        let body = super::operations::history(&[entry]).into_string();
        let summary = body.split("</summary>").next().unwrap();
        assert!(summary.contains(label), "missing operation kind: {label}");
        assert!(summary.contains("Operation 1"));
        assert!(summary.contains("07 Mar 2025, 12:34 UTC"));
        assert!(summary.contains("datetime=\"2025-03-07T12:34:56+00:00\""));
        assert!(summary.contains("Applied"));
        assert!(body.contains("href=\"#operation-history\""));
    }
}

#[test]
fn identity_counts_link_to_focusable_sections_including_empty_state() {
    let body = views::resolve(None, LONG_FORM, Ok(local_identity()), vec![]).into_string();
    assert!(body.contains("aria-label=\"Identity sections\""));
    for id in [
        "public-keys",
        "services",
        "contexts",
        "vdr-entries",
        "operation-history",
    ] {
        assert!(body.contains(&format!("href=\"#{id}\"")), "missing section link: {id}");
        assert!(
            body.contains(&format!("id=\"{id}\" tabindex=\"-1\"")),
            "missing focus target: {id}"
        );
    }
    assert!(body.contains("id=\"identity-summary\" tabindex=\"-1\""));
    assert!(body.contains("href=\"#identity-summary\""));
}

fn evidence_history() -> History {
    let mut rejected = history_entry(99, "rejected-signer");
    rejected.2 = Some(ProcessError::SignedPrismOperationInvalidSignature);
    vec![
        history_entry(7, "selected-signer"),
        rejected,
        history_entry(3, "older-signer"),
    ]
}

use super::views;
use crate::http::test_support::{LONG_FORM, local_identity, section};

#[test]
#[ignore = "exports test-only operation fixtures for the browser seam"]
fn export_operation_browser_fixtures() {
    use identus_did_prism::protocol::resolver::resolve_published;

    use crate::app::service::error::ResolutionError;

    let dir = std::env::var("PRISM_OPERATION_FIXTURES").expect("set PRISM_OPERATION_FIXTURES outside the checkout");
    let dir = std::path::Path::new(&dir);
    std::fs::create_dir_all(dir).unwrap();
    let long_signer = format!("signer<>&\"'\\\n{}", "long-original-".repeat(300));
    let mut long = history_entry(99, &long_signer);
    long.2 = Some(ProcessError::SignedPrismOperationInvalidSignature);
    let history = vec![history_entry(7, "master<&\"-signer"), long];
    for (index, (_, signed, _)) in history.iter().enumerate() {
        std::fs::write(dir.join(format!("payload-{}.txt", index + 1)), format!("{signed:?}")).unwrap();
    }
    // Unusable final state is produced by the real protocol, not an invented outcome.
    let (state, rejected) = resolve_published(history.iter().map(|(m, s, _)| (m.clone(), s.clone())).collect());
    assert!(state.is_none());
    assert_eq!(rejected.len(), 2);
    assert!(rejected.iter().all(|(_, _, e)| e.is_some()));
    std::fs::write(
        dir.join("unusable.html"),
        views::resolve(None, LONG_FORM, Err(ResolutionError::NotFound), rejected).into_string(),
    )
    .unwrap();
    // Published view is separate: these captured outcomes test the rendering contract.
    let (did, mut state) = local_identity();
    state.is_published = true;
    std::fs::write(
        dir.join("operations.html"),
        views::resolve(None, LONG_FORM, Ok((did, state)), history).into_string(),
    )
    .unwrap();
    std::fs::write(
        dir.join("empty.html"),
        views::resolve(None, LONG_FORM, Err(ResolutionError::NotFound), vec![]).into_string(),
    )
    .unwrap();
    std::fs::write(
        dir.join("internal.html"),
        views::resolve(
            None,
            LONG_FORM,
            Err(ResolutionError::InternalError {
                source: anyhow::anyhow!("secret-password /private/db"),
            }),
            vec![],
        )
        .into_string(),
    )
    .unwrap();
}

fn key(id: &str, usage: KeyUsage, curve: &str, data: Vec<u8>) -> PublicKey {
    PublicKey::parse(
        &prism_ssi::PublicKey {
            id: id.into(),
            usage: usage.into(),
            key_data: Some(Key_data::CompressedEcKeyData(prism_ssi::CompressedECKeyData {
                curve: curve.into(),
                data,
                ..Default::default()
            })),
            ..Default::default()
        },
        &OperationParameters::v1(),
    )
    .unwrap()
}

fn curve_keys() -> Vec<PublicKey> {
    let mut ed = vec![0x66; 32];
    ed[0] = 0x58;
    let mut x = vec![0; 32];
    x[0] = 9;
    let secp =
        identus_apollo::hex::HexStr::from_str("0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798")
            .unwrap()
            .to_bytes();
    vec![
        key(
            "z-agreement-01234567890123456789012345678901234567",
            KeyUsage::KEY_AGREEMENT_KEY,
            "X25519",
            x,
        ),
        key("a-master", KeyUsage::MASTER_KEY, "secp256k1", secp),
        key("m-authentication", KeyUsage::AUTHENTICATION_KEY, "Ed25519", ed),
    ]
}

fn detail_services() -> Vec<Service> {
    [
        ("messaging", "DIDCommMessaging", "https://example.test/didcomm?x=1&y=2"),
        ("profile", r#"["LinkedDomains","Profile"]"#, r#"{ "z": {"accept": ["application/json",null,7,true]}, "a": "<profile>" }"#),
        ("mirrors", r#"["LinkedDomains"]"#, r#"[ "https://example.test/one", { "uri": "https://example.test/two", "nested": {"empty": [], "object": {}} } ]"#),
    ]
    .into_iter()
    .map(|(id, ty, endpoint)| Service::parse(&prism_ssi::Service {
        id: id.into(), type_: ty.into(), service_endpoint: endpoint.into(), ..Default::default()
    }, &OperationParameters::v1()).unwrap())
    .collect()
}

#[test]
fn services_expose_complete_types_and_uri_object_array_shapes_inline() {
    let (did, mut state) = local_identity();
    state.services = detail_services();
    let body = views::resolve(None, LONG_FORM, Ok((did, state)), vec![]).into_string();
    let services = section(&body, "services");
    for expected in [
        "Services · 3",
        "<details",
        "<summary",
        "Service messaging",
        "Service profile",
        "Service mirrors",
        "Single type",
        "Multiple types · 2",
        "Multiple types · 1",
        "DIDCommMessaging",
        "LinkedDomains",
        "Profile",
        "URI endpoint",
        "Object endpoint",
        "Array endpoints · 2",
        "application/json",
        "null",
        "true",
        "7",
        "&lt;profile&gt;",
        "https://example.test/one",
        "https://example.test/two",
        "empty",
        "[]",
        "{}",
        "Readable service JSON",
        "serviceEndpoint",
    ] {
        assert!(services.contains(expected), "missing {expected}");
    }
    assert!(services.find("Service messaging").unwrap() < services.find("Service profile").unwrap());
    assert!(services.find("Service profile").unwrap() < services.find("Service mirrors").unwrap());
}

#[test]
fn service_captured_fields_keep_spacing_order_and_exact_raw_typed_representation() {
    let (did, mut state) = local_identity();
    state.services = detail_services();
    let body = views::resolve(None, LONG_FORM, Ok((did, state)), vec![]).into_string();
    for expected in [
        "Captured service representation",
        "Captured Type",
        "Captured Endpoint",
        "Raw typed representation",
        "Authored wire formatting is not available",
        "Copy service profile captured endpoint",
        r#"data-copy-value="{ &quot;z&quot;: {&quot;accept&quot;: [&quot;application/json&quot;,null,7,true]}, &quot;a&quot;: &quot;&lt;profile&gt;&quot; }""#,
        r#"data-copy-value="[ &quot;https://example.test/one&quot;, { &quot;uri&quot;: &quot;https://example.test/two&quot;, &quot;nested&quot;: {&quot;empty&quot;: [], &quot;object&quot;: {}} } ]""#,
        "Copy service messaging raw typed representation",
        "ServiceId(&quot;messaging&quot;)",
        "ServiceTypeValue(&quot;DIDCommMessaging&quot;)",
    ] {
        assert!(body.contains(expected), "missing {expected}");
    }
    assert!(!body.contains("Original authored JSON"));
}

#[test]
fn empty_contexts_and_services_are_explicit_without_empty_disclosures() {
    let body = views::resolve(None, LONG_FORM, Ok(local_identity()), vec![]).into_string();
    for (id, message) in [("contexts", "No contexts"), ("services", "No services")] {
        let section = section(&body, id);
        assert!(section.contains(message));
        assert!(!section.contains("<details"));
        assert!(!section.contains("data-copy-value"));
    }
}

#[test]
fn contexts_keep_supplied_order_exact_copy_values_and_native_disclosure() {
    let (did, mut state) = local_identity();
    state.context = vec![
        "https://example.test/z?first=1&second=2".into(),
        "https://example.test/a".into(),
    ];
    let body = views::resolve(None, LONG_FORM, Ok((did, state)), vec![]).into_string();
    let contexts = section(&body, "contexts");
    assert!(contexts.contains("<details"));
    assert!(contexts.contains("<summary"));
    assert!(contexts.contains("Contexts · 2"));
    assert!(contexts.contains("<ol"));
    assert!(contexts.find("https://example.test/z").unwrap() < contexts.find("https://example.test/a").unwrap());
    assert!(contexts.contains("data-copy-value=\"https://example.test/z?first=1&amp;second=2\""));
    assert!(contexts.contains("aria-label=\"Copy context 1\""));
}

#[test]
fn key_disclosures_preserve_sorted_ids_usage_and_complete_curve_coordinates() {
    let (did, mut state) = local_identity();
    state.public_keys = curve_keys();
    let body = views::resolve(None, LONG_FORM, Ok((did, state)), vec![]).into_string();
    let keys = body.split("id=\"public-keys\"").nth(1).expect("key section");
    assert!(keys.find("a-master").unwrap() < keys.find("m-authentication").unwrap());
    assert!(keys.find("m-authentication").unwrap() < keys.find("z-agreement").unwrap());
    for expected in [
        "<details",
        "<summary",
        "MasterKey",
        "AuthenticationKey",
        "KeyAgreementKey",
        "secp256k1",
        "Ed25519",
        "X25519",
        "Not applicable",
        "data-copy-value=\"eb5mfvncu6xVoGKVzocLBwKb_NstzijZWfKBWxb4F5g\"",
        "data-copy-value=\"SDradyajxGVdpPv8DhEIqP0XtEimhVQZnEfQj_sQ1Lg\"",
        "data-copy-value=\"WGZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmY\"",
        "data-copy-value=\"CQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\"",
        "JWK",
        "kty",
        "OKP",
        "EC",
    ] {
        assert!(keys.contains(expected), "missing {expected}");
    }
}

#[test]
fn evidence_uses_one_latest_applied_operation_not_rejected_history_or_chain_tip() {
    let (did, mut state) = local_identity();
    state.is_published = true;
    let selected_id = evidence_history()[0].1.operation_id().to_string();
    let body = views::resolve(None, LONG_FORM, Ok((did, state)), evidence_history()).into_string();
    let evidence = body.split("id=\"ledger-evidence\"").nth(1).expect("evidence panel");
    let evidence = evidence.split("</details>").next().unwrap();
    for expected in [
        "Latest applied operation",
        "not the chain tip",
        "Block",
        "Slot",
        "Signer",
        "selected-signer",
        "Block time · UTC",
        "2025-03-07T12:34:56+00:00",
        "data-copy-value=\"7\"",
        "data-copy-value=\"84\"",
        "0707070707070707070707070707070707070707070707070707070707070707",
        &selected_id,
    ] {
        assert!(evidence.contains(expected), "missing {expected}");
    }
    assert!(!evidence.contains("rejected-signer"));
    assert!(!evidence.contains("older-signer"));
    assert!(!evidence.contains("6363636363636363"));
    assert!(body.contains("3 operations"));
}

#[test]
#[ignore = "exports test-only rendered fixtures for the browser suite"]
fn export_identity_browser_fixtures() {
    let dir = std::env::var("PRISM_FIXTURE_DIR").expect("set PRISM_FIXTURE_DIR outside the checkout");
    std::fs::create_dir_all(&dir).unwrap();
    // Valid captured JSON whitespace includes both CRLF and bare CR; do not normalize it.
    let (did, mut state) = local_identity();
    state.services = vec![
        Service::parse(
            &prism_ssi::Service {
                id: "lossless".into(),
                type_: "Profile".into(),
                service_endpoint: "{\r\n\"uri\":\"https://example.test/雪?x=1&y=<tag>\",\r\"label\":\"café 🧪\"\r\n}"
                    .into(),
                ..Default::default()
            },
            &OperationParameters::v1(),
        )
        .unwrap(),
    ];
    std::fs::write(
        std::path::Path::new(&dir).join("lossless.html"),
        views::resolve(None, LONG_FORM, Ok((did, state)), vec![]).into_string(),
    )
    .unwrap();
    for (name, published, empty) in [
        ("indexed", true, false),
        ("local", false, false),
        ("deactivated", true, true),
    ] {
        let (did, mut state) = local_identity();
        state.is_published = published;
        state.public_keys = if empty { vec![] } else { curve_keys() };
        if !empty {
            state.context = vec![
                "https://example.test/z?first=1&second=2".into(),
                format!("https://example.test/a/{}", "long-context-".repeat(80)),
            ];
            state.services = detail_services();
            state.services.push(
                Service::parse(
                    &prism_ssi::Service {
                        id: "long-service-identifier-01234567890123456789012345".into(),
                        type_: "LinkedDomains".into(),
                        service_endpoint: format!("https://example.test/{}?x=1&y=2", "long-path-".repeat(25)),
                        ..Default::default()
                    },
                    &OperationParameters::v1(),
                )
                .unwrap(),
            );
            let raw = state
                .services
                .iter()
                .map(|service| format!("{service:?}"))
                .collect::<Vec<_>>();
            std::fs::write(
                std::path::Path::new(&dir).join("services-raw.json"),
                serde_json::to_string(&raw).unwrap(),
            )
            .unwrap();
        }
        let history = if published { evidence_history() } else { vec![] };
        let body = views::resolve(None, LONG_FORM, Ok((did, state)), history).into_string();
        std::fs::write(std::path::Path::new(&dir).join(format!("{name}.html")), body).unwrap();
    }
}

#[test]
fn unpublished_backend_state_suppresses_synthetic_metadata_even_with_canonical_input() {
    let (_, state) = local_identity();
    let did: PrismDid = state.did.clone().into();
    let mut synthetic = history_entry(0, "synthetic-local-signer");
    synthetic.0.block_metadata.cbt = "1970-01-01T00:00:00Z".parse().unwrap();
    let body = views::resolve(None, &did.to_string(), Ok((did.clone(), state)), vec![synthetic]).into_string();
    for absent in [
        "1970-01-01",
        "synthetic-local-signer",
        "Latest applied operation",
        "0000000000000000",
    ] {
        assert!(!body.contains(absent), "unexpected synthetic evidence: {absent}");
    }
    assert!(body.contains("Locally derived"));
    assert!(body.contains("0 operations"));
    assert!(body.contains("No indexed operations"));
}

#[test]
fn indexed_history_without_applied_operations_does_not_invent_ledger_evidence() {
    let (did, mut state) = local_identity();
    state.is_published = true;
    let mut rejected = history_entry(5, "rejected-only");
    rejected.2 = Some(ProcessError::SignedPrismOperationInvalidSignature);
    let body = views::resolve(None, LONG_FORM, Ok((did, state)), vec![rejected]).into_string();
    let evidence = body
        .split("id=\"ledger-evidence\"")
        .nth(1)
        .unwrap()
        .split("</details>")
        .next()
        .unwrap();
    assert!(evidence.contains("No applied ledger evidence available"));
    assert!(!evidence.contains("rejected-only"));
    assert!(!evidence.contains("data-copy-value"));
    assert!(body.contains("1 operation"));
}

#[test]
fn key_usage_disclosures_keep_every_exact_backend_enum() {
    let (did, mut state) = local_identity();
    let secp = curve_keys()[1].orig.clone();
    let cases = [
        (KeyUsage::MASTER_KEY, "MasterKey"),
        (KeyUsage::ISSUING_KEY, "IssuingKey"),
        (KeyUsage::KEY_AGREEMENT_KEY, "KeyAgreementKey"),
        (KeyUsage::AUTHENTICATION_KEY, "AuthenticationKey"),
        (KeyUsage::REVOCATION_KEY, "RevocationKey"),
        (KeyUsage::CAPABILITY_INVOCATION_KEY, "CapabilityInvocationKey"),
        (KeyUsage::CAPABILITY_DELEGATION_KEY, "CapabilityDelegationKey"),
        (KeyUsage::VDR_KEY, "VdrKey"),
    ];
    state.public_keys = cases
        .iter()
        .enumerate()
        .map(|(index, (usage, _))| {
            let mut original = secp.clone();
            original.id = format!("key-{index}");
            original.usage = (*usage).into();
            PublicKey::parse(&original, &OperationParameters::v1()).unwrap()
        })
        .collect();
    let body = views::resolve(None, LONG_FORM, Ok((did, state)), vec![]).into_string();
    for (_, exact) in cases {
        assert!(
            body.contains(&format!("data-copy-value=\"{exact}\"")),
            "missing exact usage {exact}"
        );
    }
}

#[test]
fn indexed_summary_uses_backend_publication_even_for_long_form_input() {
    let (did, mut state) = local_identity();
    state.is_published = true;
    state.context = vec![
        "https://example.test/context-a".into(),
        "https://example.test/context-b".into(),
    ];
    let body = views::resolve(None, LONG_FORM, Ok((did, state)), vec![]).into_string();
    assert!(body.contains("Indexed state available"));
    assert!(body.contains("2 contexts"));
    assert!(!body.contains("Locally derived"));
    assert!(!body.contains("No ledger confirmation"));
}

#[test]
fn deactivated_backend_state_explicitly_retains_empty_keys_and_history() {
    let (did, mut state) = local_identity();
    state.is_published = true;
    state.public_keys.clear();
    let body = views::resolve(None, LONG_FORM, Ok((did, state)), vec![]).into_string();
    for expected in [
        "Deactivated",
        "0 public keys",
        "No public keys",
        "No indexed operations",
    ] {
        assert!(body.contains(expected), "missing {expected}");
    }
    assert!(!body.contains("Indexed state available"));
}

#[test]
fn locally_derived_identity_has_real_counts_full_copy_value_and_separate_api_action() {
    let body = views::resolve(None, LONG_FORM, Ok(local_identity()), vec![]).into_string();
    for expected in [
        "Locally derived",
        "No ledger confirmation",
        "1 public key",
        "0 services",
        "0 contexts",
        "0 VDR entries",
        "0 operations",
        "No indexed operations",
        "Open Resolver API",
        "target=\"_blank\"",
        "rel=\"noopener noreferrer\"",
        "not a DID document response",
        "data-copy-value=\"did:prism:",
    ] {
        assert!(body.contains(expected), "missing {expected}");
    }
    assert!(body.contains(&format!("data-copy-value=\"{LONG_FORM}\"")));
    for absent in [
        "Indexed state available",
        "1970-01-01",
        "Not requested",
        "Retry API",
        "API request failed",
    ] {
        assert!(!body.contains(absent), "unexpected {absent}");
    }
}
