use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use identus_apollo::crypto::secp256k1::Secp256k1PrivateKey;
use identus_apollo::hash::Sha256Digest;
use identus_did_prism::did::{CanonicalPrismDid, PrismDidOps};
use identus_did_prism::dlt::{
    BlockMetadata, BlockNo, DltCursor, OperationMetadata, PublishedPrismObject, SlotNo, TxId,
};
use identus_did_prism::prelude::*;
use identus_did_prism::proto;
use identus_did_prism::proto::prism::{PrismBlock, PrismObject};
use identus_did_prism_indexer::repo::{
    IndexedOperation, IndexedOperationRepo, RawOperationId, RawOperationRecord, RawOperationRepo,
};
use identus_did_prism_indexer::{DltSource, run_indexer_loop, run_sync_loop};
use tokio::sync::{mpsc, watch};
use uuid::Uuid;

/// Generate a unique UUID from a monotonic counter (avoids needing uuid/v4 feature).
fn next_uuid() -> Uuid {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    Uuid::from_u128(n as u128)
}

mod test_utils;

const VDR_KEY: [u8; 32] = [2; 32];
const VDR_KEY_NAME: &str = "vdr-0";

// ---------------------------------------------------------------------------
// In-memory repo mock
// ---------------------------------------------------------------------------

struct InMemoryRepo {
    raw_operations: Mutex<Vec<RawOperationRecord>>,
    indexed: Mutex<Vec<IndexedOperation>>,
}

impl InMemoryRepo {
    fn new() -> Self {
        Self {
            raw_operations: Mutex::new(vec![]),
            indexed: Mutex::new(vec![]),
        }
    }

    fn insert(&self, meta: OperationMetadata, signed_op: SignedPrismOperation) -> RawOperationId {
        let id = RawOperationId::from(next_uuid());
        self.raw_operations.lock().unwrap().push(RawOperationRecord {
            id,
            metadata: meta,
            signed_operation: signed_op,
        });
        id
    }

    fn indexed_ops(&self) -> Vec<IndexedOperationKind> {
        self.indexed
            .lock()
            .unwrap()
            .iter()
            .map(IndexedOperationKind::from)
            .collect()
    }
}

/// Simplified view of an indexed operation for test assertions.
#[derive(Debug, Clone)]
enum IndexedOperationKind {
    Ssi(CanonicalPrismDid),
    Vdr {
        did: CanonicalPrismDid,
        init_hash: Vec<u8>,
        op_hash: Vec<u8>,
        prev_hash: Option<Vec<u8>>,
    },
    Ignored,
}

impl From<&IndexedOperation> for IndexedOperationKind {
    fn from(op: &IndexedOperation) -> Self {
        match op {
            IndexedOperation::Ssi { did, .. } => Self::Ssi(did.clone()),
            IndexedOperation::Vdr {
                did,
                init_operation_hash,
                operation_hash,
                prev_operation_hash,
                ..
            } => Self::Vdr {
                did: did.clone(),
                init_hash: init_operation_hash.clone(),
                op_hash: operation_hash.clone(),
                prev_hash: prev_operation_hash.clone(),
            },
            IndexedOperation::Ignored { .. } => Self::Ignored,
        }
    }
}

impl IndexedOperationKind {
    /// Unwrap as SSI, panicking with a descriptive message if it's not.
    fn expect_ssi(&self) -> &CanonicalPrismDid {
        match self {
            Self::Ssi(did) => did,
            other => panic!("expected Ssi, got {:?}", other),
        }
    }

    /// Unwrap as VDR, panicking with a descriptive message if it's not.
    fn expect_vdr(&self) -> (&CanonicalPrismDid, &[u8], &[u8], Option<&[u8]>) {
        match self {
            Self::Vdr {
                did,
                init_hash,
                op_hash,
                prev_hash,
            } => (did, init_hash, op_hash, prev_hash.as_deref()),
            other => panic!("expected Vdr, got {:?}", other),
        }
    }

    /// Assert this is an Ignored operation.
    fn expect_ignored(&self) {
        match self {
            Self::Ignored => {}
            other => panic!("expected Ignored, got {:?}", other),
        }
    }
}

#[derive(Debug, derive_more::Display, derive_more::Error)]
#[display("mock error")]
struct MockError;

