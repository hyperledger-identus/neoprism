use std::collections::HashMap;

use identus_apollo::hash::Sha256Digest;
use identus_apollo::hex::HexStr;
use identus_did_prism::did::CanonicalPrismDid;
use identus_did_prism::dlt::{BlockNo, OperationMetadata};
use identus_did_prism::prelude::*;
use identus_did_prism::proto::prism::prism_operation::Operation;

use crate::DltSource;
use crate::repo::{IndexedOperation, IndexedOperationRepo, RawOperationRepo};

enum IntermediateIndexedOperation {
    Ssi {
        did: CanonicalPrismDid,
    },
    VdrRoot {
        operation_hash: Vec<u8>,
        did: CanonicalPrismDid,
    },
    VdrChild {
        operation_hash: Vec<u8>,
        prev_operation_hash: Vec<u8>,
    },
}

/// Maximum number of hops walked when resolving a storage operation's root
/// event, either through the database or the in-memory pending map.
const VDR_ROOT_SEARCH_MAX_DEPTH: usize = 200;

/// A VDR operation resolved in the current transaction but not yet committed,
/// kept in the pending map so that same-transaction storage chains can be
/// resolved without a database lookup. `prev_operation_hash` is retained so the
/// chain can be walked (bounded by `VDR_ROOT_SEARCH_MAX_DEPTH`).
struct PendingVdrEntry {
    did: CanonicalPrismDid,
    init_operation_hash: Vec<u8>,
    prev_operation_hash: Option<Vec<u8>>,
}

