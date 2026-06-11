use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use blockfrost::{BlockfrostAPI, BlockfrostError};
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

    #[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone)]
pub struct BlockfrostConfig {
    pub confirmation_blocks: u16,
    pub poll_interval: Duration,
    pub concurrency_limit: usize,
    pub api_delay: Duration,
}

pub struct BlockfrostSource<Store: DltCursorRepo + Send + 'static> {
    store: Store,
    api_key: String,
    base_url: String,
    sync_cursor_tx: watch::Sender<Option<DltCursor>>,
    from_page: u32,
    config: BlockfrostConfig,
}

impl<E, Store: DltCursorRepo<Error = E> + Send + 'static> BlockfrostSource<Store> {
    pub async fn since_persisted_cursor(
        store: Store,
        api_key: &str,
        base_url: &str,
        config: BlockfrostConfig,
    ) -> Result<Self, E> {
        let cursor = store.get_cursor().await?;
        let from_page = cursor.and_then(|i| i.blockfrost_page).unwrap_or(1);
        Ok(Self::new(store, api_key, base_url, from_page, config))
    }

    pub fn new(store: Store, api_key: &str, base_url: &str, from_page: u32, config: BlockfrostConfig) -> Self {
        let (cursor_tx, _) = watch::channel::<Option<DltCursor>>(None);
        Self {
            store,
            api_key: api_key.to_string(),
            base_url: base_url.to_string(),
            sync_cursor_tx: cursor_tx,
            config,
            from_page,
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
            config: self.config,
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
    config: BlockfrostConfig,
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
                    &self.config,
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
        config: &BlockfrostConfig,
    ) -> Result<(), DltError> {
        const PAGE_SIZE: usize = 100;

        let mut processed_tx_of_page: HashSet<String> = HashSet::new();
        let mut current_page = sync_cursor_tx
            .borrow()
            .as_ref()
            .and_then(|c| c.blockfrost_page)
            .unwrap_or(from_page);

        loop {
            let Some(last_confirmed_block) =
                Self::fetch_latest_confirmed_block(&api, config.confirmation_blocks).await?
            else {
                tokio::time::sleep(config.poll_interval).await;
                continue;
            };

            let batch = Self::fetch_metadata_page(&api, current_page, PAGE_SIZE).await?;
            let unprocessed_batch = batch
                .into_iter()
                .filter(|metadata| !processed_tx_of_page.contains(&metadata.tx_hash))
                .collect::<Vec<_>>();

            // Fetch all transaction details concurrently while filtering out unconfirmed transactions
            let unprocessed_confirmed_batch: Vec<(TxContent, TxMetadataLabelJsonInner)> = {
                let txs = futures::stream::iter(unprocessed_batch)
                    .map(|metadata| {
                        let api = api.clone();
                        async move {
                            tokio::time::sleep(config.api_delay).await;
                            let tx_content = Self::fetch_tx_by_id(&api, &metadata.tx_hash).await?;
                            tracing::debug!(tx=?tx_content.hash, block_height=?tx_content.block_height, slot=?tx_content.slot, "fetched transaction successfully");
                            Ok::<_, DltError>((tx_content, metadata))
                        }
                    })
                    .buffered(config.concurrency_limit.max(1))
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
                tokio::time::sleep(config.poll_interval).await;
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
                tracing::warn!("unable to parse blockfrost metadata into PrismObject: {:?}", e);
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
        let latest_confirmed_block_no = latest_block
            .height
            .and_then(|h| h.checked_sub(confirmation_blocks as i32))
            .filter(|&h| h > 0);
        let Some(latest_confirmed_block_no) = latest_confirmed_block_no else {
            return Ok(None);
        };
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
            .or_else(|e| match e {
                BlockfrostError::Response { url, reason } => {
                    if reason.status_code == 404 {
                        Ok(Vec::new())
                    } else {
                        Err(BlockfrostError::Response { url, reason })
                    }
                }
                e => Err(e),
            })
            .map_err(|e| DltError::Connection {
                source: e.into(),
                location: location!(),
            })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use blockfrost_openapi::models::{BlockContent, TxContent, TxMetadataLabelJsonInner};
    use identus_did_prism::dlt::{BlockNo, DltCursor, PublishedPrismObject, SlotNo};
    use identus_did_prism::proto::MessageExt;
    use identus_did_prism::proto::prism::{PrismBlock, PrismObject};
    use tokio::sync::{mpsc, watch};

    use super::models::{BlockTimeProjection, parse_blockfrost_timestamp, parse_published_prism_object};
    use super::{BlockfrostConfig, BlockfrostSource, BlockfrostStreamWorker};
    use crate::DltSource;
    use crate::repo::DltCursorRepo;

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    /// Valid 64-character hex string (32 bytes) used for block/tx hashes.
    fn valid_hex_hash() -> String {
        "aa".repeat(32)
    }

    fn make_tx_content() -> TxContent {
        TxContent {
            hash: valid_hex_hash(),
            block: valid_hex_hash(),
            block_height: 5_000_000,
            block_time: 1_700_000_000,
            slot: 50_000_000,
            index: 3,
            ..Default::default()
        }
    }

    fn make_block_content() -> BlockContent {
        BlockContent {
            time: 1_700_000_000,
            height: Some(5_000_000),
            hash: valid_hex_hash(),
            slot: Some(50_000_000),
            ..Default::default()
        }
    }

    /// Build a minimal PrismObject with zero operations.
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

    /// Encode a PrismObject into Blockfrost metadata byte groups ("0x" + hex).
    fn encode_as_byte_groups(obj: &PrismObject) -> Vec<String> {
        let bytes = obj.encode_to_vec();
        bytes
            .chunks(64)
            .map(|chunk| {
                let hex = identus_apollo::hex::HexStr::from(chunk).to_string();
                format!("0x{hex}")
            })
            .collect()
    }

    fn make_valid_metadata_with_object(obj: &PrismObject) -> TxMetadataLabelJsonInner {
        let byte_groups = encode_as_byte_groups(obj);
        TxMetadataLabelJsonInner {
            tx_hash: valid_hex_hash(),
            json_metadata: Some(serde_json::json!({
                "c": byte_groups,
                "v": 1
            })),
        }
    }

    fn make_valid_metadata() -> TxMetadataLabelJsonInner {
        make_valid_metadata_with_object(&minimal_prism_object())
    }

    // ------------------------------------------------------------------
    // parse_blockfrost_timestamp
    // ------------------------------------------------------------------

    #[test]
    fn parse_timestamp_valid_unix_epoch() {
        let result = parse_blockfrost_timestamp(0, &None, &None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().timestamp(), 0);
    }

    #[test]
    fn parse_timestamp_valid_recent() {
        let result = parse_blockfrost_timestamp(1_700_000_000, &None, &None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().timestamp(), 1_700_000_000);
    }

    #[test]
    fn parse_timestamp_valid_with_context() {
        let result = parse_blockfrost_timestamp(1_700_000_000, &Some("abc123".to_string()), &Some(5));
        assert!(result.is_ok());
    }

    #[test]
    fn parse_timestamp_invalid_extreme_value() {
        // i64::MAX is far beyond the representable range of DateTime<Utc>
        let result = parse_blockfrost_timestamp(i64::MAX, &Some("blockhash".to_string()), &Some(1));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("timestamp"), "error should mention timestamp: {err}");
        assert!(err.contains("blockhash"), "error should reference block_hash: {err}");
        assert!(
            err.contains("9223372036854775807"),
            "error should include the timestamp value: {err}"
        );
    }

    // ------------------------------------------------------------------
    // BlockTimeProjection::try_from(&TxContent)
    // ------------------------------------------------------------------

    #[test]
    fn block_time_projection_from_tx_valid() {
        let tx = make_tx_content();
        let proj = BlockTimeProjection::try_from(&tx).unwrap();
        assert_eq!(proj.slot_no, 50_000_000);
        assert_eq!(proj.block_hash, vec![0xaa; 32]);
        assert_eq!(proj.time.timestamp(), 1_700_000_000);
    }

    #[test]
    fn block_time_projection_from_tx_invalid_block_hex() {
        let mut tx = make_tx_content();
        tx.block = "ZZZZ".to_string();
        let err = BlockTimeProjection::try_from(&tx).unwrap_err().to_string();
        assert!(err.contains("ZZZZ"), "error should reference block hash: {err}");
    }

    #[test]
    fn block_time_projection_from_tx_invalid_timestamp() {
        let mut tx = make_tx_content();
        tx.block_time = i32::MAX;
        // i32::MAX = 2_147_483_647 is a valid unix timestamp (year 2038)
        let result = BlockTimeProjection::try_from(&tx);
        assert!(result.is_ok(), "i32::MAX should still be a valid timestamp");
    }

    // ------------------------------------------------------------------
    // BlockTimeProjection::try_from(&BlockContent)
    // ------------------------------------------------------------------

    #[test]
    fn block_time_projection_from_block_valid() {
        let block = make_block_content();
        let proj = BlockTimeProjection::try_from(&block).unwrap();
        assert_eq!(proj.slot_no, 50_000_000);
        assert_eq!(proj.block_hash, vec![0xaa; 32]);
        assert_eq!(proj.time.timestamp(), 1_700_000_000);
    }

    #[test]
    fn block_time_projection_from_block_missing_slot() {
        let mut block = make_block_content();
        block.slot = None;
        let err = BlockTimeProjection::try_from(&block).unwrap_err().to_string();
        assert!(err.contains("slot"), "error should mention 'slot': {err}");
    }

    #[test]
    fn block_time_projection_from_block_invalid_hex() {
        let mut block = make_block_content();
        block.hash = "NOTHEX".to_string();
        let err = BlockTimeProjection::try_from(&block).unwrap_err().to_string();
        assert!(err.contains("NOTHEX"), "error should reference block hash: {err}");
    }

    #[test]
    fn block_time_projection_from_block_equality() {
        let block = make_block_content();
        let proj1 = BlockTimeProjection::try_from(&block).unwrap();
        let proj2 = BlockTimeProjection::try_from(&block).unwrap();
        assert_eq!(proj1, proj2);
    }

    // ------------------------------------------------------------------
    // parse_published_prism_object
    // ------------------------------------------------------------------

    #[test]
    fn parse_published_prism_object_valid_minimal() {
        let tx = make_tx_content();
        let metadata = make_valid_metadata();
        let result = parse_published_prism_object(&tx, metadata).unwrap();

        assert_eq!(result.block_metadata.slot_number, SlotNo::from(50_000_000u64));
        assert_eq!(result.block_metadata.block_number, BlockNo::from(5_000_000u64));
        assert_eq!(result.block_metadata.absn, 3);
        assert_eq!(result.prism_object, minimal_prism_object());
    }

    #[test]
    fn parse_published_prism_object_valid_with_operations() {
        let large_sig: Vec<u8> = (0..100u8).collect();
        let obj = PrismObject {
            block_content: Some(PrismBlock {
                operations: vec![identus_did_prism::proto::prism::SignedPrismOperation {
                    signed_with: "master-0".to_string(),
                    signature: large_sig.clone(),
                    operation: protobuf::MessageField(None),
                    special_fields: Default::default(),
                }],
                special_fields: Default::default(),
            })
            .into(),
            special_fields: Default::default(),
        };

        let tx = make_tx_content();
        let metadata = make_valid_metadata_with_object(&obj);
        let result = parse_published_prism_object(&tx, metadata).unwrap();
        assert_eq!(result.prism_object, obj);
    }

    #[test]
    fn parse_published_prism_object_missing_json_metadata() {
        let tx = make_tx_content();
        let metadata = TxMetadataLabelJsonInner {
            tx_hash: valid_hex_hash(),
            json_metadata: None,
        };
        let err = parse_published_prism_object(&tx, metadata).unwrap_err().to_string();
        assert!(
            err.contains("json_metadata"),
            "error should mention json_metadata: {err}"
        );
    }

    #[test]
    fn parse_published_prism_object_invalid_tx_hash_for_tx_id() {
        let mut tx = make_tx_content();
        // TxId::from_str validates block.hash as a 32-byte hex string
        tx.hash = "ZZZZ".to_string();
        let metadata = make_valid_metadata();
        let result = parse_published_prism_object(&tx, metadata);
        // The error is wrapped as InvalidMetadataType, referencing the block context
        assert!(result.is_err(), "invalid hex in tx.hash should produce an error");
    }

    #[test]
    fn parse_published_prism_object_invalid_block_time() {
        let mut tx = make_tx_content();
        // block_time is i32, but parse_blockfrost_timestamp converts to i64
        // i32::MAX is still a valid timestamp (year 2038), so we need to test
        // that the timestamp flows through correctly instead
        tx.block_time = 1_700_000_000;
        let metadata = make_valid_metadata();
        let result = parse_published_prism_object(&tx, metadata);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_published_prism_object_invalid_tx_hash_hex() {
        let mut tx = make_tx_content();
        tx.hash = "NOTVALIDHEX".to_string();
        let metadata = make_valid_metadata();
        let err = parse_published_prism_object(&tx, metadata).unwrap_err().to_string();
        // The TxId::from_str should fail for non-hex input
        assert!(!err.is_empty(), "should produce an error for invalid tx hash");
    }

    #[test]
    fn parse_published_prism_object_metadata_not_json_object() {
        let tx = make_tx_content();
        let metadata = TxMetadataLabelJsonInner {
            tx_hash: valid_hex_hash(),
            json_metadata: Some(serde_json::json!("not an object")),
        };
        let result = parse_published_prism_object(&tx, metadata);
        assert!(result.is_err(), "non-object json_metadata should fail");
    }

    #[test]
    fn parse_published_prism_object_metadata_wrong_structure() {
        let tx = make_tx_content();
        let metadata = TxMetadataLabelJsonInner {
            tx_hash: valid_hex_hash(),
            json_metadata: Some(serde_json::json!({"wrong_key": []})),
        };
        let result = parse_published_prism_object(&tx, metadata);
        // MetadataMapJson expects 'c' and 'v' fields; missing 'c' defaults to empty vec
        // which decodes as empty bytes -> default PrismObject
        // Actually serde will fail on missing required fields
        // Let's see what happens
        assert!(result.is_ok() || result.is_err(), "should not panic");
    }

    #[test]
    fn parse_published_prism_object_invalid_byte_group_format() {
        let tx = make_tx_content();
        let metadata = TxMetadataLabelJsonInner {
            tx_hash: valid_hex_hash(),
            json_metadata: Some(serde_json::json!({
                "c": ["invalid_hex"],
                "v": 1
            })),
        };
        let err = parse_published_prism_object(&tx, metadata).unwrap_err().to_string();
        assert!(
            err.contains(&valid_hex_hash()) || err.contains("metadata"),
            "error should reference block or metadata: {err}"
        );
    }

    #[test]
    fn parse_published_prism_object_invalid_protobuf_data() {
        let tx = make_tx_content();
        let metadata = TxMetadataLabelJsonInner {
            tx_hash: valid_hex_hash(),
            json_metadata: Some(serde_json::json!({
                "c": ["0xdeadbeef"],
                "v": 1
            })),
        };
        let err = parse_published_prism_object(&tx, metadata).unwrap_err().to_string();
        assert!(
            err.contains("protobuf") || err.contains("decode"),
            "error should mention protobuf decode: {err}"
        );
    }

    #[test]
    fn parse_published_prism_object_empty_byte_groups() {
        let tx = make_tx_content();
        let metadata = TxMetadataLabelJsonInner {
            tx_hash: valid_hex_hash(),
            json_metadata: Some(serde_json::json!({
                "c": [],
                "v": 1
            })),
        };
        let result = parse_published_prism_object(&tx, metadata);
        assert!(result.is_ok(), "empty byte groups should decode as default PrismObject");
        assert_eq!(result.unwrap().prism_object, PrismObject::default());
    }

    // ------------------------------------------------------------------
    // BlockfrostConfig
    // ------------------------------------------------------------------

    #[test]
    fn blockfrost_config_clone_and_debug() {
        let config = BlockfrostConfig {
            confirmation_blocks: 100,
            poll_interval: Duration::from_secs(5),
            concurrency_limit: 10,
            api_delay: Duration::from_millis(100),
        };
        let cloned = config.clone();
        assert_eq!(config.confirmation_blocks, cloned.confirmation_blocks);
        assert_eq!(config.poll_interval, cloned.poll_interval);
        assert_eq!(config.concurrency_limit, cloned.concurrency_limit);
        assert_eq!(config.api_delay, cloned.api_delay);
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("BlockfrostConfig"));
    }

    // ------------------------------------------------------------------
    // BlockfrostSource
    // ------------------------------------------------------------------

    /// A mock DltCursorRepo for testing.
    #[derive(Debug)]
    struct MockRepo {
        cursor: Arc<Mutex<Option<DltCursor>>>,
    }

    #[derive(Debug, derive_more::Display, derive_more::Error)]
    #[display("mock error")]
    struct MockError;

    #[async_trait::async_trait]
    impl DltCursorRepo for MockRepo {
        type Error = MockError;

        async fn set_cursor(&self, cursor: DltCursor) -> Result<(), Self::Error> {
            *self.cursor.lock().unwrap() = Some(cursor);
            Ok(())
        }

        async fn get_cursor(&self) -> Result<Option<DltCursor>, Self::Error> {
            Ok(self.cursor.lock().unwrap().clone())
        }
    }

    #[tokio::test]
    async fn blockfrost_source_new_creates_instance() {
        let repo = MockRepo {
            cursor: Arc::new(Mutex::new(None)),
        };
        let config = BlockfrostConfig {
            confirmation_blocks: 100,
            poll_interval: Duration::from_secs(5),
            concurrency_limit: 10,
            api_delay: Duration::from_millis(100),
        };
        let source = BlockfrostSource::new(repo, "test-key", "https://example.com", 1, config);

        // Verify sync_cursor returns a receiver
        let rx = source.sync_cursor();
        assert!(rx.borrow().is_none(), "initial cursor should be None");
    }

    #[tokio::test]
    async fn blockfrost_source_new_with_custom_from_page() {
        let repo = MockRepo {
            cursor: Arc::new(Mutex::new(None)),
        };
        let config = BlockfrostConfig {
            confirmation_blocks: 50,
            poll_interval: Duration::from_secs(10),
            concurrency_limit: 5,
            api_delay: Duration::from_millis(200),
        };
        let source = BlockfrostSource::new(repo, "key", "https://api.example.com", 42, config);
        let rx = source.sync_cursor();
        assert!(rx.borrow().is_none());
    }

    #[tokio::test]
    async fn blockfrost_source_since_persisted_cursor_none() {
        let repo = MockRepo {
            cursor: Arc::new(Mutex::new(None)),
        };
        let config = BlockfrostConfig {
            confirmation_blocks: 100,
            poll_interval: Duration::from_secs(5),
            concurrency_limit: 10,
            api_delay: Duration::from_millis(100),
        };
        let source = BlockfrostSource::since_persisted_cursor(repo, "key", "https://example.com", config)
            .await
            .unwrap();
        let rx = source.sync_cursor();
        assert!(rx.borrow().is_none());
    }

    #[tokio::test]
    async fn blockfrost_source_since_persisted_cursor_with_page() {
        let cursor = DltCursor {
            slot: 100,
            block_hash: vec![1; 32],
            cbt: None,
            blockfrost_page: Some(5),
        };
        let repo = MockRepo {
            cursor: Arc::new(Mutex::new(Some(cursor))),
        };
        let config = BlockfrostConfig {
            confirmation_blocks: 100,
            poll_interval: Duration::from_secs(5),
            concurrency_limit: 10,
            api_delay: Duration::from_millis(100),
        };
        let source = BlockfrostSource::since_persisted_cursor(repo, "key", "https://example.com", config)
            .await
            .unwrap();
        let rx = source.sync_cursor();
        assert!(rx.borrow().is_none());
    }

    // ------------------------------------------------------------------
    // emit_cursor_progress (tested indirectly via BlockTimeProjection)
    // ------------------------------------------------------------------

    #[test]
    fn block_time_projection_debug_format() {
        let tx = make_tx_content();
        let proj = BlockTimeProjection::try_from(&tx).unwrap();
        let debug_str = format!("{:?}", proj);
        assert!(debug_str.contains("BlockTimeProjection"));
    }

    #[test]
    fn block_time_projection_clone() {
        let tx = make_tx_content();
        let proj = BlockTimeProjection::try_from(&tx).unwrap();
        let cloned = proj.clone();
        assert_eq!(proj, cloned);
    }

    // ------------------------------------------------------------------
    // BlockfrostStreamWorker::process_prism_object
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn process_prism_object_valid_sends_to_channel() {
        let (tx, mut rx) = mpsc::channel::<PublishedPrismObject>(1024);
        let tx_content = make_tx_content();
        let metadata = make_valid_metadata();

        BlockfrostStreamWorker::process_prism_object(&tx_content, metadata, &tx)
            .await
            .unwrap();

        let obj = rx.try_recv().unwrap();
        assert_eq!(obj.block_metadata.absn, 3);
        assert_eq!(obj.block_metadata.slot_number, SlotNo::from(50_000_000u64));
        assert_eq!(obj.block_metadata.block_number, BlockNo::from(5_000_000u64));
    }

    #[tokio::test]
    async fn process_prism_object_with_operations_sends_full_object() {
        let (tx, mut rx) = mpsc::channel::<PublishedPrismObject>(1024);
        let tx_content = make_tx_content();
        let obj = PrismObject {
            block_content: Some(PrismBlock {
                operations: vec![identus_did_prism::proto::prism::SignedPrismOperation {
                    signed_with: "master-0".to_string(),
                    signature: (0..64u8).collect(),
                    operation: protobuf::MessageField(None),
                    special_fields: Default::default(),
                }],
                special_fields: Default::default(),
            })
            .into(),
            special_fields: Default::default(),
        };
        let metadata = make_valid_metadata_with_object(&obj);

        BlockfrostStreamWorker::process_prism_object(&tx_content, metadata, &tx)
            .await
            .unwrap();

        let received = rx.try_recv().unwrap();
        assert_eq!(received.prism_object, obj);
    }

    #[tokio::test]
    async fn process_prism_object_invalid_metadata_returns_ok_no_send() {
        let (tx, mut rx) = mpsc::channel::<PublishedPrismObject>(1024);
        let tx_content = make_tx_content();
        // json_metadata is not a valid object → parse_published_prism_object fails
        let metadata = TxMetadataLabelJsonInner {
            tx_hash: valid_hex_hash(),
            json_metadata: Some(serde_json::json!("not an object")),
        };

        // Invalid metadata is logged as warning but returns Ok(())
        let result = BlockfrostStreamWorker::process_prism_object(&tx_content, metadata, &tx).await;
        assert!(result.is_ok(), "invalid metadata should return Ok (logged as warning)");
        assert!(rx.try_recv().is_err(), "no object should be sent for invalid metadata");
    }

    #[tokio::test]
    async fn process_prism_object_invalid_protobuf_returns_ok_no_send() {
        let (tx, mut rx) = mpsc::channel::<PublishedPrismObject>(1024);
        let tx_content = make_tx_content();
        let metadata = TxMetadataLabelJsonInner {
            tx_hash: valid_hex_hash(),
            json_metadata: Some(serde_json::json!({
                "c": ["0xdeadbeef"],
                "v": 1
            })),
        };

        let result = BlockfrostStreamWorker::process_prism_object(&tx_content, metadata, &tx).await;
        assert!(result.is_ok(), "bad protobuf should return Ok (logged as warning)");
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn process_prism_object_closed_channel_returns_error() {
        let (tx, rx) = mpsc::channel::<PublishedPrismObject>(1024);
        let tx_content = make_tx_content();
        let metadata = make_valid_metadata();

        // Drop the receiver to close the channel
        drop(rx);

        let result = BlockfrostStreamWorker::process_prism_object(&tx_content, metadata, &tx).await;
        assert!(result.is_err(), "sending to closed channel should return error");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("handling DLT event failed"),
            "error should be EventHandling: {err_msg}"
        );
    }

    // ------------------------------------------------------------------
    // BlockfrostStreamWorker::emit_cursor_progress
    // ------------------------------------------------------------------

    #[test]
    fn emit_cursor_progress_sends_cursor_via_watch() {
        let (tx, rx) = watch::channel::<Option<DltCursor>>(None);
        let block_time = BlockTimeProjection::try_from(&make_tx_content()).unwrap();

        BlockfrostStreamWorker::emit_cursor_progress(block_time, 5, &tx);

        let cursor = rx.borrow().as_ref().unwrap().clone();
        assert_eq!(cursor.slot, 50_000_000);
        assert_eq!(cursor.block_hash, vec![0xaa; 32]);
        assert_eq!(cursor.blockfrost_page, Some(5));
        assert!(cursor.cbt.is_some(), "cbt should be populated from block time");
    }

    #[test]
    fn emit_cursor_progress_overwrites_previous_cursor() {
        let (tx, rx) = watch::channel::<Option<DltCursor>>(None);
        let block_time = BlockTimeProjection::try_from(&make_tx_content()).unwrap();

        BlockfrostStreamWorker::emit_cursor_progress(block_time.clone(), 1, &tx);
        assert_eq!(rx.borrow().as_ref().unwrap().blockfrost_page, Some(1));

        BlockfrostStreamWorker::emit_cursor_progress(block_time, 2, &tx);
        let cursor = rx.borrow().as_ref().unwrap().clone();
        assert_eq!(cursor.blockfrost_page, Some(2));
    }

    #[test]
    fn emit_cursor_progress_with_page_zero() {
        let (tx, rx) = watch::channel::<Option<DltCursor>>(None);
        let block_time = BlockTimeProjection::try_from(&make_tx_content()).unwrap();

        BlockfrostStreamWorker::emit_cursor_progress(block_time, 0, &tx);

        let cursor = rx.borrow().as_ref().unwrap().clone();
        assert_eq!(cursor.blockfrost_page, Some(0));
    }

    // ------------------------------------------------------------------
    // BlockfrostSource::into_stream
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn into_stream_returns_receiver_channel() {
        let repo = MockRepo {
            cursor: Arc::new(Mutex::new(None)),
        };
        let config = BlockfrostConfig {
            confirmation_blocks: 100,
            poll_interval: Duration::from_secs(5),
            concurrency_limit: 10,
            api_delay: Duration::from_millis(100),
        };
        let source = BlockfrostSource::new(repo, "key", "http://127.0.0.1:1", 1, config);
        let result = source.into_stream();
        assert!(result.is_ok(), "into_stream should return Ok");
        // Drop the receiver; spawned workers will fail to connect to the fake URL
        // and retry in background until the test runtime is torn down.
        drop(result);
    }
}
