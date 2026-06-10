use std::sync::Arc;

use identus_apollo::hash::Sha256Digest;
use identus_apollo::hex::HexStr;
use identus_did_core::{Did, DidResolver, ResolutionOptions, ResolutionResult};
use identus_did_prism::did::operation::{OperationId, StorageData};
use identus_did_prism::did::{CanonicalPrismDid, DidState, PrismDid, PrismDidOps, StorageState};
use identus_did_prism::dlt::{BlockNo, OperationMetadata, SlotNo, TxId};
use identus_did_prism::prelude::SignedPrismOperation;
use identus_did_prism::protocol::resolver::{ResolutionDebug, resolve_published, resolve_unpublished};
use identus_did_prism::utils::paging::Paginated;
use identus_did_prism_indexer::repo::{IndexerStateRepo, RawOperationRepo};
use node_storage::StorageBackend;

use super::error::{InvalidDid, ResolutionError};

/// Metadata about a VDR entry, including the latest event hash and status.
#[derive(Debug, Clone)]
pub struct VdrEntryMetadata {
    pub entry_hash: String,
    pub latest_event_hash: String,
    pub status: String,
}

#[derive(Clone)]
pub struct PrismDidService {
    db: Arc<dyn StorageBackend>,
}

impl PrismDidService {
    pub fn new(db: Arc<dyn StorageBackend>) -> Self {
        Self { db }
    }

    pub async fn get_indexer_stats(&self) -> anyhow::Result<Option<(SlotNo, BlockNo)>> {
        let result = self.db.get_last_indexed_block().await?;
        Ok(result)
    }

    /// Resolve VDR storage state by entry hash.
    /// Returns the parsed hex, the DID state, and the matching storage entry if found.
    async fn resolve_vdr_storage(
        &self,
        entry_hash_hex: &str,
    ) -> anyhow::Result<Option<(HexStr, DidState, StorageState)>> {
        let entry_hash_hex: HexStr = entry_hash_hex.parse()?;
        let entry_hash = Sha256Digest::from_bytes(&entry_hash_hex.to_bytes())?;
        let Some(owner) = self.db.get_did_by_vdr_entry(&entry_hash).await? else {
            return Ok(None);
        };
        let mut debug_acc = vec![];
        let (_, did_state) = self.resolve_did_logic(&owner.to_string(), &mut debug_acc).await?;
        let storage = did_state
            .storage
            .iter()
            .find(|i| *i.init_operation_hash == entry_hash)
            .cloned();
        Ok(storage.map(|s| (entry_hash_hex, did_state, s)))
    }

    pub async fn resolve_vdr(&self, entry_hash_hex: &str) -> anyhow::Result<Option<Vec<u8>>> {
        let Some((_, _, storage)) = self.resolve_vdr_storage(entry_hash_hex).await? else {
            return Ok(None);
        };
        match &*storage.data {
            StorageData::Bytes(items) => Ok(Some(items.clone())),
            _ => anyhow::bail!("vdr storage data types other than bytes are not yet supported"),
        }
    }

    pub async fn resolve_vdr_entry_metadata(&self, entry_hash_hex: &str) -> anyhow::Result<Option<VdrEntryMetadata>> {
        let Some((hex, _, storage)) = self.resolve_vdr_storage(entry_hash_hex).await? else {
            return Ok(None);
        };
        Ok(Some(VdrEntryMetadata {
            entry_hash: hex.to_string(),
            latest_event_hash: HexStr::from(storage.last_operation_hash.to_vec()).to_string(),
            // Status is always "active" here: deactivated DIDs have their storage entries
            // revoked and filtered out during resolution, so resolve_vdr_storage() returns
            // None (404) before we reach this point.
            status: "active".to_string(),
        }))
    }

    pub async fn resolve_did(&self, did: &str) -> (Result<(PrismDid, DidState), ResolutionError>, ResolutionDebug) {
        let mut debug_acc = vec![];
        let result = self.resolve_did_logic(did, &mut debug_acc).await;
        (result, debug_acc)
    }