#[async_trait::async_trait]
impl RawOperationRepo for InMemoryRepo {
    type Error = MockError;

    async fn get_raw_operations_unindexed(&self) -> Result<Vec<RawOperationRecord>, Self::Error> {
        let raw = self.raw_operations.lock().unwrap();
        let indexed_ids: Vec<Uuid> = self
            .indexed
            .lock()
            .unwrap()
            .iter()
            .map(|op| *op.raw_operation_id().as_ref())
            .collect();
        Ok(raw
            .iter()
            .filter(|r| !indexed_ids.contains(r.id.as_ref()))
            .cloned()
            .collect())
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
        let raw = self.raw_operations.lock().unwrap();
        let target = operation_hash.to_vec();
        let found = raw.iter().find(|record| {
            record
                .signed_operation
                .operation
                .as_ref()
                .and_then(|o| o.operation.as_ref())
                .map(|op| {
                    let prism_op = PrismOperation {
                        operation: Some(op.clone()),
                        special_fields: Default::default(),
                    };
                    prism_op.operation_hash().to_vec() == target
                })
                .unwrap_or(false)
        });
        Ok(found.cloned())
    }

    async fn get_raw_operations_by_tx_id(
        &self,
        _tx_id: &identus_did_prism::dlt::TxId,
    ) -> Result<Vec<(RawOperationRecord, CanonicalPrismDid)>, Self::Error> {
        Ok(vec![])
    }

    async fn get_raw_operation_by_operation_id(
        &self,
        _operation_id: &identus_did_prism::did::operation::OperationId,
    ) -> Result<Option<(RawOperationRecord, CanonicalPrismDid)>, Self::Error> {
        Ok(None)
    }

