use std::time::Duration;

use identus_apollo::hex::HexStr;
use identus_did_prism::dlt::{DltCursor, PublishedPrismObject};
use identus_did_prism::location;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;

use crate::DltSource;
use crate::dlt::common::CursorPersistWorker;
use crate::dlt::dbsync::models::{BlockTimeProjection, MetadataProjection};
use crate::dlt::error::DltError;
use crate::repo::DltCursorRepo;

mod models {
    use chrono::{DateTime, Utc};
    use identus_apollo::hex::HexStr;
    use identus_did_prism::dlt::{BlockMetadata, BlockNo, PublishedPrismObject, SlotNo, TxId};
    use sqlx::FromRow;

    use crate::dlt::common::metadata_map::MetadataMapJson;
    use crate::dlt::error::MetadataReadError;

    #[derive(Debug, Clone, FromRow)]
    pub struct MetadataProjection {
        pub time: DateTime<Utc>,
        pub slot_no: i64,
        pub block_no: i32,
        pub block_hash: Vec<u8>,
        pub tx_idx: i32,
        pub tx_hash: Vec<u8>,
        pub metadata: serde_json::Value,
    }

    #[derive(Debug, Clone, FromRow)]
    pub struct BlockTimeProjection {
        pub time: DateTime<Utc>,
        pub slot_no: i64,
        pub block_hash: Vec<u8>,
    }

    impl From<MetadataProjection> for BlockTimeProjection {
        fn from(value: MetadataProjection) -> Self {
            Self {
                time: value.time,
                slot_no: value.slot_no,
                block_hash: value.block_hash,
            }
        }
    }

    fn parse_block_metadata(
        metadata: &MetadataProjection,
        block_hash: &Option<String>,
        tx_idx: &Option<usize>,
    ) -> Result<BlockMetadata, MetadataReadError> {
        let tx_id = TxId::from_bytes(&metadata.tx_hash).map_err(|e| MetadataReadError::InvalidMetadataType {
            source: e.to_string().into(),
            block_hash: block_hash.clone(),
            tx_idx: *tx_idx,
        })?;

        Ok(BlockMetadata {
            slot_number: SlotNo::from(metadata.slot_no as u64),
            block_number: BlockNo::from(metadata.block_no as u64),
            cbt: metadata.time,
            absn: metadata.tx_idx as u32,
            tx_id,
        })
    }

    pub fn parse_published_prism_object(
        metadata: MetadataProjection,
    ) -> Result<PublishedPrismObject, MetadataReadError> {
        let block_hash_str = HexStr::from(&metadata.block_hash).to_string();
        let block_hash = Some(block_hash_str.clone());
        let tx_idx = Some(metadata.tx_idx as usize);

        let block_metadata = parse_block_metadata(&metadata, &block_hash, &tx_idx)?;

        let metadata_json: MetadataMapJson =
            serde_json::from_value(metadata.metadata).map_err(|e| MetadataReadError::InvalidMetadataType {
                source: e.into(),
                block_hash,
                tx_idx,
            })?;

        let prism_object = metadata_json.parse_prism_object(&block_hash_str, tx_idx)?;

        Ok(PublishedPrismObject {
            block_metadata,
            prism_object,
        })
    }
}

pub struct DbSyncSource<Store: DltCursorRepo + Send + 'static> {
    store: Store,
    dbsync_url: String,
    sync_cursor_tx: watch::Sender<Option<DltCursor>>,
    from_slot: u64,
    confirmation_blocks: u16,
    poll_interval: Duration,
}

impl<E, Store: DltCursorRepo<Error = E> + Send + 'static> DbSyncSource<Store> {
    pub async fn since_persisted_cursor(
        store: Store,
        dbsync_url: &str,
        confirmation_blocks: u16,
        poll_interval: Duration,
    ) -> Result<Self, E> {
        let cursor = store.get_cursor().await?;
        Ok(Self::new(
            store,
            dbsync_url,
            cursor.map(|i| i.slot).unwrap_or_default(),
            confirmation_blocks,
            poll_interval,
        ))
    }

    pub fn new(
        store: Store,
        dbsync_url: &str,
        from_slot: u64,
        confirmation_blocks: u16,
        poll_interval: Duration,
    ) -> Self {
        let (cursor_tx, _) = watch::channel::<Option<DltCursor>>(None);
        Self {
            store,
            dbsync_url: dbsync_url.to_string(),
            sync_cursor_tx: cursor_tx,
            from_slot,
            confirmation_blocks,
            poll_interval,
        }
    }
}