    async fn resolve_did_logic(
        &self,
        did: &str,
        debug_acc: &mut ResolutionDebug,
    ) -> Result<(PrismDid, DidState), ResolutionError> {
        let did: PrismDid = did.parse().map_err(|e| InvalidDid::InvalidPrismDid { source: e })?;
        let canonical_did = did.clone().into_canonical();

        let operations = self
            .db
            .get_raw_operations_by_did(&canonical_did)
            .await
            .map_err(|e| ResolutionError::InternalError { source: e.into() })?
            .into_iter()
            .map(|record| (record.metadata, record.signed_operation))
            .collect::<Vec<_>>();

        if operations.is_empty() {
            match &did {
                PrismDid::Canonical(_) => Err(ResolutionError::NotFound)?,
                PrismDid::LongForm(long_form_did) => {
                    let operation = long_form_did
                        .operation()
                        .map_err(|e| InvalidDid::InvalidPrismDid { source: e })?;
                    let did_state =
                        resolve_unpublished(operation).map_err(|e| InvalidDid::ProcessStateFailed { source: e })?;
                    Ok((did, did_state))
                }
            }
        } else {
            let (did_state, debug) = resolve_published(operations);
            debug_acc.extend(debug);
            match did_state {
                Some(did_state) => Ok((did, did_state)),
                None => Err(ResolutionError::NotFound),
            }
        }
    }

    pub async fn get_all_dids(&self, page: Option<u32>) -> anyhow::Result<Paginated<CanonicalPrismDid>> {
        let page = page.unwrap_or(0);
        let dids = self.db.get_all_dids(page, 100).await?;
        Ok(dids)
    }

    pub async fn get_raw_operations_by_tx_id(
        &self,
        tx_id: &TxId,
    ) -> anyhow::Result<Vec<(OperationMetadata, SignedPrismOperation, CanonicalPrismDid)>> {
        Ok(self
            .db
            .get_raw_operations_by_tx_id(tx_id)
            .await?
            .into_iter()
            .map(|(record, did)| (record.metadata, record.signed_operation, did))
            .collect())
    }

    pub async fn get_raw_operation_by_operation_id(
        &self,
        operation_id: &OperationId,
    ) -> anyhow::Result<Option<(OperationMetadata, SignedPrismOperation, CanonicalPrismDid)>> {
        Ok(self
            .db
            .get_raw_operation_by_operation_id(operation_id)
            .await?
            .map(|(record, did)| (record.metadata, record.signed_operation, did)))
    }
}

