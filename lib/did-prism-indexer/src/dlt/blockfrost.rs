use std::str::FromStr;
use std::sync::Arc;

use blockfrost::BlockfrostAPI;
use blockfrost_openapi::models::TxContent;
use identus_apollo::hex::HexStr;
use identus_did_prism::dlt::{DltCursor, PublishedPrismObject};
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;

use crate::DltSource;
use crate::dlt::common::CursorPersistWorker;
use crate::dlt::error::DltError;
use crate::repo::DltCursorRepo;

mod models {
    use std::str::FromStr;

    use blockfrost_openapi::models::{TxContent, TxMetadataLabelJsonInner};
    use chrono::DateTime;
    use identus_did_prism::dlt::{BlockMetadata, BlockNo, PublishedPrismObject, SlotNo, TxId};

    use crate::dlt::common::metadata_map::MetadataMapJson;
    use crate::dlt::error::MetadataReadError;

    fn parse_block_metadata(
        block: &TxContent,
        block_hash: &Option<String>,
        tx_idx: &Option<usize>,
    ) -> Result<BlockMetadata, MetadataReadError> {
        let cbt =
            DateTime::from_timestamp(block.block_time as i64, 0).ok_or(MetadataReadError::InvalidBlockTimestamp {
                block_hash: block_hash.clone(),
                tx_idx: *tx_idx,
                timestamp: block.block_time as i64,
            })?;

        let tx_id = TxId::from_str(&block.hash).map_err(|e| MetadataReadError::InvalidMetadataType {
            source: e.into(),
            block_hash: block_hash.clone(),
            tx_idx: *tx_idx,
        })?;

        Ok(BlockMetadata {
            slot_number: SlotNo::from(block.slot as u64),
            block_number: BlockNo::from(block.block_height as u64),
            cbt,
            absn: block.index as u32,
            tx_id,
        })
    }

    pub fn parse_published_prism_object(
        block: TxContent,
        metadata: TxMetadataLabelJsonInner,
    ) -> Result<PublishedPrismObject, MetadataReadError> {
        let block_hash = Some(block.block.clone());
        let tx_idx = Some(block.index as usize);

        let block_metadata = parse_block_metadata(&block, &block_hash, &tx_idx)?;

        let json_metadata = metadata.json_metadata.ok_or(MetadataReadError::MissingBlockProperty {
            block_hash: block_hash.clone(),
            tx_idx,
            name: "json_metadata",
        })?;

        let metadata_map: MetadataMapJson =
            serde_json::from_value(json_metadata).map_err(|e| MetadataReadError::InvalidMetadataType {
                source: e.into(),
                block_hash: block_hash.clone(),
                tx_idx,
            })?;

        let prism_object = metadata_map.parse_prism_object(&block.block, tx_idx)?;

        Ok(PublishedPrismObject {
            block_metadata,
            prism_object,
        })
    }
}

pub struct BlockfrostSource<Store: DltCursorRepo + Send + 'static> {
    store: Store,
    api_key: String,
    base_url: String,
    sync_cursor_tx: watch::Sender<Option<DltCursor>>,
    from_slot: u64,
    confirmation_blocks: u16,
    poll_interval: u64,
}

impl<E, Store: DltCursorRepo<Error = E> + Send + 'static> BlockfrostSource<Store> {
    pub async fn since_persisted_cursor(
        store: Store,
        api_key: &str,
        base_url: &str,
        confirmation_blocks: u16,
        poll_interval: u64,
    ) -> Result<Self, E> {
        let cursor = store.get_cursor().await?;
        Ok(Self::new(
            store,
            api_key,
            base_url,
            cursor.map(|i| i.slot).unwrap_or_default(),
            confirmation_blocks,
            poll_interval,
        ))
    }

    pub fn new(
        store: Store,
        api_key: &str,
        base_url: &str,
        from_slot: u64,
        confirmation_blocks: u16,
        poll_interval: u64,
    ) -> Self {
        let (cursor_tx, _) = watch::channel::<Option<DltCursor>>(None);
        Self {
            store,
            api_key: api_key.to_string(),
            base_url: base_url.to_string(),
            sync_cursor_tx: cursor_tx,
            from_slot,
            confirmation_blocks,
            poll_interval,
        }
    }
}

