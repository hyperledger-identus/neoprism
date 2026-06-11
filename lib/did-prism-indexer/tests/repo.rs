use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use identus_apollo::hash::{Sha256Digest, sha256};
use identus_did_prism::did::CanonicalPrismDid;
use identus_did_prism::did::operation::OperationId;
use identus_did_prism::dlt::{BlockMetadata, BlockNo, DltCursor, OperationMetadata, SlotNo, TxId};
use identus_did_prism::prelude::*;
use identus_did_prism::utils::paging::Paginated;
use identus_did_prism_indexer::repo::{
    DltCursorRepo, IndexedOperation, IndexedOperationRepo, IndexerStateRepo, RawOperationId, RawOperationRecord,
    RawOperationRepo,
};
use uuid::Uuid;

/// Generate a unique UUID from a monotonic counter.
fn next_uuid() -> Uuid {
    static COUNTER: AtomicU64 = AtomicU64::new(1000);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    Uuid::from_u128(n as u128)
}

// ---------------------------------------------------------------------------
// Mock error
// ---------------------------------------------------------------------------

#[derive(Debug, derive_more::Display, derive_more::Error)]
#[display("mock error")]
struct MockError;

// ---------------------------------------------------------------------------
// Mock implementations for each trait
// ---------------------------------------------------------------------------

struct MockRawRepo {
    unindexed_result: Mutex<Vec<RawOperationRecord>>,
    by_did_result: Mutex<Vec<RawOperationRecord>>,
    vdr_by_hash_result: Mutex<Option<RawOperationRecord>>,
    by_tx_id_result: Mutex<Vec<(RawOperationRecord, CanonicalPrismDid)>>,
    by_op_id_result: Mutex<Option<(RawOperationRecord, CanonicalPrismDid)>>,
    inserted: Mutex<Vec<(OperationMetadata, SignedPrismOperation)>>,
}

impl MockRawRepo {
    fn new() -> Self {
        Self {
            unindexed_result: Mutex::new(vec![]),
            by_did_result: Mutex::new(vec![]),
            vdr_by_hash_result: Mutex::new(None),
            by_tx_id_result: Mutex::new(vec![]),
            by_op_id_result: Mutex::new(None),
            inserted: Mutex::new(vec![]),
        }
    }
}

#[async_trait::async_trait]
impl RawOperationRepo for MockRawRepo {
    type Error = MockError;

    async fn get_raw_operations_unindexed(&self) -> Result<Vec<RawOperationRecord>, Self::Error> {
        Ok(self.unindexed_result.lock().unwrap().clone())
    }

    async fn get_raw_operations_by_did(
        &self,
        _did: &CanonicalPrismDid,
    ) -> Result<Vec<RawOperationRecord>, Self::Error> {
        Ok(self.by_did_result.lock().unwrap().clone())
    }

    async fn get_raw_operation_vdr_by_operation_hash(
        &self,
        _operation_hash: &Sha256Digest,
    ) -> Result<Option<RawOperationRecord>, Self::Error> {
        Ok(self.vdr_by_hash_result.lock().unwrap().clone())
    }

    async fn get_raw_operations_by_tx_id(
        &self,
        _tx_id: &TxId,
    ) -> Result<Vec<(RawOperationRecord, CanonicalPrismDid)>, Self::Error> {
        Ok(self.by_tx_id_result.lock().unwrap().clone())
    }

    async fn get_raw_operation_by_operation_id(
        &self,
        _operation_id: &OperationId,
    ) -> Result<Option<(RawOperationRecord, CanonicalPrismDid)>, Self::Error> {
        Ok(self.by_op_id_result.lock().unwrap().clone())
    }

    async fn insert_raw_operations(
        &self,
        operations: Vec<(OperationMetadata, SignedPrismOperation)>,
    ) -> Result<(), Self::Error> {
        self.inserted.lock().unwrap().extend(operations);
        Ok(())
    }
}

struct MockIndexedRepo {
    inserted: Mutex<Vec<IndexedOperation>>,
}

impl MockIndexedRepo {
    fn new() -> Self {
        Self {
            inserted: Mutex::new(vec![]),
        }
    }
}

#[async_trait::async_trait]
impl IndexedOperationRepo for MockIndexedRepo {
    type Error = MockError;

