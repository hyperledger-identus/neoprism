//! Client-safe diagnostic projection. Never walk arbitrary `Error::source` chains here.
//!
//! Domain messages and submitted values are public; crypto/protobuf/hash/base64 leaves
//! are not. Size errors are translated because their Display includes Rust types and
//! source locations. Exhaustive matches make new domain variants require safety review.
use identus_did_prism::did::error::{
    CreateDidOperationError, CreateStorageOperationError, DeactivateDidOperationError, DeactivateStorageOperationError,
    DidSyntaxError, Error, PublicKeyError, PublicKeyIdError, ServiceEndpointError, ServiceError, ServiceIdError,
    ServiceTypeError, UpdateDidOperationError, UpdateStorageOperationError,
};
use identus_did_prism::error::InvalidInputSizeError;
use identus_did_prism::protocol::error::{DidStateConflictError, ProcessError};

use crate::app::service::error::{InvalidDid, ResolutionError};

/// Logs full unexpected causes even when the UI transport is HTML 200.
/// Returned strings are safe to render (with normal HTML escaping).
pub(super) fn resolution(error: &ResolutionError) -> String {
    error.log_internal_error();
    match error {
        ResolutionError::InternalError { .. } | ResolutionError::NotFound | ResolutionError::MethodNotSupported => {
            error.to_string()
        }
        ResolutionError::InvalidDid { source } => {
            let detail = match source {
                InvalidDid::ProcessStateFailed { source } => process(source),
                InvalidDid::InvalidPrismDid { source } => did(source),
            };
            format!("{error}: {detail}")
        }
    }
}

pub(super) fn process(error: &ProcessError) -> String {
    let detail = match error {
        ProcessError::DidOperationInvalid { source } => Some(did(source)),
        ProcessError::SignedPrismOperationInvalidSignedWith { source } => Some(key_id(source)),
        ProcessError::DidStateConflict { source } => Some(conflict(source)),
        ProcessError::DidStateInitFromNonCreateOperation
        | ProcessError::DidStateUpdateFromCreateOperation
        | ProcessError::SignedPrismOperationMissingOperation
        | ProcessError::SignedPrismOperationSignedWithKeyNotFound { .. }
        | ProcessError::SignedPrismOperationSignedWithRevokedKey { .. }
        | ProcessError::SignedPrismOperationSignedWithInvalidKey { .. }
        | ProcessError::SignedPrismOperationInvalidSignature => None,
    };
    join(error, detail)
}

fn join(error: &impl std::fmt::Display, detail: Option<String>) -> String {
    match detail {
        Some(detail) => format!("{error}: {detail}"),
        None => error.to_string(),
    }
}

fn size(error: &InvalidInputSizeError) -> String {
    match error {
        InvalidInputSizeError::NotExact { expected, actual, .. } => {
            format!("expected input size of {expected} but got {actual}")
        }
        InvalidInputSizeError::TooBig { limit, actual, .. } => {
            format!("input size has a limit of {limit} but got {actual}")
        }
    }
}

fn key_id(error: &PublicKeyIdError) -> String {
    join(
        error,
        match error {
            PublicKeyIdError::TooLong { source } => Some(size(source)),
            PublicKeyIdError::Empty | PublicKeyIdError::InvalidUriFragment => None,
        },
    )
}

fn service_id(error: &ServiceIdError) -> String {
    join(
        error,
        match error {
            ServiceIdError::TooLong { source } => Some(size(source)),
            ServiceIdError::Empty | ServiceIdError::InvalidUriFragment => None,
        },
    )
}

fn did(error: &Error) -> String {
    let detail = match error {
        Error::LongFormDidTooLong { source } => Some(size(source)),
        Error::InvalidDidSyntax { source } => Some(syntax(source)),
        Error::CreateDidOperation { source } => Some(create(source)),
        Error::UpdateDidOperation { source } => Some(update(source)),
        Error::DeactivateDidOperation { source } => Some(deactivate(source)),
        Error::CreateStorageOperation { source } => Some(create_storage(source)),
        Error::UpdateStorageOperation { source } => Some(update_storage(source)),
        Error::DeactivateStorageOperation { source } => Some(deactivate_storage(source)),
        Error::LongFormDidNotFromCreateOperation | Error::OperationMissingFromPrismOperation => None,
    };
    join(error, detail)
}

fn syntax(error: &DidSyntaxError) -> String {
    // These Display messages contain submitted DID data only. Deliberately stop
    // before hash, hex, base64 and protobuf sources (which may include internals).
    match error {
        DidSyntaxError::DidSuffixInvalidHex { .. }
        | DidSyntaxError::DidSuffixInvalidStr { .. }
        | DidSyntaxError::DidEncodedStateInvalidStr { .. }
        | DidSyntaxError::DidEncodedStateInvalidProto { .. }
        | DidSyntaxError::DidSyntaxInvalid { .. }
        | DidSyntaxError::DidSuffixEncodedStateUnmatched { .. } => error.to_string(),
    }
}

