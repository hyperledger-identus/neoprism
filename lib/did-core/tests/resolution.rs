use chrono::{DateTime, NaiveDateTime, Utc};
use identus_did_core::{
    Did, DidDocument, DidDocumentMetadata, DidResolutionError, DidResolutionErrorCode, DidResolutionMetadata,
    ResolutionOptions, ResolutionResult,
};

fn sample_did() -> Did {
    "did:example:123456789abcdefghi".parse().unwrap()
}

fn sample_did_document(did: &Did) -> DidDocument {
    DidDocument {
        context: vec!["https://www.w3.org/ns/did/v1".to_string()],
        id: did.clone(),
        also_known_as: None,
        verification_method: vec![],
        authentication: None,
        assertion_method: None,
        key_agreement: None,
        capability_invocation: None,
        capability_delegation: None,
        service: None,
    }
}

// ------------------------------------------------------------------
// ResolutionOptions
// ------------------------------------------------------------------

#[test]
fn resolution_options_default() {
    let opts = ResolutionOptions::default();
    assert!(opts.accept.is_none());
    assert!(opts.expand_relative_urls.is_none());
    assert!(opts.version_id.is_none());
    assert!(opts.version_time.is_none());
}

#[test]
fn resolution_options_construction() {
    let dt = DateTime::<Utc>::from_naive_utc_and_offset(
        NaiveDateTime::parse_from_str("2024-01-15 10:30:00", "%Y-%m-%d %H:%M:%S").unwrap(),
        Utc,
    );
    let opts = ResolutionOptions {
        accept: Some("application/did+ld+json".to_string()),
        expand_relative_urls: Some(true),
        version_id: Some("v1".to_string()),
        version_time: Some(dt),
    };
    assert_eq!(opts.accept.as_deref(), Some("application/did+ld+json"));
    assert_eq!(opts.expand_relative_urls, Some(true));
    assert_eq!(opts.version_id.as_deref(), Some("v1"));
    assert_eq!(opts.version_time, Some(dt));
}

#[test]
fn resolution_options_serialization_roundtrip() {
    let opts = ResolutionOptions {
        accept: Some("application/did".to_string()),
        expand_relative_urls: Some(false),
        version_id: None,
        version_time: None,
    };
    let json = serde_json::to_string(&opts).unwrap();
    // Verify camelCase serialization
    assert!(json.contains("expandRelativeUrls"), "expected camelCase: {json}");
    assert!(json.contains("versionId"), "expected camelCase: {json}");
    assert!(json.contains("versionTime"), "expected camelCase: {json}");

    let deserialized: ResolutionOptions = serde_json::from_str(&json).unwrap();
    assert_eq!(opts.accept, deserialized.accept);
    assert_eq!(opts.expand_relative_urls, deserialized.expand_relative_urls);
}

// ------------------------------------------------------------------
// ResolutionResult::success
// ------------------------------------------------------------------

#[test]
fn resolution_result_success() {
    let did = sample_did();
    let did_doc = sample_did_document(&did);
    let result = ResolutionResult::success(did_doc.clone());

    assert!(result.did_document.is_some());
    let doc = result.did_document.unwrap();
    assert_eq!(doc.id.to_string(), did.to_string());

    assert_eq!(
        result.did_resolution_metadata.content_type.as_deref(),
        Some("application/did")
    );
    assert!(result.did_resolution_metadata.error.is_none());
    // Default document metadata
    assert!(result.did_document_metadata.created.is_none());
    assert!(result.did_document_metadata.updated.is_none());
    assert!(result.did_document_metadata.deactivated.is_none());
    assert!(result.did_document_metadata.canonical_id.is_none());
    assert!(result.did_document_metadata.version_id.is_none());
}

// ------------------------------------------------------------------
// ResolutionResult::deactivated
// ------------------------------------------------------------------

#[test]
fn resolution_result_deactivated() {
    let result = ResolutionResult::deactivated();

    assert!(result.did_document.is_none());
    assert!(result.did_resolution_metadata.content_type.is_none());
    assert!(result.did_resolution_metadata.error.is_none());
    assert_eq!(result.did_document_metadata.deactivated, Some(true));
    assert!(result.did_document_metadata.created.is_none());
    assert!(result.did_document_metadata.updated.is_none());
}

