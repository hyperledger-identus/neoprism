use std::str::FromStr;

use identus_did_prism::did::operation::{OperationParameters, PublicKey, Service};
use identus_did_prism::did::{DidState, PrismDid};
use identus_did_prism::dlt::{BlockMetadata, OperationMetadata, TxId};
use identus_did_prism::prelude::SignedPrismOperation;
use identus_did_prism::proto::prism_ssi::public_key::Key_data;
use identus_did_prism::proto::prism_ssi::{self, KeyUsage};
use identus_did_prism::protocol::error::ProcessError;
use identus_did_prism::protocol::resolver::resolve_unpublished;

type History = Vec<(OperationMetadata, SignedPrismOperation, Option<ProcessError>)>;

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

// Deterministic fixture only: never offered as a production sample.
const LONG_FORM: &str = "did:prism:9b5118411248d9663b6ab15128fba8106511230ff654e7514cdcc4ce919bde9b:Cj8KPRI7CgdtYXN0ZXIwEAFKLgoJc2VjcDI1NmsxEiEDHpf-yhIns-LP3tLvA8icC5FJ1ZlBwbllPtIdNZ3q0jU";

fn local_identity() -> (PrismDid, DidState) {
    let did: PrismDid = LONG_FORM.parse().unwrap();
    let PrismDid::LongForm(long) = &did else { unreachable!() };
    let state = resolve_unpublished(long.operation().unwrap()).unwrap();
    (did, state)
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
    let services = body.split("id=\"services\"").nth(1).expect("services section");
    let services = services.split("</section>").next().unwrap();
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
        let section = body.split(&format!("id=\"{id}\"")).nth(1).expect("section");
        let section = section.split("</section>").next().unwrap();
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
    let contexts = body.split("id=\"contexts\"").nth(1).expect("contexts section");
    let contexts = contexts.split("</section>").next().unwrap();
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
        "1 public key ·",
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
