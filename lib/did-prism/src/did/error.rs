use identus_apollo::hex::HexStr;

use super::CanonicalPrismDid;
use super::operation::PublicKeyId;
use crate::error::InvalidInputSizeError;

#[derive(Debug, derive_more::From, derive_more::Display, derive_more::Error)]
pub enum Error {
    #[display("operation type provided when creating a long-form DID is not CreateDidOperation")]
    LongFormDidNotFromCreateOperation,
    #[display("operation does not exist in PrismOperation")]
    OperationMissingFromPrismOperation,
    #[from]
    #[display("did syntax is invalid")]
    InvalidDidSyntax { source: DidSyntaxError },
    #[from]
    #[display("failed to create did operation")]
    CreateDidOperation { source: CreateDidOperationError },
    #[from]
    #[display("failed to update did operation")]
    UpdateDidOperation { source: UpdateDidOperationError },
    #[from]
    #[display("failed to deactivate did operation")]
    DeactivateDidOperation { source: DeactivateDidOperationError },
    #[from]
    #[display("failed to create storage operation")]
    CreateStorageOperation { source: CreateStorageOperationError },
    #[from]
    #[display("failed to update storage operation")]
    UpdateStorageOperation { source: UpdateStorageOperationError },
    #[from]
    #[display("failed to deactivate storage operation")]
    DeactivateStorageOperation { source: DeactivateStorageOperationError },
}

#[derive(Debug, derive_more::Display, derive_more::Error)]
pub enum DidSyntaxError {
    #[display("did suffix {suffix} has invalid length")]
    DidSuffixInvalidHex {
        source: identus_apollo::hash::Error,
        suffix: HexStr,
    },
    #[display("did suffix {suffix} is invalid")]
    DidSuffixInvalidStr {
        source: identus_apollo::hex::Error,
        suffix: String,
    },
    #[display("did encoded state {encoded_state} is invalid")]
    DidEncodedStateInvalidStr {
        source: identus_apollo::base64::Error,
        encoded_state: String,
    },
    #[display("did suffix {did} cannot be decoded into protobuf message")]
    DidEncodedStateInvalidProto { source: protobuf::Error, did: String },
    #[display("unrecognized did pattern {did}")]
    DidSyntaxInvalid {
        #[error(not(source))]
        did: String,
    },
    #[display("encoded state and did suffix do not match for {did} (expected {expected_did})")]
    DidSuffixEncodedStateUnmatched {
        did: String,
        expected_did: CanonicalPrismDid,
    },
}

#[derive(Debug, derive_more::From, derive_more::Display, derive_more::Error)]
pub enum CreateDidOperationError {
    #[display("missing did_data in CreateDidOperation")]
    MissingDidData,
    #[display("master key is missing in CreateDidOperation")]
    MissingMasterKey,
    #[from]
    #[display("invalid public key found in CreateDidOperation")]
    InvalidPublicKey { source: PublicKeyError },
    #[from]
    #[display("invalid service found in CreateDidOperation")]
    InvalidService { source: ServiceError },
    #[display("invalid input size for public keys")]
    TooManyPublicKeys { source: InvalidInputSizeError },
    #[display("invalid input size for services")]
    TooManyServices { source: InvalidInputSizeError },
    #[display("duplicate context found in CreateDidOperation")]
    DuplicateContext,
}

#[derive(Debug, derive_more::From, derive_more::Display, derive_more::Error)]
pub enum UpdateDidOperationError {
    #[display("update action does not exist in UpdateDidOperation")]
    EmptyAction,
    #[from]
    #[display("invalid previous operation hash in UpdateDidOperation")]
    InvalidPreviousOperationHash { source: identus_apollo::hash::Error },
    #[from]
    #[display("did provided in UpdateDidOperation is invalid")]
    InvalidDidSyntax { source: DidSyntaxError },
    #[display("update action type '{action_type}' in UpdateDidOperation is missing field '{field_name}'")]
    MissingUpdateActionData {
        action_type: &'static str,
        field_name: &'static str,
    },
    #[from]
    #[display("invalid public key found in UpdateDidOperation")]
    InvalidPublicKey { source: PublicKeyError },
    #[from]
    #[display("invalid service found in UpdateDidOperation")]
    InvalidService { source: ServiceError },
    #[display("duplicate context found in UpdateDidOperation")]
    DuplicateContext,
}

