use std::str::FromStr;
use std::sync::Arc;
use std::sync::mpsc::RecvTimeoutError;

use identus_apollo::hex::HexStr;
use identus_did_prism::dlt::{DltCursor, PublishedPrismObject};
use identus_did_prism::location;
use oura::model::{Event, EventData};
use oura::pipelining::{SourceProvider, StageReceiver};
use oura::sources::n2n::Config;
use oura::sources::{AddressArg, IntersectArg, MagicArg, PointArg};
use oura::utils::{ChainWellKnownInfo, Utils, WithUtils};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;

use super::error::DltError;
use crate::DltSource;
use crate::dlt::NetworkIdentifier;
use crate::repo::DltCursorRepo;

mod model {
    use chrono::{DateTime, Utc};
    use identus_did_prism::dlt::{BlockMetadata, PublishedPrismObject};
    use identus_did_prism::proto::PrismObject;
    use oura::model::{EventContext, MetadataRecord};
    use prost::Message;
    use serde::{Deserialize, Serialize};

    use crate::dlt::error::MetadataReadError;

    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct MetadataEvent {
        pub context: MetadataContext,
        pub metadata: Metadata,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct MetadataContext {
        pub block_hash: String,
        pub block_number: u64,
        pub slot: u64,
        pub timestamp: i64,
        pub tx_hash: String,
        pub tx_idx: u32,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Metadata {
        pub label: String,
        pub map_json: MetadataMapJson,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct MetadataMapJson {
        pub c: Vec<String>,
        pub v: u64,
    }

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

    pub fn parse_oura_event(
        context: EventContext,
        metadata: MetadataRecord,
    ) -> Result<PublishedPrismObject, MetadataReadError> {
        // parse metadata
        let block_hash = &context.block_hash;
        let tx_idx = context.tx_idx;
        let timestamp = parse_oura_timestamp(&context)?;
        let block_metadata = BlockMetadata {
            cbt: timestamp,
            absn: context.tx_idx.ok_or(MetadataReadError::MissingBlockProperty {
                block_hash: block_hash.clone(),
                tx_idx,
                name: "tx_idx",
            })? as u32,
            block_number: context.block_number.ok_or(MetadataReadError::MissingBlockProperty {
                block_hash: block_hash.clone(),
                tx_idx,
                name: "block_number",
            })?,
            slot_number: context.slot.ok_or(MetadataReadError::MissingBlockProperty {
                block_hash: block_hash.clone(),
                tx_idx,
                name: "slot",
            })?,
        };

        // parse prism_block
        let byte_group = match metadata.metadadum {
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
            source: "Metadata is not a valid type".to_string().into(),
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
                block_hash: block_hash.clone(),
                tx_idx,
            })?;

        Ok(PublishedPrismObject {
            block_metadata,
            prism_object,
        })
    }
}

impl NetworkIdentifier {
    fn magic_args(&self) -> MagicArg {
        let chain_magic = MagicArg::from_str(&self.to_string());
        chain_magic.expect("The chain magic value cannot be parsed")
    }

    fn chain_wellknown_info(&self) -> ChainWellKnownInfo {
        match self {
            NetworkIdentifier::Mainnet => ChainWellKnownInfo::mainnet(),
            NetworkIdentifier::Preprod => ChainWellKnownInfo::preprod(),
            NetworkIdentifier::Preview => ChainWellKnownInfo::preview(),
        }
    }
}

pub struct OuraN2NSource<Store: DltCursorRepo + Send + 'static> {
    with_utils: WithUtils<Config>,
    store: Store,
    cursor_tx: tokio::sync::watch::Sender<Option<DltCursor>>,
}

impl<E, Store: DltCursorRepo<Error = E> + Send + 'static> OuraN2NSource<Store> {
    pub fn since_genesis(store: Store, remote_addr: &str, chain: &NetworkIdentifier) -> Self {
        let intersect = match chain {
            NetworkIdentifier::Mainnet => oura::sources::IntersectArg::Point(PointArg(
                71482683,
                "f3fd56f7e390d4e45d06bb797d83b7814b1d32c2112bc997779e34de1579fa7d".to_string(),
            )),
            NetworkIdentifier::Preprod => oura::sources::IntersectArg::Point(PointArg(
                10718532,
                "cb95a5effb12871b69c27c184ffb1355e6208c4071956df67248bad1cc329ca4".to_string(),
            )),
            _ => oura::sources::IntersectArg::Origin,
        };
        Self::new(store, remote_addr, chain, intersect)
    }