impl<E, Store: DltCursorRepo<Error = E> + Send + 'static> DltSource for DbSyncSource<Store> {
    fn sync_cursor(&self) -> watch::Receiver<Option<DltCursor>> {
        self.sync_cursor_tx.subscribe()
    }

    fn into_stream(self) -> Result<mpsc::Receiver<PublishedPrismObject>, String> {
        let (event_tx, rx) = mpsc::channel::<PublishedPrismObject>(1024);

        let cursor_persist_worker = CursorPersistWorker::new(self.store, self.sync_cursor_tx.subscribe());
        let stream_worker = DbSyncStreamWorker {
            dbsync_url: self.dbsync_url,
            sync_cursor_tx: self.sync_cursor_tx,
            event_tx,
            from_slot: self.from_slot,
            confirmation_blocks: self.confirmation_blocks,
            poll_interval: self.poll_interval,
        };

        cursor_persist_worker.spawn();
        stream_worker.spawn();

        Ok(rx)
    }
}

struct DbSyncStreamWorker {
    dbsync_url: String,
    sync_cursor_tx: watch::Sender<Option<DltCursor>>,
    event_tx: mpsc::Sender<PublishedPrismObject>,
    from_slot: u64,
    confirmation_blocks: u16,
    poll_interval: Duration,
}

impl DbSyncStreamWorker {
    fn spawn(self) -> JoinHandle<Result<(), DltError>> {
        const RESTART_DELAY: tokio::time::Duration = tokio::time::Duration::from_secs(10);
        tokio::spawn(async move {
            let db_url = self.dbsync_url;
            let event_tx = self.event_tx;
            let sync_cursor_tx = self.sync_cursor_tx;
            loop {
                tracing::info!("starting dbsync stream worker");
                let pool = PgPoolOptions::new().max_connections(1).connect(&db_url).await;
                match pool {
                    Ok(pool) => {
                        if let Err(e) = Self::stream_loop(
                            pool,
                            event_tx.clone(),
                            sync_cursor_tx.clone(),
                            self.from_slot,
                            self.confirmation_blocks,
                            self.poll_interval,
                        )
                        .await
                        {
                            tracing::error!("stream loop terminated with error");
                            let report = std::error::Report::new(&e).pretty(true);
                            tracing::error!("{}", report);
                        }
                    }
                    Err(e) => {
                        tracing::error!("unable to connect to dbsync database");
                        let report = std::error::Report::new(&e).pretty(true);
                        tracing::error!("{}", report);
                    }
                }

                tracing::error!("dbsync pipeline terminated, restarting in {}s", RESTART_DELAY.as_secs());

                tokio::time::sleep(RESTART_DELAY).await;
            }
        })
    }

    async fn stream_loop(
        pool: PgPool,
        event_tx: mpsc::Sender<PublishedPrismObject>,
        sync_cursor_tx: watch::Sender<Option<DltCursor>>,
        from_slot: u64,
        confirmation_blocks: u16,
        poll_interval: Duration,
    ) -> Result<(), DltError> {
        let mut sync_cursor = sync_cursor_tx
            .subscribe()
            .borrow()
            .as_ref()
            .map(|i| i.slot)
            .unwrap_or(from_slot) as i64;
        loop {
            let metadata_rows = Self::fetch_metadata(&pool, sync_cursor, confirmation_blocks).await?;
            if let Some(latest_slot) = metadata_rows.iter().map(|i| i.slot_no).max() {
                sync_cursor = latest_slot;
            }
            let row_count = metadata_rows.len();
            for row in metadata_rows {
                let process_result = Self::process_prism_object(row.clone(), &event_tx).await;
                // Advance the cursor before propagating any error so a poison row
                // (e.g. unparseable metadata) doesn't get re-fetched and re-failed
                // forever. The watch channel carries the latest value to both
                // CursorPersistWorker (durable resume across restarts) and the
                // next stream_loop iteration (in-process restart).
                Self::emit_cursor_progress(row.into(), &sync_cursor_tx);
                if let Err(e) = process_result {
                    tracing::error!("error handling event from dbsync source");
                    let report = std::error::Report::new(&e).pretty(true);
                    tracing::error!("{}", report);
                    return Err(e);
                }
            }

            if row_count == 0 {
                // get latest block if we don't find any prism block just to know where we are
                if let Ok(block_time) = Self::fetch_latest_confirmed_block(&pool, confirmation_blocks)
                    .await
                    .inspect_err(|e| tracing::error!("unable to get the latest block: {:?}", e))
                {
                    Self::emit_cursor_progress(block_time, &sync_cursor_tx);
                }

                // sleep if we don't find a new block to avoid spamming db sync
                tokio::time::sleep(poll_interval).await;
            }
        }
    }

