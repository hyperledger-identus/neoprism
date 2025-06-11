use std::marker::PhantomData;
use std::ops::Deref;
use std::rc::Rc;

use chrono::DateTime;
use enum_dispatch::enum_dispatch;
use error::{DidStateConflictError, ProcessError};
use identus_apollo::hash::Sha256Digest;

use self::v1::V1Processor;
use crate::did::operation::{PublicKey, PublicKeyId, Service, ServiceEndpoint, ServiceId, ServiceType, StorageData};
use crate::did::{CanonicalPrismDid, DidState, StorageState};
use crate::dlt::{BlockMetadata, OperationMetadata};
use crate::prelude::PrismOperation;
use crate::proto::prism_operation::Operation;
use crate::proto::{
    ProtoCreateDid, ProtoCreateStorageEntry, ProtoDeactivateDid, ProtoDeactivateStorageEntry,
    ProtoProtocolVersionUpdate, ProtoUpdateDid, ProtoUpdateStorageEntry, SignedPrismOperation,
};

pub mod error;
pub mod resolver;
mod v1;

#[derive(Debug, Clone)]
pub struct ProtocolParameter {
    pub max_services: usize,
    pub max_public_keys: usize,
    pub max_id_size: usize,
    pub max_type_size: usize,
    pub max_service_endpoint_size: usize,
}

impl ProtocolParameter {
    fn v1() -> Self {
        Self {
            max_services: 50,
            max_public_keys: 50,
            max_id_size: 50,
            max_type_size: 100,
            max_service_endpoint_size: 300,
        }
    }
}

#[derive(Debug, Clone)]
struct Revocable<T> {
    inner: T,
    #[allow(unused)]
    added_at: OperationMetadata,
    revoked_at: Option<OperationMetadata>,
}

impl<T> Revocable<T> {
    fn new(item: T, added_at: &OperationMetadata) -> Self {
        Self {
            inner: item,
            added_at: added_at.clone(),
            revoked_at: None,
        }
    }

    fn is_revoked(&self) -> bool {
        self.revoked_at.is_some()
    }

    fn revoke(&mut self, revoked_at: &OperationMetadata) {
        self.revoked_at = Some(revoked_at.clone());
    }

    fn into_item(self) -> T {
        self.inner
    }

    fn get(&self) -> &T {
        &self.inner
    }

    fn get_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

type InternalMap<K, V> = im_rc::HashMap<K, Revocable<V>>;

/// A struct optimized for mutating DID state when processing an operation.
#[derive(Debug, Clone)]
struct DidStateRc {
    did: Rc<CanonicalPrismDid>,
    context: Rc<Vec<String>>,
    prev_operation_hash: Rc<Sha256Digest>,
    public_keys: InternalMap<PublicKeyId, PublicKey>,
    services: InternalMap<ServiceId, Service>,
    /// Mapping of initial_operation_hash and the storage state
    storage: InternalMap<Sha256Digest, StorageStateRc>,
}

#[derive(Debug, Clone)]
struct StorageStateRc {
    prev_operation_hash: Rc<Sha256Digest>,
    data: Rc<StorageData>,
}

impl DidStateRc {
    fn new(did: CanonicalPrismDid) -> Self {
        let last_operation_hash = did.suffix.clone();
        Self {
            did: Rc::new(did),
            prev_operation_hash: Rc::new(last_operation_hash),
            context: Default::default(),
            public_keys: Default::default(),
            services: Default::default(),
            storage: Default::default(),
        }
    }

    fn with_context(&mut self, context: Vec<String>) {
        self.context = context.into();
    }

    fn with_last_operation_hash(&mut self, last_operation_hash: Sha256Digest) {
        self.prev_operation_hash = Rc::new(last_operation_hash)
    }

    fn add_public_key(
        &mut self,
        public_key: PublicKey,
        added_at: &OperationMetadata,
    ) -> Result<(), DidStateConflictError> {
        if self.public_keys.contains_key(&public_key.id) {
            return Err(DidStateConflictError::AddPublicKeyWithExistingId { id: public_key.id });
        }

        let updated_map = self
            .public_keys
            .update(public_key.id.clone(), Revocable::new(public_key, added_at));
        self.public_keys = updated_map;
        Ok(())
    }

