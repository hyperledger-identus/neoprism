use std::rc::Rc;
use std::str::FromStr;

use chrono::{DateTime, TimeZone, Utc};
use identus_apollo::crypto::secp256k1::Secp256k1PrivateKey;
use identus_apollo::hash::Sha256Digest;
use identus_apollo::hex::HexStr;
use identus_did_core::{
    Did, ServiceEndpoint as CoreServiceEndpoint, ServiceType as CoreServiceType, StringOrMap, VerificationMethodOrRef,
};
use identus_did_prism::did::operation::{
    KeyUsage, NonOperationPublicKey, PublicKey, PublicKeyData, PublicKeyId, Service, ServiceEndpoint,
    ServiceEndpointValue, ServiceId, ServiceType, ServiceTypeValue,
};
use identus_did_prism::did::{CanonicalPrismDid, DidState, LongFormPrismDid, PrismDid, PrismDidOps, StorageState};
use identus_did_prism::proto;

mod test_utils;

// ---------- helpers to construct test fixtures ----------

fn make_suffix() -> Sha256Digest {
    Sha256Digest::from_bytes(&[0u8; 32]).unwrap()
}

fn make_canonical_did() -> CanonicalPrismDid {
    CanonicalPrismDid { suffix: make_suffix() }
}

fn make_did() -> Did {
    make_canonical_did().to_did()
}

fn make_operation_hash() -> Rc<Sha256Digest> {
    Rc::new(Sha256Digest::from_bytes(&[1u8; 32]).unwrap())
}

fn make_timestamp() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap()
}

fn make_master_key(id: &str) -> PublicKey {
    let sk = Secp256k1PrivateKey::from_slice(&[1u8; 32]).unwrap();
    PublicKey {
        id: PublicKeyId::parse(id, 50).unwrap(),
        data: PublicKeyData::Master {
            data: sk.to_public_key(),
        },
        orig: test_utils::new_public_key(id, proto::prism_ssi::KeyUsage::MASTER_KEY, &sk),
    }
}

fn make_vdr_key(id: &str) -> PublicKey {
    let sk = Secp256k1PrivateKey::from_slice(&[2u8; 32]).unwrap();
    PublicKey {
        id: PublicKeyId::parse(id, 50).unwrap(),
        data: PublicKeyData::Vdr {
            data: sk.to_public_key(),
        },
        orig: test_utils::new_public_key(id, proto::prism_ssi::KeyUsage::VDR_KEY, &sk),
    }
}

fn make_auth_key(id: &str) -> PublicKey {
    let sk = Secp256k1PrivateKey::from_slice(&[3u8; 32]).unwrap();
    PublicKey {
        id: PublicKeyId::parse(id, 50).unwrap(),
        data: PublicKeyData::Other {
            data: NonOperationPublicKey::Secp256k1(sk.to_public_key()),
            usage: KeyUsage::AuthenticationKey,
        },
        orig: test_utils::new_public_key(id, proto::prism_ssi::KeyUsage::AUTHENTICATION_KEY, &sk),
    }
}

fn make_issuing_key(id: &str) -> PublicKey {
    let sk = Secp256k1PrivateKey::from_slice(&[4u8; 32]).unwrap();
    PublicKey {
        id: PublicKeyId::parse(id, 50).unwrap(),
        data: PublicKeyData::Other {
            data: NonOperationPublicKey::Secp256k1(sk.to_public_key()),
            usage: KeyUsage::IssuingKey,
        },
        orig: test_utils::new_public_key(id, proto::prism_ssi::KeyUsage::ISSUING_KEY, &sk),
    }
}

fn make_key_agreement_key(id: &str) -> PublicKey {
    let sk = Secp256k1PrivateKey::from_slice(&[5u8; 32]).unwrap();
    PublicKey {
        id: PublicKeyId::parse(id, 50).unwrap(),
        data: PublicKeyData::Other {
            data: NonOperationPublicKey::Secp256k1(sk.to_public_key()),
            usage: KeyUsage::KeyAgreementKey,
        },
        orig: test_utils::new_public_key(id, proto::prism_ssi::KeyUsage::KEY_AGREEMENT_KEY, &sk),
    }
}