#[async_trait::async_trait]
impl DidResolver for PrismDidService {
    async fn resolve(&self, did: &Did, _options: &ResolutionOptions) -> ResolutionResult {
        let did_str = did.to_string();
        let mut debug_acc = vec![];
        match self.resolve_did_logic(&did_str, &mut debug_acc).await {
            Ok((prism_did, state)) => state.to_resolution_result(&prism_did),
            Err(e) => e.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use identus_apollo::crypto::secp256k1::Secp256k1PrivateKey;
    use identus_apollo::hash::sha256;
    use identus_did_prism::did::LongFormPrismDid;
    use identus_did_prism::dlt::{BlockMetadata, TxId};
    use identus_did_prism::prelude::MessageExt;
    use identus_did_prism::proto;
    use identus_did_prism::proto::prism_ssi::KeyUsage;
    use identus_did_prism_indexer::run_indexer_loop;
    use node_storage::SqliteDb;

    use super::*;

    const MASTER_KEY: [u8; 32] = [1; 32];
    const MASTER_KEY_NAME: &str = "master-0";

    fn master_sk() -> Secp256k1PrivateKey {
        Secp256k1PrivateKey::from_slice(&MASTER_KEY).unwrap()
    }

    fn new_public_key(id: &str, usage: KeyUsage, sk: &Secp256k1PrivateKey) -> proto::prism_ssi::PublicKey {
        let pk = sk.to_public_key();
        proto::prism_ssi::PublicKey {
            id: id.to_string(),
            usage: usage.into(),
            key_data: Some(proto::prism_ssi::public_key::Key_data::CompressedEcKeyData(
                proto::prism_ssi::CompressedECKeyData {
                    curve: "secp256k1".to_string(),
                    data: pk.encode_compressed().into(),
                    special_fields: Default::default(),
                },
            )),
            special_fields: Default::default(),
        }
    }

    fn new_create_did_operation() -> (proto::prism::SignedPrismOperation, Sha256Digest) {
        let sk = master_sk();
        let operation_inner = proto::prism::prism_operation::Operation::CreateDid(proto::prism_ssi::ProtoCreateDID {
            did_data: Some(proto::prism_ssi::proto_create_did::DIDCreationData {
                public_keys: vec![new_public_key(MASTER_KEY_NAME, KeyUsage::MASTER_KEY, &sk)],
                services: vec![],
                context: vec![],
                special_fields: Default::default(),
            })
            .into(),
            special_fields: Default::default(),
        });
        let operation = proto::prism::PrismOperation {
            operation: Some(operation_inner),
            special_fields: Default::default(),
        };
        let operation_hash = operation.operation_hash();
        let signed_operation = proto::prism::SignedPrismOperation {
            signed_with: MASTER_KEY_NAME.to_string(),
            signature: sk.sign(&operation.encode_to_vec()),
            operation: Some(operation).into(),
            special_fields: Default::default(),
        };
        (signed_operation, operation_hash)
    }

    fn new_update_did_operation(
        did_suffix: &str,
        signed_with: &str,
        signing_key: &Secp256k1PrivateKey,
        previous_hash: &Sha256Digest,
    ) -> proto::prism::SignedPrismOperation {
        let add_service = proto::prism_ssi::UpdateDIDAction {
            action: Some(proto::prism_ssi::update_didaction::Action::AddService(
                proto::prism_ssi::AddServiceAction {
                    service: Some(proto::prism_ssi::Service {
                        id: "service-1".to_string(),
                        type_: "LinkedDomains".to_string(),
                        service_endpoint: "https://example.com".to_string(),
                        special_fields: Default::default(),
                    })
                    .into(),
                    special_fields: Default::default(),
                },
            )),
            special_fields: Default::default(),
        };

        let operation_inner = proto::prism::prism_operation::Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            id: did_suffix.to_string(),
            actions: vec![add_service],
            previous_operation_hash: previous_hash.to_vec(),
            special_fields: Default::default(),
        });
        let operation = proto::prism::PrismOperation {
            operation: Some(operation_inner),
            special_fields: Default::default(),
        };
        proto::prism::SignedPrismOperation {
            signed_with: signed_with.to_string(),
            signature: signing_key.sign(&operation.encode_to_vec()),
            operation: Some(operation).into(),
            special_fields: Default::default(),
        }
    }

    fn new_deactivate_did_operation(
        did_suffix: &str,
        signed_with: &str,
        signing_key: &Secp256k1PrivateKey,
        previous_hash: &Sha256Digest,
    ) -> proto::prism::SignedPrismOperation {
        let operation_inner =
            proto::prism::prism_operation::Operation::DeactivateDid(proto::prism_ssi::ProtoDeactivateDID {
                id: did_suffix.to_string(),
                previous_operation_hash: previous_hash.to_vec(),
                special_fields: Default::default(),
            });
        let operation = proto::prism::PrismOperation {
            operation: Some(operation_inner),
            special_fields: Default::default(),
        };
        proto::prism::SignedPrismOperation {
            signed_with: signed_with.to_string(),
            signature: signing_key.sign(&operation.encode_to_vec()),
            operation: Some(operation).into(),
            special_fields: Default::default(),
        }
    }

    fn dummy_metadata(osn: u32) -> OperationMetadata {
        OperationMetadata {
            block_metadata: BlockMetadata {
                slot_number: 0.into(),
                block_number: 0.into(),
                cbt: DateTime::UNIX_EPOCH,
                absn: 0,
                tx_id: TxId::from(sha256([0u8; 32])),
            },
            osn,
        }
    }