#[derive(Debug, derive_more::From, derive_more::Display, derive_more::Error)]
pub enum DeactivateDidOperationError {
    #[display("invalid previous operation hash in DeactivateDidOperation")]
    InvalidPreviousOperationHash { source: identus_apollo::hash::Error },
    #[from]
    #[display("did provided in DeactivateDidOperation is invalid")]
    InvalidDidSyntax { source: DidSyntaxError },
}

#[derive(Debug, derive_more::From, derive_more::Display, derive_more::Error)]
pub enum PublicKeyError {
    #[from]
    #[display("public key id {id} is invalid")]
    InvalidKeyId { source: PublicKeyIdError, id: String },
    #[display("key data for key id {id} is missing")]
    MissingKeyData {
        #[error(not(source))]
        id: PublicKeyId,
    },
    #[display("unknown key usage for key id {id}")]
    UnknownKeyUsage {
        #[error(not(source))]
        id: PublicKeyId,
    },
    #[display("master key id {id} does not have key type of secp256k1")]
    MasterKeyNotSecp256k1 {
        #[error(not(source))]
        id: PublicKeyId,
    },
    #[display("vdr key id {id} does not have key type of secp256k1")]
    VdrKeyNotSecp256k1 {
        #[error(not(source))]
        id: PublicKeyId,
    },
    #[display("unable to parse key data to a public key for id {id}")]
    InvalidKeyData {
        source: identus_apollo::crypto::Error,
        id: PublicKeyId,
    },
    #[display("unsupported curve for key id {id}")]
    UnsupportedCurve { id: PublicKeyId },
}

#[derive(Debug, derive_more::Display, derive_more::Error)]
pub enum PublicKeyIdError {
    #[display("public key id is empty")]
    Empty,
    #[display("public key id is not a valid uri fragment")]
    InvalidUriFragment,
    #[display("public key id is too long")]
    TooLong { source: InvalidInputSizeError },
}

#[derive(Debug, derive_more::From, derive_more::Display, derive_more::Error)]
pub enum ServiceError {
    #[from]
    #[display("service id {id} is invalid")]
    InvalidServiceId { source: ServiceIdError, id: String },
    #[from]
    #[display("invalid service type {type_name}")]
    InvalidServiceType {
        source: ServiceTypeError,
        type_name: String,
    },
    #[from]
    #[display("service endpoint {endpoint} is invalid")]
    InvalidServiceEndpoint {
        source: ServiceEndpointError,
        endpoint: String,
    },
}

#[derive(Debug, derive_more::Display, derive_more::Error)]
pub enum ServiceIdError {
    #[display("service id is empty")]
    Empty,
    #[display("service id is not a valid uri fragment")]
    InvalidUriFragment,
    #[display("service id is too long")]
    TooLong { source: InvalidInputSizeError },
}

#[derive(Debug, derive_more::Display, derive_more::Error)]
pub enum ServiceTypeError {
    #[display("service type exceeds max size of {limit}")]
    ExceedMaxSize {
        #[error(not(source))]
        limit: usize,
    },
    #[display("service type is empty")]
    Empty,
    #[display("service type does not conform to the syntax")]
    InvalidSyntax,
}

#[derive(Debug, derive_more::Display, derive_more::Error)]
pub enum ServiceEndpointError {
    #[display("service endpoint exceeds max size of {limit}")]
    ExceedMaxSize {
        #[error(not(source))]
        limit: usize,
    },
    #[display("service endpoint is empty")]
    Empty,
    #[display("service endpoint does not conform to the syntax")]
    InvalidSyntax,
}

#[derive(Debug, derive_more::From, derive_more::Display, derive_more::Error)]
pub enum CreateStorageOperationError {
    #[from]
    #[display("did provided in CreateStorageOperation is invalid")]
    InvalidDidSyntax { source: DidSyntaxError },
    #[display("missing storage data in CreateStorageOperation")]
    EmptyStorageData,
}

#[derive(Debug, derive_more::From, derive_more::Display, derive_more::Error)]
pub enum UpdateStorageOperationError {
    #[display("invalid previous operation hash in UpdateStorageOperation")]
    InvalidPreviousOperationHash { source: identus_apollo::hash::Error },
    #[display("missing storage data in UpdateStorageOperation")]
    EmptyStorageData,
}

#[derive(Debug, derive_more::From, derive_more::Display, derive_more::Error)]
pub enum DeactivateStorageOperationError {
    #[display("invalid previous operation hash in UpdateStorageOperation")]
    InvalidPreviousOperationHash { source: identus_apollo::hash::Error },
}