    fn revoke_public_key(
        &mut self,
        id: &PublicKeyId,
        revoke_at: &OperationMetadata,
    ) -> Result<(), DidStateConflictError> {
        let Some(public_key) = self.public_keys.get_mut(id) else {
            Err(DidStateConflictError::RevokePublicKeyNotExists { id: id.clone() })?
        };

        if public_key.is_revoked() {
            Err(DidStateConflictError::RevokePublicKeyIsAlreadyRevoked { id: id.clone() })?
        }

        public_key.revoke(revoke_at);
        Ok(())
    }

    fn add_service(&mut self, service: Service, added_at: &OperationMetadata) -> Result<(), DidStateConflictError> {
        if self.services.contains_key(&service.id) {
            return Err(DidStateConflictError::AddServiceWithExistingId { id: service.id });
        }

        let updated_map = self
            .services
            .update(service.id.clone(), Revocable::new(service, added_at));
        self.services = updated_map;
        Ok(())
    }

    fn revoke_service(&mut self, id: &ServiceId, revoke_at: &OperationMetadata) -> Result<(), DidStateConflictError> {
        let Some(service) = self.services.get_mut(id) else {
            Err(DidStateConflictError::RevokeServiceNotExists { id: id.clone() })?
        };

        if service.is_revoked() {
            Err(DidStateConflictError::RevokeServiceIsAlreadyRevoked { id: id.clone() })?
        }

        service.revoke(revoke_at);
        Ok(())
    }

    fn update_service_type(&mut self, id: &ServiceId, new_type: ServiceType) -> Result<(), DidStateConflictError> {
        let Some(service) = self.services.get_mut(id) else {
            Err(DidStateConflictError::UpdateServiceNotExists { id: id.clone() })?
        };

        if service.is_revoked() {
            Err(DidStateConflictError::UpdateServiceIsRevoked { id: id.clone() })?
        }

        service.get_mut().r#type = new_type;
        Ok(())
    }

    fn update_service_endpoint(
        &mut self,
        id: &ServiceId,
        new_endpoint: ServiceEndpoint,
    ) -> Result<(), DidStateConflictError> {
        let Some(service) = self.services.get_mut(id) else {
            Err(DidStateConflictError::UpdateServiceNotExists { id: id.clone() })?
        };

        if service.is_revoked() {
            Err(DidStateConflictError::UpdateServiceIsRevoked { id: id.clone() })?
        }

        service.get_mut().service_endpoint = new_endpoint;
        Ok(())
    }

    fn add_storage(
        &mut self,
        operation_hash: &Sha256Digest,
        data: StorageData,
        added_at: &OperationMetadata,
    ) -> Result<(), DidStateConflictError> {
        if self.storage.contains_key(operation_hash) {
            return Err(DidStateConflictError::AddStorageEntryWithExistingHash {
                initial_hash: operation_hash.clone(),
            });
        }

        let updated_map = self.storage.update(
            operation_hash.clone(),
            Revocable::new(
                StorageStateRc {
                    prev_operation_hash: operation_hash.clone().into(),
                    data: data.into(),
                },
                added_at,
            ),
        );
        self.storage = updated_map;
        Ok(())
    }

    fn revoke_storage(
        &mut self,
        prev_operation_hash: &Sha256Digest,
        operation_hash: &Sha256Digest,
        revoke_at: &OperationMetadata,
    ) -> Result<(), DidStateConflictError> {
        let Some(storage) = self
            .storage
            .iter_mut()
            .find_map(|(_, s)| Some(s).filter(|v| v.get().prev_operation_hash.deref() == prev_operation_hash))
        else {
            Err(DidStateConflictError::RevokeStorageEntryNotExists {
                previous_operation_hash: prev_operation_hash.clone(),
            })?
        };

        if storage.is_revoked() {
            Err(DidStateConflictError::RevokeStorageEntryAlreadyRevoked {
                previous_operation_hash: prev_operation_hash.clone(),
            })?
        }

        storage.revoke(revoke_at);
        storage.get_mut().prev_operation_hash = operation_hash.clone().into();
        Ok(())
    }