    async fn setup_db() -> Arc<dyn StorageBackend> {
        let db = SqliteDb::connect("sqlite::memory:").await.unwrap();
        db.migrate().await.unwrap();
        Arc::new(db)
    }

    async fn setup_service() -> (PrismDidService, Arc<dyn StorageBackend>) {
        let db = setup_db().await;
        let service = PrismDidService::new(db.clone());
        (service, db)
    }

    // --- get_indexer_stats ---

    #[tokio::test]
    async fn get_indexer_stats_returns_none_when_empty() {
        let (service, _) = setup_service().await;
        let stats = service.get_indexer_stats().await.unwrap();
        assert!(stats.is_none(), "should return None when no blocks indexed");
    }

    // --- resolve_did: not found ---

    #[tokio::test]
    async fn resolve_did_canonical_not_found() {
        let (service, _) = setup_service().await;
        // A valid canonical DID format that does not exist in the database
        let did_str = "did:prism:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
        let (result, _debug) = service.resolve_did(did_str).await;
        let err = result.unwrap_err();
        matches!(err, ResolutionError::NotFound);
    }

    #[tokio::test]
    async fn resolve_did_invalid_format() {
        let (service, _) = setup_service().await;
        let (result, _debug) = service.resolve_did("not-a-did").await;
        let err = result.unwrap_err();
        matches!(err, ResolutionError::InvalidDid { .. });
    }

    // --- resolve_did: published canonical DID ---

    #[tokio::test]
    async fn resolve_did_published_create_did() {
        let (service, db) = setup_service().await;

        // Create and insert a DID operation
        let (signed_op, _op_hash) = new_create_did_operation();
        db.insert_raw_operations(vec![(dummy_metadata(0), signed_op)])
            .await
            .unwrap();
        run_indexer_loop(db.as_ref()).await.unwrap();

        // After indexing, no unindexed should remain
        assert!(db.get_raw_operations_unindexed().await.unwrap().is_empty());

        // Get the DID via get_all_dids
        let all_dids = service.get_all_dids(None).await.unwrap();
        assert_eq!(all_dids.items.len(), 1);
        let did_str = all_dids.items[0].to_string();

        // Resolve the DID
        let (result, _debug) = service.resolve_did(&did_str).await;
        let (_prism_did, did_state) = result.expect("resolve should succeed for published DID");
        assert!(!did_state.is_deactivated(), "new DID should not be deactivated");
        assert!(!did_state.public_keys.is_empty(), "new DID should have public keys");
    }

    // --- resolve_did: long-form DID (unpublished) ---

    #[tokio::test]
    async fn resolve_did_long_form_unpublished() {
        let (service, _db) = setup_service().await;

        // Create a long-form DID from the operation
        let (signed_op, _op_hash) = new_create_did_operation();
        let operation = signed_op.operation.clone().into_option().unwrap();

        // Build the long-form DID from the PrismOperation (not SignedPrismOperation)
        let long_form_did = LongFormPrismDid::from_operation(&operation).unwrap();
        let long_form_str = long_form_did.to_string();

        let (result, _debug) = service.resolve_did(&long_form_str).await;
        let (_prism_did, did_state) = result.expect("long-form DID should resolve when unpublished");
        assert!(!did_state.is_deactivated());
        assert!(!did_state.public_keys.is_empty());
    }

    // --- resolve_did: update DID ---

    #[tokio::test]
    async fn resolve_did_after_update_adds_service() {
        let (service, db) = setup_service().await;
        let sk = master_sk();

        // Create DID
        let (create_op, create_hash) = new_create_did_operation();
        db.insert_raw_operations(vec![(dummy_metadata(0), create_op)])
            .await
            .unwrap();
        run_indexer_loop(db.as_ref()).await.unwrap();

        // Get the DID suffix as hex
        let all_dids = service.get_all_dids(None).await.unwrap();
        assert_eq!(all_dids.items.len(), 1);
        let did_suffix_hex = HexStr::from(all_dids.items[0].suffix().as_bytes().to_owned());

        // Update DID — add a service
        let update_op = new_update_did_operation(&did_suffix_hex.to_string(), MASTER_KEY_NAME, &sk, &create_hash);
        db.insert_raw_operations(vec![(dummy_metadata(1), update_op)])
            .await
            .unwrap();
        run_indexer_loop(db.as_ref()).await.unwrap();

        // Resolve the DID
        let canonical_str = all_dids.items[0].to_string();
        let (result, _debug) = service.resolve_did(&canonical_str).await;
        let (_, did_state) = result.expect("resolve should succeed");
        assert_eq!(did_state.services.len(), 1, "should have one service after update");
        assert_eq!(did_state.services[0].id.to_string(), "service-1");
    }