impl<E, Store: DltCursorRepo<Error = E> + Send + 'static> DltSource for BlockfrostSource<Store> {
    fn sync_cursor(&self) -> watch::Receiver<Option<DltCursor>> {
        self.sync_cursor_tx.subscribe()
    }

    fn into_stream(self) -> Result<mpsc::Receiver<PublishedPrismObject>, String> {
        let (event_tx, rx) = mpsc::channel::<PublishedPrismObject>(1024);

        let cursor_persist_worker = CursorPersistWorker::new(self.store, self.sync_cursor_tx.subscribe());
        let stream_worker = BlockfrostStreamWorker {
            api_key: self.api_key,
            base_url: self.base_url,
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

struct BlockfrostStreamWorker {
    api_key: String,
    base_url: String,
    sync_cursor_tx: watch::Sender<Option<DltCursor>>,
    event_tx: mpsc::Sender<PublishedPrismObject>,
    from_slot: u64,
    confirmation_blocks: u16,
    poll_interval: u64,
}

impl BlockfrostStreamWorker {
    fn spawn(self) -> JoinHandle<Result<(), DltError>> {
        const RESTART_DELAY: tokio::time::Duration = tokio::time::Duration::from_secs(10);
        tokio::spawn(async move {
            let api_key = self.api_key;
            let base_url = self.base_url;
            let event_tx = self.event_tx;
            let sync_cursor_tx = self.sync_cursor_tx;

            loop {
                tracing::info!("Starting Blockfrost stream worker");

                let mut settings = blockfrost::BlockFrostSettings::default();
                settings.base_url = Some(base_url.clone());
                let api = Arc::new(BlockfrostAPI::new(&api_key, settings));

                if let Err(e) = Self::stream_loop(
                    api.clone(),
                    event_tx.clone(),
                    sync_cursor_tx.clone(),
                    self.from_slot,
                    self.confirmation_blocks,
                    self.poll_interval,
                )
                .await
                {
                    tracing::error!("Blockfrost stream loop terminated with error: {}", e);
                }

                tracing::error!(
                    "Blockfrost pipeline terminated, restarting in {} seconds",
                    RESTART_DELAY.as_secs()
                );

                tokio::time::sleep(RESTART_DELAY).await;
            }
        })
    }

    fn persist_cursor(tx: &TxContent, sync_cursor_tx: &watch::Sender<Option<DltCursor>>) {
        let hex_str = match HexStr::from_str(&tx.hash) {
            Ok(h) => h,
            Err(e) => {
                tracing::error!("Failed to parse block hash for cursor: {}, error: {}", tx.hash, e);
                return;
            }
        };
        let block_hash_bytes = hex_str.to_bytes();
        let Some(cbt) = chrono::DateTime::from_timestamp(tx.block_time as i64, 0) else {
            return;
        };
        let cursor = DltCursor {
            slot: tx.slot as u64,
            block_hash: block_hash_bytes,
            cbt: Some(cbt),
        };
        let _ = sync_cursor_tx.send(Some(cursor));
        tracing::debug!("Cursor persisted to slot={}, height={}", tx.slot, tx.block_height);
    }

    async fn stream_loop(
        api: Arc<BlockfrostAPI>,
        event_tx: mpsc::Sender<PublishedPrismObject>,
        sync_cursor_tx: watch::Sender<Option<DltCursor>>,
        from_slot: u64,
        confirmation_blocks: u16,
        poll_interval: u64,
    ) -> Result<(), DltError> {
        let initial_cursor = sync_cursor_tx.borrow().as_ref().map(|c| c.slot).unwrap_or(from_slot);
        let mut current_slot = initial_cursor;

        loop {
            unimplemented!();
        }
    }
}
