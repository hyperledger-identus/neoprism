use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use identus_apollo::crypto::secp256k1::Secp256k1PrivateKey;
use identus_apollo::hash::Sha256Digest;
use identus_did_prism::did::{CanonicalPrismDid, PrismDidOps};
use identus_did_prism::dlt::OperationMetadata;
use identus_did_prism::prelude::*;
use identus_did_prism::proto;
use identus_did_prism_indexer::repo::{
    IndexedOperation, IndexedOperationRepo, RawOperationId, RawOperationRecord, RawOperationRepo,
};
use identus_did_prism_indexer::run_indexer_loop;
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