    async fn insert_indexed_operations(&self, operations: Vec<IndexedOperation>) -> Result<(), Self::Error> {
        self.inserted.lock().unwrap().extend(operations);
        Ok(())
    }
}

struct MockStateRepo {
    last_block: Mutex<Option<(SlotNo, BlockNo)>>,
    all_dids: Mutex<Paginated<CanonicalPrismDid>>,
    did_by_vdr: Mutex<Option<CanonicalPrismDid>>,
}

impl MockStateRepo {
    fn new() -> Self {
        Self {
            last_block: Mutex::new(None),
            all_dids: Mutex::new(Paginated {
                items: vec![],
                current_page: 0,
                page_size: 10,
                total_items: 0,
            }),
            did_by_vdr: Mutex::new(None),
        }
    }
}

#[async_trait::async_trait]
impl IndexerStateRepo for MockStateRepo {
    type Error = MockError;

    async fn get_last_indexed_block(&self) -> Result<Option<(SlotNo, BlockNo)>, Self::Error> {
        Ok(*self.last_block.lock().unwrap())
    }

    async fn get_all_dids(&self, _page: u32, _page_size: u32) -> Result<Paginated<CanonicalPrismDid>, Self::Error> {
        Ok(self.all_dids.lock().unwrap().clone())
    }

    async fn get_did_by_vdr_entry(
        &self,
        _operation_hash: &Sha256Digest,
    ) -> Result<Option<CanonicalPrismDid>, Self::Error> {
        Ok(self.did_by_vdr.lock().unwrap().clone())
    }
}

struct MockCursorRepo {
    cursor: Mutex<Option<DltCursor>>,
}

impl MockCursorRepo {
    fn new() -> Self {
        Self {
            cursor: Mutex::new(None),
        }
    }
}

#[async_trait::async_trait]
impl DltCursorRepo for MockCursorRepo {
    type Error = MockError;

    async fn set_cursor(&self, cursor: DltCursor) -> Result<(), Self::Error> {
        *self.cursor.lock().unwrap() = Some(cursor);
        Ok(())
    }