fn create(error: &CreateDidOperationError) -> String {
    let detail = match error {
        CreateDidOperationError::InvalidPublicKey { source } => Some(public_key(source)),
        CreateDidOperationError::InvalidService { source } => Some(service(source)),
        CreateDidOperationError::TooManyPublicKeys { source }
        | CreateDidOperationError::TooManyServices { source }
        | CreateDidOperationError::TooManyContexts { source } => Some(size(source)),
        CreateDidOperationError::MissingDidData
        | CreateDidOperationError::MissingMasterKey
        | CreateDidOperationError::DuplicateContext
        | CreateDidOperationError::InvalidContext { .. } => None,
    };
    join(error, detail)
}

fn update(error: &UpdateDidOperationError) -> String {
    let detail = match error {
        UpdateDidOperationError::InvalidDidSyntax { source } => Some(syntax(source)),
        UpdateDidOperationError::InvalidPublicKey { source } => Some(public_key(source)),
        UpdateDidOperationError::InvalidService { source } => Some(service(source)),
        UpdateDidOperationError::TooManyContexts { source } => Some(size(source)),
        // Hash source is a crypto-library diagnostic, not a domain explanation.
        UpdateDidOperationError::InvalidPreviousOperationHash { .. }
        | UpdateDidOperationError::EmptyAction
        | UpdateDidOperationError::MissingUpdateActionData { .. }
        | UpdateDidOperationError::DuplicateContext
        | UpdateDidOperationError::InvalidContext { .. } => None,
    };
    join(error, detail)
}

fn deactivate(error: &DeactivateDidOperationError) -> String {
    join(
        error,
        match error {
            DeactivateDidOperationError::InvalidDidSyntax { source } => Some(syntax(source)),
            DeactivateDidOperationError::InvalidPreviousOperationHash { .. } => None,
        },
    )
}

fn public_key(error: &PublicKeyError) -> String {
    join(
        error,
        match error {
            PublicKeyError::InvalidKeyId { source, .. } => Some(key_id(source)),
            // InvalidKeyData's crypto source is intentionally not rendered.
            PublicKeyError::InvalidKeyData { .. }
            | PublicKeyError::MissingKeyData { .. }
            | PublicKeyError::UnknownKeyUsage { .. }
            | PublicKeyError::MasterKeyNotSecp256k1 { .. }
            | PublicKeyError::VdrKeyNotSecp256k1 { .. }
            | PublicKeyError::UnsupportedCurve { .. } => None,
        },
    )
}

fn service(error: &ServiceError) -> String {
    let detail = match error {
        ServiceError::InvalidServiceId { source, .. } => service_id(source),
        ServiceError::InvalidServiceType { source, .. } => match source {
            ServiceTypeError::ExceedMaxSize { .. } | ServiceTypeError::Empty | ServiceTypeError::InvalidSyntax => {
                source.to_string()
            }
        },
        ServiceError::InvalidServiceEndpoint { source, .. } => match source {
            ServiceEndpointError::ExceedMaxSize { .. }
            | ServiceEndpointError::Empty
            | ServiceEndpointError::InvalidSyntax => source.to_string(),
        },
    };
    join(error, Some(detail))
}

fn create_storage(error: &CreateStorageOperationError) -> String {
    join(
        error,
        match error {
            CreateStorageOperationError::InvalidDidSyntax { source } => Some(syntax(source)),
            CreateStorageOperationError::EmptyStorageData => None,
        },
    )
}

fn update_storage(error: &UpdateStorageOperationError) -> String {
    match error {
        UpdateStorageOperationError::InvalidPreviousOperationHash { .. }
        | UpdateStorageOperationError::EmptyStorageData => error.to_string(),
    }
}

fn deactivate_storage(error: &DeactivateStorageOperationError) -> String {
    match error {
        DeactivateStorageOperationError::InvalidPreviousOperationHash { .. } => error.to_string(),
    }
}

fn conflict(error: &DidStateConflictError) -> String {
    // These variants contain public ledger IDs, hashes and domain counts only;
    // there are no nested implementation errors.
    match error {
        DidStateConflictError::UnmatchedPreviousOperationHash
        | DidStateConflictError::AddPublicKeyWithExistingId { .. }
        | DidStateConflictError::RevokePublicKeyNotExists { .. }
        | DidStateConflictError::RevokePublicKeyIsAlreadyRevoked { .. }
        | DidStateConflictError::AddServiceWithExistingId { .. }
        | DidStateConflictError::RevokeServiceNotExists { .. }
        | DidStateConflictError::RevokeServiceIsAlreadyRevoked { .. }
        | DidStateConflictError::UpdateServiceNotExists { .. }
        | DidStateConflictError::UpdateServiceIsRevoked { .. }
        | DidStateConflictError::AfterUpdateMissingMasterKey
        | DidStateConflictError::AfterUpdatePublicKeyExceedLimit { .. }
        | DidStateConflictError::AfterUpdateServiceExceedLimit { .. }
        | DidStateConflictError::AddStorageEntryWithExistingHash { .. }
        | DidStateConflictError::UpdateStorageEntryNotExists { .. }
        | DidStateConflictError::UpdateStorageEntryAlreadyRevoked { .. }
        | DidStateConflictError::RevokeStorageEntryNotExists { .. }
        | DidStateConflictError::RevokeStorageEntryAlreadyRevoked { .. } => error.to_string(),
    }
}