    async fn process_prism_object(
        row: MetadataProjection,
        event_tx: &mpsc::Sender<PublishedPrismObject>,
    ) -> Result<(), DltError> {
        tracing::info!(
            "detected a new prism_block on slot ({}, {})",
            row.slot_no,
            HexStr::from(&row.block_hash).to_string()
        );

        let parsed_prism_object = models::parse_published_prism_object(row);
        match parsed_prism_object {
            Ok(prism_object) => event_tx.send(prism_object).await.map_err(|e| DltError::EventHandling {
                source: e.to_string().into(),
                location: location!(),
            })?,
            Err(e) => {
                tracing::warn!("unable to parse dbsync row into PrismObject: {:?}", e);
            }
        }
        Ok(())
    }

    fn emit_cursor_progress(block_time: BlockTimeProjection, sync_cursor_tx: &watch::Sender<Option<DltCursor>>) {
        let slot = block_time.slot_no as u64;
        let block_hash = HexStr::from(block_time.block_hash);
        let timestamp = block_time.time;
        let cursor = DltCursor {
            slot,
            block_hash: block_hash.to_bytes(),
            cbt: Some(timestamp),
            blockfrost_page: None,
        };
        let _ = sync_cursor_tx.send(Some(cursor));
    }

    async fn fetch_latest_confirmed_block(
        pool: &PgPool,
        confirmation_blocks: u16,
    ) -> Result<BlockTimeProjection, DltError> {
        let row = sqlx::query_as(
            r#"
SELECT
    b."time" AT TIME ZONE 'UTC' AS "time",
    b.slot_no,
    b.hash AS block_hash
FROM block AS b
WHERE b.block_no <= (SELECT max(block_no) - $1 FROM block)
ORDER BY b.block_no DESC
LIMIT 1
            "#,
        )
        .bind(i64::from(confirmation_blocks))
        .fetch_one(pool)
        .await
        .inspect_err(|e| tracing::error!("failed to get data from dbsync: {:?}", e))
        .map_err(|e| DltError::Connection {
            source: e.into(),
            location: location!(),
        })?;

        Ok(row)
    }

