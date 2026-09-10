//! Explicit compatibility projections into the generic sdk-rust DID model.
//!
//! NeoPRISM keeps its established public DTOs while the beta branch adopts
//! sdk-rust one boundary at a time. These projections use the shared JSON wire
//! contract so sdk-rust remains the final validation and resource-boundary
//! authority without exposing its private implementation dependencies.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

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
    /// The SDK request uses options the current NeoPRISM resolver cannot honor.
    #[display("sdk-rust resolution options are not supported by NeoPRISM")]
    UnsupportedSdkResolutionOptions,
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
    if value.no_cache().is_some() || !value.extensions().is_empty() {
        return Err(SdkProjectionError::UnsupportedSdkResolutionOptions);
    }
    let wire = serde_json::to_vec(value).map_err(|_| SdkProjectionError::EncodeNeoPrism)?;
    serde_json::from_slice(&wire).map_err(|_| SdkProjectionError::InvalidNeoPrismResolutionOptions)
}

/// Validate and project a NeoPRISM DID document into the generic SDK model.
pub fn did_document_to_sdk(value: &DidDocument) -> Result<identus_did::DidDocument, SdkProjectionError> {
    let mut wire = to_wire_value(value)?;
    normalize_legacy_document(&mut wire);
    let wire = serde_json::to_vec(&wire).map_err(|_| SdkProjectionError::EncodeNeoPrism)?;
    identus_did::DidDocument::from_json_slice(&wire).map_err(|_| SdkProjectionError::InvalidSdkDidDocument)
}

/// Validate and project a NeoPRISM resolution result into the generic SDK model.
pub fn resolution_result_to_sdk(
    value: &ResolutionResult,
) -> Result<identus_did::DidResolutionResult, SdkProjectionError> {
    let mut wire = to_wire_value(value)?;
    if let Some(document) = wire.get_mut("didDocument") {
        normalize_legacy_document(document);
    }
    let wire = serde_json::to_vec(&wire).map_err(|_| SdkProjectionError::EncodeNeoPrism)?;
    identus_did::DidResolutionResult::from_json_slice(&wire).map_err(|_| SdkProjectionError::InvalidSdkResolutionResult)
}

/// Present an existing NeoPRISM resolver through the generic sdk-rust port.
///
/// The adapter keeps PRISM resolution and persistence downstream while making
/// its validated results consumable by sdk-rust clients. Unsupported sdk-rust
/// options fail explicitly instead of being ignored.
#[derive(Debug)]
pub struct SdkDidResolverAdapter<R> {
    inner: R,
}

impl<R> SdkDidResolverAdapter<R> {
    /// Wrap a NeoPRISM resolver without changing its public facade.
    #[must_use]
    pub const fn new(inner: R) -> Self {
        Self { inner }
    }

    /// Return the wrapped resolver.
    #[must_use]
    pub const fn inner(&self) -> &R {
        &self.inner
    }

    /// Consume the adapter and return the wrapped resolver.
    #[must_use]
    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<R> identus_did::DidResolver for SdkDidResolverAdapter<R>
where
    R: crate::DidResolver + Send + Sync,
{
    fn resolve<'a>(
        &'a self,
        did: &'a identus_did::Did,
        options: &'a identus_did::ResolutionOptions,
    ) -> identus_did::DidResolutionFuture<'a> {
        Box::pin(async move {
            let did = match did_from_sdk(did) {
                Ok(did) => did,
                Err(_) => return sdk_failure(identus_did::DidResolutionErrorKind::InvalidDid),
            };
            let options = match resolution_options_from_sdk(options) {
                Ok(options) => options,
                Err(_) => {
                    return sdk_failure(identus_did::DidResolutionErrorKind::InvalidOptions);
                }
            };
            let result = self.inner.resolve(&did, &options).await;
            resolution_result_to_sdk(&result)
                .unwrap_or_else(|_| sdk_failure(identus_did::DidResolutionErrorKind::InternalError))
        })
    }
}

fn sdk_failure(kind: identus_did::DidResolutionErrorKind) -> identus_did::DidResolutionResult {
    let metadata = identus_did::DidResolutionMetadata::new(
        None,
        Some(identus_did::DidResolutionError::standard(kind)),
        BTreeMap::new(),
    )
    .expect("a standard SDK resolution error is valid");
    identus_did::DidResolutionResult::failure(metadata).expect("standard SDK failure metadata forms a valid result")
}

fn to_wire_value(value: &impl Serialize) -> Result<Value, SdkProjectionError> {
    serde_json::to_value(value).map_err(|_| SdkProjectionError::EncodeNeoPrism)
}

fn normalize_legacy_document(value: &mut Value) {
    let Some(document) = value.as_object_mut() else {
        return;
    };
    for name in [
        "alsoKnownAs",
        "verificationMethod",
        "authentication",
        "assertionMethod",
        "keyAgreement",
        "capabilityInvocation",
        "capabilityDelegation",
        "service",
    ] {
        let should_remove = document
            .get(name)
            .is_some_and(|value| value.is_null() || value.as_array().is_some_and(Vec::is_empty));
        if should_remove {
            document.remove(name);
        }
    }
}
