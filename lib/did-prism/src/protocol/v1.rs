use identus_apollo::crypto::Verifiable;
use protobuf::SpecialFields;

use super::{DidStateConflictError, DidStateRc, OperationProcessor, OperationProcessorOps, ProcessError};
use crate::did::Error as DidError;
use crate::did::operation::{
    CreateDidOperation, CreateStorageOperation, DeactivateDidOperation, DeactivateStorageOperation, KeyUsage,
    OperationParameters, PublicKeyData, PublicKeyId, UpdateDidOperation, UpdateOperationAction, UpdateStorageOperation,
};
use crate::dlt::OperationMetadata;
use crate::prelude::*;
use crate::proto::prism::prism_operation::Operation;
use crate::proto::prism_ssi::{ProtoCreateDID, ProtoDeactivateDID, ProtoUpdateDID};
use crate::proto::prism_storage::{ProtoCreateStorageEntry, ProtoDeactivateStorageEntry, ProtoUpdateStorageEntry};
use crate::proto::prism_version::ProtoProtocolVersionUpdate;

#[derive(Debug, Clone)]
pub struct V1Processor {
    parameters: OperationParameters,
}

impl Default for V1Processor {
    fn default() -> Self {
        Self {
            parameters: OperationParameters::v1(),
        }
    }
}

impl OperationProcessorOps for V1Processor {
    fn check_signature(&self, state: &DidStateRc, signed_operation: &SignedPrismOperation) -> Result<(), ProcessError> {
        let key_id = PublicKeyId::parse(&signed_operation.signed_with, self.parameters.max_id_size)
            .map_err(|e| ProcessError::SignedPrismOperationInvalidSignedWith { source: e })?;

        let Some(pk) = state.public_keys.get(&key_id) else {
            Err(ProcessError::SignedPrismOperationSignedWithKeyNotFound { id: key_id })?
        };

        if pk.is_revoked() {
            Err(ProcessError::SignedPrismOperationSignedWithRevokedKey { id: key_id.clone() })?
        }

        let operation = signed_operation
            .operation
            .as_ref()
            .ok_or(ProcessError::SignedPrismOperationMissingOperation)?;
        let operation_inner = operation
            .operation
            .as_ref()
            .ok_or(ProcessError::SignedPrismOperationMissingOperation)?;
        let is_storage_entry = matches!(
            operation_inner,
            Operation::CreateStorageEntry(_) | Operation::UpdateStorageEntry(_) | Operation::DeactivateStorageEntry(_)
        );

        match &pk.get().data {
            PublicKeyData::Master { data } if !is_storage_entry => {
                let signature = signed_operation.signature.as_slice();
                let message = operation.encode_to_vec();
                if !data.verify(&message, signature) {
                    Err(ProcessError::SignedPrismOperationInvalidSignature)?
                }
            }
            PublicKeyData::Vdr { data } if is_storage_entry => {
                let signature = signed_operation.signature.as_slice();
                let message = operation.encode_to_vec();
                if !data.verify(&message, signature) {
                    Err(ProcessError::SignedPrismOperationInvalidSignature)?
                }
            }
            pk => Err(ProcessError::SignedPrismOperationSignedWithInvalidKey {
                id: key_id,
                usage: pk.usage(),
            })?,
        }

        Ok(())
    }

    fn create_did(
        &self,
        state: &DidStateRc,
        metadata: OperationMetadata,
        operation: ProtoCreateDID,
        _prism_operation_special_fields: SpecialFields,
    ) -> Result<DidStateRc, ProcessError> {
        let parsed_operation = CreateDidOperation::parse(&self.parameters, &operation).map_err(DidError::from)?;

        // clone and mutate candidate state
        let mut candidate_state = state.clone();
        candidate_state.with_context(parsed_operation.context);
        candidate_state.with_last_operation_hash(state.did.suffix.clone());
        for pk in parsed_operation.public_keys {
            candidate_state.add_public_key(pk, &metadata)?;
        }
        for service in parsed_operation.services {
            candidate_state.add_service(service, &metadata)?;
        }

        CreateDidValidator::validate_candidate_state(&self.parameters, &candidate_state)?;
        Ok(candidate_state)
    }

