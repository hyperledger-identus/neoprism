use std::str::FromStr;
use std::sync::Arc;

use blockfrost::{BlockfrostAPI, Order, Pagination};
use blockfrost_openapi::models::{BlockContent, TxMetadataLabelJsonInner};
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

    use chrono::DateTime;
    use identus_apollo::hex::HexStr;
    use identus_did_prism::dlt::{BlockMetadata, PublishedPrismObject, TxId};
    use identus_did_prism::proto::MessageExt;
    use identus_did_prism::proto::prism::PrismObject;
    use serde::{Deserialize, Serialize};

    use crate::dlt::error::MetadataReadError;

    #[derive(Debug, Clone)]
    pub struct BlockfrostBlock {
        pub hash: String,
        pub height: u64,
        pub slot: u64,
        pub time: i64,
    }

    #[derive(Debug, Clone)]
    pub struct BlockfrostMetadata {
        pub tx_hash: String,
        pub tx_index: u32,
        pub json_metadata: serde_json::Value,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct MetadataMapJson {
        pub c: Vec<String>,
        pub v: u64,
    }

    pub fn parse_blockfrost_metadata(
        block: BlockfrostBlock,
        metadata: BlockfrostMetadata,
    ) -> Result<PublishedPrismObject, MetadataReadError> {
        let tx_idx = Some(metadata.tx_index as usize);
        let block_hash = HexStr::from_str(&block.hash).map_err(|e| MetadataReadError::InvalidMetadataType {
            source: e.into(),
            block_hash: None,
            tx_idx,
        })?;
        let block_hash_string = block_hash.to_string();

        let tx_id = TxId::from_str(&metadata.tx_hash).map_err(|e| MetadataReadError::InvalidMetadataType {
            source: e.into(),
            block_hash: Some(block_hash_string.clone()),
            tx_idx,
        })?;

        let cbt = DateTime::from_timestamp(block.time, 0).ok_or(MetadataReadError::InvalidBlockTimestamp {
            block_hash: Some(block_hash_string.clone()),
            tx_idx,
            timestamp: block.time,
        })?;

        let block_metadata = BlockMetadata {
            slot_number: block.slot.into(),
            block_number: block.height.into(),
            cbt,
            absn: metadata.tx_index,
            tx_id,
        };

        let metadata_json: MetadataMapJson =
            serde_json::from_value(metadata.json_metadata).map_err(|e| MetadataReadError::InvalidMetadataType {
                source: e.into(),
                block_hash: Some(block_hash_string.clone()),
                tx_idx,
            })?;

        let byte_group = metadata_json
            .c
            .into_iter()
            .map(|s| {
                if let Some((prefix, hex_suffix)) = s.split_at_checked(2)
                    && let Ok(hex_str) = HexStr::from_str(hex_suffix)
                    && prefix == "0x"
                {
                    Ok(hex_str.to_bytes())
                } else {
                    Err(MetadataReadError::InvalidMetadataType {
                        source: "expect metadata byte group to be in hex format".into(),
                        block_hash: Some(block_hash_string.clone()),
                        tx_idx,
                    })
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut bytes = Vec::with_capacity(64 * byte_group.len());
        for mut b in byte_group.into_iter() {
            bytes.append(&mut b);
        }

        let prism_object =
            PrismObject::decode(bytes.as_slice()).map_err(|e| MetadataReadError::PrismBlockProtoDecode {
                source: e,
                block_hash: Some(block_hash_string.clone()),
                tx_idx,
            })?;

        Ok(PublishedPrismObject {
            block_metadata,
            prism_object,
        })
    }
}

async fn fetch_latest_confirmed_block(api: &BlockfrostAPI, confirmation_blocks: u16) -> Result<BlockContent, DltError> {
    let block = api
        .blocks_latest()
        .await
        .map_err(|_| DltError::Connection { location: location!() })?;

    let tip_height = block
        .height
        .ok_or_else(|| DltError::Connection { location: location!() })? as i64;

    let confirmed_height = tip_height - confirmation_blocks as i64;

    if confirmed_height < 0 {
        return Err(DltError::Connection { location: location!() });
    }

    if confirmed_height == tip_height {
        Ok(block)
    } else {
        api.blocks_by_id(&confirmed_height.to_string())
            .await
            .map_err(|_| DltError::Connection { location: location!() })
    }
}

async fn fetch_prism_metadata_pages(api: &BlockfrostAPI) -> Result<Vec<TxMetadataLabelJsonInner>, DltError> {
    let mut results = Vec::new();
    let mut page = 1;

    loop {
        let pagination = Pagination::new(Order::Asc, page, 100);
        let page_results = api
            .metadata_txs_by_label("21325", pagination)
            .await
            .map_err(|_| DltError::Connection { location: location!() })?;

        if page_results.is_empty() {
            break;
        }

        results.extend(page_results);
        page += 1;
    }

    Ok(results)
}

async fn get_block_for_tx(api: &BlockfrostAPI, tx_hash: &str) -> Result<(models::BlockfrostBlock, u32), DltError> {
    let tx = api
        .transaction_by_hash(tx_hash)
        .await
        .map_err(|_| DltError::Connection { location: location!() })?;

    let block = models::BlockfrostBlock {
        hash: tx.block,
        height: tx.block_height as u64,
        slot: tx.slot as u64,
        time: tx.block_time as i64,
    };
    let tx_index = tx.index as u32;

    Ok((block, tx_index))
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

    fn persist_cursor(block: &models::BlockfrostBlock, sync_cursor_tx: &watch::Sender<Option<DltCursor>>) {
        let hex_str = match HexStr::from_str(&block.hash) {
            Ok(h) => h,
            Err(e) => {
                tracing::error!("Failed to parse block hash for cursor: {}, error: {}", block.hash, e);
                return;
            }
        };
        let block_hash_bytes = hex_str.to_bytes();
        let Some(cbt) = chrono::DateTime::from_timestamp(block.time, 0) else {
            return;
        };
        let cursor = DltCursor {
            slot: block.slot,
            block_hash: block_hash_bytes,
            cbt: Some(cbt),
        };
        let _ = sync_cursor_tx.send(Some(cursor));
        tracing::debug!("Cursor persisted to slot={}, height={}", block.slot, block.height);
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
            let confirmed_block = fetch_latest_confirmed_block(&api, confirmation_blocks).await?;
            let confirmed_height = confirmed_block
                .height
                .ok_or_else(|| DltError::Connection { location: location!() })?
                as u64;
            let confirmed_slot = confirmed_block
                .slot
                .ok_or_else(|| DltError::Connection { location: location!() })? as u64;
            let confirmed_time = confirmed_block.time as i64;

            let prism_txs = fetch_prism_metadata_pages(&api).await?;

            let mut new_prism_blocks = false;

            for tx_meta in prism_txs {
                let (block, tx_index) = get_block_for_tx(&api, &tx_meta.tx_hash).await?;

                if block.slot > current_slot && block.height <= confirmed_height {
                    let json_metadata = tx_meta
                        .json_metadata
                        .ok_or_else(|| DltError::Connection { location: location!() })?;
                    let metadata = models::BlockfrostMetadata {
                        tx_hash: tx_meta.tx_hash.clone(),
                        tx_index,
                        json_metadata,
                    };

                    match models::parse_blockfrost_metadata(block.clone(), metadata) {
                        Ok(prism_object) => {
                            tracing::info!(
                                "Detected PRISM metadata in tx={}, slot={}, index={}",
                                tx_meta.tx_hash,
                                block.slot,
                                tx_index
                            );

                            event_tx.send(prism_object).await.map_err(|e| DltError::EventHandling {
                                source: e.to_string().into(),
                                location: location!(),
                            })?;

                            Self::persist_cursor(&block, &sync_cursor_tx);

                            current_slot = block.slot;
                            new_prism_blocks = true;
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to parse PRISM metadata in tx={}, slot={}, index={}: {}",
                                tx_meta.tx_hash,
                                block.slot,
                                tx_index,
                                e
                            );
                        }
                    }
                }
            }

            if !new_prism_blocks {
                let block_for_cursor = models::BlockfrostBlock {
                    hash: confirmed_block.hash,
                    height: confirmed_height,
                    slot: confirmed_slot,
                    time: confirmed_time,
                };
                Self::persist_cursor(&block_for_cursor, &sync_cursor_tx);

                tokio::time::sleep(tokio::time::Duration::from_secs(poll_interval)).await;
            }
        }
    }
}