fn make_capability_invocation_key(id: &str) -> PublicKey {
    let sk = Secp256k1PrivateKey::from_slice(&[6u8; 32]).unwrap();
    PublicKey {
        id: PublicKeyId::parse(id, 50).unwrap(),
        data: PublicKeyData::Other {
            data: NonOperationPublicKey::Secp256k1(sk.to_public_key()),
            usage: KeyUsage::CapabilityInvocationKey,
        },
        orig: test_utils::new_public_key(id, proto::prism_ssi::KeyUsage::CAPABILITY_INVOCATION_KEY, &sk),
    }
}

fn make_capability_delegation_key(id: &str) -> PublicKey {
    let sk = Secp256k1PrivateKey::from_slice(&[7u8; 32]).unwrap();
    PublicKey {
        id: PublicKeyId::parse(id, 50).unwrap(),
        data: PublicKeyData::Other {
            data: NonOperationPublicKey::Secp256k1(sk.to_public_key()),
            usage: KeyUsage::CapabilityDelegationKey,
        },
        orig: test_utils::new_public_key(id, proto::prism_ssi::KeyUsage::CAPABILITY_DELEGATION_KEY, &sk),
    }
}

fn make_revocation_key(id: &str) -> PublicKey {
    let sk = Secp256k1PrivateKey::from_slice(&[8u8; 32]).unwrap();
    PublicKey {
        id: PublicKeyId::parse(id, 50).unwrap(),
        data: PublicKeyData::Other {
            data: NonOperationPublicKey::Secp256k1(sk.to_public_key()),
            usage: KeyUsage::RevocationKey,
        },
        orig: test_utils::new_public_key(id, proto::prism_ssi::KeyUsage::REVOCATION_KEY, &sk),
    }
}

fn make_uri_service(id: &str, type_name: &str, uri: &str) -> Service {
    let orig = proto::prism_ssi::Service {
        id: id.to_string(),
        type_: type_name.to_string(),
        service_endpoint: uri.to_string(),
        special_fields: Default::default(),
    };
    Service {
        id: ServiceId::parse(id, 50).unwrap(),
        r#type: ServiceType::One(ServiceTypeValue::from_str(type_name).unwrap()),
        service_endpoint: ServiceEndpoint::One(ServiceEndpointValue::Uri(uri.to_string())),
        orig,
    }
}

fn make_json_endpoint_service(id: &str, type_name: &str) -> Service {
    let json_obj: serde_json::Map<String, serde_json::Value> = serde_json::from_str(r#"{"foo": "bar"}"#).unwrap();
    let endpoint_str = serde_json::to_string(&json_obj).unwrap();
    let orig = proto::prism_ssi::Service {
        id: id.to_string(),
        type_: type_name.to_string(),
        service_endpoint: endpoint_str,
        special_fields: Default::default(),
    };
    Service {
        id: ServiceId::parse(id, 50).unwrap(),
        r#type: ServiceType::One(ServiceTypeValue::from_str(type_name).unwrap()),
        service_endpoint: ServiceEndpoint::One(ServiceEndpointValue::Json(json_obj)),
        orig,
    }
}

fn make_multi_type_multi_endpoint_service(id: &str) -> Service {
    let endpoint_str = r#"["https://a.com","https://b.com"]"#.to_string();
    let orig = proto::prism_ssi::Service {
        id: id.to_string(),
        type_: r#"["TypeA","TypeB"]"#.to_string(),
        service_endpoint: endpoint_str,
        special_fields: Default::default(),
    };
    Service {
        id: ServiceId::parse(id, 50).unwrap(),
        r#type: ServiceType::Many(vec![
            ServiceTypeValue::from_str("TypeA").unwrap(),
            ServiceTypeValue::from_str("TypeB").unwrap(),
        ]),
        service_endpoint: ServiceEndpoint::Many(vec![
            ServiceEndpointValue::Uri("https://a.com".to_string()),
            ServiceEndpointValue::Uri("https://b.com".to_string()),
        ]),
        orig,
    }
}

fn base_state() -> DidState {
    DidState {
        did: make_canonical_did(),
        context: vec![],
        last_operation_hash: make_operation_hash(),
        public_keys: vec![],
        services: vec![],
        storage: vec![],
        created_at: make_timestamp(),
        updated_at: make_timestamp(),
        is_published: false,
    }
}