    pub async fn since_persisted_cursor_or_genesis(
        store: Store,
        remote_addr: &str,
        chain: &NetworkIdentifier,
    ) -> Result<Self, E> {
        let cursor = store.get_cursor().await?;
        match cursor {
            Some(cursor) => {
                let blockhash_hex = HexStr::from(cursor.block_hash).to_string();
                tracing::info!(
                    "Persisted cursor found, starting syncing from ({}, {})",
                    cursor.slot,
                    blockhash_hex
                );
                let intersect = oura::sources::IntersectArg::Point(PointArg(cursor.slot, blockhash_hex));
                Ok(Self::new(store, remote_addr, chain, intersect))
            }
            None => {
                tracing::info!("Persisted cursor not found, staring syncing from PRISM genesis slot");
                Ok(Self::since_genesis(store, remote_addr, chain))
            }
        }
    }

    pub fn new(store: Store, remote_addr: &str, chain: &NetworkIdentifier, intersect: IntersectArg) -> Self {
        #[allow(deprecated)]
        let config = Config {
            address: AddressArg(oura::sources::BearerKind::Tcp, remote_addr.to_string()),
            magic: Some(chain.magic_args()),
            since: None,
            intersect: Some(intersect),
            well_known: None,
            mapper: Default::default(),
            min_depth: 112,
            retry_policy: Some(oura::sources::RetryPolicy {
                chainsync_max_retries: 0,
                chainsync_max_backoff: 60,
                connection_max_retries: 0,
                connection_max_backoff: 60,
            }),
            finalize: None,
        };
        let utils = Utils::new(chain.chain_wellknown_info());
        let with_utils = WithUtils::new(config, Arc::new(utils));
        let (cursor_tx, _) = tokio::sync::watch::channel::<Option<DltCursor>>(None);
        Self {
            with_utils,
            store,
            cursor_tx,
        }
    }

    pub fn cursor_receiver(&self) -> tokio::sync::watch::Receiver<Option<DltCursor>> {
        self.cursor_tx.subscribe()
    }
}

impl<Store: DltCursorRepo + Send> DltSource for OuraN2NSource<Store> {
    fn receiver(self) -> Result<Receiver<PublishedPrismObject>, String> {
        let (event_tx, rx) = tokio::sync::mpsc::channel::<PublishedPrismObject>(1024);

        let cursor_persist_worker = CursorPersistWorker {
            cursor_rx: self.cursor_tx.subscribe(),
            store: self.store,
        };

        let oura_stream_worker = OuraStreamWorker {
            with_utils: self.with_utils,
            cursor_tx: self.cursor_tx,
            event_tx,
        };

        oura_stream_worker.spawn();
        cursor_persist_worker.spawn();

        Ok(rx)
    }
}

struct OuraStreamWorker {
    with_utils: WithUtils<Config>,
    cursor_tx: tokio::sync::watch::Sender<Option<DltCursor>>,
    event_tx: Sender<PublishedPrismObject>,
}

impl OuraStreamWorker {
    fn spawn(self) -> std::thread::JoinHandle<Result<(), DltError>> {
        const RESTART_DELAY: std::time::Duration = std::time::Duration::from_secs(10);
        std::thread::spawn(move || {
            loop {
                let with_utils = self.build_with_util();
                tracing::info!("Bootstraping oura pipeline thread");
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

                tracing::error!(
                    "Oura pipeline terminated. Restarting in {} seconds",
                    RESTART_DELAY.as_secs()
                );
                std::thread::sleep(RESTART_DELAY);
            }
        })
    }