    // --- resolve_did: deactivate DID ---

    #[tokio::test]
    async fn resolve_did_deactivated() {
        let (service, db) = setup_service().await;
        let sk = master_sk();

        // Create DID
        let (create_op, create_hash) = new_create_did_operation();
        db.insert_raw_operations(vec![(dummy_metadata(0), create_op)])
            .await
            .unwrap();
        run_indexer_loop(db.as_ref()).await.unwrap();

        let all_dids = service.get_all_dids(None).await.unwrap();
        let did_suffix_hex = HexStr::from(all_dids.items[0].suffix().as_bytes().to_owned());
        let canonical_str = all_dids.items[0].to_string();

        // Deactivate DID
        let deactivate_op =
            new_deactivate_did_operation(&did_suffix_hex.to_string(), MASTER_KEY_NAME, &sk, &create_hash);
        db.insert_raw_operations(vec![(dummy_metadata(1), deactivate_op)])
            .await
            .unwrap();
        run_indexer_loop(db.as_ref()).await.unwrap();

        // Resolve should indicate deactivated
        let (result, _debug) = service.resolve_did(&canonical_str).await;
        let (_, did_state) = result.expect("resolve should succeed");
        assert!(did_state.is_deactivated(), "DID should be deactivated");
    }

    // --- get_all_dids ---

    #[tokio::test]
    async fn get_all_dids_empty() {
        let (service, _) = setup_service().await;
        let result = service.get_all_dids(None).await.unwrap();
        assert!(result.items.is_empty(), "should be empty with no operations");
    }

    #[tokio::test]
    async fn get_all_dids_multiple() {
        let (service, db) = setup_service().await;

        // Create two DIDs with different master keys
        let sk1 = Secp256k1PrivateKey::from_slice(&[1; 32]).unwrap();
        let sk2 = Secp256k1PrivateKey::from_slice(&[2; 32]).unwrap();

        let mut operations = vec![];
        for (sk, key_name) in [(sk1, "master-a"), (sk2, "master-b")] {
            let operation_inner =
                proto::prism::prism_operation::Operation::CreateDid(proto::prism_ssi::ProtoCreateDID {
                    did_data: Some(proto::prism_ssi::proto_create_did::DIDCreationData {
                        public_keys: vec![new_public_key(key_name, KeyUsage::MASTER_KEY, &sk)],
                        services: vec![],
                        context: vec![],
                        special_fields: Default::default(),
                    })
                    .into(),
                    special_fields: Default::default(),
                });
            let operation = proto::prism::PrismOperation {
                operation: Some(operation_inner),
                special_fields: Default::default(),
            };
            let signed_operation = proto::prism::SignedPrismOperation {
                signed_with: key_name.to_string(),
                signature: sk.sign(&operation.encode_to_vec()),
                operation: Some(operation).into(),
                special_fields: Default::default(),
            };
            operations.push((dummy_metadata(operations.len() as u32), signed_operation));
        }
        db.insert_raw_operations(operations).await.unwrap();
        run_indexer_loop(db.as_ref()).await.unwrap();

        let result = service.get_all_dids(None).await.unwrap();
        assert_eq!(result.items.len(), 2, "should have two DIDs");
    }

    // --- get_raw_operations_by_tx_id ---