    fn update_did(
        &self,
        state: &DidStateRc,
        metadata: OperationMetadata,
        operation: ProtoUpdateDID,
        prism_operation_special_fields: SpecialFields,
    ) -> Result<DidStateRc, ProcessError> {
        let parsed_operation = UpdateDidOperation::parse(&self.parameters, &operation).map_err(DidError::from)?;
        if parsed_operation.prev_operation_hash != *state.prev_operation_hash {
            Err(DidStateConflictError::UnmatchedPreviousOperationHash)?
        }

        // clone and mutate candidate state
        let mut candidate_state = state.clone();
        let prism_operation = PrismOperation {
            operation: Some(Operation::UpdateDid(operation)),
            special_fields: prism_operation_special_fields,
        };
        candidate_state.with_last_operation_hash(prism_operation.operation_hash());
        for action in parsed_operation.actions {
            apply_update_action(&mut candidate_state, action, &metadata)?;
        }

        UpdateDidValidator::validate_candidate_state(&self.parameters, &candidate_state)?;
        Ok(candidate_state)
    }

    fn deactivate_did(
        &self,
        state: &DidStateRc,
        metadata: OperationMetadata,
        operation: ProtoDeactivateDID,
        prism_operation_special_fields: SpecialFields,
    ) -> Result<DidStateRc, ProcessError> {
        let parsed_operation = DeactivateDidOperation::parse(&operation).map_err(DidError::from)?;
        if parsed_operation.prev_operation_hash != *state.prev_operation_hash {
            Err(DidStateConflictError::UnmatchedPreviousOperationHash)?
        }

        // clone and mutate candidate state
        let mut candidate_state = state.clone();
        let prism_operation = PrismOperation {
            operation: Some(Operation::DeactivateDid(operation)),
            special_fields: prism_operation_special_fields,
        };
        let operation_hash = prism_operation.operation_hash();
        for (id, pk) in &state.public_keys {
            if !pk.is_revoked() {
                candidate_state.revoke_public_key(id, &metadata)?;
            }
        }
        for (id, s) in &state.services {
            if !s.is_revoked() {
                candidate_state.revoke_service(id, &metadata)?;
            }
        }
        for (_, s) in &state.storage {
            if !s.is_revoked() {
                let prev_operation_hash = s.get().prev_operation_hash.clone();
                candidate_state.revoke_storage(&prev_operation_hash, &operation_hash, &metadata)?;
            }
        }
        candidate_state.with_last_operation_hash(operation_hash);

        DeactivateDidValidator::validate_candidate_state(&self.parameters, &candidate_state)?;
        Ok(candidate_state)
    }

    fn protocol_version_update(
        &self,
        _: OperationMetadata,
        _: ProtoProtocolVersionUpdate,
        _: SpecialFields,
    ) -> Result<OperationProcessor, ProcessError> {
        // TODO: add support for protocol version update
        tracing::warn!("Protocol version update is not yet supported");
        Ok(self.clone().into())
    }

    fn create_storage(
        &self,
        state: &DidStateRc,
        metadata: OperationMetadata,
        operation: ProtoCreateStorageEntry,
        prism_operation_special_fields: SpecialFields,
    ) -> Result<DidStateRc, ProcessError> {
        let parsed_operation = CreateStorageOperation::parse(&operation).map_err(DidError::from)?;

        // clone and mutate candidate state
        let mut candidate_state = state.clone();
        let prism_operation = PrismOperation {
            operation: Some(Operation::CreateStorageEntry(operation)),
            special_fields: prism_operation_special_fields,
        };
        let operation_hash = prism_operation.operation_hash();
        candidate_state.add_storage(&operation_hash, parsed_operation.data, &metadata)?;
        // NOTE: Storage operations do NOT update the DID's prev_operation_hash.
        // SSI chain (CreateDID/UpdateDID/DeactivateDID) and storage chains are independent per spec.

        UpdateDidValidator::validate_candidate_state(&self.parameters, &candidate_state)?;
        Ok(candidate_state)
    }