    async fn fetch_metadata(
        pool: &PgPool,
        from_slot: i64,
        confirmation_blocks: u16,
    ) -> Result<Vec<MetadataProjection>, DltError> {
        let rows = sqlx::query_as(
            r#"
WITH eligible AS (
    SELECT
        b."time" AT TIME ZONE 'UTC' AS "time",
        b.slot_no,
        b.block_no,
        b.hash AS block_hash,
        tx.block_index AS tx_idx,
        tx.hash AS tx_hash,
        tx_meta.json AS metadata,
        ROW_NUMBER() OVER (ORDER BY b.slot_no, tx.block_index) as rn
    FROM tx_metadata AS tx_meta
    LEFT JOIN tx ON tx_meta.tx_id = tx.id
    LEFT JOIN block AS b ON block_id = b.id
    WHERE tx_meta.key = 21325
      AND b.slot_no > $1
      AND b.block_no <= (SELECT max(block_no) - $2 FROM block)
),
boundary AS (
    SELECT MAX(slot_no) as cutoff_slot
    FROM eligible
    WHERE rn <= 1000
)
SELECT
    "time",
    slot_no,
    block_no,
    block_hash,
    tx_idx,
    tx_hash,
    metadata
FROM eligible
WHERE slot_no <= (SELECT cutoff_slot FROM boundary)
ORDER BY slot_no, tx_idx
            "#,
        )
        .bind(from_slot)
        .bind(i64::from(confirmation_blocks))
        .fetch_all(pool)
        .await
        .inspect_err(|e| tracing::error!("failed to get data from dbsync: {}", e))
        .map_err(|e| DltError::Connection {
            source: e.into(),
            location: location!(),
        })?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use chrono::{DateTime, TimeZone, Utc};
    use identus_did_prism::dlt::{DltCursor, TxId};
    use identus_did_prism::proto::MessageExt;
    use identus_did_prism::proto::prism::{PrismBlock, PrismObject};
    use tokio::sync::{mpsc, watch};

    use super::models::{BlockTimeProjection, MetadataProjection};
    use super::{DbSyncSource, DbSyncStreamWorker};
    use crate::DltSource;
    use crate::dlt::dbsync::models;
    use crate::repo::DltCursorRepo;

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    /// A valid 32-byte transaction hash.
    fn valid_tx_hash() -> Vec<u8> {
        (0..32).collect()
    }

    /// A valid 32-byte block hash.
    fn valid_block_hash() -> Vec<u8> {
        (32..64).collect()
    }

    /// Encode a PrismObject into metadata byte groups ("0x" + hex).
    fn encode_object_as_byte_groups(obj: &PrismObject) -> Vec<String> {
        let bytes = obj.encode_to_vec();
        bytes
            .chunks(64)
            .map(|chunk| {
                let hex = identus_apollo::hex::HexStr::from(chunk).to_string();
                format!("0x{hex}")
            })
            .collect()
    }

    /// Build a minimal PrismObject with one empty operation.
    fn minimal_prism_object() -> PrismObject {
        PrismObject {
            block_content: Some(PrismBlock {
                operations: vec![],
                special_fields: Default::default(),
            })
            .into(),
            special_fields: Default::default(),
        }
    }

    /// Build a valid MetadataProjection with the given metadata JSON.
    fn valid_projection(metadata: serde_json::Value) -> MetadataProjection {
        MetadataProjection {
            time: DateTime::UNIX_EPOCH,
            slot_no: 1000,
            block_no: 500,
            block_hash: valid_block_hash(),
            tx_idx: 0,
            tx_hash: valid_tx_hash(),
            metadata,
        }
    }

    /// Build valid metadata JSON from a PrismObject.
    fn valid_metadata_json(obj: &PrismObject) -> serde_json::Value {
        let byte_groups = encode_object_as_byte_groups(obj);
        serde_json::json!({
            "c": byte_groups,
            "v": 1
        })
    }

    // ------------------------------------------------------------------
    // Mock DltCursorRepo
    // ------------------------------------------------------------------

    #[derive(Debug, derive_more::Display, derive_more::Error)]
    #[display("test error")]
    struct TestError;

    #[derive(Debug)]
    struct MockRepo {
        cursor: Arc<Mutex<Option<DltCursor>>>,
    }

    impl MockRepo {
        fn new(cursor: Option<DltCursor>) -> Self {
            Self {
                cursor: Arc::new(Mutex::new(cursor)),
            }
        }
    }

    #[async_trait::async_trait]
    impl DltCursorRepo for MockRepo {
        type Error = TestError;

        async fn set_cursor(&self, cursor: DltCursor) -> Result<(), Self::Error> {
            *self.cursor.lock().unwrap() = Some(cursor);
            Ok(())
        }

        async fn get_cursor(&self) -> Result<Option<DltCursor>, Self::Error> {
            Ok(self.cursor.lock().unwrap().clone())
        }
    }

    // ------------------------------------------------------------------
    // models::parse_published_prism_object tests
    // ------------------------------------------------------------------

    #[test]
    fn parse_published_prism_object_valid_minimal() {
        let obj = minimal_prism_object();
        let projection = valid_projection(valid_metadata_json(&obj));

        let result = models::parse_published_prism_object(projection).unwrap();

        assert_eq!(result.block_metadata.slot_number.inner(), 1000);
        assert_eq!(result.block_metadata.block_number.inner(), 500);
        assert_eq!(result.block_metadata.absn, 0);
        assert_eq!(result.prism_object, obj);
    }

    #[test]
    fn parse_published_prism_object_valid_preserves_tx_id() {
        let obj = minimal_prism_object();
        let projection = valid_projection(valid_metadata_json(&obj));
        let expected_tx_id = TxId::from_bytes(&valid_tx_hash()).unwrap();

        let result = models::parse_published_prism_object(projection).unwrap();

        assert_eq!(result.block_metadata.tx_id, expected_tx_id);
    }

    #[test]
    fn parse_published_prism_object_valid_preserves_timestamp() {
        let obj = minimal_prism_object();
        let mut projection = valid_projection(valid_metadata_json(&obj));
        let ts = Utc.with_ymd_and_hms(2024, 3, 15, 10, 30, 0).unwrap();
        projection.time = ts;

        let result = models::parse_published_prism_object(projection).unwrap();

        assert_eq!(result.block_metadata.cbt, ts);
    }

    #[test]
    fn parse_published_prism_object_multiple_byte_groups() {
        // Create an object large enough to span multiple 64-byte chunks.
        let large_sig: Vec<u8> = (0..200u8).collect();
        let obj = PrismObject {
            block_content: Some(PrismBlock {
                operations: (0..5)
                    .map(|_| identus_did_prism::proto::prism::SignedPrismOperation {
                        signed_with: "master-0".to_string(),
                        signature: large_sig.clone(),
                        operation: protobuf::MessageField(None),
                        special_fields: Default::default(),
                    })
                    .collect(),
                special_fields: Default::default(),
            })
            .into(),
            special_fields: Default::default(),
        };

        let projection = valid_projection(valid_metadata_json(&obj));

        let result = models::parse_published_prism_object(projection).unwrap();
        assert_eq!(result.prism_object, obj);
    }

    #[test]
    fn parse_published_prism_object_invalid_tx_hash_wrong_length() {
        let obj = minimal_prism_object();
        let mut projection = valid_projection(valid_metadata_json(&obj));
        // Tx hash must be 32 bytes — give 16 bytes instead.
        projection.tx_hash = vec![0u8; 16];

        let err = models::parse_published_prism_object(projection).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("metadata is not a valid"),
            "expected invalid metadata error, got: {msg}"
        );
    }