    #[tokio::test]
    async fn get_raw_operations_by_tx_id_returns_matching() {
        let (service, db) = setup_service().await;

        let tx_id = TxId::from(sha256([42u8; 32]));
        let (signed_op, _) = new_create_did_operation();
        let metadata = OperationMetadata {
            block_metadata: BlockMetadata {
                slot_number: 10.into(),
                block_number: 5.into(),
                cbt: DateTime::UNIX_EPOCH,
                absn: 0,
                tx_id: tx_id.clone(),
            },
            osn: 0,
        };
        db.insert_raw_operations(vec![(metadata, signed_op)]).await.unwrap();
        run_indexer_loop(db.as_ref()).await.unwrap();

        let result = service.get_raw_operations_by_tx_id(&tx_id).await.unwrap();
        assert_eq!(result.len(), 1, "should find one operation by tx_id");
    }

    #[tokio::test]
    async fn get_raw_operations_by_tx_id_empty_when_not_found() {
        let (service, _) = setup_service().await;
        let tx_id = TxId::from(sha256([99u8; 32]));
        let result = service.get_raw_operations_by_tx_id(&tx_id).await.unwrap();
        assert!(result.is_empty(), "should return empty for unknown tx_id");
    }

    // --- get_raw_operation_by_operation_id ---

    #[tokio::test]
    async fn get_raw_operation_by_operation_id_returns_none_when_not_found() {
        let (service, _) = setup_service().await;
        let op_id = OperationId::from_bytes(sha256([0u8; 32]).as_bytes()).unwrap();
        let result = service.get_raw_operation_by_operation_id(&op_id).await.unwrap();
        assert!(result.is_none(), "should return None for unknown operation_id");
    }

    #[tokio::test]
    async fn get_raw_operation_by_operation_id_returns_record() {
        let (service, db) = setup_service().await;

        let (signed_op, _op_hash) = new_create_did_operation();
        let op_id = signed_op.operation_id();
        db.insert_raw_operations(vec![(dummy_metadata(0), signed_op)])
            .await
            .unwrap();
        run_indexer_loop(db.as_ref()).await.unwrap();

        let result = service.get_raw_operation_by_operation_id(&op_id).await.unwrap();
        assert!(result.is_some(), "should find operation by operation_id");
        let (metadata, _retrieved_op, _did) = result.unwrap();
        assert_eq!(metadata.osn, 0);
    }

    // --- DidResolver trait impl ---

    #[tokio::test]
    async fn did_resolver_trait_resolve_success() {
        let (service, db) = setup_service().await;

        let (signed_op, _) = new_create_did_operation();
        db.insert_raw_operations(vec![(dummy_metadata(0), signed_op)])
            .await
            .unwrap();
        run_indexer_loop(db.as_ref()).await.unwrap();

        let all_dids = service.get_all_dids(None).await.unwrap();
        let did_str = all_dids.items[0].to_string();
        let did: Did = did_str.parse().unwrap();

        let result = service.resolve(&did, &ResolutionOptions::default()).await;
        assert!(
            result.did_resolution_metadata.error.is_none(),
            "should resolve without error"
        );
        assert!(result.did_document.is_some(), "should have a DID document");
    }

    #[tokio::test]
    async fn did_resolver_trait_resolve_not_found() {
        let (service, _) = setup_service().await;
        let did: Did = "did:prism:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
            .parse()
            .unwrap();

        let result = service.resolve(&did, &ResolutionOptions::default()).await;
        assert!(
            result.did_resolution_metadata.error.is_some(),
            "should have error for non-existent DID"
        );
    }

    // --- VdrEntryMetadata debug/clone ---

    #[test]
    fn vdr_entry_metadata_debug_clone() {
        let meta = VdrEntryMetadata {
            entry_hash: "abc".to_string(),
            latest_event_hash: "def".to_string(),
            status: "active".to_string(),
        };
        let cloned = meta.clone();
        assert_eq!(meta.entry_hash, cloned.entry_hash);
        assert_eq!(meta.latest_event_hash, cloned.latest_event_hash);
        assert_eq!(meta.status, cloned.status);
        let debug_str = format!("{:?}", meta);
        assert!(debug_str.contains("abc"));
        assert!(debug_str.contains("active"));
    }
}