    async fn get_cursor(&self) -> Result<Option<DltCursor>, Self::Error> {
        Ok(self.cursor.lock().unwrap().clone())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn dummy_metadata(osn: u32) -> OperationMetadata {
    use chrono::DateTime;
    OperationMetadata {
        block_metadata: BlockMetadata {
            slot_number: SlotNo::from(100),
            block_number: BlockNo::from(50),
            cbt: DateTime::UNIX_EPOCH,
            absn: 0,
            tx_id: TxId::from(sha256([0u8; 32])),
        },
        osn,
    }
}

fn dummy_signed_operation() -> SignedPrismOperation {
    SignedPrismOperation {
        signed_with: "master-0".to_string(),
        signature: vec![],
        operation: None.into(),
        special_fields: Default::default(),
    }
}

fn dummy_raw_record(id: Uuid) -> RawOperationRecord {
    RawOperationRecord {
        id: RawOperationId::from(id),
        metadata: dummy_metadata(0),
        signed_operation: dummy_signed_operation(),
    }
}

fn dummy_cursor() -> DltCursor {
    DltCursor {
        slot: 42,
        block_hash: vec![1, 2, 3],
        cbt: None,
        blockfrost_page: None,
    }
}

fn dummy_did() -> CanonicalPrismDid {
    CanonicalPrismDid::from_suffix_str("0000000000000000000000000000000000000000000000000000000000000000").unwrap()
}

// ---------------------------------------------------------------------------
// RawOperationId tests
// ---------------------------------------------------------------------------

#[test]
fn raw_operation_id_from_uuid() {
    let uuid = next_uuid();
    let id = RawOperationId::from(uuid);
    assert_eq!(*id.as_ref(), uuid);
}

#[test]
fn raw_operation_id_into_uuid() {
    let uuid = next_uuid();
    let id = RawOperationId::from(uuid);
    let roundtrip: Uuid = id.into();
    assert_eq!(roundtrip, uuid);
}

#[test]
fn raw_operation_id_as_ref() {
    let uuid = next_uuid();
    let id = RawOperationId::from(uuid);
    assert_eq!(id.as_ref(), &uuid);
}

#[test]
fn raw_operation_id_clone() {
    let uuid = next_uuid();
    let id1 = RawOperationId::from(uuid);
    let id2 = id1;
    assert_eq!(id1.as_ref(), id2.as_ref());
}

#[test]
fn raw_operation_id_debug() {
    let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
    let id = RawOperationId::from(uuid);
    let debug_str = format!("{:?}", id);
    assert!(debug_str.contains("550e8400"));
}

// ---------------------------------------------------------------------------
// RawOperationRecord tests
// ---------------------------------------------------------------------------

#[test]
fn raw_operation_record_construction() {
    let uuid = next_uuid();
    let record = dummy_raw_record(uuid);
    assert_eq!(*record.id.as_ref(), uuid);
    assert_eq!(record.metadata.osn, 0);
    assert_eq!(record.signed_operation.signed_with, "master-0");
}

#[test]
fn raw_operation_record_clone() {
    let uuid = next_uuid();
    let record = dummy_raw_record(uuid);
    let cloned = record.clone();
    assert_eq!(*cloned.id.as_ref(), *record.id.as_ref());
}

// ---------------------------------------------------------------------------
// IndexedOperation::raw_operation_id() tests
// ---------------------------------------------------------------------------

#[test]
fn indexed_operation_raw_operation_id_ssi_variant() {
    let uuid = next_uuid();
    let raw_id = RawOperationId::from(uuid);
    let op = IndexedOperation::Ssi {
        raw_operation_id: raw_id,
        did: dummy_did(),
    };
    assert_eq!(*op.raw_operation_id().as_ref(), uuid);
}

#[test]
fn indexed_operation_raw_operation_id_vdr_variant() {
    let uuid = next_uuid();
    let raw_id = RawOperationId::from(uuid);
    let op = IndexedOperation::Vdr {
        raw_operation_id: raw_id,
        operation_hash: vec![1, 2, 3],
        init_operation_hash: vec![4, 5, 6],
        prev_operation_hash: Some(vec![7, 8, 9]),
        did: dummy_did(),
    };
    assert_eq!(*op.raw_operation_id().as_ref(), uuid);
}

#[test]
fn indexed_operation_raw_operation_id_vdr_variant_no_prev() {
    let uuid = next_uuid();
    let raw_id = RawOperationId::from(uuid);
    let op = IndexedOperation::Vdr {
        raw_operation_id: raw_id,
        operation_hash: vec![1, 2, 3],
        init_operation_hash: vec![4, 5, 6],
        prev_operation_hash: None,
        did: dummy_did(),
    };
    assert_eq!(*op.raw_operation_id().as_ref(), uuid);
}

#[test]
fn indexed_operation_raw_operation_id_ignored_variant() {
    let uuid = next_uuid();
    let raw_id = RawOperationId::from(uuid);
    let op = IndexedOperation::Ignored {
        raw_operation_id: raw_id,
    };
    assert_eq!(*op.raw_operation_id().as_ref(), uuid);
}

// ---------------------------------------------------------------------------
// Arc<T> delegation: RawOperationRepo
// ---------------------------------------------------------------------------

#[tokio::test]
async fn arc_raw_repo_get_raw_operations_unindexed() {
    let mock = MockRawRepo::new();
    let record = dummy_raw_record(next_uuid());
    mock.unindexed_result.lock().unwrap().push(record.clone());

    let arc: Arc<dyn RawOperationRepo<Error = MockError> + Send + Sync> = Arc::new(mock);
    let result = arc.get_raw_operations_unindexed().await.unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(*result[0].id.as_ref(), *record.id.as_ref());
}

#[tokio::test]
async fn arc_raw_repo_get_raw_operations_unindexed_empty() {
    let mock = MockRawRepo::new();
    let arc: Arc<dyn RawOperationRepo<Error = MockError> + Send + Sync> = Arc::new(mock);
    let result = arc.get_raw_operations_unindexed().await.unwrap();
    assert!(result.is_empty());
}

#[tokio::test]
async fn arc_raw_repo_get_raw_operations_by_did() {
    let mock = MockRawRepo::new();
    let record = dummy_raw_record(next_uuid());
    mock.by_did_result.lock().unwrap().push(record);

    let did = dummy_did();

    let arc: Arc<dyn RawOperationRepo<Error = MockError> + Send + Sync> = Arc::new(mock);
    let result = arc.get_raw_operations_by_did(&did).await.unwrap();
    assert_eq!(result.len(), 1);
}

#[tokio::test]
async fn arc_raw_repo_get_raw_operation_vdr_by_operation_hash() {
    let mock = MockRawRepo::new();
    let record = dummy_raw_record(next_uuid());
    mock.vdr_by_hash_result.lock().unwrap().replace(record);

    let hash = sha256([1u8; 32]);

    let arc: Arc<dyn RawOperationRepo<Error = MockError> + Send + Sync> = Arc::new(mock);
    let result = arc.get_raw_operation_vdr_by_operation_hash(&hash).await.unwrap();
    assert!(result.is_some());
}

#[tokio::test]
async fn arc_raw_repo_get_raw_operation_vdr_by_operation_hash_none() {
    let mock = MockRawRepo::new();
    let hash = sha256([1u8; 32]);

    let arc: Arc<dyn RawOperationRepo<Error = MockError> + Send + Sync> = Arc::new(mock);
    let result = arc.get_raw_operation_vdr_by_operation_hash(&hash).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn arc_raw_repo_get_raw_operations_by_tx_id() {
    let mock = MockRawRepo::new();
    let did = dummy_did();
    let record = dummy_raw_record(next_uuid());
    mock.by_tx_id_result.lock().unwrap().push((record, did.clone()));

    let tx_id = TxId::from(sha256([2u8; 32]));

    let arc: Arc<dyn RawOperationRepo<Error = MockError> + Send + Sync> = Arc::new(mock);
    let result = arc.get_raw_operations_by_tx_id(&tx_id).await.unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].1, did);
}

#[tokio::test]
async fn arc_raw_repo_get_raw_operation_by_operation_id() {
    let mock = MockRawRepo::new();
    let did = dummy_did();
    let record = dummy_raw_record(next_uuid());
    mock.by_op_id_result.lock().unwrap().replace((record, did.clone()));

    let op_id = OperationId::from(Sha256Digest::from_bytes(&[3u8; 32]).unwrap());

    let arc: Arc<dyn RawOperationRepo<Error = MockError> + Send + Sync> = Arc::new(mock);
    let result = arc.get_raw_operation_by_operation_id(&op_id).await.unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().1, did);
}

