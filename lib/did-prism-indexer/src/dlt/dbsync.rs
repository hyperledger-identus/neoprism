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
                    .inspect_err(|e| tracing::error!("unable to get the latest block: {}", e))
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
                tracing::warn!("unable to parse dbsync row into PrismObject: {}", e);
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
        .inspect_err(|e| tracing::error!("failed to get data from dbsync: {}", e))
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
SELECT
    b."time" AT TIME ZONE 'UTC' AS "time",
    b.slot_no,
    b.block_no,
    b.hash AS block_hash,
    tx.block_index AS tx_idx,
    tx.hash AS tx_hash,
    tx_meta.json AS metadata
FROM tx_metadata AS tx_meta
LEFT JOIN tx ON tx_meta.tx_id = tx.id
LEFT JOIN block AS b ON block_id = b.id
WHERE tx_meta.key = 21325 AND b.slot_no > $1 AND b.block_no <= (SELECT max(block_no) - $2 FROM block)
ORDER BY b.block_no, tx.block_index
LIMIT 1000
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