/// Run indexer loop until no more operation to index
pub async fn run_indexer_loop<Repo>(repo: &Repo) -> anyhow::Result<()>
where
    Repo: RawOperationRepo + IndexedOperationRepo + ?Sized,
    <Repo as RawOperationRepo>::Error: Send + Sync + 'static,
    <Repo as IndexedOperationRepo>::Error: Send + Sync + 'static,
{
    loop {
        let unindexed_operations = repo.get_raw_operations_unindexed().await?;
        if unindexed_operations.is_empty() {
            return Ok(());
        }

        tracing::info!("Indexing {} operations", unindexed_operations.len());

        // Operations are returned ordered by (block_number, absn, osn), so all
        // operations of the same transaction — identified by (block_number, absn)
        // — appear contiguously. We index each transaction with a single atomic
        // `insert_indexed_operations` call so that readers (e.g. transaction
        // confirmation and DID resolution) can never observe a partially-indexed
        // transaction.
        let mut current_tx: Option<(BlockNo, u32)> = None;
        let mut batch: Vec<IndexedOperation> = Vec::new();
        // `operation_hash -> entry` for every VDR operation already resolved in
        // the current transaction. This lets a storage update/deactivate whose
        // root operation lives in the *same* transaction resolve without a
        // database read, since that root is not committed yet while the batch is
        // being built.
        let mut pending_vdr: HashMap<Vec<u8>, PendingVdrEntry> = HashMap::new();

        for record in unindexed_operations {
            let raw_operation_id = record.id;
            let meta = record.metadata;
            let signed_operation = record.signed_operation;
            let tx_key = (meta.block_metadata.block_number, meta.block_metadata.absn);

            // Flush the previous transaction atomically when the boundary changes.
            if current_tx != Some(tx_key) {
                if !batch.is_empty() {
                    let to_insert = std::mem::take(&mut batch);
                    pending_vdr.clear();
                    repo.insert_indexed_operations(to_insert).await?;
                }
                current_tx = Some(tx_key);
            }

            let intermediate_indexed_op = index_from_signed_operation(signed_operation);
            let indexed_op = match intermediate_indexed_op {
                Ok(IntermediateIndexedOperation::Ssi { did }) => IndexedOperation::Ssi { raw_operation_id, did },
                Ok(IntermediateIndexedOperation::VdrRoot { operation_hash, did }) => {
                    let init_operation_hash = operation_hash.clone();
                    pending_vdr.insert(
                        operation_hash.clone(),
                        PendingVdrEntry {
                            did: did.clone(),
                            init_operation_hash: init_operation_hash.clone(),
                            prev_operation_hash: None,
                        },
                    );
                    IndexedOperation::Vdr {
                        raw_operation_id,
                        init_operation_hash,
                        operation_hash,
                        did,
                        prev_operation_hash: None,
                    }
                }
                Ok(IntermediateIndexedOperation::VdrChild {
                    prev_operation_hash,
                    operation_hash,
                }) => {
                    // Resolve the root by walking the in-memory pending chain
                    // (same transaction) first, falling back to the database once
                    // the chain crosses into a prior, already-indexed transaction.
                    // Each phase is independently bounded by
                    // `VDR_ROOT_SEARCH_MAX_DEPTH`.
                    let vdr_root = resolve_vdr_root(repo, &pending_vdr, &prev_operation_hash).await?;
                    match vdr_root {
                        Some((did, init_operation_hash)) => {
                            pending_vdr.insert(
                                operation_hash.clone(),
                                PendingVdrEntry {
                                    did: did.clone(),
                                    init_operation_hash: init_operation_hash.clone(),
                                    prev_operation_hash: Some(prev_operation_hash.clone()),
                                },
                            );
                            IndexedOperation::Vdr {
                                raw_operation_id,
                                init_operation_hash,
                                operation_hash,
                                prev_operation_hash: Some(prev_operation_hash),
                                did,
                            }
                        }
                        None => {
                            tracing::warn!("SignedPrismOperation {:?} is ignored since it cannot be indexed.", meta);
                            IndexedOperation::Ignored { raw_operation_id }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "SignedPrismOperation {:?} is ignored since it cannot be indexed. ({})",
                        meta,
                        e
                    );
                    IndexedOperation::Ignored { raw_operation_id }
                }
            };
            batch.push(indexed_op);
        }

        // Flush the final transaction.
        if !batch.is_empty() {
            let to_insert = std::mem::take(&mut batch);
            pending_vdr.clear();
            repo.insert_indexed_operations(to_insert).await?;
        }
    }
}

/// Run sync loop until DLT source is closed
pub async fn run_sync_loop<Repo, Src>(repo: &Repo, source: Src) -> anyhow::Result<()>
where
    Src: DltSource,
    Repo: RawOperationRepo + IndexedOperationRepo + Send + Sync + ?Sized,
    <Repo as RawOperationRepo>::Error: Send + Sync + 'static,
    <Repo as IndexedOperationRepo>::Error: Send + Sync + 'static,
{
    let mut rx = source.into_stream().expect("Unable to create a DLT source");

    while let Some(published_prism_object) = rx.recv().await {
        let block = published_prism_object.prism_object.block_content;
        let block_metadata = published_prism_object.block_metadata;
        let signed_operations = block.map(|i| i.operations).unwrap_or_default();

        let mut insert_batch = Vec::with_capacity(signed_operations.len());
        for (idx, signed_operation) in signed_operations.into_iter().enumerate() {
            let has_operation = signed_operation
                .operation
                .as_ref()
                .and_then(|i| i.operation.as_ref())
                .is_some();

            if !has_operation {
                continue;
            }

            insert_batch.push((
                OperationMetadata {
                    block_metadata: block_metadata.clone(),
                    osn: idx as u32,
                },
                signed_operation,
            ));
        }

        let insert_result = repo.insert_raw_operations(insert_batch).await;
        if let Err(e) = insert_result {
            tracing::error!("Failed to insert operation into database: {:?}", e);
        }
    }
    Ok(())
}

/// Returns DID that create a root operation and its operation hash
async fn recursively_find_vdr_root<Repo>(
    repo: &Repo,
    prev_operation_hash: &[u8],
) -> anyhow::Result<Option<(CanonicalPrismDid, Vec<u8>)>>
where
    Repo: RawOperationRepo + IndexedOperationRepo + ?Sized,
    <Repo as RawOperationRepo>::Error: Send + Sync + 'static,
    <Repo as IndexedOperationRepo>::Error: Send + Sync + 'static,
{
    let mut parent_hash = prev_operation_hash.to_vec();
    for _ in 1..VDR_ROOT_SEARCH_MAX_DEPTH {
        let Ok(parsed_parent_hash) = Sha256Digest::from_bytes(&parent_hash) else {
            return Ok(None); // invalid parent
        };

        let parent = repo
            .get_raw_operation_vdr_by_operation_hash(&parsed_parent_hash)
            .await?;
        match parent {
            None => return Ok(None), // no root found
            Some(record) => {
                let signed_operation = record.signed_operation;
                match index_from_signed_operation(signed_operation) {
                    Ok(IntermediateIndexedOperation::VdrRoot { did, operation_hash }) => {
                        return Ok(Some((did, operation_hash))); // found root
                    }
                    Ok(IntermediateIndexedOperation::VdrChild {
                        prev_operation_hash, ..
                    }) => {
                        parent_hash = prev_operation_hash; // go to next parent
                    }
                    _ => return Ok(None), // invalid parent
                }
            }
        }
    }

    Ok(None) // exceed max depth
}

/// Resolve the root of a storage operation by walking the in-memory pending map
/// (operations of the current transaction) first, and falling back to the
/// database once the chain crosses into a prior, already-indexed transaction.
/// Each phase is independently bounded by `VDR_ROOT_SEARCH_MAX_DEPTH`, so a
/// storage chain that is too deep is treated as unresolvable regardless of
/// where it lives.
async fn resolve_vdr_root<Repo>(
    repo: &Repo,
    pending_vdr: &HashMap<Vec<u8>, PendingVdrEntry>,
    prev_operation_hash: &[u8],
) -> anyhow::Result<Option<(CanonicalPrismDid, Vec<u8>)>>
where
    Repo: RawOperationRepo + IndexedOperationRepo + ?Sized,
    <Repo as RawOperationRepo>::Error: Send + Sync + 'static,
    <Repo as IndexedOperationRepo>::Error: Send + Sync + 'static,
{
    let mut current = prev_operation_hash.to_vec();
    for _ in 1..VDR_ROOT_SEARCH_MAX_DEPTH {
        match pending_vdr.get(&current) {
            Some(entry) => match &entry.prev_operation_hash {
                None => return Ok(Some((entry.did.clone(), entry.init_operation_hash.clone()))),
                Some(prev) => current = prev.clone(),
            },
            None => return recursively_find_vdr_root(repo, &current).await,
        }
    }
    Ok(None) // exceeded max depth within the pending chain
}

fn index_from_signed_operation(signed_operation: SignedPrismOperation) -> anyhow::Result<IntermediateIndexedOperation> {
    match signed_operation.operation.into_option() {
        Some(operation) => index_from_operation(operation),
        None => Err(anyhow::anyhow!("operation does not exist in PrismOperation")),
    }
}

fn index_from_operation(prism_operation: PrismOperation) -> anyhow::Result<IntermediateIndexedOperation> {
    let operation_hash = prism_operation.operation_hash();
    match prism_operation.operation {
        Some(Operation::CreateDid(_)) => Ok(IntermediateIndexedOperation::Ssi {
            did: CanonicalPrismDid::from_operation(&prism_operation)?,
        }),
        Some(Operation::UpdateDid(op)) => Ok(IntermediateIndexedOperation::Ssi {
            did: CanonicalPrismDid::from_suffix_str(&op.id)?,
        }),
        Some(Operation::DeactivateDid(op)) => Ok(IntermediateIndexedOperation::Ssi {
            did: CanonicalPrismDid::from_suffix_str(&op.id)?,
        }),
        Some(Operation::ProtocolVersionUpdate(op)) => Ok(IntermediateIndexedOperation::Ssi {
            did: CanonicalPrismDid::from_suffix_str(&op.proposer_did)?,
        }),
        Some(Operation::CreateStorageEntry(op)) => Ok(IntermediateIndexedOperation::VdrRoot {
            operation_hash: operation_hash.to_vec(),
            did: CanonicalPrismDid::from_suffix(HexStr::from(op.did_prism_hash.as_slice()))?,
        }),
        Some(Operation::UpdateStorageEntry(op)) => Ok(IntermediateIndexedOperation::VdrChild {
            operation_hash: operation_hash.to_vec(),
            prev_operation_hash: op.previous_event_hash,
        }),
        Some(Operation::DeactivateStorageEntry(op)) => Ok(IntermediateIndexedOperation::VdrChild {
            operation_hash: operation_hash.to_vec(),
            prev_operation_hash: op.previous_event_hash,
        }),
        None => Err(anyhow::anyhow!("operation does not exist in PrismOperation")),
        Some(_) => {
            let operation_hash_hex = HexStr::from(operation_hash.as_bytes());
            Err(anyhow::anyhow!(
                "operation type in PrismOperation is not support (operation_hash: {operation_hash_hex})"
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use chrono::TimeZone;
    use identus_apollo::crypto::secp256k1::Secp256k1PrivateKey;
    use identus_apollo::hash::sha256;
    use identus_did_prism::did::operation::OperationId;
    use identus_did_prism::dlt::{BlockMetadata, SlotNo, TxId};
    use identus_did_prism::proto;

    use super::*;
    use crate::repo::{RawOperationId, RawOperationRecord};

    #[derive(Debug, derive_more::Display, derive_more::Error)]
    struct SpyError;

    /// A minimal in-memory repo that records `insert_indexed_operations` calls.
    #[derive(Default)]
    struct SpyRepo {
        /// Operations returned by `get_raw_operations_unindexed` (drained on first call).
        queue: Mutex<Vec<RawOperationRecord>>,
        /// Each `insert_indexed_operations` call, in order.
        inserts: Mutex<Vec<Vec<IndexedOperation>>>,
        /// Map `operation_hash -> record` returned by `get_raw_operation_vdr_by_operation_hash`.
        vdr_lookup: Mutex<HashMap<Vec<u8>, RawOperationRecord>>,
        /// Number of calls to `get_raw_operation_vdr_by_operation_hash`.
        vdr_lookup_calls: Mutex<usize>,
    }

    #[async_trait::async_trait]
    impl RawOperationRepo for SpyRepo {
        type Error = SpyError;

        async fn get_raw_operations_unindexed(&self) -> Result<Vec<RawOperationRecord>, Self::Error> {
            Ok(self.queue.lock().unwrap().drain(..).collect())
        }

        async fn get_raw_operations_by_did(
            &self,
            _did: &CanonicalPrismDid,
        ) -> Result<Vec<RawOperationRecord>, Self::Error> {
            Ok(vec![])
        }

        async fn get_raw_operation_vdr_by_operation_hash(
            &self,
            operation_hash: &Sha256Digest,
        ) -> Result<Option<RawOperationRecord>, Self::Error> {
            *self.vdr_lookup_calls.lock().unwrap() += 1;
            Ok(self.vdr_lookup.lock().unwrap().get(&operation_hash.to_vec()).cloned())
        }

        async fn get_raw_operations_by_tx_id(
            &self,
            _tx_id: &TxId,
        ) -> Result<Vec<(RawOperationRecord, CanonicalPrismDid)>, Self::Error> {
            Ok(vec![])
        }

        async fn get_raw_operation_by_operation_id(
            &self,
            _operation_id: &OperationId,
        ) -> Result<Option<(RawOperationRecord, CanonicalPrismDid)>, Self::Error> {
            Ok(None)
        }

        async fn insert_raw_operations(
            &self,
            _operations: Vec<(OperationMetadata, SignedPrismOperation)>,
        ) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl IndexedOperationRepo for SpyRepo {
        type Error = SpyError;

        async fn insert_indexed_operations(&self, operations: Vec<IndexedOperation>) -> Result<(), Self::Error> {
            self.inserts.lock().unwrap().push(operations);
            Ok(())
        }
    }

    // ── helpers ──

    fn signed_storage_create(did_hash: Vec<u8>, key_byte: u8) -> (SignedPrismOperation, Vec<u8>) {
        let sk = Secp256k1PrivateKey::from_slice(&[key_byte; 32]).unwrap();
        let prism_op = proto::prism::PrismOperation {
            operation: Some(proto::prism::prism_operation::Operation::CreateStorageEntry(
                proto::prism_storage::ProtoCreateStorageEntry {
                    did_prism_hash: did_hash,
                    nonce: vec![0],
                    data: Some(proto::prism_storage::proto_create_storage_entry::Data::Bytes(vec![
                        1, 2, 3,
                    ])),
                    special_fields: Default::default(),
                },
            )),
            special_fields: Default::default(),
        };
        let hash = prism_op.operation_hash().to_vec();
        let signed = SignedPrismOperation {
            signed_with: format!("k-{key_byte}"),
            signature: sk.sign(&prism_op.encode_to_vec()),
            operation: Some(prism_op).into(),
            special_fields: Default::default(),
        };
        (signed, hash)
    }

    fn signed_storage_update(prev_hash: Vec<u8>, key_byte: u8) -> SignedPrismOperation {
        let sk = Secp256k1PrivateKey::from_slice(&[key_byte; 32]).unwrap();
        let prism_op = proto::prism::PrismOperation {
            operation: Some(proto::prism::prism_operation::Operation::UpdateStorageEntry(
                proto::prism_storage::ProtoUpdateStorageEntry {
                    previous_event_hash: prev_hash,
                    data: Some(proto::prism_storage::proto_update_storage_entry::Data::Bytes(vec![
                        4, 5, 6,
                    ])),
                    special_fields: Default::default(),
                },
            )),
            special_fields: Default::default(),
        };
        SignedPrismOperation {
            signed_with: format!("k-{key_byte}"),
            signature: sk.sign(&prism_op.encode_to_vec()),
            operation: Some(prism_op).into(),
            special_fields: Default::default(),
        }
    }

    fn record(id_hi: u64, block: u64, absn: u32, osn: u32, signed: SignedPrismOperation) -> RawOperationRecord {
        RawOperationRecord {
            id: RawOperationId::from(uuid::Uuid::from_u64_pair(id_hi, 0)),
            metadata: OperationMetadata {
                block_metadata: BlockMetadata {
                    slot_number: SlotNo::from(block),
                    block_number: BlockNo::from(block),
                    cbt: chrono::Utc.timestamp_opt(0, 0).single().unwrap(),
                    absn,
                    tx_id: TxId::from(sha256([block as u8; 32])),
                },
                osn,
            },
            signed_operation: signed,
        }
    }

    fn did_suffix(byte: u8) -> Vec<u8> {
        vec![byte; 32]
    }

    // ── tests ──

    /// Indexing must call `insert_indexed_operations` exactly once per
    /// transaction, batching all of the transaction's operations together so
    /// they become visible to readers atomically.
    #[tokio::test]
    async fn run_indexer_loop_batches_one_insert_per_transaction() {
        // Transaction A (block 1): two storage roots. Transaction B (block 2): one root.
        let (op_a0, _) = signed_storage_create(did_suffix(1), 1);
        let (op_a1, _) = signed_storage_create(did_suffix(2), 2);
        let (op_b0, _) = signed_storage_create(did_suffix(3), 3);

        let repo = SpyRepo {
            queue: Mutex::new(vec![
                record(10, 1, 0, 0, op_a0),
                record(11, 1, 0, 1, op_a1),
                record(12, 2, 0, 0, op_b0),
            ]),
            ..Default::default()
        };

        run_indexer_loop(&repo).await.unwrap();

        let inserts = repo.inserts.lock().unwrap();
        assert_eq!(inserts.len(), 2, "one insert call per transaction");
        assert_eq!(inserts[0].len(), 2, "first transaction has 2 ops");
        assert_eq!(inserts[1].len(), 1, "second transaction has 1 op");
    }

    /// A storage chain whose root lives in the *same* transaction must resolve
    /// from the in-memory pending map, without a database lookup (the root is
    /// not committed yet while the batch is being built).
    #[tokio::test]
    async fn run_indexer_loop_resolves_same_tx_storage_chain_without_db() {
        let (root_op, root_hash) = signed_storage_create(did_suffix(7), 7);
        let child_op = signed_storage_update(root_hash.clone(), 8);

        let repo = SpyRepo {
            queue: Mutex::new(vec![record(20, 1, 0, 0, root_op), record(21, 1, 0, 1, child_op)]),
            ..Default::default()
        };

        run_indexer_loop(&repo).await.unwrap();

        assert_eq!(
            *repo.vdr_lookup_calls.lock().unwrap(),
            0,
            "same-tx root must resolve in-memory"
        );

        let inserts = repo.inserts.lock().unwrap();
        assert_eq!(inserts.len(), 1, "single transaction -> single atomic insert");
        assert_eq!(inserts[0].len(), 2);

        let root = inserts[0]
            .iter()
            .find(|o| {
                matches!(
                    o,
                    IndexedOperation::Vdr {
                        prev_operation_hash: None,
                        ..
                    }
                )
            })
            .expect("root should be a Vdr op");
        let child = inserts[0]
            .iter()
            .find(|o| {
                matches!(
                    o,
                    IndexedOperation::Vdr {
                        prev_operation_hash: Some(_),
                        ..
                    }
                )
            })
            .expect("child should be a Vdr op");

        let IndexedOperation::Vdr {
            did: root_did,
            operation_hash: root_op_hash,
            init_operation_hash: root_init,
            prev_operation_hash: root_prev,
            raw_operation_id: _,
        } = root
        else {
            unreachable!()
        };
        let IndexedOperation::Vdr {
            did: child_did,
            init_operation_hash: child_init,
            prev_operation_hash: child_prev,
            operation_hash: _,
            raw_operation_id: _,
        } = child
        else {
            unreachable!()
        };

        assert!(root_prev.is_none());
        assert_eq!(child_prev.as_ref().unwrap(), root_op_hash);
        assert_eq!(child_init, root_op_hash, "child init must equal root operation_hash");
        assert_eq!(root_init, root_op_hash, "root init must equal its own operation_hash");
        assert_eq!(child_did, root_did, "child did must equal root did");
    }

    /// A child whose root lives in a PRIOR, already-indexed transaction must be
    /// resolved by falling back to the database lookup.
    #[tokio::test]
    async fn run_indexer_loop_falls_back_to_db_for_cross_tx_chain() {
        let (root_op, root_hash) = signed_storage_create(did_suffix(9), 9);
        let root_did = CanonicalPrismDid::from_suffix(HexStr::from(did_suffix(9).as_slice())).unwrap();
        let child_op = signed_storage_update(root_hash.clone(), 10);

        let repo = SpyRepo {
            queue: Mutex::new(vec![record(30, 5, 0, 0, child_op)]),
            vdr_lookup: Mutex::new({
                let mut m = HashMap::new();
                m.insert(root_hash.clone(), record(31, 4, 0, 0, root_op));
                m
            }),
            ..Default::default()
        };

        run_indexer_loop(&repo).await.unwrap();

        assert!(
            *repo.vdr_lookup_calls.lock().unwrap() >= 1,
            "cross-tx root must hit the DB"
        );
        let inserts = repo.inserts.lock().unwrap();
        assert_eq!(inserts.len(), 1);
        let child = inserts[0]
            .iter()
            .find_map(|o| match o {
                IndexedOperation::Vdr {
                    prev_operation_hash: Some(_),
                    did,
                    init_operation_hash,
                    ..
                } => Some((did, init_operation_hash)),
                _ => None,
            })
            .expect("child should be a resolved Vdr op");
        let (child_did, child_init) = child;
        assert_eq!(*child_did, root_did);
        assert_eq!(child_init, &root_hash);
    }

    /// A child referencing an unknown root, with nothing in the pending map or
    /// the database, is marked `Ignored`.
    #[tokio::test]
    async fn run_indexer_loop_ignores_child_with_no_root() {
        let child_op = signed_storage_update(vec![0xab; 32], 11);
        let repo = SpyRepo {
            queue: Mutex::new(vec![record(40, 1, 0, 0, child_op)]),
            ..Default::default()
        };

        run_indexer_loop(&repo).await.unwrap();

        let inserts = repo.inserts.lock().unwrap();
        assert_eq!(inserts.len(), 1);
        assert!(matches!(inserts[0][0], IndexedOperation::Ignored { .. }));
    }
}
