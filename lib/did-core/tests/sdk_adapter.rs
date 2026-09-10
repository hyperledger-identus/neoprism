use std::collections::BTreeMap;

use chrono::{TimeZone, Utc};
use identus_apollo::base64::Base64UrlStrNoPad;
use identus_apollo::jwk::Jwk;
use identus_did_core::sdk_adapter::{
    did_document_to_sdk, did_from_sdk, resolution_options_from_sdk, resolution_result_to_sdk,
};
use identus_did_core::{
    Did, DidDocument, DidDocumentMetadata, DidResolutionError, DidResolutionErrorCode, DidResolutionMetadata,
    ResolutionResult, Service, ServiceEndpoint, ServiceType, StringOrMap, Uri, VerificationMethod,
    VerificationMethodOrRef,
};

fn sample_document() -> DidDocument {
    let did: Did = "did:prism:1234".parse().unwrap();
    let key = |id: &str, method_type: &str, kty: &str, crv: &str, y: Option<[u8; 32]>| VerificationMethod {
        id: format!("did:prism:1234#{id}"),
        r#type: method_type.to_owned(),
        controller: did.to_string(),
        public_key_jwk: Some(Jwk {
            kty: kty.to_owned(),
            crv: crv.to_owned(),
            x: Some(Base64UrlStrNoPad::from([7_u8; 32])),
            y: y.map(Base64UrlStrNoPad::from),
        }),
    };
    let methods = vec![
        key("signing", "Ed25519VerificationKey2020", "OKP", "Ed25519", None),
        key("agreement", "X25519KeyAgreementKey2020", "OKP", "X25519", None),
        key(
            "master",
            "EcdsaSecp256k1VerificationKey2019",
            "EC",
            "secp256k1",
            Some([9_u8; 32]),
        ),
    ];

    DidDocument {
        context: vec!["https://www.w3.org/ns/did/v1".to_owned()],
        id: did,
        also_known_as: Some(vec!["https://example.com/subject".parse::<Uri>().unwrap()]),
        verification_method: methods,
        authentication: Some(vec![VerificationMethodOrRef::Ref("did:prism:1234#signing".to_owned())]),
        assertion_method: Some(vec![VerificationMethodOrRef::Ref("did:prism:1234#master".to_owned())]),
        key_agreement: Some(vec![VerificationMethodOrRef::Ref(
            "did:prism:1234#agreement".to_owned(),
        )]),
        capability_invocation: None,
        capability_delegation: None,
        service: Some(vec![Service {
            id: "did:prism:1234#messages".to_owned(),
            r#type: ServiceType::Str("DIDCommMessaging".to_owned()),
            service_endpoint: ServiceEndpoint::StrOrMap(StringOrMap::Map(
                BTreeMap::from([(
                    "uri".to_owned(),
                    serde_json::Value::String("https://example.com/messages".to_owned()),
                )])
                .into_iter()
                .collect(),
            )),
        }]),
    }
}

#[test]
fn projects_sdk_did_into_legacy_facade() {
    let sdk = identus_did::Did::parse("did:prism:1234").unwrap();
    assert_eq!(did_from_sdk(&sdk).unwrap().to_string(), sdk.as_str());
}

#[test]
fn projects_resolution_options_from_sdk() {
    let sdk = identus_did::ResolutionOptions::builder()
        .accept(identus_did::MediaType::parse("application/did").unwrap())
        .expand_relative_urls(true)
        .version_id(identus_did::VersionId::parse("42").unwrap())
        .version_time(identus_did::DidResolutionDateTime::parse("2026-09-10T07:00:00Z").unwrap())
        .build()
        .unwrap();

    let projected = resolution_options_from_sdk(&sdk).unwrap();
    assert_eq!(projected.accept.as_deref(), Some("application/did"));
    assert_eq!(projected.expand_relative_urls, Some(true));
    assert_eq!(projected.version_id.as_deref(), Some("42"));
    assert_eq!(
        projected.version_time,
        Some(Utc.with_ymd_and_hms(2026, 9, 10, 7, 0, 0).unwrap())
    );
}

#[test]
fn validates_active_document_and_resolution_result_with_sdk() {
    let document = sample_document();
    let sdk_document = did_document_to_sdk(&document).unwrap();
    assert_eq!(sdk_document.id().as_str(), "did:prism:1234");
    assert_eq!(sdk_document.verification_methods().unwrap().len(), 3);
    assert_eq!(sdk_document.services().unwrap().len(), 1);

    let result = ResolutionResult {
        did_document: Some(document),
        did_resolution_metadata: DidResolutionMetadata {
            content_type: Some("application/did".to_owned()),
            error: None,
        },
        did_document_metadata: DidDocumentMetadata {
            created: Some(Utc.with_ymd_and_hms(2026, 9, 10, 7, 0, 0).unwrap()),
            updated: None,
            deactivated: Some(false),
            canonical_id: None,
            version_id: Some("42".to_owned()),
        },
    };

    let sdk_result = resolution_result_to_sdk(&result).unwrap();
    assert!(sdk_result.document().is_some());
    assert_eq!(sdk_result.document_metadata().version_id().unwrap().as_str(), "42");
}

#[test]
fn validates_deactivated_and_error_results_with_sdk() {
    let deactivated = resolution_result_to_sdk(&ResolutionResult::deactivated()).unwrap();
    assert_eq!(deactivated.document_metadata().deactivated(), Some(true));

    let error = ResolutionResult {
        did_document: None,
        did_resolution_metadata: DidResolutionMetadata {
            content_type: None,
            error: Some(DidResolutionError {
                r#type: DidResolutionErrorCode::NotFound,
                title: Some("Not found".to_owned()),
                detail: Some("No matching PRISM DID".to_owned()),
            }),
        },
        did_document_metadata: DidDocumentMetadata::default(),
    };
    let sdk_error = resolution_result_to_sdk(&error).unwrap();
    assert_eq!(
        sdk_error.metadata().error().unwrap().kind(),
        Some(identus_did::DidResolutionErrorKind::NotFound)
    );
}

#[test]
fn rejects_unbounded_legacy_document_at_sdk_boundary() {
    let mut document = sample_document();
    document.context = (0..=identus_did::MAX_DOCUMENT_ITEMS)
        .map(|index| format!("https://example.com/context/{index}"))
        .collect();

    assert!(did_document_to_sdk(&document).is_err());
}
