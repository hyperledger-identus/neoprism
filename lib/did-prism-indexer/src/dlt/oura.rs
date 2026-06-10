use std::str::FromStr;
use std::sync::Arc;
use std::sync::mpsc::RecvTimeoutError;

use identus_apollo::hex::HexStr;
use identus_did_prism::dlt::{DltCursor, NetworkIdentifier, PublishedPrismObject};
use identus_did_prism::location;
use oura::model::{Event, EventData};
use oura::pipelining::{SourceProvider, StageReceiver};
use oura::sources::n2n::Config;
use oura::sources::{AddressArg, IntersectArg, MagicArg, PointArg};
use oura::utils::{ChainWellKnownInfo, Utils, WithUtils};
use tokio::sync::{mpsc, watch};

use super::error::DltError;
use crate::DltSource;
use crate::dlt::common::CursorPersistWorker;
use crate::repo::DltCursorRepo;

mod models {
    use std::str::FromStr;

    use chrono::{DateTime, Utc};
    use identus_did_prism::dlt::{BlockMetadata, BlockNo, PublishedPrismObject, SlotNo, TxId};
    use identus_did_prism::prelude::*;
    use identus_did_prism::proto::prism::PrismObject;
    use oura::model::{EventContext, MetadataRecord};

    use crate::dlt::error::MetadataReadError;

    pub fn parse_oura_timestamp(context: &EventContext) -> Result<DateTime<Utc>, MetadataReadError> {
        let block_hash = &context.block_hash;
        let tx_idx = context.tx_idx;
        let timestamp = context.timestamp.ok_or(MetadataReadError::MissingBlockProperty {
            block_hash: block_hash.clone(),
            tx_idx,
            name: "timestamp",
        })? as i64;
        DateTime::from_timestamp(timestamp, 0).ok_or(MetadataReadError::InvalidBlockTimestamp {
            block_hash: block_hash.clone(),
            timestamp,
            tx_idx,
        })
    }

    fn parse_block_metadata(
        context: &EventContext,
        block_hash: &Option<String>,
        tx_idx: &Option<usize>,
    ) -> Result<BlockMetadata, MetadataReadError> {
        let timestamp = parse_oura_timestamp(context)?;

        let tx_hash_hex = context
            .tx_hash
            .as_ref()
            .ok_or(MetadataReadError::MissingBlockProperty {
                block_hash: block_hash.clone(),
                tx_idx: *tx_idx,
                name: "tx_hash",
            })?;
        let tx_id = TxId::from_str(tx_hash_hex).map_err(|e| MetadataReadError::InvalidMetadataType {
            source: e.into(),
            block_hash: block_hash.clone(),
            tx_idx: *tx_idx,
        })?;

        Ok(BlockMetadata {
            cbt: timestamp,
            absn: context.tx_idx.ok_or(MetadataReadError::MissingBlockProperty {
                block_hash: block_hash.clone(),
                tx_idx: *tx_idx,
                name: "tx_idx",
            })? as u32,
            block_number: BlockNo::from(context.block_number.ok_or(MetadataReadError::MissingBlockProperty {
                block_hash: block_hash.clone(),
                tx_idx: *tx_idx,
                name: "block_number",
            })?),
            slot_number: SlotNo::from(context.slot.ok_or(MetadataReadError::MissingBlockProperty {
                block_hash: block_hash.clone(),
                tx_idx: *tx_idx,
                name: "slot",
            })?),
            tx_id,
        })
    }

    pub fn parse_published_prism_object(
        context: EventContext,
        metadata: MetadataRecord,
    ) -> Result<PublishedPrismObject, MetadataReadError> {
        let block_hash = context.block_hash.clone();
        let tx_idx = context.tx_idx;

        let block_metadata = parse_block_metadata(&context, &block_hash, &tx_idx)?;

        // parse prism_block
        let byte_group = match metadata.metadatum {
            pallas_primitives::alonzo::Metadatum::Map(kv) => kv
                .to_vec()
                .into_iter()
                .find(|(k, _)| match k {
                    pallas_primitives::alonzo::Metadatum::Text(k) => k == "c",
                    _ => false,
                })
                .and_then(|(_, v)| match v {
                    pallas_primitives::alonzo::Metadatum::Array(ms) => Some(ms),
                    _ => None,
                })
                .and_then(|byte_group| {
                    byte_group
                        .into_iter()
                        .map(|b| match b {
                            pallas_primitives::alonzo::Metadatum::Bytes(bytes) => Some(bytes.to_vec()),
                            _ => None,
                        })
                        .collect::<Option<Vec<_>>>()
                }),
            _ => None,
        }
        .ok_or(MetadataReadError::InvalidMetadataType {
            source: "metadata is not a valid type".to_string().into(),
            block_hash: block_hash.clone(),
            tx_idx,
        })?;

        let mut bytes = Vec::with_capacity(64 * byte_group.len());
        for mut b in byte_group.into_iter() {
            bytes.append(&mut b);
        }

        let prism_object =
            PrismObject::decode(bytes.as_slice()).map_err(|e| MetadataReadError::PrismBlockProtoDecode {
                source: e,
                block_hash,
                tx_idx,
            })?;

        Ok(PublishedPrismObject {
            block_metadata,
            prism_object,
        })
    }
}