    fn update_storage(
        &mut self,
        prev_operation_hash: &Sha256Digest,
        operation_hash: &Sha256Digest,
        data: StorageData,
    ) -> Result<(), DidStateConflictError> {
        let Some(storage) = self
            .storage
            .iter_mut()
            .find_map(|(_, s)| Some(s).filter(|v| v.get().prev_operation_hash.deref() == prev_operation_hash))
        else {
            Err(DidStateConflictError::UpdateStorageEntryNotExists {
                prev_operation_hash: prev_operation_hash.clone(),
            })?
        };

        if storage.is_revoked() {
            Err(DidStateConflictError::UpdateStorageEntryAlreadyRevoked {
                prev_operation_hash: prev_operation_hash.clone(),
            })?
        }

        let storage_inner = storage.get_mut();
        storage_inner.prev_operation_hash = operation_hash.clone().into();
        storage_inner.data = data.into();
        Ok(())
    }

    fn finalize(self) -> DidState {
        let did: CanonicalPrismDid = (*self.did).clone();
        let context: Vec<String> = self.context.iter().map(|s| s.as_str().to_string()).collect();
        let last_operation_hash = self.prev_operation_hash.clone();
        let public_keys: Vec<PublicKey> = self
            .public_keys
            .into_iter()
            .filter(|(_, i)| !i.is_revoked())
            .map(|(_, i)| i.into_item())
            .collect();
        let services: Vec<Service> = self
            .services
            .into_iter()
            .filter(|(_, i)| !i.is_revoked())
            .map(|(_, i)| i.into_item())
            .collect();
        let storage = self
            .storage
            .into_iter()
            .filter(|(_, i)| !i.is_revoked())
            .map(|(k, v)| {
                let s = v.into_item();
                StorageState {
                    init_operation_hash: k.into(),
                    prev_operation_hash: s.prev_operation_hash,
                    data: s.data,
                }
            })
            .collect();
        DidState {
            did,
            context,
            prev_operation_hash: last_operation_hash,
            public_keys,
            services,
            storage,
        }
    }
}

struct Published;
struct Unpublished;

struct OperationProcessingContext<CtxType> {
    r#type: PhantomData<CtxType>,
    state: DidStateRc,
    processor: OperationProcessor,
}

fn init_published_context(
    signed_operation: SignedPrismOperation,
    metadata: OperationMetadata,
) -> Result<OperationProcessingContext<Published>, ProcessError> {
    let Some(operation) = &signed_operation.operation else {
        Err(ProcessError::SignedPrismOperationMissingOperation)?
    };

    let did = CanonicalPrismDid::from_operation(operation)?;
    match &operation.operation {
        Some(Operation::CreateDid(op)) => {
            let initial_state = DidStateRc::new(did);
            let processor = OperationProcessor::V1(V1Processor::default());
            let candidate_state = processor.create_did(&initial_state, op.clone(), metadata)?;
            processor.check_signature(&candidate_state, &signed_operation)?;
            Ok(OperationProcessingContext {
                r#type: PhantomData,
                state: candidate_state,
                processor,
            })
        }
        Some(_) => Err(ProcessError::DidStateInitFromNonCreateOperation),
        None => Err(ProcessError::SignedPrismOperationMissingOperation),
    }
}

fn init_unpublished_context(
    operation: PrismOperation,
) -> Result<OperationProcessingContext<Unpublished>, ProcessError> {
    let unpublished_metadata = OperationMetadata {
        block_metadata: BlockMetadata {
            slot_number: 0,
            block_number: 0,
            cbt: DateTime::UNIX_EPOCH,
            absn: 0,
        },
        osn: 0,
    };
    let did = CanonicalPrismDid::from_operation(&operation)?;
    match &operation.operation {
        Some(Operation::CreateDid(op)) => {
            let initial_state = DidStateRc::new(did);
            let processor = OperationProcessor::V1(V1Processor::default());
            let candidate_state = processor.create_did(&initial_state, op.clone(), unpublished_metadata)?;
            Ok(OperationProcessingContext {
                r#type: PhantomData,
                state: candidate_state,
                processor,
            })
        }
        Some(_) => Err(ProcessError::DidStateInitFromNonCreateOperation),
        None => Err(ProcessError::SignedPrismOperationMissingOperation),
    }
}

impl<T> OperationProcessingContext<T> {
    fn finalize(self) -> DidState {
        self.state.finalize()
    }
}

impl OperationProcessingContext<Published> {
    fn process(
        mut self,
        signed_operation: SignedPrismOperation,
        metadata: OperationMetadata,
    ) -> (Self, Option<ProcessError>) {
        let signature_verification = self.processor.check_signature(&self.state, &signed_operation);
        if let Err(e) = signature_verification {
            return (self, Some(e));
        }

        let Some(operation) = signed_operation.operation else {
            return (self, Some(ProcessError::SignedPrismOperationMissingOperation));
        };

        let process_result = match operation.operation {
            Some(Operation::CreateDid(_)) => Err(ProcessError::DidStateUpdateFromCreateOperation),
            Some(Operation::UpdateDid(op)) => self
                .processor
                .update_did(&self.state, op, metadata)
                .map(|s| (Some(s), None)),
            Some(Operation::DeactivateDid(op)) => self
                .processor
                .deactivate_did(&self.state, op, metadata)
                .map(|s| (Some(s), None)),
            Some(Operation::ProtocolVersionUpdate(op)) => self
                .processor
                .protocol_version_update(op, metadata)
                .map(|s| (None, Some(s))),
            Some(Operation::CreateStorageEntry(op)) => self
                .processor
                .create_storage(&self.state, op, metadata)
                .map(|s| (Some(s), None)),
            Some(Operation::UpdateStorageEntry(op)) => self
                .processor
                .update_storage(&self.state, op, metadata)
                .map(|s| (Some(s), None)),
            Some(Operation::DeactivateStorageEntry(op)) => self
                .processor
                .deactivate_storage(&self.state, op, metadata)
                .map(|s| (Some(s), None)),
            None => Err(ProcessError::SignedPrismOperationMissingOperation),
        };

        match process_result {
            Ok((state, processor)) => {
                if let Some(state) = state {
                    self.state = state;
                };
                if let Some(processor) = processor {
                    self.processor = processor;
                }
                (self, None)
            }
            Err(e) => (self, Some(e)),
        }
    }
}

#[enum_dispatch]
trait OperationProcessorOps {
    fn check_signature(&self, state: &DidStateRc, signed_operation: &SignedPrismOperation) -> Result<(), ProcessError>;