    fn update_storage(
        &self,
        state: &DidStateRc,
        _metadata: OperationMetadata,
        operation: ProtoUpdateStorageEntry,
        prism_operation_special_fields: SpecialFields,
    ) -> Result<DidStateRc, ProcessError> {
        let parsed_operation = UpdateStorageOperation::parse(&operation).map_err(DidError::from)?;

        // clone and mutate candidate state
        let mut candidate_state = state.clone();
        let prism_operation = PrismOperation {
            operation: Some(Operation::UpdateStorageEntry(operation)),
            special_fields: prism_operation_special_fields,
        };
        let operation_hash = prism_operation.operation_hash();
        candidate_state.update_storage(
            &parsed_operation.prev_event_hash,
            &operation_hash,
            parsed_operation.data,
        )?;
        // NOTE: Storage operations do NOT update the DID's prev_operation_hash.

        UpdateDidValidator::validate_candidate_state(&self.parameters, &candidate_state)?;
        Ok(candidate_state)
    }

    fn deactivate_storage(
        &self,
        state: &DidStateRc,
        metadata: OperationMetadata,
        operation: ProtoDeactivateStorageEntry,
        prism_operation_special_fields: SpecialFields,
    ) -> Result<DidStateRc, ProcessError> {
        let parsed_operation = DeactivateStorageOperation::parse(&operation).map_err(DidError::from)?;

        // clone and mutate candidate state
        let mut candidate_state = state.clone();
        let prism_operation = PrismOperation {
            operation: Some(Operation::DeactivateStorageEntry(operation)),
            special_fields: prism_operation_special_fields,
        };
        let operation_hash = prism_operation.operation_hash();
        candidate_state.revoke_storage(&parsed_operation.prev_operation_hash, &operation_hash, &metadata)?;
        // NOTE: Storage operations do NOT update the DID's prev_operation_hash.

        UpdateDidValidator::validate_candidate_state(&self.parameters, &candidate_state)?;
        Ok(candidate_state)
    }
}

trait Validator {
    fn validate_candidate_state(param: &OperationParameters, state: &DidStateRc) -> Result<(), ProcessError>;
}

struct CreateDidValidator;
struct UpdateDidValidator;
struct DeactivateDidValidator;

impl Validator for CreateDidValidator {
    fn validate_candidate_state(_: &OperationParameters, _: &DidStateRc) -> Result<(), ProcessError> {
        Ok(())
    }
}

impl Validator for UpdateDidValidator {
    fn validate_candidate_state(param: &OperationParameters, state: &DidStateRc) -> Result<(), ProcessError> {
        // check at least one master key exists
        let contains_master_key = state
            .public_keys
            .iter()
            .any(|(_, pk)| !pk.is_revoked() && pk.get().data.usage() == KeyUsage::MasterKey);
        if !contains_master_key {
            Err(DidStateConflictError::AfterUpdateMissingMasterKey)?
        }

        // check public key count does not exceed limit
        if state.public_keys.len() > param.max_public_keys {
            Err(DidStateConflictError::AfterUpdatePublicKeyExceedLimit {
                limit: param.max_public_keys,
                actual: state.public_keys.len(),
            })?
        }

        // check service count does not exeed limit
        if state.services.len() > param.max_services {
            Err(DidStateConflictError::AfterUpdateServiceExceedLimit {
                limit: param.max_services,
                actual: state.services.len(),
            })?
        }

        Ok(())
    }
}

impl Validator for DeactivateDidValidator {
    fn validate_candidate_state(_: &OperationParameters, _: &DidStateRc) -> Result<(), ProcessError> {
        Ok(())
    }
}

fn apply_update_action(
    state: &mut DidStateRc,
    action: UpdateOperationAction,
    metadata: &OperationMetadata,
) -> Result<(), DidStateConflictError> {
    match action {
        UpdateOperationAction::AddKey(pk) => state.add_public_key(pk, metadata)?,
        UpdateOperationAction::RemoveKey(id) => state.revoke_public_key(&id, metadata)?,
        UpdateOperationAction::AddService(service) => state.add_service(service, metadata)?,
        UpdateOperationAction::RemoveService(id) => state.revoke_service(&id, metadata)?,
        UpdateOperationAction::UpdateService {
            id,
            r#type,
            service_endpoints,
        } => {
            if let Some(t) = r#type {
                state.update_service_type(&id, t)?;
            }
            if let Some(ep) = service_endpoints {
                state.update_service_endpoint(&id, ep)?;
            }
        }
        UpdateOperationAction::PatchContext(ctx) => {
            state.with_context(ctx);
        }
    }

    Ok(())
}
