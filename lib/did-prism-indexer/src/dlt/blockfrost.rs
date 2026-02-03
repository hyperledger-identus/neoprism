use std::collections::HashSet;
use std::sync::Arc;

use blockfrost::BlockfrostAPI;
use blockfrost_openapi::models::{BlockContent, TxContent, TxMetadataLabelJsonInner};
use futures::{StreamExt, TryStreamExt};
use identus_did_prism::dlt::{DltCursor, PublishedPrismObject};
use identus_did_prism::location;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;

use crate::DltSource;
use crate::dlt::blockfrost::models::BlockTimeProjection;
use crate::dlt::common::CursorPersistWorker;
use crate::dlt::error::DltError;
use crate::repo::DltCursorRepo;

mod models {
    use std::str::FromStr;

    use blockfrost_openapi::models::{BlockContent, TxContent, TxMetadataLabelJsonInner};
    use chrono::{DateTime, Utc};
    use identus_apollo::hex::HexStr;
    use identus_did_prism::dlt::{BlockMetadata, BlockNo, PublishedPrismObject, SlotNo, TxId};

    use crate::dlt::common::metadata_map::MetadataMapJson;
    use crate::dlt::error::MetadataReadError;

    #[derive(Debug, Clone)]
    pub(crate) struct BlockTimeProjection {
        pub time: DateTime<Utc>,
        pub slot_no: i64,
        pub block_hash: Vec<u8>,
    }

    impl TryFrom<&TxContent> for BlockTimeProjection {
        type Error = MetadataReadError;

        fn try_from(tx: &TxContent) -> Result<Self, Self::Error> {
            let block_hash_hex = HexStr::from_str(&tx.block).map_err(|e| MetadataReadError::PrismBlockHexDecode {
                source: e,
                block_hash: Some(tx.block.clone()),
                tx_idx: Some(tx.index as usize),
            })?;
            let time =
                parse_blockfrost_timestamp(tx.block_time as i64, &Some(tx.block.clone()), &Some(tx.index as usize))?;

            Ok(BlockTimeProjection {
                time,
                slot_no: tx.slot as i64,
                block_hash: block_hash_hex.to_bytes(),
            })
        }
    }

    impl TryFrom<&BlockContent> for BlockTimeProjection {
        type Error = MetadataReadError;

        fn try_from(block: &BlockContent) -> Result<Self, Self::Error> {
            let block_hash_hex = HexStr::from_str(&block.hash).map_err(|e| MetadataReadError::PrismBlockHexDecode {
                source: e,
                block_hash: Some(block.hash.clone()),
                tx_idx: None,
            })?;
            let Some(slot_no) = block.slot else {
                Err(MetadataReadError::MissingBlockProperty {
                    block_hash: Some(block.hash.clone()),
                    tx_idx: None,
                    name: "slot",
                })?
            };
            let time = parse_blockfrost_timestamp(block.time as i64, &Some(block.hash.clone()), &None)?;

            Ok(BlockTimeProjection {
                time,
                slot_no: slot_no as i64,
                block_hash: block_hash_hex.to_bytes(),
            })
        }
    }

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

        let block_metadata = parse_block_metadata(block, &block_hash, &tx_idx)?;

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
    concurrency_limit: usize,
}

