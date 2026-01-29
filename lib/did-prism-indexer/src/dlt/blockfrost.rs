use std::str::FromStr;
use std::sync::Arc;

use blockfrost::BlockfrostAPI;
use blockfrost_openapi::models::{TxContent, TxMetadataLabelJsonInner};
use identus_apollo::hex::HexStr;
use identus_did_prism::dlt::{DltCursor, PublishedPrismObject};
use identus_did_prism::location;
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

    pub fn parse_blockfrost_timestamp(
        block_time: i64,
        block_hash: &Option<String>,
        tx_idx: &Option<usize>,
    ) -> Result<DateTime<chrono::Utc>, MetadataReadError> {
        DateTime::from_timestamp(block_time, 0).ok_or(MetadataReadError::InvalidBlockTimestamp {
            block_hash: block_hash.clone(),
            tx_idx: *tx_idx,
            timestamp: block_time,
        })
    }

    fn parse_block_metadata(
        block: &TxContent,
        block_hash: &Option<String>,
        tx_idx: &Option<usize>,
    ) -> Result<BlockMetadata, MetadataReadError> {
        let cbt = parse_blockfrost_timestamp(block.block_time as i64, block_hash, tx_idx)?;

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
        block: &TxContent,
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
    from_page: u32,
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
            cursor.and_then(|i| i.blockfrost_page).unwrap_or(1),
            confirmation_blocks,
            poll_interval,
        ))
    }

    pub fn new(
        store: Store,
        api_key: &str,
        base_url: &str,
        from_page: u32,
        confirmation_blocks: u16,
        poll_interval: u64,
    ) -> Self {
        let (cursor_tx, _) = watch::channel::<Option<DltCursor>>(None);
        Self {
            store,
            api_key: api_key.to_string(),
            base_url: base_url.to_string(),
            sync_cursor_tx: cursor_tx,
            from_page,
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
            from_page: self.from_page,
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
    from_page: u32,
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
                tracing::info!("starting blockfrost stream worker");

                let mut settings = blockfrost::BlockFrostSettings::default();
                settings.base_url = Some(base_url.clone());
                let api = Arc::new(BlockfrostAPI::new(&api_key, settings));

                if let Err(e) = Self::stream_loop(
                    api.clone(),
                    event_tx.clone(),
                    sync_cursor_tx.clone(),
                    self.from_page,
                    self.confirmation_blocks,
                    self.poll_interval,
                )
                .await
                {
                    tracing::error!("stream loop terminated with error");
                    let report = std::error::Report::new(&e).pretty(true);
                    tracing::error!("{}", report);
                }

                tracing::error!(
                    "blockfrost pipeline terminated, restarting in {}s",
                    RESTART_DELAY.as_secs()
                );

                tokio::time::sleep(RESTART_DELAY).await;
            }
        })
    }

    fn emit_cursor_progress(tx: &TxContent, page: u32, sync_cursor_tx: &watch::Sender<Option<DltCursor>>) {
        let hex_str = match HexStr::from_str(&tx.hash) {
            Ok(h) => h,
            Err(e) => {
                tracing::error!("failed to parse block hash for cursor: {}, error: {}", tx.hash, e);
                return;
            }
        };
        let block_hash_bytes = hex_str.to_bytes();
        let cbt = match models::parse_blockfrost_timestamp(
            tx.block_time as i64,
            &Some(tx.block.clone()),
            &Some(tx.index as usize),
        ) {
            Ok(cbt) => cbt,
            Err(e) => {
                tracing::error!("failed to parse block timestamp for cursor: {}", e);
                return;
            }
        };
        let cursor = DltCursor {
            slot: tx.slot as u64,
            block_hash: block_hash_bytes,
            cbt: Some(cbt),
            blockfrost_page: Some(page),
        };
        let _ = sync_cursor_tx.send(Some(cursor));
        tracing::debug!("cursor progress emitted to slot={}", tx.slot);
    }

    async fn stream_loop(
        api: Arc<BlockfrostAPI>,
        event_tx: mpsc::Sender<PublishedPrismObject>,
        sync_cursor_tx: watch::Sender<Option<DltCursor>>,
        from_page: u32,
        confirmation_blocks: u16,
        poll_interval: u64,
    ) -> Result<(), DltError> {
        let mut current_page = sync_cursor_tx
            .borrow()
            .as_ref()
            .and_then(|c| c.blockfrost_page)
            .unwrap_or(from_page);

        // TODO: handle the logic of getting latest confirmed block
        loop {
            let metadata = Self::fetch_metadata_page(&api, current_page).await?;

            match metadata {
                Some(metadata) => {
                    let tx_content = Self::fetch_tx_by_id(&api, &metadata.tx_hash).await?;
                    let handle_result = Self::handle_metadata(&tx_content, metadata, &event_tx).await;
                    Self::emit_cursor_progress(&tx_content, current_page, &sync_cursor_tx);
                    if let Err(e) = handle_result {
                        tracing::error!("error handling event from blockfrost source");
                        let report = std::error::Report::new(&e).pretty(true);
                        tracing::error!("{}", report);
                        return Err(e);
                    }
                    current_page += 1;
                }
                None => {
                    // TODO: get latest block just to broadcast the progress

                    // sleep if we don't find a new block to avoid spamming db sync
                    tokio::time::sleep(tokio::time::Duration::from_secs(poll_interval)).await;
                }
            }
        }
    }

    async fn handle_metadata(
        tx_content: &TxContent,
        metadata: TxMetadataLabelJsonInner,
        event_tx: &mpsc::Sender<PublishedPrismObject>,
    ) -> Result<(), DltError> {
        tracing::info!(
            "detected a new prism_block on slot ({}, {})",
            tx_content.slot,
            tx_content.block
        );

        let parsed_prism_object = models::parse_published_prism_object(tx_content, metadata);
        match parsed_prism_object {
            Ok(prism_object) => event_tx.send(prism_object).await.map_err(|e| DltError::EventHandling {
                source: e.to_string().into(),
                location: location!(),
            })?,
            Err(e) => {
                tracing::warn!("unable to parse blockfrost metadata into PrismObject: {}", e);
            }
        }
        Ok(())
    }

    async fn fetch_tx_by_id(api: &BlockfrostAPI, tx_hash: &str) -> Result<TxContent, DltError> {
        api.transaction_by_hash(tx_hash)
            .await
            .map_err(|e| DltError::Connection {
                source: e.into(),
                location: location!(),
            })
    }

    async fn fetch_metadata_page(api: &BlockfrostAPI, page: u32) -> Result<Option<TxMetadataLabelJsonInner>, DltError> {
        let pagination = blockfrost::Pagination::new(blockfrost::Order::Asc, page as usize, 1);
        let result = api
            .metadata_txs_by_label("21325", pagination)
            .await
            .map_err(|e| DltError::Connection {
                source: e.into(),
                location: location!(),
            })?;
        Ok(result.into_iter().next())
    }
}