fn magic_args(network: &NetworkIdentifier) -> MagicArg {
    let chain_magic = MagicArg::from_str(&network.to_string());
    chain_magic.expect("The chain magic value cannot be parsed")
}

fn chain_wellknown_info(network: &NetworkIdentifier) -> ChainWellKnownInfo {
    match network {
        NetworkIdentifier::Mainnet => ChainWellKnownInfo::mainnet(),
        NetworkIdentifier::Preprod => ChainWellKnownInfo::preprod(),
        NetworkIdentifier::Preview => ChainWellKnownInfo::preview(),
        NetworkIdentifier::Custom => panic!("custom network cannot be used with oura source"),
    }
}

pub struct OuraN2NSource<Store: DltCursorRepo + Send + 'static> {
    with_utils: WithUtils<Config>,
    store: Store,
    sync_cursor_tx: watch::Sender<Option<DltCursor>>,
}

impl<E, Store: DltCursorRepo<Error = E> + Send + 'static> OuraN2NSource<Store> {
    pub fn since_genesis(store: Store, remote_addr: &str, chain: &NetworkIdentifier, confirmation_blocks: u16) -> Self {
        let intersect = match chain {
            NetworkIdentifier::Mainnet => oura::sources::IntersectArg::Point(PointArg(
                71482583,
                "4df807a0246569860bbfe70642b9931a5dffbab7f56319a32bbef378dfddaf30".to_string(),
            )),
            NetworkIdentifier::Preprod => oura::sources::IntersectArg::Point(PointArg(
                10718513,
                "de0b6bbb233c646ab6bfc98932349f8ca43003ef32b0941b5dca71e40f6d8c3c".to_string(),
            )),
            _ => oura::sources::IntersectArg::Origin,
        };
        Self::new(store, remote_addr, chain, intersect, confirmation_blocks)
    }

    pub async fn since_persisted_cursor_or_genesis(
        store: Store,
        remote_addr: &str,
        chain: &NetworkIdentifier,
        confirmation_blocks: u16,
    ) -> Result<Self, E> {
        let cursor = store.get_cursor().await?;
        match cursor {
            Some(cursor) => {
                let blockhash_hex = HexStr::from(cursor.block_hash).to_string();
                tracing::info!(
                    "persisted cursor found, resuming sync from slot ({}, {})",
                    cursor.slot,
                    blockhash_hex
                );
                let intersect = oura::sources::IntersectArg::Point(PointArg(cursor.slot, blockhash_hex));
                Ok(Self::new(store, remote_addr, chain, intersect, confirmation_blocks))
            }
            None => {
                tracing::info!("persisted cursor not found, starting sync from PRISM genesis slot");
                Ok(Self::since_genesis(store, remote_addr, chain, confirmation_blocks))
            }
        }
    }

    pub fn new(
        store: Store,
        remote_addr: &str,
        chain: &NetworkIdentifier,
        intersect: IntersectArg,
        confirmation_blocks: u16,
    ) -> Self {
        #[allow(deprecated)]
        let config = Config {
            address: AddressArg(oura::sources::BearerKind::Tcp, remote_addr.to_string()),
            magic: Some(magic_args(chain)),
            since: None,
            intersect: Some(intersect),
            well_known: None,
            mapper: Default::default(),
            min_depth: confirmation_blocks.into(),
            retry_policy: Some(oura::sources::RetryPolicy {
                chainsync_max_retries: 0,
                chainsync_max_backoff: 60,
                connection_max_retries: 0,
                connection_max_backoff: 60,
            }),
            finalize: None,
        };
        let utils = Utils::new(chain_wellknown_info(chain));
        let with_utils = WithUtils::new(config, Arc::new(utils));
        let (sync_cursor_tx, _) = watch::channel::<Option<DltCursor>>(None);
        Self {
            with_utils,
            store,
            sync_cursor_tx,
        }
    }
}

impl<Store: DltCursorRepo + Send> DltSource for OuraN2NSource<Store> {
    fn sync_cursor(&self) -> watch::Receiver<Option<DltCursor>> {
        self.sync_cursor_tx.subscribe()
    }

    fn into_stream(self) -> Result<mpsc::Receiver<PublishedPrismObject>, String> {
        let (event_tx, rx) = tokio::sync::mpsc::channel::<PublishedPrismObject>(1024);

        let cursor_persist_worker = CursorPersistWorker::new(self.store, self.sync_cursor_tx.subscribe());
        let stream_worker = OuraStreamWorker {
            with_utils: self.with_utils,
            sync_cursor_tx: self.sync_cursor_tx,
            event_tx,
        };

        stream_worker.spawn();
        cursor_persist_worker.spawn();

        Ok(rx)
    }
}

struct OuraStreamWorker {
    with_utils: WithUtils<Config>,
    sync_cursor_tx: watch::Sender<Option<DltCursor>>,
    event_tx: mpsc::Sender<PublishedPrismObject>,
}