impl<E, Store: DltCursorRepo<Error = E> + Send + 'static> BlockfrostSource<Store> {
    pub async fn since_persisted_cursor(
        store: Store,
        api_key: &str,
        base_url: &str,
        confirmation_blocks: u16,
        poll_interval: u64,
        concurrency_limit: usize,
    ) -> Result<Self, E> {
        let cursor = store.get_cursor().await?;
        Ok(Self::new(
            store,
            api_key,
            base_url,
            cursor.and_then(|i| i.blockfrost_page).unwrap_or(1),
            confirmation_blocks,
            poll_interval,
            concurrency_limit,
        ))
    }

    pub fn new(
        store: Store,
        api_key: &str,
        base_url: &str,
        from_page: u32,
        confirmation_blocks: u16,
        poll_interval: u64,
        concurrency_limit: usize,
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
            concurrency_limit,
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
            concurrency_limit: self.concurrency_limit,
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
    concurrency_limit: usize,
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
                    self.concurrency_limit,
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

    fn emit_cursor_progress(
        block_time: BlockTimeProjection,
        page: u32,
        sync_cursor_tx: &watch::Sender<Option<DltCursor>>,
    ) {
        let cursor = DltCursor {
            slot: block_time.slot_no as u64,
            block_hash: block_time.block_hash,
            cbt: Some(block_time.time),
            blockfrost_page: Some(page),
        };
        let _ = sync_cursor_tx.send(Some(cursor));
    }

    async fn stream_loop(
        api: Arc<BlockfrostAPI>,
        event_tx: mpsc::Sender<PublishedPrismObject>,
        sync_cursor_tx: watch::Sender<Option<DltCursor>>,
        from_page: u32,
        confirmation_blocks: u16,
        poll_interval: u64,
        concurrency_limit: usize,
    ) -> Result<(), DltError> {
        const PAGE_SIZE: usize = 100;

        let mut processed_tx_of_page: HashSet<String> = HashSet::new();
        let mut current_page = sync_cursor_tx
            .borrow()
            .as_ref()
            .and_then(|c| c.blockfrost_page)
            .unwrap_or(from_page);

        loop {
            let Some(last_confirmed_block) = Self::fetch_latest_confirmed_block(&api, confirmation_blocks).await?
            else {
                tokio::time::sleep(tokio::time::Duration::from_secs(poll_interval)).await;
                continue;
            };

            let batch = Self::fetch_metadata_page(&api, current_page, PAGE_SIZE).await?;
            let unprocessed_batch = batch
                .into_iter()
                .filter(|metadata| !processed_tx_of_page.contains(&metadata.tx_hash))
                .collect::<Vec<_>>();

            // Fetch all transaction details concurrently while filtering out unconfirmed transactions
            let unprocessed_confirmed_batch: Vec<(TxContent, TxMetadataLabelJsonInner)> = {
                let txs = futures::stream::iter(unprocessed_batch.into_iter())
                    .map(|metadata| {
                        let api = api.clone();
                        async move {
                            let tx_content = Self::fetch_tx_by_id(&api, &metadata.tx_hash).await?;
                            tracing::debug!(tx=?tx_content.hash, block_height=?tx_content.block_height, slot=?tx_content.slot, "fetched transaction successfully");
                            Ok::<_, DltError>((tx_content, metadata))
                        }
                    })
                    .buffered(concurrency_limit.max(1))
                    .try_collect::<Vec<_>>()
                    .await?;

                txs.into_iter()
                    .filter(|(tx, _)| {
                        last_confirmed_block
                            .height
                            .map(|height| tx.block_height <= height)
                            .unwrap_or(false)
                    })
                    .collect::<Vec<_>>()
            };

            if unprocessed_confirmed_batch.is_empty() {
                // No new data, emit cursor from latest confirmed block and sleep
                tracing::debug!(last_confirmed_block_height=?last_confirmed_block.height, ?current_page, processed=?processed_tx_of_page.len(), "no more confirmed transaction to process");
                if let Ok(block_time) = BlockTimeProjection::try_from(&last_confirmed_block) {
                    Self::emit_cursor_progress(block_time, current_page, &sync_cursor_tx);
                };
                tokio::time::sleep(tokio::time::Duration::from_secs(poll_interval)).await;
                continue;
            }

            for (tx_content, metadata) in unprocessed_confirmed_batch {
                let process_result = Self::process_prism_object(&tx_content, metadata, &event_tx).await;
                if let Err(e) = process_result {
                    tracing::error!("error handling event from blockfrost source");
                    let report = std::error::Report::new(&e).pretty(true);
                    tracing::error!("{}", report);
                    return Err(e);
                }

                if let Ok(block_time) = BlockTimeProjection::try_from(&tx_content) {
                    Self::emit_cursor_progress(block_time, current_page, &sync_cursor_tx);
                }
                processed_tx_of_page.insert(tx_content.hash);
            }

            // stay on the same page until all transactions are confirmed and processed.
            // transactions are guaranteed to eventually be confirmed, so we don't
            // advance to the next page until processed_tx_of_page reaches PAGE_SIZE.
            if processed_tx_of_page.len() >= PAGE_SIZE {
                current_page += 1;
                processed_tx_of_page.clear();
            }
        }
    }

    async fn process_prism_object(
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

    async fn fetch_latest_confirmed_block(
        api: &BlockfrostAPI,
        confirmation_blocks: u16,
    ) -> Result<Option<BlockContent>, DltError> {
        let latest_block = api.blocks_latest().await.map_err(|e| DltError::Connection {
            source: e.into(),
            location: location!(),
        })?;
        let Some(latest_confirmed_block_no) = latest_block.height.map(|h| h - (confirmation_blocks as i32)) else {
            return Ok(None);
        };
        if latest_confirmed_block_no <= 0 {
            return Ok(None);
        }
        let latest_confirmed_block = api
            .blocks_by_id(&latest_confirmed_block_no.to_string())
            .await
            .map_err(|e| DltError::Connection {
                source: e.into(),
                location: location!(),
            })?;
        Ok(Some(latest_confirmed_block))
    }

    async fn fetch_tx_by_id(api: &BlockfrostAPI, tx_hash: &str) -> Result<TxContent, DltError> {
        api.transaction_by_hash(tx_hash)
            .await
            .map_err(|e| DltError::Connection {
                source: e.into(),
                location: location!(),
            })
    }

    async fn fetch_metadata_page(
        api: &BlockfrostAPI,
        page: u32,
        page_size: usize,
    ) -> Result<Vec<TxMetadataLabelJsonInner>, DltError> {
        let pagination = blockfrost::Pagination::new(blockfrost::Order::Asc, page as usize, page_size);
        api.metadata_txs_by_label("21325", pagination)
            .await
            .map_err(|e| DltError::Connection {
                source: e.into(),
                location: location!(),
            })
    }
}
