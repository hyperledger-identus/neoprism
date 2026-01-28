//! Blockfrost DLT source implementation.
//!
//! This module provides a DLT source that fetches PRISM data from the Blockfrost API.
//! It follows the same pattern as DbSyncSource, using REST API polling.
//!
//! TODO: Implement actual Blockfrost API calls in the stream_loop method.

use identus_did_prism::dlt::{DltCursor, PublishedPrismObject};
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
        let block_hash = HexStr::from(block.hash.as_bytes());
        let block_hash_string = block_hash.to_string();
        let tx_idx = Some(metadata.tx_index as usize);

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

                if let Err(e) = Self::stream_loop(
                    &api_key,
                    &base_url,
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

    async fn stream_loop(
        _api_key: &str,
        _base_url: &str,
        _event_tx: mpsc::Sender<PublishedPrismObject>,
        _sync_cursor_tx: watch::Sender<Option<DltCursor>>,
        _from_slot: u64,
        _confirmation_blocks: u16,
        _poll_interval: u64,
    ) -> Result<(), DltError> {
        todo!("Implement Blockfrost streaming loop with API polling")
    }
}