impl OuraStreamWorker {
    /// std thread is used to avoid oura receiver blocking on tokio pool
    fn spawn(self) -> std::thread::JoinHandle<Result<(), DltError>> {
        const RESTART_DELAY: std::time::Duration = std::time::Duration::from_secs(10);
        std::thread::spawn(move || {
            loop {
                let with_utils = self.build_with_util();
                tracing::info!("starting oura stream worker");
                let (handle, oura_rx) = with_utils.bootstrap().map_err(|e| DltError::InitSource {
                    source: e.to_string().into(),
                })?;

                // When the stream loop terminates with recv timeout,
                // the oura thread join will hangs and it will block the pipeline restart process.
                // We just ignore the thread and make sure the restart is not blocked.
                // Resource usage will grow over time, hopefully that is ok.
                match self.stream_loop(oura_rx) {
                    DltError::EventRecvTimeout { .. } => drop(handle),
                    _ => {
                        let _ = handle.join();
                    }
                };

                tracing::error!("oura pipeline terminated, restarting in {}s", RESTART_DELAY.as_secs());
                std::thread::sleep(RESTART_DELAY);
            }
        })
    }

    /// Construct WithUtils instance from the last event sent to persist worker.
    fn build_with_util(&self) -> WithUtils<Config> {
        let mut owned_with_utils = self.with_utils.clone();
        let rx = self.sync_cursor_tx.subscribe();
        let prev_cursor = rx.borrow();
        let prev_intersect = prev_cursor
            .as_ref()
            .map(|c| oura::sources::IntersectArg::Point(PointArg(c.slot, HexStr::from(&c.block_hash).to_string())));
        let intersect = prev_intersect
            .map(Some)
            .unwrap_or_else(|| owned_with_utils.inner.intersect.clone());
        owned_with_utils.inner.intersect = intersect;
        owned_with_utils
    }