#[tokio::test]
async fn arc_raw_repo_get_raw_operation_by_operation_id_none() {
    let mock = MockRawRepo::new();
    let op_id = OperationId::from(Sha256Digest::from_bytes(&[3u8; 32]).unwrap());

    let arc: Arc<dyn RawOperationRepo<Error = MockError> + Send + Sync> = Arc::new(mock);
    let result = arc.get_raw_operation_by_operation_id(&op_id).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn arc_raw_repo_insert_raw_operations() {
    let mock = MockRawRepo::new();

    // Keep a reference to the mock to inspect later
    let mock_ptr = Arc::new(mock);
    let meta = dummy_metadata(0);
    let signed_op = dummy_signed_operation();

    let arc: Arc<dyn RawOperationRepo<Error = MockError> + Send + Sync> = mock_ptr.clone();
    arc.insert_raw_operations(vec![(meta, signed_op)]).await.unwrap();

    // Access through the concrete Arc<MockRawRepo>
    let inner = mock_ptr.as_ref() as &MockRawRepo;
    assert_eq!(inner.inserted.lock().unwrap().len(), 1);
}

// ---------------------------------------------------------------------------
// Arc<T> delegation: IndexedOperationRepo
// ---------------------------------------------------------------------------

#[tokio::test]
async fn arc_indexed_repo_insert_indexed_operations() {
    let mock = MockIndexedRepo::new();
    let mock_ptr = Arc::new(mock);

    let raw_id = RawOperationId::from(next_uuid());
    let ops = vec![IndexedOperation::Ssi {
        raw_operation_id: raw_id,
        did: dummy_did(),
    }];

    let arc: Arc<dyn IndexedOperationRepo<Error = MockError> + Send + Sync> = mock_ptr.clone();
    arc.insert_indexed_operations(ops).await.unwrap();

    let inner = mock_ptr.as_ref() as &MockIndexedRepo;
    assert_eq!(inner.inserted.lock().unwrap().len(), 1);
}

// ---------------------------------------------------------------------------
// Arc<T> delegation: IndexerStateRepo
// ---------------------------------------------------------------------------

#[tokio::test]
async fn arc_state_repo_get_last_indexed_block_some() {
    let mock = MockStateRepo::new();
    *mock.last_block.lock().unwrap() = Some((SlotNo::from(42), BlockNo::from(10)));

    let arc: Arc<dyn IndexerStateRepo<Error = MockError> + Send + Sync> = Arc::new(mock);
    let result = arc.get_last_indexed_block().await.unwrap();
    assert_eq!(result, Some((SlotNo::from(42), BlockNo::from(10))));
}

#[tokio::test]
async fn arc_state_repo_get_last_indexed_block_none() {
    let mock = MockStateRepo::new();

    let arc: Arc<dyn IndexerStateRepo<Error = MockError> + Send + Sync> = Arc::new(mock);
    let result = arc.get_last_indexed_block().await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn arc_state_repo_get_all_dids() {
    let mock = MockStateRepo::new();
    let did = dummy_did();
    *mock.all_dids.lock().unwrap() = Paginated {
        items: vec![did],
        current_page: 1,
        page_size: 10,
        total_items: 1,
    };

    let arc: Arc<dyn IndexerStateRepo<Error = MockError> + Send + Sync> = Arc::new(mock);
    let result = arc.get_all_dids(1, 10).await.unwrap();
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.total_items, 1);
}

#[tokio::test]
async fn arc_state_repo_get_did_by_vdr_entry_some() {
    let mock = MockStateRepo::new();
    let did = dummy_did();
    *mock.did_by_vdr.lock().unwrap() = Some(did.clone());

    let hash = sha256([4u8; 32]);

    let arc: Arc<dyn IndexerStateRepo<Error = MockError> + Send + Sync> = Arc::new(mock);
    let result = arc.get_did_by_vdr_entry(&hash).await.unwrap();
    assert_eq!(result, Some(did));
}

#[tokio::test]
async fn arc_state_repo_get_did_by_vdr_entry_none() {
    let mock = MockStateRepo::new();
    let hash = sha256([4u8; 32]);

    let arc: Arc<dyn IndexerStateRepo<Error = MockError> + Send + Sync> = Arc::new(mock);
    let result = arc.get_did_by_vdr_entry(&hash).await.unwrap();
    assert!(result.is_none());
}

// ---------------------------------------------------------------------------
// Arc<T> delegation: DltCursorRepo
// ---------------------------------------------------------------------------

#[tokio::test]
async fn arc_cursor_repo_set_and_get_cursor() {
    let mock = MockCursorRepo::new();
    let cursor = dummy_cursor();

    let arc: Arc<dyn DltCursorRepo<Error = MockError> + Send + Sync> = Arc::new(mock);

    arc.set_cursor(cursor.clone()).await.unwrap();
    let result = arc.get_cursor().await.unwrap();
    assert_eq!(result, Some(cursor));
}

#[tokio::test]
async fn arc_cursor_repo_get_cursor_none() {
    let mock = MockCursorRepo::new();

    let arc: Arc<dyn DltCursorRepo<Error = MockError> + Send + Sync> = Arc::new(mock);
    let result = arc.get_cursor().await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn arc_cursor_repo_set_cursor_overwrites() {
    let mock = MockCursorRepo::new();

    let arc: Arc<dyn DltCursorRepo<Error = MockError> + Send + Sync> = Arc::new(mock);

    let cursor1 = DltCursor {
        slot: 1,
        block_hash: vec![1],
        cbt: None,
        blockfrost_page: None,
    };
    let cursor2 = DltCursor {
        slot: 2,
        block_hash: vec![2],
        cbt: None,
        blockfrost_page: Some(5),
    };

    arc.set_cursor(cursor1).await.unwrap();
    arc.set_cursor(cursor2.clone()).await.unwrap();

    let result = arc.get_cursor().await.unwrap();
    assert_eq!(result, Some(cursor2));
}