    /// Construct WithUtils instance from the last event sent to persist worker.
    fn build_with_util(&self) -> WithUtils<Config> {
        let mut owned_with_utils = self.with_utils.clone();
        let rx = self.cursor_tx.subscribe();
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
            let handle_result = match receiver.recv_timeout(TIMEOUT) {
                Ok(event) => {
                    let handle_result = self.handle_prism_event(event.clone());
                    self.persist_cursor(&event);
                    handle_result
                }
                Err(RecvTimeoutError::Timeout) => Err(DltError::EventRecvTimeout { location: location!() }),
                Err(RecvTimeoutError::Disconnected) => Err(DltError::Disconnected { location: location!() }),
            };
            if let Err(e) = handle_result {
                tracing::error!("Error handling event from oura source");
                let report = std::error::Report::new(&e).pretty(true);
                tracing::error!("{}", report);
                return e;
            }
        }
    }

    fn persist_cursor(&self, event: &Event) {
        let Some(slot) = event.context.slot else {
            return;
        };
        let Some(block_hash_hex) = &event.context.block_hash else {
            return;
        };
        let Ok(block_hash) = HexStr::from_str(block_hash_hex) else {
            return;
        };
        let Ok(timestamp) = model::parse_oura_timestamp(&event.context) else {
            return;
        };
        let cursor = DltCursor {
            slot,
            block_hash: block_hash.to_bytes(),
            cbt: Some(timestamp),
        };
        let _ = self.cursor_tx.send(Some(cursor));
    }

    fn handle_prism_event(&self, event: Event) -> Result<(), DltError> {
        let EventData::Metadata(meta) = event.data else {
            return Ok(());
        };
        if meta.label != "21325" {
            return Ok(());
        }

        let context = event.context;
        tracing::info!(
            "Detected a new prism_block on slot ({}, {})",
            context.slot.unwrap_or_default(),
            context.block_hash.as_deref().unwrap_or_default(),
        );

        let parsed_prism_object = self::model::parse_oura_event(context, meta);
        match parsed_prism_object {
            Ok(prism_object) => self
                .event_tx
                .blocking_send(prism_object)
                .map_err(|e| DltError::EventHandling {
                    source: e.to_string().into(),
                    location: location!(),
                })?,
            Err(e) => {
                // TODO: add debug level error report
                tracing::warn!("Unable to parse oura event into PrismObject. ({})", e);
            }
        }

        Ok(())
    }
}

struct CursorPersistWorker<Store: DltCursorRepo> {
    cursor_rx: tokio::sync::watch::Receiver<Option<DltCursor>>,
    store: Store,
}

impl<Store: DltCursorRepo + Send + 'static> CursorPersistWorker<Store> {
    fn spawn(mut self) -> JoinHandle<Result<(), DltError>> {
        const DELAY: tokio::time::Duration = tokio::time::Duration::from_secs(60);
        tracing::info!("Spawn cursor persist worker with {:?} interval", DELAY);
        tokio::spawn(async move {
            loop {
                let recv_result = self.cursor_rx.changed().await;
                tokio::time::sleep(DELAY).await;

                if let Err(e) = recv_result {
                    tracing::error!("Error getting cursor to persist: {}", e);
                }

                let cursor = self.cursor_rx.borrow_and_update().clone();
                let Some(cursor) = cursor else { continue };
                tracing::info!(
                    "Persisting cursor on slot ({}, {})",
                    cursor.slot,
                    HexStr::from(cursor.block_hash.as_slice()).to_string(),
                );

                let persist_result = self.store.set_cursor(cursor).await;
                if let Err(e) = persist_result {
                    tracing::error!("Error persisting cursor: {}", e);
                }
            }
        })
    }
}