    fn create_did(
        &self,
        state: &DidStateRc,
        operation: ProtoCreateDid,
        metadata: OperationMetadata,
    ) -> Result<DidStateRc, ProcessError>;

    fn update_did(
        &self,
        state: &DidStateRc,
        operation: ProtoUpdateDid,
        metadata: OperationMetadata,
    ) -> Result<DidStateRc, ProcessError>;

    fn deactivate_did(
        &self,
        state: &DidStateRc,
        operation: ProtoDeactivateDid,
        metadata: OperationMetadata,
    ) -> Result<DidStateRc, ProcessError>;

    fn protocol_version_update(
        &self,
        operation: ProtoProtocolVersionUpdate,
        metadata: OperationMetadata,
    ) -> Result<OperationProcessor, ProcessError>;

    fn create_storage(
        &self,
        state: &DidStateRc,
        operation: ProtoCreateStorageEntry,
        metadata: OperationMetadata,
    ) -> Result<DidStateRc, ProcessError>;

    fn update_storage(
        &self,
        state: &DidStateRc,
        operation: ProtoUpdateStorageEntry,
        metadata: OperationMetadata,
    ) -> Result<DidStateRc, ProcessError>;

    fn deactivate_storage(
        &self,
        state: &DidStateRc,
        operation: ProtoDeactivateStorageEntry,
        metadata: OperationMetadata,
    ) -> Result<DidStateRc, ProcessError>;
}

#[derive(Debug, Clone)]
#[enum_dispatch(OperationProcessorOps)]
enum OperationProcessor {
    V1(V1Processor),
}
