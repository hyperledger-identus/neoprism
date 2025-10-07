use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{Did, DidDocument};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct ResolutionOptions {
    pub accept: Option<String>,
    pub expand_relative_urls: Option<bool>,
    pub version_id: Option<String>,
    pub version_time: Option<DateTime<Utc>>,
}

#[async_trait::async_trait]
pub trait DidResolver {
    async fn resolve(&self, did: &Did, options: &ResolutionOptions) -> ResolutionResult;
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct ResolutionResult {
    pub did_document: Option<DidDocument>,
    pub did_resolution_metadata: DidResolutionMetadata,
    pub did_document_metadata: DidDocumentMetadata,
}

impl ResolutionResult {
    pub fn success(did_doc: DidDocument) -> Self {
        ResolutionResult {
            did_document: Some(did_doc),
            did_resolution_metadata: DidResolutionMetadata {
                content_type: Some("application/did".to_string()),
                ..Default::default()
            },
            did_document_metadata: Default::default(),
        }
    }

    pub fn invalid_did(error: super::Error) -> Self {
        let error = DidResolutionError {
            r#type: DidResolutionErrorCode::InvalidDid,
            title: Some("Invalid DID".to_string()),
            detail: Some(error.to_string()),
        };

        ResolutionResult {
            did_resolution_metadata: DidResolutionMetadata {
                content_type: None,
                error: Some(error),
            },
            did_document_metadata: Default::default(),
            did_document: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct DidDocumentMetadata {
    pub created: Option<DateTime<Utc>>,
    pub updated: Option<DateTime<Utc>>,
    pub deactivated: Option<bool>,
    pub canonical_id: Option<Did>,
    pub version_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct DidResolutionMetadata {
    pub content_type: Option<String>,
    pub error: Option<DidResolutionError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct DidResolutionError {
    pub r#type: DidResolutionErrorCode,
    pub title: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum DidResolutionErrorCode {
    #[serde(rename = "https://www.w3.org/ns/did#INVALID_DID")]
    InvalidDid,
    #[serde(rename = "https://www.w3.org/ns/did#INVALID_DID_DOCUMENT")]
    InvalidDidDocument,
    #[serde(rename = "https://www.w3.org/ns/did#NOT_FOUND")]
    NotFound,
    #[serde(rename = "https://www.w3.org/ns/did#REPRESENTATION_NOT_SUPPORTED")]
    RepresentationNotSupported,
    #[serde(rename = "https://www.w3.org/ns/did#INVALID_DID_URL")]
    InvalidDidUrl,
    #[serde(rename = "https://www.w3.org/ns/did#METHOD_NOT_SUPPORTED")]
    MethodNotSupported,
    #[serde(rename = "https://www.w3.org/ns/did#INVALID_OPTIONS")]
    InvalidOptions,
    #[serde(rename = "https://www.w3.org/ns/did#INTERNAL_ERROR")]
    InternalError,

    // Additional error codes from w3id.org/security
    #[serde(rename = "https://w3id.org/security#INVALID_PUBLIC_KEY")]
    InvalidPublicKey,
    #[serde(rename = "https://w3id.org/security#INVALID_PUBLIC_KEY_LENGTH")]
    InvalidPublicKeyLength,
    #[serde(rename = "https://w3id.org/security#INVALID_PUBLIC_KEY_TYPE")]
    InvalidPublicKeyType,
    #[serde(rename = "https://w3id.org/security#UNSUPPORTED_PUBLIC_KEY_TYPE")]
    UnsupportedPublicKeyType,

    // Additional error codes from CID specs
    #[serde(rename = "https://w3id.org/security#INVALID_VERIFICATION_METHOD_URL")]
    InvalidVerificationMethodUrl,
    #[serde(rename = "https://w3id.org/security#INVALID_CONTROLLED_IDENTIFIER_DOCUMENT_ID")]
    InvalidControlledIdentifierDocumentId,
    #[serde(rename = "https://w3id.org/security#INVALID_CONTROLLED_IDENTIFIER_DOCUMENT")]
    InvalidControlledIdentifierDocument,
    #[serde(rename = "https://w3id.org/security#INVALID_VERIFICATION_METHOD")]
    InvalidVerificationMethod,
    #[serde(rename = "https://w3id.org/security#INVALID_RELATIONSHIP_FOR_VERIFICATION_METHOD")]
    InvalidRelationshipForVerificationMethod,
}
