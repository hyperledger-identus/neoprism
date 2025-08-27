use identus_apollo::hash::Sha256Digest;

use crate::did::error::{Error as DidError, PublicKeyIdError};
use crate::did::operation::{KeyUsage, PublicKeyId, ServiceId};

#[derive(Debug, derive_more::From, derive_more::Display, derive_more::Error)]
pub enum ProcessError {
    #[from]
    #[display("invalid did operation was processed")]
    DidOperationInvalid { source: DidError },
    #[display("did state initialization requires operation to be CreateOperation")]
    DidStateInitFromNonCreateOperation,
    #[display("failed to update did state: operation is create-operation")]
    DidStateUpdateFromCreateOperation,
    #[display("operation is missing from signed-prism-operation")]
    SignedPrismOperationMissingOperation,
    #[display("invalid signed_with key id in signed-prism-operation")]
    SignedPrismOperationInvalidSignedWith { source: PublicKeyIdError },
    #[display("signed_with key id {id} not found in signed-prism-operation")]
    SignedPrismOperationSignedWithKeyNotFound { id: PublicKeyId },
    #[display("signed_with key id {id} is revoked in signed-prism-operation")]
    SignedPrismOperationSignedWithRevokedKey { id: PublicKeyId },
    #[display("signed_with key id {id} has usage of {usage:?} which is not expected key in signed-prism-operation")]
    SignedPrismOperationSignedWithInvalidKey { id: PublicKeyId, usage: KeyUsage },
    #[display("signature verification failed for signed-prism-operation")]
    SignedPrismOperationInvalidSignature,
    #[from]
    #[display("applied operation has conflict with the current did state")]
    DidStateConflict { source: DidStateConflictError },
}

#[derive(Debug, derive_more::Display, derive_more::Error)]
pub enum DidStateConflictError {
    #[display("applied operation does not have matching previous_operation_hash in the current did state")]
    UnmatchedPreviousOperationHash,
    #[display("failed to add public key: key id {id} already exists in did state")]
    AddPublicKeyWithExistingId { id: PublicKeyId },
    #[display("failed to revoke public key: key id {id} does not exist in did state")]
    RevokePublicKeyNotExists { id: PublicKeyId },
    #[display("failed to revoke public key: key id {id} is already revoked in did state")]
    RevokePublicKeyIsAlreadyRevoked { id: PublicKeyId },
    #[display("failed to add service: service id {id} already exists in did state")]
    AddServiceWithExistingId { id: ServiceId },
    #[display("failed to revoke service: service id {id} does not exist in did state")]
    RevokeServiceNotExists { id: ServiceId },
    #[display("failed to revoke service: service id {id} is already revoked in did state")]
    RevokeServiceIsAlreadyRevoked { id: ServiceId },
    #[display("failed to update service: service id {id} does not exist in did state")]
    UpdateServiceNotExists { id: ServiceId },
    #[display("failed to update service: service id {id} is already revoked in did state")]
    UpdateServiceIsRevoked { id: ServiceId },
    #[display("did state must have at least one master key after update")]
    AfterUpdateMissingMasterKey,
    #[display("did state has {actual} public keys which exceed limit {limit}")]
    AfterUpdatePublicKeyExceedLimit { limit: usize, actual: usize },
    #[display("did state has {actual} services which exceed limit {limit}")]
    AfterUpdateServiceExceedLimit { limit: usize, actual: usize },
    #[display("failed to add storage entry: entry with hash {initial_hash:?} already exists")]
    AddStorageEntryWithExistingHash { initial_hash: Sha256Digest },
    #[display("failed to update storage entry: entry with hash {prev_operation_hash:?} does not exist in did state")]
    UpdateStorageEntryNotExists { prev_operation_hash: Sha256Digest },
    #[display(
        "failed to update storage entry: entry with hash {prev_operation_hash:?} is already revoked in did state"
    )]
    UpdateStorageEntryAlreadyRevoked { prev_operation_hash: Sha256Digest },
    #[display(
        "failed to revoke storage entry since entry with hash {previous_operation_hash:?} does not exist in the did state"
    )]
    RevokeStorageEntryNotExists { previous_operation_hash: Sha256Digest },
    #[display("cannot revoke storage entry: entry with hash {previous_operation_hash:?} is already revoked")]
    RevokeStorageEntryAlreadyRevoked { previous_operation_hash: Sha256Digest },
}