    async fn insert_raw_operations(
        &self,
        operations: Vec<(OperationMetadata, SignedPrismOperation)>,
    ) -> Result<(), Self::Error> {
        for (meta, op) in operations {
            self.insert(meta, op);
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl IndexedOperationRepo for InMemoryRepo {
    type Error = MockError;

    async fn insert_indexed_operations(&self, operations: Vec<IndexedOperation>) -> Result<(), Self::Error> {
        self.indexed.lock().unwrap().extend(operations);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helpers: create a DID with a VDR key, build VDR operations
// ---------------------------------------------------------------------------

fn create_did_with_vdr_key() -> (
    SignedPrismOperation,
    Sha256Digest,
    CanonicalPrismDid,
    Secp256k1PrivateKey,
    Secp256k1PrivateKey,
) {
    let vdr_sk = Secp256k1PrivateKey::from_slice(&VDR_KEY).unwrap();
    let vdr_pk = test_utils::new_public_key(VDR_KEY_NAME, proto::prism_ssi::KeyUsage::VDR_KEY, &vdr_sk);
    let (create_did_op, create_did_op_hash, master_sk) =
        test_utils::new_create_did_operation(Some(test_utils::CreateDidOptions {
            public_keys: Some(vec![vdr_pk]),
        }));
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();
    (create_did_op, create_did_op_hash, did, master_sk, vdr_sk)
}

/// Build a signed CreateStorageEntry operation.
fn new_create_storage_op(
    did: &CanonicalPrismDid,
    vdr_sk: &Secp256k1PrivateKey,
    nonce: u8,
    data: Vec<u8>,
) -> (SignedPrismOperation, Sha256Digest) {
    test_utils::new_signed_operation(
        VDR_KEY_NAME,
        vdr_sk,
        proto::prism::prism_operation::Operation::CreateStorageEntry(proto::prism_storage::ProtoCreateStorageEntry {
            did_prism_hash: did.suffix.to_vec(),
            nonce: vec![nonce],
            data: Some(proto::prism_storage::proto_create_storage_entry::Data::Bytes(data)),
            special_fields: Default::default(),
        }),
    )
}

/// Build a signed UpdateStorageEntry operation.
fn new_update_storage_op(
    vdr_sk: &Secp256k1PrivateKey,
    prev_hash: &Sha256Digest,
    data: Vec<u8>,
) -> (SignedPrismOperation, Sha256Digest) {
    test_utils::new_signed_operation(
        VDR_KEY_NAME,
        vdr_sk,
        proto::prism::prism_operation::Operation::UpdateStorageEntry(proto::prism_storage::ProtoUpdateStorageEntry {
            previous_event_hash: prev_hash.to_vec(),
            data: Some(proto::prism_storage::proto_update_storage_entry::Data::Bytes(data)),
            special_fields: Default::default(),
        }),
    )
}

/// Build a signed DeactivateStorageEntry operation.
fn new_deactivate_storage_op(
    vdr_sk: &Secp256k1PrivateKey,
    prev_hash: &Sha256Digest,
) -> (SignedPrismOperation, Sha256Digest) {
    test_utils::new_signed_operation(
        VDR_KEY_NAME,
        vdr_sk,
        proto::prism::prism_operation::Operation::DeactivateStorageEntry(
            proto::prism_storage::ProtoDeactivateStorageEntry {
                previous_event_hash: prev_hash.to_vec(),
                special_fields: Default::default(),
            },
        ),
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn index_ssi_create_did_operation() {
    let repo = InMemoryRepo::new();
    let (create_did_op, _, did, _, _) = create_did_with_vdr_key();
    repo.insert(test_utils::dummy_metadata(0), create_did_op);

    run_indexer_loop(&repo).await.unwrap();

    let indexed = repo.indexed_ops();
    assert_eq!(indexed.len(), 1);
    assert_eq!(indexed[0].expect_ssi(), &did);
}

#[tokio::test]
async fn index_ssi_update_did_operation() {
    let repo = InMemoryRepo::new();
    let (create_did_op, create_did_op_hash, did, master_sk, _) = create_did_with_vdr_key();
    let (update_did_op, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        proto::prism::prism_operation::Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![],
            special_fields: Default::default(),
        }),
    );
    repo.insert(test_utils::dummy_metadata(0), create_did_op);
    repo.insert(test_utils::dummy_metadata(1), update_did_op);

    run_indexer_loop(&repo).await.unwrap();

    let indexed = repo.indexed_ops();
    assert_eq!(indexed.len(), 2);
    for op in &indexed {
        assert_eq!(op.expect_ssi(), &did);
    }
}

#[tokio::test]
async fn index_ssi_deactivate_did_operation() {
    let repo = InMemoryRepo::new();
    let (create_did_op, create_did_op_hash, did, master_sk, _) = create_did_with_vdr_key();
    let (deactivate_did_op, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        proto::prism::prism_operation::Operation::DeactivateDid(proto::prism_ssi::ProtoDeactivateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            special_fields: Default::default(),
        }),
    );
    repo.insert(test_utils::dummy_metadata(0), create_did_op);
    repo.insert(test_utils::dummy_metadata(1), deactivate_did_op);

    run_indexer_loop(&repo).await.unwrap();

    let indexed = repo.indexed_ops();
    assert_eq!(indexed.len(), 2);
    for op in &indexed {
        assert_eq!(op.expect_ssi(), &did);
    }
}

#[tokio::test]
async fn index_vdr_create_storage_entry_as_root() {
    let repo = InMemoryRepo::new();
    let (create_did_op, _, did, _, vdr_sk) = create_did_with_vdr_key();
    let (create_storage_op, create_storage_hash) = new_create_storage_op(&did, &vdr_sk, 0, vec![1, 2, 3]);
    repo.insert(test_utils::dummy_metadata(0), create_did_op);
    repo.insert(test_utils::dummy_metadata(1), create_storage_op);

    run_indexer_loop(&repo).await.unwrap();

    let indexed = repo.indexed_ops();
    assert_eq!(indexed.len(), 2);
    assert_eq!(indexed[0].expect_ssi(), &did);

    let (vdr_did, init_hash, op_hash, prev_hash) = indexed[1].expect_vdr();
    assert_eq!(vdr_did, &did);
    assert_eq!(init_hash, &create_storage_hash.to_vec());
    assert_eq!(op_hash, &create_storage_hash.to_vec());
    assert!(prev_hash.is_none(), "root VDR entry should have no prev_hash");
}

#[tokio::test]
async fn index_vdr_update_storage_entry_links_to_root() {
    let repo = InMemoryRepo::new();
    let (create_did_op, _, did, _, vdr_sk) = create_did_with_vdr_key();
    let (create_storage_op, create_hash) = new_create_storage_op(&did, &vdr_sk, 0, vec![1, 2, 3]);
    let (update_storage_op, update_hash) = new_update_storage_op(&vdr_sk, &create_hash, vec![4, 5, 6]);
    repo.insert(test_utils::dummy_metadata(0), create_did_op);
    repo.insert(test_utils::dummy_metadata(1), create_storage_op);
    repo.insert(test_utils::dummy_metadata(2), update_storage_op);

    run_indexer_loop(&repo).await.unwrap();

    let indexed = repo.indexed_ops();
    assert_eq!(indexed.len(), 3);

    let (vdr_did, init_hash, op_hash, prev_hash) = indexed[2].expect_vdr();
    assert_eq!(vdr_did, &did);
    assert_eq!(init_hash, &create_hash.to_vec(), "init_hash should point to root");
    assert_eq!(op_hash, &update_hash.to_vec());
    assert_eq!(prev_hash.unwrap(), &create_hash.to_vec());
}

#[tokio::test]
async fn index_vdr_deactivate_storage_entry_links_to_root() {
    let repo = InMemoryRepo::new();
    let (create_did_op, _, did, _, vdr_sk) = create_did_with_vdr_key();
    let (create_storage_op, create_hash) = new_create_storage_op(&did, &vdr_sk, 0, vec![1, 2, 3]);
    let (deactivate_op, _) = new_deactivate_storage_op(&vdr_sk, &create_hash);
    repo.insert(test_utils::dummy_metadata(0), create_did_op);
    repo.insert(test_utils::dummy_metadata(1), create_storage_op);
    repo.insert(test_utils::dummy_metadata(2), deactivate_op);

    run_indexer_loop(&repo).await.unwrap();

    let indexed = repo.indexed_ops();
    assert_eq!(indexed.len(), 3);

    let (vdr_did, init_hash, _, prev_hash) = indexed[2].expect_vdr();
    assert_eq!(vdr_did, &did);
    assert_eq!(init_hash, &create_hash.to_vec());
    assert_eq!(prev_hash.unwrap(), &create_hash.to_vec());
}

#[tokio::test]
async fn index_vdr_chain_update_then_deactivate() {
    let repo = InMemoryRepo::new();
    let (create_did_op, _, did, _, vdr_sk) = create_did_with_vdr_key();
    let (create_storage_op, create_hash) = new_create_storage_op(&did, &vdr_sk, 0, vec![1]);
    let (update_op, update_hash) = new_update_storage_op(&vdr_sk, &create_hash, vec![2]);
    let (deactivate_op, _) = new_deactivate_storage_op(&vdr_sk, &update_hash);
    repo.insert(test_utils::dummy_metadata(0), create_did_op);
    repo.insert(test_utils::dummy_metadata(1), create_storage_op);
    repo.insert(test_utils::dummy_metadata(2), update_op);
    repo.insert(test_utils::dummy_metadata(3), deactivate_op);

    run_indexer_loop(&repo).await.unwrap();

    let indexed = repo.indexed_ops();
    assert_eq!(indexed.len(), 4);

    // All VDR operations should trace back to the same root init_hash
    for op in &indexed[1..] {
        let (vdr_did, init_hash, _, _) = op.expect_vdr();
        assert_eq!(init_hash, &create_hash.to_vec());
        assert_eq!(vdr_did, &did);
    }
}

#[tokio::test]
async fn index_orphan_vdr_child_is_ignored() {
    let repo = InMemoryRepo::new();
    let (create_did_op, _, _, _, vdr_sk) = create_did_with_vdr_key();

    // UpdateStorageEntry referencing a non-existent parent
    let fake_parent_hash = Sha256Digest::from_bytes(&[0xAA; 32]).unwrap();
    let (orphan_op, _) = new_update_storage_op(&vdr_sk, &fake_parent_hash, vec![9, 9, 9]);
    repo.insert(test_utils::dummy_metadata(0), create_did_op);
    repo.insert(test_utils::dummy_metadata(1), orphan_op);

    run_indexer_loop(&repo).await.unwrap();

    let indexed = repo.indexed_ops();
    assert_eq!(indexed.len(), 2);
    indexed[1].expect_ignored();
}

#[tokio::test]
async fn index_empty_operation_is_ignored() {
    let repo = InMemoryRepo::new();
    let empty_signed_op = SignedPrismOperation {
        signed_with: "master-0".to_string(),
        signature: vec![],
        operation: None.into(),
        special_fields: Default::default(),
    };
    repo.insert(test_utils::dummy_metadata(0), empty_signed_op);

    run_indexer_loop(&repo).await.unwrap();

    let indexed = repo.indexed_ops();
    assert_eq!(indexed.len(), 1);
    indexed[0].expect_ignored();
}

#[tokio::test]
async fn index_loop_terminates_when_all_indexed() {
    let repo = InMemoryRepo::new();
    let (create_did_op, _, _, _, _) = create_did_with_vdr_key();
    repo.insert(test_utils::dummy_metadata(0), create_did_op);

    run_indexer_loop(&repo).await.unwrap();
    assert_eq!(repo.indexed_ops().len(), 1);

    // Second run should be a no-op
    run_indexer_loop(&repo).await.unwrap();
    assert_eq!(repo.indexed_ops().len(), 1);
}

#[tokio::test]
async fn index_multiple_independent_vdr_entries() {
    let repo = InMemoryRepo::new();
    let (create_did_op, _, did, _, vdr_sk) = create_did_with_vdr_key();
    let (storage1, _) = new_create_storage_op(&did, &vdr_sk, 1, vec![10]);
    let (storage2, _) = new_create_storage_op(&did, &vdr_sk, 2, vec![20]);
    repo.insert(test_utils::dummy_metadata(0), create_did_op);
    repo.insert(test_utils::dummy_metadata(1), storage1);
    repo.insert(test_utils::dummy_metadata(2), storage2);

    run_indexer_loop(&repo).await.unwrap();

    let indexed = repo.indexed_ops();
    assert_eq!(indexed.len(), 3);

    // Each VDR root should have its own distinct init_hash
    let init_hashes: Vec<_> = indexed[1..].iter().map(|op| op.expect_vdr().1.to_vec()).collect();
    assert_ne!(
        init_hashes[0], init_hashes[1],
        "independent VDR entries should have different init hashes"
    );
}

// ---------------------------------------------------------------------------
// Tests for index_from_operation branches
// ---------------------------------------------------------------------------

#[tokio::test]
async fn index_protocol_version_update_operation() {
    let repo = InMemoryRepo::new();
    let (create_did_op, _create_did_op_hash, did, master_sk, _) = create_did_with_vdr_key();

    let (pvu_op, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        proto::prism::prism_operation::Operation::ProtocolVersionUpdate(
            proto::prism_version::ProtoProtocolVersionUpdate {
                proposer_did: did.suffix_hex().to_string(),
                version: Some(proto::prism_version::ProtocolVersionInfo {
                    version_name: "2.0.0".to_string(),
                    effective_since: 1000,
                    protocol_version: Some(proto::prism_version::ProtocolVersion {
                        major_version: 2,
                        minor_version: 0,
                        special_fields: Default::default(),
                    })
                    .into(),
                    special_fields: Default::default(),
                })
                .into(),
                special_fields: Default::default(),
            },
        ),
    );
    repo.insert(test_utils::dummy_metadata(0), create_did_op);
    repo.insert(test_utils::dummy_metadata(1), pvu_op);

    run_indexer_loop(&repo).await.unwrap();

    let indexed = repo.indexed_ops();
    assert_eq!(indexed.len(), 2);
    // ProtocolVersionUpdate is indexed as SSI with the proposer DID
    assert_eq!(indexed[1].expect_ssi(), &did);
}

#[tokio::test]
async fn index_prism_operation_with_no_inner_operation() {
    let repo = InMemoryRepo::new();

    // Create a PrismOperation with operation = None, wrapped in SignedPrismOperation
    let empty_prism_op = proto::prism::PrismOperation {
        operation: None,
        special_fields: Default::default(),
    };
    let signed_op = proto::prism::SignedPrismOperation {
        signed_with: "master-0".to_string(),
        signature: vec![],
        operation: Some(empty_prism_op).into(),
        special_fields: Default::default(),
    };
    repo.insert(test_utils::dummy_metadata(0), signed_op);

    run_indexer_loop(&repo).await.unwrap();

    let indexed = repo.indexed_ops();
    assert_eq!(indexed.len(), 1);
    indexed[0].expect_ignored();
}

// ---------------------------------------------------------------------------
// Tests for recursively_find_vdr_root edge cases
// ---------------------------------------------------------------------------

#[tokio::test]
async fn index_vdr_child_with_invalid_parent_hash_length() {
    let repo = InMemoryRepo::new();
    let (_, _, _, _, vdr_sk) = create_did_with_vdr_key();

    // Construct an UpdateStorageEntry with a prev hash that is NOT 32 bytes
    let malformed_op = test_utils::new_signed_operation(
        VDR_KEY_NAME,
        &vdr_sk,
        proto::prism::prism_operation::Operation::UpdateStorageEntry(proto::prism_storage::ProtoUpdateStorageEntry {
            previous_event_hash: vec![0xAA; 16], // Only 16 bytes, not 32
            data: Some(proto::prism_storage::proto_update_storage_entry::Data::Bytes(vec![
                1, 2, 3,
            ])),
            special_fields: Default::default(),
        }),
    );
    repo.insert(test_utils::dummy_metadata(0), malformed_op.0);

    run_indexer_loop(&repo).await.unwrap();

    let indexed = repo.indexed_ops();
    assert_eq!(indexed.len(), 1);
    indexed[0].expect_ignored();
}

#[tokio::test]
async fn index_vdr_child_pointing_to_ssi_operation_is_ignored() {
    let repo = InMemoryRepo::new();
    let (create_did_op, create_did_op_hash, _, _, vdr_sk) = create_did_with_vdr_key();

    // An UpdateStorageEntry whose previous_event_hash matches the SSI CreateDid operation hash
    let (vdr_child_op, _) = new_update_storage_op(&vdr_sk, &create_did_op_hash, vec![9, 9, 9]);
    repo.insert(test_utils::dummy_metadata(0), create_did_op);
    repo.insert(test_utils::dummy_metadata(1), vdr_child_op);

    run_indexer_loop(&repo).await.unwrap();

    let indexed = repo.indexed_ops();
    assert_eq!(indexed.len(), 2);
    // CreateDid is indexed as SSI
    indexed[0].expect_ssi();
    // The VDR child whose parent is an SSI operation should be ignored
    indexed[1].expect_ignored();
}

#[tokio::test]
async fn index_vdr_child_exceeding_max_depth_is_ignored() {
    let repo = InMemoryRepo::new();
    let (create_did_op, _, did, _, vdr_sk) = create_did_with_vdr_key();
    repo.insert(test_utils::dummy_metadata(0), create_did_op);

    // Build a chain of VDR operations longer than the indexer can walk back.
    // recursively_find_vdr_root iterates `1..SEARCH_MAX_DEPTH` (199 lookups),
    // so update #199 is the deepest indexable entry and update #200 falls off.
    let mut prev_hash = Sha256Digest::from_bytes(&[0u8; 32]).unwrap();

    // Insert a root CreateStorageEntry followed by 200 chained UpdateStorageEntry operations
    for i in 0..201u32 {
        let (op, hash) = if i == 0 {
            new_create_storage_op(&did, &vdr_sk, 0, vec![1])
        } else {
            new_update_storage_op(&vdr_sk, &prev_hash, vec![i as u8])
        };
        repo.insert(test_utils::dummy_metadata(i + 1), op);
        prev_hash = hash;
    }

    run_indexer_loop(&repo).await.unwrap();

    let indexed = repo.indexed_ops();
    // CreateDid + CreateStorageEntry root + 200 UpdateStorageEntry = 202 operations
    assert_eq!(indexed.len(), 202);

    // Pin the exact boundary: update #199 is still indexed as VDR,
    // update #200 exceeds max depth and is ignored
    indexed[indexed.len() - 2].expect_vdr();
    indexed.last().unwrap().expect_ignored();
}

// ---------------------------------------------------------------------------
// Tests for run_sync_loop
// ---------------------------------------------------------------------------

struct MockDltSource {
    rx: Option<mpsc::Receiver<PublishedPrismObject>>,
    cursor_rx: watch::Receiver<Option<DltCursor>>,
}

impl DltSource for MockDltSource {
    fn sync_cursor(&self) -> watch::Receiver<Option<DltCursor>> {
        self.cursor_rx.clone()
    }

    fn into_stream(self) -> Result<mpsc::Receiver<PublishedPrismObject>, String> {
        self.rx.ok_or_else(|| "no receiver configured".to_string())
    }
}

fn mock_dlt_source_with_channel() -> (MockDltSource, mpsc::Sender<PublishedPrismObject>) {
    let (tx, rx) = mpsc::channel::<PublishedPrismObject>(64);
    let (_, cursor_rx) = watch::channel::<Option<DltCursor>>(None);
    let source = MockDltSource {
        rx: Some(rx),
        cursor_rx,
    };
    (source, tx)
}

fn make_published_object(block_metadata: BlockMetadata, operations: Vec<SignedPrismOperation>) -> PublishedPrismObject {
    PublishedPrismObject {
        block_metadata,
        prism_object: PrismObject {
            block_content: Some(PrismBlock {
                operations,
                special_fields: Default::default(),
            })
            .into(),
            special_fields: Default::default(),
        },
    }
}

#[tokio::test]
async fn sync_loop_inserts_valid_operations() {
    let repo = InMemoryRepo::new();
    let (source, tx) = mock_dlt_source_with_channel();
    let (create_did_op, _, _did, _, _) = create_did_with_vdr_key();

    let meta = test_utils::dummy_metadata(0).block_metadata;
    let obj = make_published_object(meta.clone(), vec![create_did_op]);
    tx.send(obj).await.unwrap();
    drop(tx); // Close the source

    run_sync_loop(&repo, source).await.unwrap();

    // Verify the raw operation was inserted
    let unindexed = repo.raw_operations.lock().unwrap();
    assert_eq!(unindexed.len(), 1);
    assert_eq!(unindexed[0].metadata.osn, 0);
    assert_eq!(unindexed[0].metadata.block_metadata.tx_id, meta.tx_id);
}

#[tokio::test]
async fn sync_loop_skips_operations_without_inner_operation() {
    let repo = InMemoryRepo::new();
    let (source, tx) = mock_dlt_source_with_channel();

    // Create a valid operation
    let (create_did_op, _, _, _, _) = create_did_with_vdr_key();

    // Create an operation with no inner operation (empty PrismOperation)
    let empty_op = proto::prism::SignedPrismOperation {
        signed_with: "master-0".to_string(),
        signature: vec![],
        operation: Some(proto::prism::PrismOperation {
            operation: None,
            special_fields: Default::default(),
        })
        .into(),
        special_fields: Default::default(),
    };

    // Create an operation with no PrismOperation at all
    let no_op = proto::prism::SignedPrismOperation {
        signed_with: "master-0".to_string(),
        signature: vec![],
        operation: None.into(),
        special_fields: Default::default(),
    };

    let meta = test_utils::dummy_metadata(0).block_metadata;
    let obj = make_published_object(meta.clone(), vec![create_did_op, empty_op, no_op]);
    tx.send(obj).await.unwrap();
    drop(tx);

    run_sync_loop(&repo, source).await.unwrap();

    // Only the valid operation should be inserted
    let unindexed = repo.raw_operations.lock().unwrap();
    assert_eq!(unindexed.len(), 1);
}

#[tokio::test]
async fn sync_loop_handles_none_block_content() {
    let repo = InMemoryRepo::new();
    let (source, tx) = mock_dlt_source_with_channel();

    let meta = test_utils::dummy_metadata(0).block_metadata;
    let obj = PublishedPrismObject {
        block_metadata: meta,
        prism_object: PrismObject {
            block_content: None.into(),
            special_fields: Default::default(),
        },
    };
    tx.send(obj).await.unwrap();
    drop(tx);

    run_sync_loop(&repo, source).await.unwrap();

    // No operations should be inserted
    let unindexed = repo.raw_operations.lock().unwrap();
    assert!(unindexed.is_empty());
}

#[tokio::test]
async fn sync_loop_processes_multiple_blocks() {
    let repo = InMemoryRepo::new();
    let (source, tx) = mock_dlt_source_with_channel();
    let (create_did_op, _, _, _, _) = create_did_with_vdr_key();

    // Send two blocks with different metadata
    for i in 0..2u32 {
        let meta = BlockMetadata {
            slot_number: SlotNo::from(i as u64),
            block_number: BlockNo::from(i as u64),
            cbt: chrono::DateTime::UNIX_EPOCH,
            absn: 0,
            tx_id: TxId::from(identus_apollo::hash::sha256([i as u8; 32])),
        };
        let obj = make_published_object(meta, vec![create_did_op.clone()]);
        tx.send(obj).await.unwrap();
    }
    drop(tx);

    run_sync_loop(&repo, source).await.unwrap();

    let unindexed = repo.raw_operations.lock().unwrap();
    assert_eq!(unindexed.len(), 2);
    // Verify the operations came from different blocks
    assert_ne!(
        unindexed[0].metadata.block_metadata.block_number,
        unindexed[1].metadata.block_metadata.block_number
    );
}

#[tokio::test]
async fn sync_loop_continues_on_insert_error() {
    let repo = FailingInsertRepo::new();
    let (source, tx) = mock_dlt_source_with_channel();
    let (create_did_op, _, _, _, _) = create_did_with_vdr_key();

    // Send two blocks: the first insert fails, the loop must keep going and store the second
    for i in 0..2u32 {
        let meta = BlockMetadata {
            slot_number: SlotNo::from(i as u64),
            block_number: BlockNo::from(i as u64),
            cbt: chrono::DateTime::UNIX_EPOCH,
            absn: 0,
            tx_id: TxId::from(identus_apollo::hash::sha256([i as u8; 32])),
        };
        let obj = make_published_object(meta, vec![create_did_op.clone()]);
        tx.send(obj).await.unwrap();
    }
    drop(tx);

    // The first insert error is logged and the loop continues
    run_sync_loop(&repo, source).await.unwrap();

    let inserted = repo.inserted.lock().unwrap();
    assert_eq!(inserted.len(), 1, "second block should be stored after the first fails");
    assert_eq!(inserted[0].0.block_metadata.block_number, BlockNo::from(1u64));
}

/// A repo whose first insert_raw_operations call fails and subsequent calls succeed,
/// to test that run_sync_loop continues after an insert error.
struct FailingInsertRepo {
    failed_once: Mutex<bool>,
    inserted: Mutex<Vec<(OperationMetadata, SignedPrismOperation)>>,
}

impl FailingInsertRepo {
    fn new() -> Self {
        Self {
            failed_once: Mutex::new(false),
            inserted: Mutex::new(vec![]),
        }
    }
}

#[async_trait::async_trait]
impl RawOperationRepo for FailingInsertRepo {
    type Error = MockError;

    async fn get_raw_operations_unindexed(&self) -> Result<Vec<RawOperationRecord>, Self::Error> {
        Ok(vec![])
    }

    async fn get_raw_operations_by_did(
        &self,
        _did: &CanonicalPrismDid,
    ) -> Result<Vec<RawOperationRecord>, Self::Error> {
        Ok(vec![])
    }

    async fn get_raw_operation_vdr_by_operation_hash(
        &self,
        _operation_hash: &Sha256Digest,
    ) -> Result<Option<RawOperationRecord>, Self::Error> {
        Ok(None)
    }

    async fn get_raw_operations_by_tx_id(
        &self,
        _tx_id: &TxId,
    ) -> Result<Vec<(RawOperationRecord, CanonicalPrismDid)>, Self::Error> {
        Ok(vec![])
    }

    async fn get_raw_operation_by_operation_id(
        &self,
        _operation_id: &identus_did_prism::did::operation::OperationId,
    ) -> Result<Option<(RawOperationRecord, CanonicalPrismDid)>, Self::Error> {
        Ok(None)
    }

    async fn insert_raw_operations(
        &self,
        operations: Vec<(OperationMetadata, SignedPrismOperation)>,
    ) -> Result<(), Self::Error> {
        let mut failed_once = self.failed_once.lock().unwrap();
        if !*failed_once {
            *failed_once = true;
            return Err(MockError);
        }
        self.inserted.lock().unwrap().extend(operations);
        Ok(())
    }
}

#[async_trait::async_trait]
impl IndexedOperationRepo for FailingInsertRepo {
    type Error = MockError;

    async fn insert_indexed_operations(&self, _operations: Vec<IndexedOperation>) -> Result<(), Self::Error> {
        Ok(())
    }
}