// ---------- to_did_document tests ----------

#[test]
fn to_did_document_empty_deactivated_state() {
    let state = base_state();
    let did = make_did();
    let doc = state.to_did_document(&did);

    assert_eq!(doc.context, vec!["https://www.w3.org/ns/did/v1"]);
    assert_eq!(doc.id.to_string(), did.to_string());
    assert!(doc.verification_method.is_empty());
    assert_eq!(doc.authentication.as_ref().map(|v| v.len()), Some(0));
    assert_eq!(doc.service.as_ref().map(|v| v.len()), Some(0));
}

#[test]
fn to_did_document_master_key_only_no_verification_methods() {
    let mut state = base_state();
    state.public_keys = vec![make_master_key("master-0")];

    let doc = state.to_did_document(&make_did());
    // Master keys are filtered out from verification methods
    assert!(doc.verification_method.is_empty());
    // No W3C relationship references for master key
    assert_eq!(doc.authentication.as_ref().map(|v| v.len()), Some(0));
    assert_eq!(doc.assertion_method.as_ref().map(|v| v.len()), Some(0));
}

#[test]
fn to_did_document_vdr_key_only_no_verification_methods() {
    let mut state = base_state();
    state.public_keys = vec![make_vdr_key("vdr-0")];

    let doc = state.to_did_document(&make_did());
    // VDR keys are filtered out from verification methods
    assert!(doc.verification_method.is_empty());
    assert_eq!(doc.authentication.as_ref().map(|v| v.len()), Some(0));
}