    #[test]
    fn parse_published_prism_object_invalid_tx_hash_empty() {
        let obj = minimal_prism_object();
        let mut projection = valid_projection(valid_metadata_json(&obj));
        projection.tx_hash = vec![];

        let err = models::parse_published_prism_object(projection).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("metadata is not a valid"),
            "expected invalid metadata error, got: {msg}"
        );
    }

    #[test]
    fn parse_published_prism_object_invalid_metadata_not_json_object() {
        let mut projection = valid_projection(serde_json::json!("not an object"));
        projection.metadata = serde_json::json!("not a struct");

        let err = models::parse_published_prism_object(projection).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("metadata is not a valid"),
            "expected invalid metadata error, got: {msg}"
        );
    }

    #[test]
    fn parse_published_prism_object_invalid_metadata_missing_fields() {
        let mut projection = valid_projection(serde_json::json!({
            "v": 1
            // missing "c" field
        }));
        projection.metadata = serde_json::json!({"v": 1});

        let err = models::parse_published_prism_object(projection).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("metadata is not a valid"),
            "expected invalid metadata error, got: {msg}"
        );
    }

    #[test]
    fn parse_published_prism_object_invalid_protobuf_bytes() {
        // Valid metadata structure but the hex bytes are not valid protobuf.
        let projection = valid_projection(serde_json::json!({
            "c": ["0xdeadbeef"],
            "v": 1
        }));

        let err = models::parse_published_prism_object(projection).unwrap_err();
        let msg = err.to_string();
        // The error should mention protobuf decode failure
        assert!(
            msg.contains("protobuf") || msg.contains("decode"),
            "expected protobuf decode error, got: {msg}"
        );
    }

    #[test]
    fn parse_published_prism_object_empty_byte_groups_produces_default() {
        let projection = valid_projection(serde_json::json!({
            "c": [],
            "v": 1
        }));

        let result = models::parse_published_prism_object(projection);
        assert!(result.is_ok(), "empty byte groups should decode as default PrismObject");
        assert_eq!(result.unwrap().prism_object, PrismObject::default());
    }

    #[test]
    fn parse_published_prism_object_missing_0x_prefix() {
        let projection = valid_projection(serde_json::json!({
            "c": ["aabbccdd"],
            "v": 1
        }));

        let err = models::parse_published_prism_object(projection).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("metadata is not a valid"),
            "expected invalid metadata error, got: {msg}"
        );
    }

    // ------------------------------------------------------------------
    // BlockTimeProjection::from tests
    // ------------------------------------------------------------------

    #[test]
    fn block_time_projection_from_metadata_projection() {
        let ts = Utc.with_ymd_and_hms(2024, 6, 1, 12, 0, 0).unwrap();
        let projection = MetadataProjection {
            time: ts,
            slot_no: 42,
            block_no: 21,
            block_hash: vec![1u8; 32],
            tx_idx: 3,
            tx_hash: vec![2u8; 32],
            metadata: serde_json::json!({}),
        };

        let block_time: BlockTimeProjection = projection.into();

        assert_eq!(block_time.time, ts);
        assert_eq!(block_time.slot_no, 42);
        assert_eq!(block_time.block_hash, vec![1u8; 32]);
    }

    // ------------------------------------------------------------------
    // DbSyncSource construction tests
    // ------------------------------------------------------------------

    #[test]
    fn dbsync_source_new_creates_instance() {
        let repo = MockRepo::new(None);
        let source = DbSyncSource::new(
            repo,
            "postgres://localhost:5432/dbsync",
            100,
            2160,
            Duration::from_secs(10),
        );

        assert_eq!(source.from_slot, 100);
        assert_eq!(source.confirmation_blocks, 2160);
    }

    #[tokio::test]
    async fn dbsync_source_sync_cursor_returns_none_initially() {
        let repo = MockRepo::new(None);
        let source = DbSyncSource::new(repo, "postgres://localhost:5432/dbsync", 0, 100, Duration::from_secs(5));

        let mut rx = source.sync_cursor();
        let cursor = rx.borrow_and_update().clone();
        assert!(cursor.is_none());
    }

    #[tokio::test]
    async fn dbsync_source_since_persisted_cursor_with_none() {
        let repo = MockRepo::new(None);
        let source = DbSyncSource::since_persisted_cursor(
            repo,
            "postgres://localhost:5432/dbsync",
            100,
            Duration::from_secs(10),
        )
        .await
        .unwrap();

        // from_slot should default to 0 when no persisted cursor
        assert_eq!(source.from_slot, 0);
    }

    #[tokio::test]
    async fn dbsync_source_since_persisted_cursor_with_existing_cursor() {
        let cursor = DltCursor {
            slot: 42,
            block_hash: vec![0u8; 32],
            cbt: None,
            blockfrost_page: None,
        };
        let repo = MockRepo::new(Some(cursor));
        let source = DbSyncSource::since_persisted_cursor(
            repo,
            "postgres://localhost:5432/dbsync",
            100,
            Duration::from_secs(10),
        )
        .await
        .unwrap();

        assert_eq!(source.from_slot, 42);
    }

    #[tokio::test]
    async fn dbsync_source_into_stream_creates_channel() {
        let repo = MockRepo::new(None);
        let source = DbSyncSource::new(repo, "postgres://localhost:5432/dbsync", 0, 100, Duration::from_secs(5));

        let rx = source.into_stream().unwrap();
        // Channel should be open (not closed)
        assert!(!rx.is_closed());
    }

    // ------------------------------------------------------------------
    // DbSyncStreamWorker::process_prism_object tests
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn process_prism_object_valid_sends_to_channel() {
        let (tx, mut rx) = mpsc::channel::<identus_did_prism::dlt::PublishedPrismObject>(1024);
        let obj = minimal_prism_object();
        let projection = valid_projection(valid_metadata_json(&obj));

        DbSyncStreamWorker::process_prism_object(projection, &tx).await.unwrap();

        let received = rx.try_recv().unwrap();
        assert_eq!(received.prism_object, obj);
    }

    #[tokio::test]
    async fn process_prism_object_invalid_metadata_returns_ok_no_send() {
        let (tx, rx) = mpsc::channel::<identus_did_prism::dlt::PublishedPrismObject>(1024);
        let projection = valid_projection(serde_json::json!({
            "c": ["not_valid_hex"],
            "v": 1
        }));

        // Invalid metadata should return Ok (logs a warning but doesn't error)
        DbSyncStreamWorker::process_prism_object(projection, &tx).await.unwrap();

        // Nothing should have been sent
        assert!(rx.is_empty());
    }

    #[tokio::test]
    async fn process_prism_object_invalid_tx_hash_returns_ok_no_send() {
        let (tx, rx) = mpsc::channel::<identus_did_prism::dlt::PublishedPrismObject>(1024);
        let obj = minimal_prism_object();
        let mut projection = valid_projection(valid_metadata_json(&obj));
        projection.tx_hash = vec![0u8; 10]; // wrong length

        DbSyncStreamWorker::process_prism_object(projection, &tx).await.unwrap();

        assert!(rx.is_empty());
    }

    #[tokio::test]
    async fn process_prism_object_closed_channel_returns_error() {
        let (tx, rx) = mpsc::channel::<identus_did_prism::dlt::PublishedPrismObject>(1024);
        let obj = minimal_prism_object();
        let projection = valid_projection(valid_metadata_json(&obj));

        // Close the receiving end so the send fails
        drop(rx);

        let result = DbSyncStreamWorker::process_prism_object(projection, &tx).await;
        assert!(result.is_err(), "expected error when channel is closed");
    }

    // ------------------------------------------------------------------
    // DbSyncStreamWorker::emit_cursor_progress tests
    // ------------------------------------------------------------------

    #[test]
    fn emit_cursor_progress_sends_cursor_via_watch() {
        let (tx, rx) = watch::channel(None);

        let ts = Utc.with_ymd_and_hms(2024, 1, 15, 8, 30, 0).unwrap();
        let block_time = BlockTimeProjection {
            time: ts,
            slot_no: 555,
            block_hash: vec![7u8; 32],
        };

        DbSyncStreamWorker::emit_cursor_progress(block_time, &tx);

        let cursor = rx.borrow().clone().unwrap();
        assert_eq!(cursor.slot, 555);
        assert_eq!(cursor.block_hash, vec![7u8; 32]);
        assert_eq!(cursor.cbt, Some(ts));
        assert_eq!(cursor.blockfrost_page, None);
    }

    #[test]
    fn emit_cursor_progress_overwrites_previous_cursor() {
        let (tx, rx) = watch::channel(None);

        let block_time1 = BlockTimeProjection {
            time: DateTime::UNIX_EPOCH,
            slot_no: 100,
            block_hash: vec![1u8; 32],
        };
        DbSyncStreamWorker::emit_cursor_progress(block_time1, &tx);
        assert_eq!(rx.borrow().as_ref().unwrap().slot, 100);

        let block_time2 = BlockTimeProjection {
            time: DateTime::UNIX_EPOCH,
            slot_no: 200,
            block_hash: vec![2u8; 32],
        };
        DbSyncStreamWorker::emit_cursor_progress(block_time2, &tx);
        assert_eq!(rx.borrow().as_ref().unwrap().slot, 200);
    }

    // ------------------------------------------------------------------
    // DbSyncStreamWorker::spawn tests (connection error path)
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn spawn_with_invalid_url_handles_connection_error() {
        tokio::time::pause();

        let (event_tx, _event_rx) = mpsc::channel::<identus_did_prism::dlt::PublishedPrismObject>(1024);
        let (sync_cursor_tx, sync_cursor_rx) = watch::channel(None);

        let worker = DbSyncStreamWorker {
            dbsync_url: "postgres://invalid:invalid@localhost:99999/nonexistent".to_string(),
            sync_cursor_tx,
            event_tx,
            from_slot: 0,
            confirmation_blocks: 100,
            poll_interval: Duration::from_millis(10),
        };

        let handle = worker.spawn();

        // Yield to let the spawned task start and attempt connection (fails immediately).
        tokio::task::yield_now().await;

        // The sync_cursor_rx should still be None since no data was processed.
        assert!(sync_cursor_rx.borrow().is_none());

        // Advance time past the 10s restart delay to trigger a retry.
        tokio::time::advance(Duration::from_secs(11)).await;
        tokio::task::yield_now().await;

        // The worker should still be running (retrying).
        assert!(!handle.is_finished());

        // Cursor should still be None — no data was ever processed.
        assert!(sync_cursor_rx.borrow().is_none());

        handle.abort();
    }

    // ------------------------------------------------------------------
    // into_stream integration test with connection failure
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn into_stream_worker_handles_connection_failure_gracefully() {
        tokio::time::pause();

        let repo = MockRepo::new(None);
        let source = DbSyncSource::new(
            repo,
            "postgres://invalid:invalid@localhost:99999/nonexistent",
            0,
            100,
            Duration::from_millis(10),
        );

        let rx = source.into_stream().unwrap();
        // Channel should be open even though connection will fail.
        assert!(!rx.is_closed());

        // Advance time to let the worker attempt connection and retry.
        tokio::time::advance(Duration::from_secs(11)).await;
        tokio::task::yield_now().await;

        // Channel should still be open — the worker keeps retrying.
        assert!(!rx.is_closed());
    }

    // ------------------------------------------------------------------
    // since_persisted_cursor error propagation
    // ------------------------------------------------------------------

    /// A repo that always fails on get_cursor.
    #[derive(Debug)]
    struct FailingRepo;

    #[async_trait::async_trait]
    impl DltCursorRepo for FailingRepo {
        type Error = TestError;

        async fn set_cursor(&self, _cursor: DltCursor) -> Result<(), Self::Error> {
            Ok(())
        }

        async fn get_cursor(&self) -> Result<Option<DltCursor>, Self::Error> {
            Err(TestError)
        }
    }

    #[tokio::test]
    async fn dbsync_source_since_persisted_cursor_store_error() {
        let result = DbSyncSource::since_persisted_cursor(
            FailingRepo,
            "postgres://localhost:5432/dbsync",
            100,
            Duration::from_secs(10),
        )
        .await;

        assert!(result.is_err(), "expected error when store fails");
    }

    // ------------------------------------------------------------------
    // parse_published_prism_object edge cases
    // ------------------------------------------------------------------

    #[test]
    fn parse_published_prism_object_zero_slot_and_block() {
        let obj = minimal_prism_object();
        let mut projection = valid_projection(valid_metadata_json(&obj));
        projection.slot_no = 0;
        projection.block_no = 0;

        let result = models::parse_published_prism_object(projection).unwrap();
        assert_eq!(result.block_metadata.slot_number.inner(), 0);
        assert_eq!(result.block_metadata.block_number.inner(), 0);
    }

    #[test]
    fn parse_published_prism_object_preserves_absn_from_tx_idx() {
        let obj = minimal_prism_object();
        let mut projection = valid_projection(valid_metadata_json(&obj));
        projection.tx_idx = 7;

        let result = models::parse_published_prism_object(projection).unwrap();
        assert_eq!(result.block_metadata.absn, 7);
    }

    #[test]
    fn parse_published_prism_object_metadata_null_value() {
        let mut projection = valid_projection(serde_json::json!("not an object"));
        projection.metadata = serde_json::Value::Null;

        let err = models::parse_published_prism_object(projection).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("metadata is not a valid"),
            "expected invalid metadata error for null, got: {msg}"
        );
    }

    #[test]
    fn parse_published_prism_object_metadata_wrong_type_for_c() {
        let projection = valid_projection(serde_json::json!({
            "c": 42,
            "v": 1
        }));

        let err = models::parse_published_prism_object(projection).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("metadata is not a valid"),
            "expected invalid metadata error when c is not array, got: {msg}"
        );
    }

    #[test]
    fn parse_published_prism_object_metadata_c_with_empty_strings() {
        let projection = valid_projection(serde_json::json!({
            "c": [""],
            "v": 1
        }));

        let err = models::parse_published_prism_object(projection).unwrap_err();
        let msg = err.to_string();
        // Empty string fails the "0x" prefix check
        assert!(
            msg.contains("metadata is not a valid") || msg.contains("hex"),
            "expected error for empty byte group, got: {msg}"
        );
    }

    #[test]
    fn parse_published_prism_object_tx_hash_exactly_32_bytes() {
        let obj = minimal_prism_object();
        let mut projection = valid_projection(valid_metadata_json(&obj));
        projection.tx_hash = vec![0xABu8; 32];

        let result = models::parse_published_prism_object(projection).unwrap();
        let expected_tx_id = TxId::from_bytes(&[0xABu8; 32]).unwrap();
        assert_eq!(result.block_metadata.tx_id, expected_tx_id);
    }

    #[test]
    fn parse_published_prism_object_tx_hash_33_bytes_fails() {
        let obj = minimal_prism_object();
        let mut projection = valid_projection(valid_metadata_json(&obj));
        projection.tx_hash = vec![0u8; 33];

        let err = models::parse_published_prism_object(projection).unwrap_err();
        assert!(
            err.to_string().contains("metadata is not a valid"),
            "expected error for 33-byte tx_hash"
        );
    }

    // ------------------------------------------------------------------
    // process_prism_object error quality
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn process_prism_object_error_includes_event_handling_context() {
        let (tx, rx) = mpsc::channel::<identus_did_prism::dlt::PublishedPrismObject>(1024);
        let obj = minimal_prism_object();
        let projection = valid_projection(valid_metadata_json(&obj));

        // Close the receiver to cause a send failure
        drop(rx);

        let err = DbSyncStreamWorker::process_prism_object(projection, &tx)
            .await
            .unwrap_err();

        let err_msg = err.to_string();
        assert!(
            err_msg.contains("event") || err_msg.contains("handling"),
            "error should mention event handling, got: {err_msg}"
        );
    }
}
