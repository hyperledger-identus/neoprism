//! Explicit compatibility projections into the generic sdk-rust DID model.
//!
//! NeoPRISM keeps its established public DTOs while the beta branch adopts
//! sdk-rust one boundary at a time. These projections use the shared JSON wire
//! contract so sdk-rust remains the final validation and resource-boundary
//! authority without exposing its private implementation dependencies.

use crate::{Did, DidDocument, ResolutionOptions, ResolutionResult};

/// Stable failure categories for the NeoPRISM-to-sdk-rust compatibility edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, derive_more::Display, derive_more::Error)]
pub enum SdkProjectionError {
    /// A legacy NeoPRISM DTO could not be serialized to its wire form.
    #[display("could not encode the NeoPRISM DID value")]
    EncodeNeoPrism,
    /// An sdk-rust DID could not be represented by the legacy NeoPRISM facade.
    #[display("could not project the sdk-rust DID into NeoPRISM")]
    InvalidNeoPrismDid,
    /// sdk-rust resolution options could not be represented by the facade.
    #[display("could not project sdk-rust resolution options into NeoPRISM")]
    InvalidNeoPrismResolutionOptions,
    /// The legacy document does not satisfy the sdk-rust DID document contract.
    #[display("NeoPRISM DID document does not satisfy the sdk-rust contract")]
    InvalidSdkDidDocument,
    /// The legacy result does not satisfy the sdk-rust DID resolution contract.
    #[display("NeoPRISM DID resolution result does not satisfy the sdk-rust contract")]
    InvalidSdkResolutionResult,
}

/// Project one validated sdk-rust DID into the NeoPRISM compatibility facade.
pub fn did_from_sdk(value: &identus_did::Did) -> Result<Did, SdkProjectionError> {
    value
        .as_str()
        .parse()
        .map_err(|_| SdkProjectionError::InvalidNeoPrismDid)
}

/// Project sdk-rust resolution options into NeoPRISM's current DTO.
pub fn resolution_options_from_sdk(
    value: &identus_did::ResolutionOptions,
) -> Result<ResolutionOptions, SdkProjectionError> {
    let wire = serde_json::to_vec(value).map_err(|_| SdkProjectionError::EncodeNeoPrism)?;
    serde_json::from_slice(&wire).map_err(|_| SdkProjectionError::InvalidNeoPrismResolutionOptions)
}

/// Validate and project a NeoPRISM DID document into the generic SDK model.
pub fn did_document_to_sdk(value: &DidDocument) -> Result<identus_did::DidDocument, SdkProjectionError> {
    let wire = serde_json::to_vec(value).map_err(|_| SdkProjectionError::EncodeNeoPrism)?;
    identus_did::DidDocument::from_json_slice(&wire).map_err(|_| SdkProjectionError::InvalidSdkDidDocument)
}

/// Validate and project a NeoPRISM resolution result into the generic SDK model.
pub fn resolution_result_to_sdk(
    value: &ResolutionResult,
) -> Result<identus_did::DidResolutionResult, SdkProjectionError> {
    let wire = serde_json::to_vec(value).map_err(|_| SdkProjectionError::EncodeNeoPrism)?;
    identus_did::DidResolutionResult::from_json_slice(&wire).map_err(|_| SdkProjectionError::InvalidSdkResolutionResult)
}