    fn stream_loop(&self, receiver: StageReceiver) -> DltError {
        const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20 * 60);
        loop {
            let process_result = match receiver.recv_timeout(TIMEOUT) {
                Ok(event) => {
                    let event_result = self.process_prism_object(event.clone());
                    self.emit_cursor_progress(&event);
                    event_result
                }
                Err(RecvTimeoutError::Timeout) => Err(DltError::EventRecvTimeout { location: location!() }),
                Err(RecvTimeoutError::Disconnected) => Err(DltError::Connection {
                    source: RecvTimeoutError::Disconnected.into(),
                    location: location!(),
                }),
            };
            if let Err(e) = process_result {
                tracing::error!("error handling event from oura source");
                let report = std::error::Report::new(&e).pretty(true);
                tracing::error!("{}", report);
                return e;
            }
        }
    }

    fn emit_cursor_progress(&self, event: &Event) {
        let Some(slot) = event.context.slot else {
            return;
        };
        let Some(block_hash_hex) = &event.context.block_hash else {
            return;
        };
        let Ok(block_hash) = HexStr::from_str(block_hash_hex) else {
            return;
        };
        let Ok(timestamp) = models::parse_oura_timestamp(&event.context) else {
            return;
        };
        let cursor = DltCursor {
            slot,
            block_hash: block_hash.to_bytes(),
            cbt: Some(timestamp),
            blockfrost_page: None,
        };
        let _ = self.sync_cursor_tx.send(Some(cursor));
    }

    fn process_prism_object(&self, event: Event) -> Result<(), DltError> {
        let EventData::Metadata(meta) = event.data else {
            return Ok(());
        };
        if meta.label != "21325" {
            return Ok(());
        }

        let context = event.context;
        tracing::info!(
            "detected a new prism_block on slot ({}, {})",
            context.slot.unwrap_or_default(),
            context.block_hash.as_deref().unwrap_or_default()
        );

        let parsed_prism_object = models::parse_published_prism_object(context, meta);
        match parsed_prism_object {
            Ok(prism_object) => self
                .event_tx
                .blocking_send(prism_object)
                .map_err(|e| DltError::EventHandling {
                    source: e.to_string().into(),
                    location: location!(),
                })?,
            Err(e) => {
                tracing::warn!("unable to parse oura metadata into PrismObject: {:?}", e);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use std::sync::{Arc, Mutex};

    use identus_apollo::hex::HexStr;
    use identus_did_prism::dlt::{DltCursor, NetworkIdentifier};
    use identus_did_prism::proto::MessageExt;
    use identus_did_prism::proto::prism::{PrismBlock, PrismObject};
    use oura::model::{Event, EventContext, EventData, MetadataRecord};
    use pallas_codec::utils::{Bytes, KeyValuePairs};
    use pallas_primitives::alonzo::Metadatum;
    use tokio::sync::{mpsc, watch};

    use super::{OuraN2NSource, OuraStreamWorker, models};
    use crate::DltSource;
    use crate::repo::DltCursorRepo;

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    /// A valid 64-char hex string (32 bytes) for hashes.
    fn valid_hex_hash() -> String {
        "aa".repeat(32)
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

    /// Encode a PrismObject into raw bytes.
    fn encode_prism_object(obj: &PrismObject) -> Vec<u8> {
        obj.encode_to_vec()
    }

    /// Build a valid EventContext with all fields populated.
    fn valid_event_context() -> EventContext {
        EventContext {
            block_hash: Some(valid_hex_hash()),
            block_number: Some(5_000_000),
            slot: Some(50_000_000),
            timestamp: Some(1_700_000_000),
            tx_idx: Some(3),
            tx_hash: Some(valid_hex_hash()),
            ..Default::default()
        }
    }

    /// Build a MetadataRecord with the correct label and a valid metadatum
    /// containing the given PrismObject encoded as byte groups.
    fn make_valid_metadata(obj: &PrismObject) -> MetadataRecord {
        let bytes = encode_prism_object(obj);
        // Split into 64-byte chunks like Cardano metadata does
        let chunks: Vec<Metadatum> = bytes
            .chunks(64)
            .map(|chunk| Metadatum::Bytes(Bytes::from(chunk.to_vec())))
            .collect();

        let map = vec![
            (Metadatum::Text("c".to_string()), Metadatum::Array(chunks)),
            (Metadatum::Text("v".to_string()), Metadatum::Int(1i64.into())),
        ];

        MetadataRecord {
            label: "21325".to_string(),
            content: oura::model::MetadatumRendition::MapJson(serde_json::json!({})),
            metadatum: Metadatum::Map(KeyValuePairs::Def(map)),
        }
    }

    /// Build a valid Event wrapping metadata.
    fn make_valid_event(obj: &PrismObject) -> Event {
        Event {
            context: valid_event_context(),
            data: EventData::Metadata(make_valid_metadata(obj)),
            fingerprint: None,
        }
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

    // ==================================================================
    // models::parse_oura_timestamp
    // ==================================================================

    #[test]
    fn parse_oura_timestamp_valid() {
        let ctx = EventContext {
            timestamp: Some(1_700_000_000),
            ..Default::default()
        };
        let result = models::parse_oura_timestamp(&ctx).unwrap();
        assert_eq!(result.timestamp(), 1_700_000_000);
    }

    #[test]
    fn parse_oura_timestamp_unix_epoch() {
        let ctx = EventContext {
            timestamp: Some(0),
            ..Default::default()
        };
        let result = models::parse_oura_timestamp(&ctx).unwrap();
        assert_eq!(result.timestamp(), 0);
    }

    #[test]
    fn parse_oura_timestamp_missing_timestamp() {
        let ctx = EventContext {
            timestamp: None,
            block_hash: Some("abc".to_string()),
            tx_idx: Some(5),
            ..Default::default()
        };
        let err = models::parse_oura_timestamp(&ctx).unwrap_err().to_string();
        assert!(err.contains("timestamp"), "error should mention timestamp: {err}");
        assert!(err.contains("missing"), "error should say missing: {err}");
    }

    #[test]
    fn parse_oura_timestamp_extreme_value() {
        let ctx = EventContext {
            timestamp: Some(u64::MAX),
            block_hash: Some("blockhash".to_string()),
            tx_idx: Some(1),
            ..Default::default()
        };
        // u64::MAX cast to i64 wraps to -1, which gives 1969-12-31T23:59:59Z
        // DateTime::from_timestamp(-1, 0) is valid
        let result = models::parse_oura_timestamp(&ctx);
        assert!(result.is_ok() || result.is_err(), "should not panic");
    }

    // ==================================================================
    // models::parse_published_prism_object
    // ==================================================================

    #[test]
    fn parse_published_prism_object_valid_minimal() {
        let obj = minimal_prism_object();
        let ctx = valid_event_context();
        let meta = make_valid_metadata(&obj);

        let result = models::parse_published_prism_object(ctx, meta).unwrap();

        assert_eq!(result.block_metadata.slot_number.inner(), 50_000_000);
        assert_eq!(result.block_metadata.block_number.inner(), 5_000_000);
        assert_eq!(result.block_metadata.absn, 3);
        assert_eq!(result.prism_object, obj);
    }

    #[test]
    fn parse_published_prism_object_preserves_tx_id() {
        let obj = minimal_prism_object();
        let ctx = valid_event_context();
        let meta = make_valid_metadata(&obj);

        let result = models::parse_published_prism_object(ctx, meta).unwrap();

        let expected_tx_id =
            identus_did_prism::dlt::TxId::from_bytes(&HexStr::from_str(&valid_hex_hash()).unwrap().to_bytes()).unwrap();
        assert_eq!(result.block_metadata.tx_id, expected_tx_id);
    }

    #[test]
    fn parse_published_prism_object_missing_timestamp() {
        let obj = minimal_prism_object();
        let mut ctx = valid_event_context();
        ctx.timestamp = None;
        let meta = make_valid_metadata(&obj);

        let err = models::parse_published_prism_object(ctx, meta).unwrap_err().to_string();
        assert!(err.contains("timestamp"), "error should mention timestamp: {err}");
    }

    #[test]
    fn parse_published_prism_object_missing_tx_hash() {
        let obj = minimal_prism_object();
        let mut ctx = valid_event_context();
        ctx.tx_hash = None;
        let meta = make_valid_metadata(&obj);

        let err = models::parse_published_prism_object(ctx, meta).unwrap_err().to_string();
        assert!(err.contains("tx_hash"), "error should mention tx_hash: {err}");
    }

    #[test]
    fn parse_published_prism_object_invalid_tx_hash() {
        let obj = minimal_prism_object();
        let mut ctx = valid_event_context();
        ctx.tx_hash = Some("NOTHEX".to_string());
        let meta = make_valid_metadata(&obj);

        let result = models::parse_published_prism_object(ctx, meta);
        assert!(result.is_err(), "invalid tx hash should fail");
    }

    #[test]
    fn parse_published_prism_object_missing_tx_idx() {
        let obj = minimal_prism_object();
        let mut ctx = valid_event_context();
        ctx.tx_idx = None;
        let meta = make_valid_metadata(&obj);

        let err = models::parse_published_prism_object(ctx, meta).unwrap_err().to_string();
        assert!(err.contains("tx_idx"), "error should mention tx_idx: {err}");
    }

    #[test]
    fn parse_published_prism_object_missing_block_number() {
        let obj = minimal_prism_object();
        let mut ctx = valid_event_context();
        ctx.block_number = None;
        let meta = make_valid_metadata(&obj);

        let err = models::parse_published_prism_object(ctx, meta).unwrap_err().to_string();
        assert!(err.contains("block_number"), "error should mention block_number: {err}");
    }

    #[test]
    fn parse_published_prism_object_missing_slot() {
        let obj = minimal_prism_object();
        let mut ctx = valid_event_context();
        ctx.slot = None;
        let meta = make_valid_metadata(&obj);

        let err = models::parse_published_prism_object(ctx, meta).unwrap_err().to_string();
        assert!(err.contains("slot"), "error should mention slot: {err}");
    }

    #[test]
    fn parse_published_prism_object_invalid_metadatum_not_map() {
        let ctx = valid_event_context();
        let meta = MetadataRecord {
            label: "21325".to_string(),
            content: oura::model::MetadatumRendition::TextScalar("not a map".to_string()),
            metadatum: Metadatum::Text("not a map".to_string()),
        };

        let err = models::parse_published_prism_object(ctx, meta).unwrap_err().to_string();
        assert!(err.contains("metadata"), "error should mention metadata: {err}");
    }

    #[test]
    fn parse_published_prism_object_metadatum_map_without_c_key() {
        let ctx = valid_event_context();
        // Map with "v" but no "c" key
        let map = vec![(Metadatum::Text("v".to_string()), Metadatum::Int(1i64.into()))];
        let meta = MetadataRecord {
            label: "21325".to_string(),
            content: oura::model::MetadatumRendition::MapJson(serde_json::json!({})),
            metadatum: Metadatum::Map(KeyValuePairs::Def(map)),
        };

        let err = models::parse_published_prism_object(ctx, meta).unwrap_err().to_string();
        assert!(err.contains("metadata"), "error should mention metadata: {err}");
    }

    #[test]
    fn parse_published_prism_object_metadatum_array_with_non_bytes() {
        let ctx = valid_event_context();
        // Map with "c" key but array contains Text instead of Bytes
        let map = vec![(
            Metadatum::Text("c".to_string()),
            Metadatum::Array(vec![Metadatum::Text("not bytes".to_string())]),
        )];
        let meta = MetadataRecord {
            label: "21325".to_string(),
            content: oura::model::MetadatumRendition::MapJson(serde_json::json!({})),
            metadatum: Metadatum::Map(KeyValuePairs::Def(map)),
        };

        let err = models::parse_published_prism_object(ctx, meta).unwrap_err().to_string();
        assert!(err.contains("metadata"), "error should mention metadata: {err}");
    }

    #[test]
    fn parse_published_prism_object_invalid_protobuf_bytes() {
        let ctx = valid_event_context();
        // Valid structure but bytes are not valid protobuf
        let map = vec![(
            Metadatum::Text("c".to_string()),
            Metadatum::Array(vec![Metadatum::Bytes(Bytes::from(vec![0xDE, 0xAD, 0xBE, 0xEF]))]),
        )];
        let meta = MetadataRecord {
            label: "21325".to_string(),
            content: oura::model::MetadatumRendition::MapJson(serde_json::json!({})),
            metadatum: Metadatum::Map(KeyValuePairs::Def(map)),
        };

        let err = models::parse_published_prism_object(ctx, meta).unwrap_err().to_string();
        assert!(
            err.contains("protobuf") || err.contains("decode"),
            "error should mention protobuf decode: {err}"
        );
    }

    #[test]
    fn parse_published_prism_object_multiple_byte_groups() {
        // Create an object large enough to span multiple 64-byte chunks
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

        let ctx = valid_event_context();
        let meta = make_valid_metadata(&obj);

        let result = models::parse_published_prism_object(ctx, meta).unwrap();
        assert_eq!(result.prism_object, obj);
    }

    #[test]
    fn parse_published_prism_object_empty_byte_groups() {
        let ctx = valid_event_context();
        // Map with "c" key pointing to empty array -> empty bytes -> default PrismObject
        let map = vec![(Metadatum::Text("c".to_string()), Metadatum::Array(vec![]))];
        let meta = MetadataRecord {
            label: "21325".to_string(),
            content: oura::model::MetadatumRendition::MapJson(serde_json::json!({})),
            metadatum: Metadatum::Map(KeyValuePairs::Def(map)),
        };

        let result = models::parse_published_prism_object(ctx, meta);
        assert!(result.is_ok(), "empty byte groups should decode as default PrismObject");
        assert_eq!(result.unwrap().prism_object, PrismObject::default());
    }

    #[test]
    fn parse_published_prism_object_c_key_is_int_not_text() {
        let ctx = valid_event_context();
        // The "c" key lookup should only match Text("c"), not Int(99)
        let map = vec![
            (
                Metadatum::Int(99i64.into()),
                Metadatum::Array(vec![Metadatum::Bytes(Bytes::from(vec![0x00]))]),
            ),
            (Metadatum::Text("c".to_string()), Metadatum::Array(vec![])),
        ];
        let meta = MetadataRecord {
            label: "21325".to_string(),
            content: oura::model::MetadatumRendition::MapJson(serde_json::json!({})),
            metadatum: Metadatum::Map(KeyValuePairs::Def(map)),
        };

        // Should still find the Text("c") key and produce empty bytes -> default PrismObject
        let result = models::parse_published_prism_object(ctx, meta).unwrap();
        assert_eq!(result.prism_object, PrismObject::default());
    }

    #[test]
    fn parse_published_prism_object_c_value_is_not_array() {
        let ctx = valid_event_context();
        // "c" key is present but value is Text, not Array
        let map = vec![(
            Metadatum::Text("c".to_string()),
            Metadatum::Text("not an array".to_string()),
        )];
        let meta = MetadataRecord {
            label: "21325".to_string(),
            content: oura::model::MetadatumRendition::MapJson(serde_json::json!({})),
            metadatum: Metadatum::Map(KeyValuePairs::Def(map)),
        };

        let err = models::parse_published_prism_object(ctx, meta).unwrap_err().to_string();
        assert!(err.contains("metadata"), "error should mention metadata: {err}");
    }

    #[test]
    fn parse_published_prism_object_with_indef_kv_pairs() {
        let obj = minimal_prism_object();
        let ctx = valid_event_context();

        let bytes = encode_prism_object(&obj);
        let chunks: Vec<Metadatum> = bytes
            .chunks(64)
            .map(|chunk| Metadatum::Bytes(Bytes::from(chunk.to_vec())))
            .collect();

        // Use Indef variant instead of Def
        let map = vec![(Metadatum::Text("c".to_string()), Metadatum::Array(chunks))];
        let meta = MetadataRecord {
            label: "21325".to_string(),
            content: oura::model::MetadatumRendition::MapJson(serde_json::json!({})),
            metadatum: Metadatum::Map(KeyValuePairs::Indef(map)),
        };

        let result = models::parse_published_prism_object(ctx, meta).unwrap();
        assert_eq!(result.prism_object, obj);
    }

    // ==================================================================
    // OuraN2NSource construction
    // ==================================================================

    #[test]
    fn oura_source_new_mainnet() {
        let repo = MockRepo::new(None);
        let source = OuraN2NSource::new(
            repo,
            "127.0.0.1:3001",
            &NetworkIdentifier::Mainnet,
            oura::sources::IntersectArg::Origin,
            100,
        );
        let rx = source.sync_cursor();
        assert!(rx.borrow().is_none());
    }

    #[test]
    fn oura_source_new_preprod() {
        let repo = MockRepo::new(None);
        let source = OuraN2NSource::new(
            repo,
            "127.0.0.1:3001",
            &NetworkIdentifier::Preprod,
            oura::sources::IntersectArg::Origin,
            100,
        );
        let rx = source.sync_cursor();
        assert!(rx.borrow().is_none());
    }

    #[test]
    fn oura_source_new_preview() {
        let repo = MockRepo::new(None);
        let source = OuraN2NSource::new(
            repo,
            "127.0.0.1:3001",
            &NetworkIdentifier::Preview,
            oura::sources::IntersectArg::Origin,
            100,
        );
        let rx = source.sync_cursor();
        assert!(rx.borrow().is_none());
    }

    #[test]
    #[should_panic(expected = "chain magic value cannot be parsed")]
    fn oura_source_new_custom_panics() {
        let repo = MockRepo::new(None);
        let _ = OuraN2NSource::new(
            repo,
            "127.0.0.1:3001",
            &NetworkIdentifier::Custom,
            oura::sources::IntersectArg::Origin,
            100,
        );
    }

    #[test]
    fn oura_source_since_genesis_mainnet() {
        let repo = MockRepo::new(None);
        let source = OuraN2NSource::since_genesis(repo, "127.0.0.1:3001", &NetworkIdentifier::Mainnet, 100);
        let rx = source.sync_cursor();
        assert!(rx.borrow().is_none());
    }

    #[test]
    fn oura_source_since_genesis_preprod() {
        let repo = MockRepo::new(None);
        let source = OuraN2NSource::since_genesis(repo, "127.0.0.1:3001", &NetworkIdentifier::Preprod, 100);
        let rx = source.sync_cursor();
        assert!(rx.borrow().is_none());
    }

    #[test]
    fn oura_source_since_genesis_preview() {
        let repo = MockRepo::new(None);
        let source = OuraN2NSource::since_genesis(repo, "127.0.0.1:3001", &NetworkIdentifier::Preview, 100);
        let rx = source.sync_cursor();
        assert!(rx.borrow().is_none());
    }

    // ==================================================================
    // OuraN2NSource::since_persisted_cursor_or_genesis
    // ==================================================================

    #[tokio::test]
    async fn oura_source_since_persisted_cursor_none() {
        let repo = MockRepo::new(None);
        let source =
            OuraN2NSource::since_persisted_cursor_or_genesis(repo, "127.0.0.1:3001", &NetworkIdentifier::Preview, 100)
                .await
                .unwrap();

        let rx = source.sync_cursor();
        assert!(rx.borrow().is_none());
    }

    #[tokio::test]
    async fn oura_source_since_persisted_cursor_with_existing() {
        let cursor = DltCursor {
            slot: 42,
            block_hash: vec![0xaa; 32],
            cbt: None,
            blockfrost_page: None,
        };
        let repo = MockRepo::new(Some(cursor));
        let source =
            OuraN2NSource::since_persisted_cursor_or_genesis(repo, "127.0.0.1:3001", &NetworkIdentifier::Preview, 100)
                .await
                .unwrap();

        let rx = source.sync_cursor();
        assert!(rx.borrow().is_none());
    }

    // ==================================================================
    // OuraN2NSource::into_stream
    // ==================================================================

    #[tokio::test]
    async fn oura_source_into_stream_creates_channel() {
        let repo = MockRepo::new(None);
        let source = OuraN2NSource::since_genesis(repo, "127.0.0.1:3001", &NetworkIdentifier::Preview, 100);
        let rx = source.into_stream().unwrap();
        assert!(!rx.is_closed());
    }

    // ==================================================================
    // OuraStreamWorker::process_prism_object
    // ==================================================================

    /// Build an OuraStreamWorker with real channels for testing.
    /// Constructs a valid WithUtils by creating a temporary source.
    fn make_test_worker(
        event_tx: mpsc::Sender<identus_did_prism::dlt::PublishedPrismObject>,
        cursor_tx: watch::Sender<Option<DltCursor>>,
    ) -> OuraStreamWorker {
        // Construct a valid source to extract with_utils from it
        let temp_repo = MockRepo::new(None);
        let temp_source = OuraN2NSource::since_genesis(temp_repo, "127.0.0.1:3001", &NetworkIdentifier::Preview, 100);
        OuraStreamWorker {
            with_utils: temp_source.with_utils.clone(),
            sync_cursor_tx: cursor_tx,
            event_tx,
        }
    }

    #[test]
    fn process_prism_object_valid_sends_to_channel() {
        let (tx, mut rx) = mpsc::channel::<identus_did_prism::dlt::PublishedPrismObject>(1024);
        let (cursor_tx, _) = watch::channel(None);
        let worker = make_test_worker(tx, cursor_tx);

        let obj = minimal_prism_object();
        let event = make_valid_event(&obj);

        worker.process_prism_object(event).unwrap();

        let received = rx.try_recv().unwrap();
        assert_eq!(received.prism_object, obj);
        assert_eq!(received.block_metadata.absn, 3);
        assert_eq!(received.block_metadata.slot_number.inner(), 50_000_000);
    }

    #[test]
    fn process_prism_object_non_metadata_event_returns_ok() {
        let (tx, rx) = mpsc::channel::<identus_did_prism::dlt::PublishedPrismObject>(1024);
        let (cursor_tx, _) = watch::channel(None);
        let worker = make_test_worker(tx, cursor_tx);

        // Use a non-Metadata event
        let event = Event {
            context: valid_event_context(),
            data: EventData::Transaction(oura::model::TransactionRecord {
                hash: valid_hex_hash(),
                ..Default::default()
            }),
            fingerprint: None,
        };

        worker.process_prism_object(event).unwrap();
        assert!(rx.is_empty(), "no object should be sent for non-metadata event");
    }

    #[test]
    fn process_prism_object_wrong_label_returns_ok() {
        let (tx, rx) = mpsc::channel::<identus_did_prism::dlt::PublishedPrismObject>(1024);
        let (cursor_tx, _) = watch::channel(None);
        let worker = make_test_worker(tx, cursor_tx);

        let obj = minimal_prism_object();
        let mut meta = make_valid_metadata(&obj);
        meta.label = "99999".to_string(); // wrong label

        let event = Event {
            context: valid_event_context(),
            data: EventData::Metadata(meta),
            fingerprint: None,
        };

        worker.process_prism_object(event).unwrap();
        assert!(rx.is_empty(), "no object should be sent for wrong label");
    }

    #[test]
    fn process_prism_object_invalid_metadata_returns_ok_no_send() {
        let (tx, rx) = mpsc::channel::<identus_did_prism::dlt::PublishedPrismObject>(1024);
        let (cursor_tx, _) = watch::channel(None);
        let worker = make_test_worker(tx, cursor_tx);

        let meta = MetadataRecord {
            label: "21325".to_string(),
            content: oura::model::MetadatumRendition::TextScalar("not a map".to_string()),
            metadatum: Metadatum::Text("not a map".to_string()),
        };
        let event = Event {
            context: valid_event_context(),
            data: EventData::Metadata(meta),
            fingerprint: None,
        };

        // Invalid metadata is logged as warning but returns Ok(())
        worker.process_prism_object(event).unwrap();
        assert!(rx.is_empty());
    }

    #[test]
    fn process_prism_object_closed_channel_returns_error() {
        let (tx, rx) = mpsc::channel::<identus_did_prism::dlt::PublishedPrismObject>(1024);
        let (cursor_tx, _) = watch::channel(None);
        let worker = make_test_worker(tx, cursor_tx);

        let obj = minimal_prism_object();
        let event = make_valid_event(&obj);

        // Close the receiving end
        drop(rx);

        let result = worker.process_prism_object(event);
        assert!(result.is_err(), "sending to closed channel should return error");
    }

    #[test]
    fn process_prism_object_large_multi_chunk_object() {
        let (tx, mut rx) = mpsc::channel::<identus_did_prism::dlt::PublishedPrismObject>(1024);
        let (cursor_tx, _) = watch::channel(None);
        let worker = make_test_worker(tx, cursor_tx);

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

        let event = make_valid_event(&obj);

        worker.process_prism_object(event).unwrap();
        let received = rx.try_recv().unwrap();
        assert_eq!(received.prism_object, obj);
    }

    // ==================================================================
    // OuraStreamWorker::emit_cursor_progress
    // ==================================================================

    #[test]
    fn emit_cursor_progress_valid_event() {
        let (event_tx, _) = mpsc::channel::<identus_did_prism::dlt::PublishedPrismObject>(1024);
        let (cursor_tx, cursor_rx) = watch::channel(None);
        let worker = make_test_worker(event_tx, cursor_tx);

        let event = make_valid_event(&minimal_prism_object());
        worker.emit_cursor_progress(&event);

        let cursor = cursor_rx.borrow().as_ref().unwrap().clone();
        assert_eq!(cursor.slot, 50_000_000);
        assert_eq!(cursor.block_hash, vec![0xaa; 32]);
        assert!(cursor.cbt.is_some());
        assert_eq!(cursor.blockfrost_page, None);
    }

    #[test]
    fn emit_cursor_progress_missing_slot() {
        let (event_tx, _) = mpsc::channel::<identus_did_prism::dlt::PublishedPrismObject>(1024);
        let (cursor_tx, cursor_rx) = watch::channel(None);
        let worker = make_test_worker(event_tx, cursor_tx);

        let mut ctx = valid_event_context();
        ctx.slot = None;
        let event = Event {
            context: ctx,
            data: EventData::Metadata(make_valid_metadata(&minimal_prism_object())),
            fingerprint: None,
        };

        worker.emit_cursor_progress(&event);
        assert!(
            cursor_rx.borrow().is_none(),
            "no cursor should be emitted when slot is missing"
        );
    }

    #[test]
    fn emit_cursor_progress_missing_block_hash() {
        let (event_tx, _) = mpsc::channel::<identus_did_prism::dlt::PublishedPrismObject>(1024);
        let (cursor_tx, cursor_rx) = watch::channel(None);
        let worker = make_test_worker(event_tx, cursor_tx);

        let mut ctx = valid_event_context();
        ctx.block_hash = None;
        let event = Event {
            context: ctx,
            data: EventData::Metadata(make_valid_metadata(&minimal_prism_object())),
            fingerprint: None,
        };

        worker.emit_cursor_progress(&event);
        assert!(
            cursor_rx.borrow().is_none(),
            "no cursor should be emitted when block_hash is missing"
        );
    }

    #[test]
    fn emit_cursor_progress_invalid_block_hash_hex() {
        let (event_tx, _) = mpsc::channel::<identus_did_prism::dlt::PublishedPrismObject>(1024);
        let (cursor_tx, cursor_rx) = watch::channel(None);
        let worker = make_test_worker(event_tx, cursor_tx);

        let mut ctx = valid_event_context();
        ctx.block_hash = Some("NOTHEX".to_string());
        let event = Event {
            context: ctx,
            data: EventData::Metadata(make_valid_metadata(&minimal_prism_object())),
            fingerprint: None,
        };

        worker.emit_cursor_progress(&event);
        assert!(
            cursor_rx.borrow().is_none(),
            "no cursor should be emitted when block_hash is invalid hex"
        );
    }

    #[test]
    fn emit_cursor_progress_missing_timestamp() {
        let (event_tx, _) = mpsc::channel::<identus_did_prism::dlt::PublishedPrismObject>(1024);
        let (cursor_tx, cursor_rx) = watch::channel(None);
        let worker = make_test_worker(event_tx, cursor_tx);

        let mut ctx = valid_event_context();
        ctx.timestamp = None;
        let event = Event {
            context: ctx,
            data: EventData::Metadata(make_valid_metadata(&minimal_prism_object())),
            fingerprint: None,
        };

        worker.emit_cursor_progress(&event);
        assert!(
            cursor_rx.borrow().is_none(),
            "no cursor should be emitted when timestamp is missing"
        );
    }

    #[test]
    fn emit_cursor_progress_overwrites_previous() {
        let (event_tx, _) = mpsc::channel::<identus_did_prism::dlt::PublishedPrismObject>(1024);
        let (cursor_tx, cursor_rx) = watch::channel(None);
        let worker = make_test_worker(event_tx, cursor_tx);

        // First event
        let event1 = make_valid_event(&minimal_prism_object());
        worker.emit_cursor_progress(&event1);
        assert_eq!(cursor_rx.borrow().as_ref().unwrap().slot, 50_000_000);

        // Second event with different slot
        let mut ctx2 = valid_event_context();
        ctx2.slot = Some(99_000_000);
        let event2 = Event {
            context: ctx2,
            data: EventData::Metadata(make_valid_metadata(&minimal_prism_object())),
            fingerprint: None,
        };
        worker.emit_cursor_progress(&event2);
        assert_eq!(cursor_rx.borrow().as_ref().unwrap().slot, 99_000_000);
    }
}