// ------------------------------------------------------------------
// ResolutionResult::invalid_did
// ------------------------------------------------------------------

#[test]
fn resolution_result_invalid_did_with_error() {
    let err = identus_did_core::Error::InvalidDid {
        error: identus_did_core::InvalidDid::from(identity_did::Error::Other("DID cannot contain fragment")),
    };
    let result = ResolutionResult::invalid_did(err);

    // No document
    assert!(result.did_document.is_none());

    // Resolution metadata contains error
    let meta = &result.did_resolution_metadata;
    assert!(meta.content_type.is_none());
    let res_error = meta.error.as_ref().expect("expected error in resolution metadata");
    assert!(matches!(res_error.r#type, DidResolutionErrorCode::InvalidDid));
    assert_eq!(res_error.title.as_deref(), Some("Invalid DID"));
    assert!(res_error.detail.is_some());
    assert!(res_error.detail.as_ref().unwrap().contains("fragment"));

    // Document metadata is default
    assert!(result.did_document_metadata.deactivated.is_none());
}

#[test]
fn resolution_result_invalid_did_error_detail_content() {
    let err = identus_did_core::Error::InvalidDid {
        error: identus_did_core::InvalidDid::from(identity_did::Error::Other("custom error msg")),
    };
    let result = ResolutionResult::invalid_did(err);
    let detail = result.did_resolution_metadata.error.unwrap().detail.unwrap();
    assert!(detail.contains("custom error msg"));
}

#[test]
fn resolution_result_invalid_did_preserves_error_type() {
    let err = identus_did_core::Error::InvalidUri {
        error: identus_did_core::InvalidUri { msg: "bad uri" },
    };
    let result = ResolutionResult::invalid_did(err);
    let error = result.did_resolution_metadata.error.unwrap();
    // The error type should always be InvalidDid regardless of input
    assert!(matches!(error.r#type, DidResolutionErrorCode::InvalidDid));
}

// ------------------------------------------------------------------
// ResolutionResult serialization
// ------------------------------------------------------------------

#[test]
fn resolution_result_serialization_roundtrip() {
    let did = sample_did();
    let did_doc = sample_did_document(&did);
    let result = ResolutionResult::success(did_doc);

    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("didDocument"), "expected camelCase: {json}");
    assert!(json.contains("didResolutionMetadata"), "expected camelCase: {json}");
    assert!(json.contains("didDocumentMetadata"), "expected camelCase: {json}");

    let deserialized: ResolutionResult = serde_json::from_str(&json).unwrap();
    assert!(deserialized.did_document.is_some());
}

// ------------------------------------------------------------------
// DidDocumentMetadata
// ------------------------------------------------------------------

#[test]
fn did_document_metadata_default() {
    let meta = DidDocumentMetadata::default();
    assert!(meta.created.is_none());
    assert!(meta.updated.is_none());
    assert!(meta.deactivated.is_none());
    assert!(meta.canonical_id.is_none());
    assert!(meta.version_id.is_none());
}

#[test]
fn did_document_metadata_construction() {
    let created = DateTime::<Utc>::from_naive_utc_and_offset(
        NaiveDateTime::parse_from_str("2024-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap(),
        Utc,
    );
    let updated = DateTime::<Utc>::from_naive_utc_and_offset(
        NaiveDateTime::parse_from_str("2024-06-15 12:00:00", "%Y-%m-%d %H:%M:%S").unwrap(),
        Utc,
    );
    let did = sample_did();

    let meta = DidDocumentMetadata {
        created: Some(created),
        updated: Some(updated),
        deactivated: Some(false),
        canonical_id: Some(did.clone()),
        version_id: Some("v42".to_string()),
    };
    assert_eq!(meta.created, Some(created));
    assert_eq!(meta.updated, Some(updated));
    assert_eq!(meta.deactivated, Some(false));
    assert_eq!(meta.canonical_id.as_ref().unwrap().to_string(), did.to_string());
    assert_eq!(meta.version_id.as_deref(), Some("v42"));
}

#[test]
fn did_document_metadata_serialization_camel_case() {
    let meta = DidDocumentMetadata {
        created: None,
        updated: None,
        deactivated: Some(true),
        canonical_id: None,
        version_id: Some("v1".to_string()),
    };
    let json = serde_json::to_string(&meta).unwrap();
    assert!(json.contains("versionId"), "expected camelCase: {json}");
    assert!(json.contains("canonicalId"), "expected camelCase: {json}");

    let back: DidDocumentMetadata = serde_json::from_str(&json).unwrap();
    assert_eq!(back.deactivated, Some(true));
    assert_eq!(back.version_id.as_deref(), Some("v1"));
}

// ------------------------------------------------------------------
// DidResolutionMetadata
// ------------------------------------------------------------------

#[test]
fn did_resolution_metadata_default() {
    let meta = DidResolutionMetadata::default();
    assert!(meta.content_type.is_none());
    assert!(meta.error.is_none());
}

#[test]
fn did_resolution_metadata_with_content_type() {
    let meta = DidResolutionMetadata {
        content_type: Some("application/did+ld+json".to_string()),
        error: None,
    };
    assert_eq!(meta.content_type.as_deref(), Some("application/did+ld+json"));
}

#[test]
fn did_resolution_metadata_with_error() {
    let meta = DidResolutionMetadata {
        content_type: None,
        error: Some(DidResolutionError {
            r#type: DidResolutionErrorCode::NotFound,
            title: Some("Not Found".to_string()),
            detail: Some("DID not found in the registry".to_string()),
        }),
    };
    let err = meta.error.unwrap();
    assert!(matches!(err.r#type, DidResolutionErrorCode::NotFound));
    assert_eq!(err.title.as_deref(), Some("Not Found"));
}

// ------------------------------------------------------------------
// DidResolutionError
// ------------------------------------------------------------------

#[test]
fn did_resolution_error_construction() {
    let err = DidResolutionError {
        r#type: DidResolutionErrorCode::InternalError,
        title: None,
        detail: None,
    };
    assert!(matches!(err.r#type, DidResolutionErrorCode::InternalError));
    assert!(err.title.is_none());
    assert!(err.detail.is_none());
}

#[test]
fn did_resolution_error_serialization_roundtrip() {
    let err = DidResolutionError {
        r#type: DidResolutionErrorCode::InvalidDid,
        title: Some("Invalid DID".to_string()),
        detail: Some("malformed DID string".to_string()),
    };
    let json = serde_json::to_string(&err).unwrap();
    // Verify camelCase field names
    assert!(json.contains("\"type\""), "expected 'type': {json}");
    assert!(json.contains("\"title\""), "expected 'title': {json}");
    assert!(json.contains("\"detail\""), "expected 'detail': {json}");

    let back: DidResolutionError = serde_json::from_str(&json).unwrap();
    assert!(matches!(back.r#type, DidResolutionErrorCode::InvalidDid));
    assert_eq!(back.title.as_deref(), Some("Invalid DID"));
    assert_eq!(back.detail.as_deref(), Some("malformed DID string"));
}

// ------------------------------------------------------------------
// DidResolutionErrorCode — serde rename verification
// ------------------------------------------------------------------

#[test]
fn did_resolution_error_code_serde_invalid_did() {
    let json = serde_json::to_string(&DidResolutionErrorCode::InvalidDid).unwrap();
    assert_eq!(json, r#""https://www.w3.org/ns/did#INVALID_DID""#);
}

#[test]
fn did_resolution_error_code_serde_invalid_did_document() {
    let json = serde_json::to_string(&DidResolutionErrorCode::InvalidDidDocument).unwrap();
    assert_eq!(json, r#""https://www.w3.org/ns/did#INVALID_DID_DOCUMENT""#);
}

#[test]
fn did_resolution_error_code_serde_not_found() {
    let json = serde_json::to_string(&DidResolutionErrorCode::NotFound).unwrap();
    assert_eq!(json, r#""https://www.w3.org/ns/did#NOT_FOUND""#);
}

#[test]
fn did_resolution_error_code_serde_representation_not_supported() {
    let json = serde_json::to_string(&DidResolutionErrorCode::RepresentationNotSupported).unwrap();
    assert_eq!(json, r#""https://www.w3.org/ns/did#REPRESENTATION_NOT_SUPPORTED""#);
}

#[test]
fn did_resolution_error_code_serde_invalid_did_url() {
    let json = serde_json::to_string(&DidResolutionErrorCode::InvalidDidUrl).unwrap();
    assert_eq!(json, r#""https://www.w3.org/ns/did#INVALID_DID_URL""#);
}

#[test]
fn did_resolution_error_code_serde_method_not_supported() {
    let json = serde_json::to_string(&DidResolutionErrorCode::MethodNotSupported).unwrap();
    assert_eq!(json, r#""https://www.w3.org/ns/did#METHOD_NOT_SUPPORTED""#);
}

#[test]
fn did_resolution_error_code_serde_invalid_options() {
    let json = serde_json::to_string(&DidResolutionErrorCode::InvalidOptions).unwrap();
    assert_eq!(json, r#""https://www.w3.org/ns/did#INVALID_OPTIONS""#);
}

#[test]
fn did_resolution_error_code_serde_internal_error() {
    let json = serde_json::to_string(&DidResolutionErrorCode::InternalError).unwrap();
    assert_eq!(json, r#""https://www.w3.org/ns/did#INTERNAL_ERROR""#);
}

#[test]
fn did_resolution_error_code_serde_invalid_public_key() {
    let json = serde_json::to_string(&DidResolutionErrorCode::InvalidPublicKey).unwrap();
    assert_eq!(json, r#""https://w3id.org/security#INVALID_PUBLIC_KEY""#);
}

#[test]
fn did_resolution_error_code_serde_invalid_public_key_length() {
    let json = serde_json::to_string(&DidResolutionErrorCode::InvalidPublicKeyLength).unwrap();
    assert_eq!(json, r#""https://w3id.org/security#INVALID_PUBLIC_KEY_LENGTH""#);
}

#[test]
fn did_resolution_error_code_serde_invalid_public_key_type() {
    let json = serde_json::to_string(&DidResolutionErrorCode::InvalidPublicKeyType).unwrap();
    assert_eq!(json, r#""https://w3id.org/security#INVALID_PUBLIC_KEY_TYPE""#);
}

#[test]
fn did_resolution_error_code_serde_unsupported_public_key_type() {
    let json = serde_json::to_string(&DidResolutionErrorCode::UnsupportedPublicKeyType).unwrap();
    assert_eq!(json, r#""https://w3id.org/security#UNSUPPORTED_PUBLIC_KEY_TYPE""#);
}

#[test]
fn did_resolution_error_code_serde_roundtrip_all_variants() {
    let variants = [
        DidResolutionErrorCode::InvalidDid,
        DidResolutionErrorCode::InvalidDidDocument,
        DidResolutionErrorCode::NotFound,
        DidResolutionErrorCode::RepresentationNotSupported,
        DidResolutionErrorCode::InvalidDidUrl,
        DidResolutionErrorCode::MethodNotSupported,
        DidResolutionErrorCode::InvalidOptions,
        DidResolutionErrorCode::InternalError,
        DidResolutionErrorCode::InvalidPublicKey,
        DidResolutionErrorCode::InvalidPublicKeyLength,
        DidResolutionErrorCode::InvalidPublicKeyType,
        DidResolutionErrorCode::UnsupportedPublicKeyType,
        DidResolutionErrorCode::InvalidVerificationMethodUrl,
        DidResolutionErrorCode::InvalidControlledIdentifierDocumentId,
        DidResolutionErrorCode::InvalidControlledIdentifierDocument,
        DidResolutionErrorCode::InvalidVerificationMethod,
        DidResolutionErrorCode::InvalidRelationshipForVerificationMethod,
    ];
    for variant in &variants {
        let json = serde_json::to_string(variant).unwrap();
        let back: DidResolutionErrorCode = serde_json::from_str(&json).unwrap();
        assert_eq!(json, serde_json::to_string(&back).unwrap(), "roundtrip failed");
    }
}