#[test]
fn to_did_document_auth_key_produces_verification_method_and_reference() {
    let mut state = base_state();
    state.public_keys = vec![make_auth_key("auth-0")];

    let did = make_did();
    let doc = state.to_did_document(&did);

    // Auth key is a W3C key type, so it should appear as verification method
    assert_eq!(doc.verification_method.len(), 1);
    let vm = &doc.verification_method[0];
    assert_eq!(vm.id, format!("{}#auth-0", did));
    assert_eq!(vm.r#type, "JsonWebKey2020");
    assert_eq!(vm.controller, did.to_string());
    assert!(vm.public_key_jwk.is_some());

    // Authentication reference
    let auth = doc.authentication.as_ref().unwrap();
    assert_eq!(auth.len(), 1);
    assert!(matches!(&auth[0], VerificationMethodOrRef::Ref(r) if r == &format!("{}#auth-0", did)));

    // Other relationships should be empty
    assert_eq!(doc.assertion_method.as_ref().map(|v| v.len()), Some(0));
    assert_eq!(doc.key_agreement.as_ref().map(|v| v.len()), Some(0));
    assert_eq!(doc.capability_invocation.as_ref().map(|v| v.len()), Some(0));
    assert_eq!(doc.capability_delegation.as_ref().map(|v| v.len()), Some(0));
}

#[test]
fn to_did_document_issuing_key_produces_assertion_method() {
    let mut state = base_state();
    state.public_keys = vec![make_issuing_key("issue-0")];

    let did = make_did();
    let doc = state.to_did_document(&did);

    assert_eq!(doc.verification_method.len(), 1);
    let am = doc.assertion_method.as_ref().unwrap();
    assert_eq!(am.len(), 1);
    assert!(matches!(&am[0], VerificationMethodOrRef::Ref(r) if r == &format!("{}#issue-0", did)));

    assert_eq!(doc.authentication.as_ref().map(|v| v.len()), Some(0));
}

#[test]
fn to_did_document_key_agreement_key() {
    let mut state = base_state();
    state.public_keys = vec![make_key_agreement_key("ka-0")];

    let did = make_did();
    let doc = state.to_did_document(&did);

    assert_eq!(doc.verification_method.len(), 1);
    let ka = doc.key_agreement.as_ref().unwrap();
    assert_eq!(ka.len(), 1);
    assert!(matches!(&ka[0], VerificationMethodOrRef::Ref(r) if r == &format!("{}#ka-0", did)));
}

#[test]
fn to_did_document_capability_invocation_key() {
    let mut state = base_state();
    state.public_keys = vec![make_capability_invocation_key("ci-0")];

    let did = make_did();
    let doc = state.to_did_document(&did);

    assert_eq!(doc.verification_method.len(), 1);
    let ci = doc.capability_invocation.as_ref().unwrap();
    assert_eq!(ci.len(), 1);
    assert!(matches!(&ci[0], VerificationMethodOrRef::Ref(r) if r == &format!("{}#ci-0", did)));
}

#[test]
fn to_did_document_capability_delegation_key() {
    let mut state = base_state();
    state.public_keys = vec![make_capability_delegation_key("cd-0")];

    let did = make_did();
    let doc = state.to_did_document(&did);

    assert_eq!(doc.verification_method.len(), 1);
    let cd = doc.capability_delegation.as_ref().unwrap();
    assert_eq!(cd.len(), 1);
    assert!(matches!(&cd[0], VerificationMethodOrRef::Ref(r) if r == &format!("{}#cd-0", did)));
}

#[test]
fn to_did_document_revocation_key_not_in_w3c_types() {
    let mut state = base_state();
    state.public_keys = vec![make_revocation_key("rev-0")];

    let doc = state.to_did_document(&make_did());
    // Revocation keys are NOT in W3C_KEY_TYPES, so no verification methods
    assert!(doc.verification_method.is_empty());
    assert_eq!(doc.authentication.as_ref().map(|v| v.len()), Some(0));
}

#[test]
fn to_did_document_multiple_key_types() {
    let mut state = base_state();
    state.public_keys = vec![
        make_master_key("master-0"),
        make_vdr_key("vdr-0"),
        make_auth_key("auth-0"),
        make_issuing_key("issue-0"),
        make_key_agreement_key("ka-0"),
        make_capability_invocation_key("ci-0"),
        make_capability_delegation_key("cd-0"),
    ];

    let did = make_did();
    let doc = state.to_did_document(&did);

    // Master + VDR + Revocation keys are excluded; 5 W3C keys should produce verification methods
    assert_eq!(doc.verification_method.len(), 5);
    assert_eq!(doc.authentication.as_ref().unwrap().len(), 1);
    assert_eq!(doc.assertion_method.as_ref().unwrap().len(), 1);
    assert_eq!(doc.key_agreement.as_ref().unwrap().len(), 1);
    assert_eq!(doc.capability_invocation.as_ref().unwrap().len(), 1);
    assert_eq!(doc.capability_delegation.as_ref().unwrap().len(), 1);
}

#[test]
fn to_did_document_with_custom_context() {
    let mut state = base_state();
    state.context = vec!["https://example.com/context/v1".to_string()];

    let doc = state.to_did_document(&make_did());
    assert_eq!(
        doc.context,
        vec![
            "https://www.w3.org/ns/did/v1".to_string(),
            "https://example.com/context/v1".to_string(),
        ]
    );
}

#[test]
fn to_did_document_with_multiple_custom_contexts() {
    let mut state = base_state();
    state.context = vec![
        "https://example.com/context/v1".to_string(),
        "https://example.com/context/v2".to_string(),
    ];

    let doc = state.to_did_document(&make_did());
    assert_eq!(doc.context.len(), 3);
    assert_eq!(doc.context[0], "https://www.w3.org/ns/did/v1");
    assert_eq!(doc.context[1], "https://example.com/context/v1");
    assert_eq!(doc.context[2], "https://example.com/context/v2");
}

#[test]
fn to_did_document_with_uri_service() {
    let mut state = base_state();
    state.services = vec![make_uri_service("svc-1", "LinkedDomains", "https://example.com")];

    let did = make_did();
    let doc = state.to_did_document(&did);

    let services = doc.service.as_ref().unwrap();
    assert_eq!(services.len(), 1);
    let svc = &services[0];
    assert_eq!(svc.id, format!("{}#svc-1", did));
    assert!(matches!(&svc.r#type, CoreServiceType::Str(s) if s == "LinkedDomains"));
    assert!(
        matches!(&svc.service_endpoint, CoreServiceEndpoint::StrOrMap(StringOrMap::Str(uri)) if uri == "https://example.com")
    );
}

#[test]
fn to_did_document_with_json_endpoint_service() {
    let mut state = base_state();
    state.services = vec![make_json_endpoint_service("svc-json", "LinkedDomains")];

    let did = make_did();
    let doc = state.to_did_document(&did);

    let services = doc.service.as_ref().unwrap();
    assert_eq!(services.len(), 1);
    let svc = &services[0];
    assert_eq!(svc.id, format!("{}#svc-json", did));
    assert!(matches!(
        &svc.service_endpoint,
        CoreServiceEndpoint::StrOrMap(StringOrMap::Map(_))
    ));
}

#[test]
fn to_did_document_with_multi_type_multi_endpoint_service() {
    let mut state = base_state();
    state.services = vec![make_multi_type_multi_endpoint_service("svc-multi")];

    let did = make_did();
    let doc = state.to_did_document(&did);

    let services = doc.service.as_ref().unwrap();
    assert_eq!(services.len(), 1);
    let svc = &services[0];
    assert_eq!(svc.id, format!("{}#svc-multi", did));
    assert!(matches!(&svc.r#type, CoreServiceType::List(ls) if ls.len() == 2));
    assert!(matches!(&svc.service_endpoint, CoreServiceEndpoint::List(ls) if ls.len() == 2));
}

#[test]
fn to_did_document_with_multiple_services() {
    let mut state = base_state();
    state.services = vec![
        make_uri_service("svc-1", "LinkedDomains", "https://a.com"),
        make_uri_service("svc-2", "DIDCommMessaging", "https://b.com"),
    ];

    let doc = state.to_did_document(&make_did());
    let services = doc.service.as_ref().unwrap();
    assert_eq!(services.len(), 2);
}

#[test]
fn to_did_document_no_services_returns_empty_list() {
    let state = base_state();
    let doc = state.to_did_document(&make_did());
    assert_eq!(doc.service.as_ref().map(|v| v.len()), Some(0));
}

#[test]
fn to_did_document_also_known_as_is_none() {
    let state = base_state();
    let doc = state.to_did_document(&make_did());
    assert!(doc.also_known_as.is_none());
}

// ---------- to_resolution_result tests ----------

#[test]
fn to_resolution_result_active_canonical_not_published() {
    let mut state = base_state();
    state.public_keys = vec![make_auth_key("auth-0")];

    let did = PrismDid::Canonical(make_canonical_did());
    let result = state.to_resolution_result(&did);

    // Active DID should have a document
    assert!(result.did_document.is_some());
    assert_eq!(
        result.did_resolution_metadata.content_type.as_deref(),
        Some("application/did")
    );
    // Not published, so canonical_id should be None
    assert!(result.did_document_metadata.canonical_id.is_none());
    // Deactivated should be false
    assert_eq!(result.did_document_metadata.deactivated, Some(false));
    assert_eq!(result.did_document_metadata.created, Some(make_timestamp()));
    assert_eq!(result.did_document_metadata.updated, Some(make_timestamp()));
    // version_id should be the hex of the operation hash
    let expected_version_id = HexStr::from(state.last_operation_hash.as_bytes()).to_string();
    assert_eq!(result.did_document_metadata.version_id, Some(expected_version_id));
}

#[test]
fn to_resolution_result_active_canonical_published() {
    let mut state = base_state();
    state.is_published = true;
    state.public_keys = vec![make_auth_key("auth-0")];

    let did = PrismDid::Canonical(make_canonical_did());
    let result = state.to_resolution_result(&did);

    // Canonical DID: canonical_id is None even if published
    assert!(result.did_document_metadata.canonical_id.is_none());
    assert!(result.did_document.is_some());
}

#[test]
fn to_resolution_result_active_long_form_not_published() {
    let mut state = base_state();
    state.public_keys = vec![make_auth_key("auth-0")];
    // Not published

    let (create_op, _, _) = test_utils::new_create_did_operation(None);
    let prism_operation = create_op.operation.unwrap();
    let long_form = LongFormPrismDid::from_operation(&prism_operation).unwrap();
    let did = PrismDid::LongForm(long_form);

    let result = state.to_resolution_result(&did);

    // Not published → no canonical_id
    assert!(result.did_document_metadata.canonical_id.is_none());
    assert!(result.did_document.is_some());
}

#[test]
fn to_resolution_result_active_long_form_published_has_canonical_id() {
    let mut state = base_state();
    state.is_published = true;
    state.public_keys = vec![make_auth_key("auth-0")];

    let (create_op, _, _) = test_utils::new_create_did_operation(None);
    let prism_operation = create_op.operation.unwrap();
    let long_form = LongFormPrismDid::from_operation(&prism_operation).unwrap();
    let canonical = long_form.clone().into_canonical();
    let did = PrismDid::LongForm(long_form);

    let result = state.to_resolution_result(&did);

    // Published long-form → canonical_id should be set
    assert!(result.did_document_metadata.canonical_id.is_some());
    let canonical_id = result.did_document_metadata.canonical_id.unwrap();
    assert_eq!(canonical_id.to_string(), canonical.to_did().to_string());
}

#[test]
fn to_resolution_result_deactivated_state() {
    let state = base_state(); // no public keys → deactivated

    let did = PrismDid::Canonical(make_canonical_did());
    let result = state.to_resolution_result(&did);

    // Deactivated: no document
    assert!(result.did_document.is_none());
    assert_eq!(result.did_document_metadata.deactivated, Some(true));
    // did_resolution_metadata should not have content_type
    assert!(result.did_resolution_metadata.content_type.is_none());
}

#[test]
fn to_resolution_result_deactivated_with_long_form_published() {
    let state = base_state(); // no public keys → deactivated

    let (create_op, _, _) = test_utils::new_create_did_operation(None);
    let prism_operation = create_op.operation.unwrap();
    let long_form = LongFormPrismDid::from_operation(&prism_operation).unwrap();

    let mut state = state;
    state.is_published = true;

    let did = PrismDid::LongForm(long_form.clone());
    let result = state.to_resolution_result(&did);

    // Even when published, deactivated still has no document
    assert!(result.did_document.is_none());
    assert_eq!(result.did_document_metadata.deactivated, Some(true));
    // Deactivated overrides canonical_id behavior (canonical_id set from long-form + published)
    let canonical_id = result.did_document_metadata.canonical_id.as_ref();
    assert_eq!(
        canonical_id.map(|d| d.to_string()),
        Some(long_form.into_canonical().to_did().to_string())
    );
}

#[test]
fn to_resolution_result_version_id_matches_last_operation_hash() {
    let mut state = base_state();
    state.public_keys = vec![make_auth_key("auth-0")];
    let expected_hex = HexStr::from(state.last_operation_hash.as_bytes()).to_string();

    let did = PrismDid::Canonical(make_canonical_did());
    let result = state.to_resolution_result(&did);

    assert_eq!(
        result.did_document_metadata.version_id.as_deref(),
        Some(expected_hex.as_str())
    );
}

// ---------- is_deactivated tests ----------

#[test]
fn is_deactivated_true_when_no_public_keys() {
    let state = base_state();
    assert!(state.is_deactivated());
}

#[test]
fn is_deactivated_false_when_has_public_keys() {
    let mut state = base_state();
    state.public_keys = vec![make_master_key("master-0")];
    assert!(!state.is_deactivated());
}

// ---------- StorageState round-trip (via node_api) ----------

#[test]
fn storage_state_with_bytes_converts_to_did_data() {
    use identus_did_prism::did::operation::StorageData;
    use identus_did_prism::proto::node_api;

    let storage = StorageState {
        init_operation_hash: Rc::new(Sha256Digest::from_bytes(&[0u8; 32]).unwrap()),
        last_operation_hash: Rc::new(Sha256Digest::from_bytes(&[1u8; 32]).unwrap()),
        data: Rc::new(StorageData::Bytes(vec![1, 2, 3])),
    };

    let did_data: node_api::DIDData = DidState {
        did: make_canonical_did(),
        context: vec![],
        last_operation_hash: make_operation_hash(),
        public_keys: vec![],
        services: vec![],
        storage: vec![storage],
        created_at: make_timestamp(),
        updated_at: make_timestamp(),
        is_published: false,
    }
    .into();

    assert_eq!(did_data.storage_data.len(), 1);
    assert_eq!(did_data.id, HexStr::from(make_suffix().as_bytes()).to_string());
}

// Note: the failing StorageState → node_api conversions (Ipfs, StatusList) are
// covered in did_mod.rs, where the conversion impl lives.
