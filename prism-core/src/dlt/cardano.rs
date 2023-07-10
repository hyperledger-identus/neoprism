use self::model::MetadataEvent;
use super::{DltSource, PublishedAtalaObject};
use crate::{
    crypto::codec::HexStr,
    prelude::StdError,
    store::{CursorStoreError, DltCursor, DltCursorStore},
};
use bytes::Bytes;
use core::panic;
use oura::{
    model::{Event, EventData},
    pipelining::{SourceProvider, StageReceiver},
    sources::{n2n::Config, AddressArg, IntersectArg, MagicArg, PointArg},
    utils::{ChainWellKnownInfo, Utils, WithUtils},
};
use std::{
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    task::JoinHandle,
};

pub struct OuraFileSource {
    path: PathBuf,
}

impl OuraFileSource {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    fn parse(self) -> Result<Vec<PublishedAtalaObject>, Box<dyn std::error::Error>> {
        let lines = std::fs::read_to_string(self.path)?;
        let mut atala_objects = Vec::with_capacity(lines.len());
        for line in lines.trim().split('\n') {
            let metadata_event: MetadataEvent = serde_json::from_str(line)?;
            let atala_object: PublishedAtalaObject = metadata_event.try_into()?;
            atala_objects.push(atala_object)
        }
        Ok(atala_objects)
    }
}

impl DltSource for OuraFileSource {
    fn receiver(self) -> Result<Receiver<PublishedAtalaObject>, String> {
        let (tx, rx) = mpsc::channel(2048);

        tokio::spawn(async move {
            let atala_objects = match self.parse() {
                Ok(atala_objects) => atala_objects,
                Err(e) => {
                    log::error!("Error parsing OuraFileSource: {}", e);
                    return;
                }
            };
            for atala_object in atala_objects {
                if let Err(e) = tx.send(atala_object).await {
                    log::error!("Error sending AtalaObject from OuraFileSource: {}", e);
                    break;
                }
            }
        });

        Ok(rx)
    }
}

mod model {
    use crate::{
        dlt::{BlockMetadata, PublishedAtalaObject},
        proto::AtalaObject,
    };
    use bytes::BytesMut;
    use chrono::{DateTime, TimeZone, Utc};
    use oura::model::{EventContext, MetadataRecord};
    use prost::Message;
    use serde::{Deserialize, Serialize};

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

    #[derive(Debug, thiserror::Error)]
    pub enum ConversionError {
        #[error("hex conversion error: {0}")]
        HexDecodeError(#[from] hex::FromHexError),
        #[error("protobuf decode error: {0}")]
        ProtoDecodeError(#[from] prost::DecodeError),
        #[error("metadata malformed: {0}")]
        MalformedMetadata(String),
    }

    impl TryFrom<MetadataEvent> for AtalaObject {
        type Error = ConversionError;

        fn try_from(value: MetadataEvent) -> Result<Self, Self::Error> {
            let published_atala_object = PublishedAtalaObject::try_from(value)?;
            Ok(published_atala_object.atala_object)
        }
    }

    impl TryFrom<MetadataEvent> for PublishedAtalaObject {
        type Error = ConversionError;

        fn try_from(value: MetadataEvent) -> Result<Self, Self::Error> {
            let hex_group = value.metadata.map_json.c;
            let mut buf = BytesMut::with_capacity(64 * hex_group.len());

            for hex in hex_group {
                let b = hex::decode(hex)?;
                buf.extend(b);
            }

            let bytes = buf.freeze();
            let atala_object = AtalaObject::decode(bytes)?;
            let timestamp: DateTime<Utc> = Utc
                .timestamp_opt(value.context.timestamp, 0)
                .single()
                .ok_or_else(|| {
                ConversionError::ProtoDecodeError(prost::DecodeError::new("timestamp is not valid"))
            })?;
            let block_metadata = BlockMetadata {
                slot_number: value.context.slot,
                cbt: timestamp,
                absn: value.context.tx_idx,
                block_number: value.context.block_number,
            };
            let published_atala_object = PublishedAtalaObject {
                block_metadata,
                atala_object,
            };
            Ok(published_atala_object)
        }
    }

    pub fn parse_oura_event(
        context: EventContext,
        metadata: MetadataRecord,
    ) -> Result<PublishedAtalaObject, ConversionError> {
        let hex_group = match metadata.content {
            oura::model::MetadatumRendition::MapJson(json) => {
                let meta = serde_json::from_value::<MetadataMapJson>(json)
                    .map_err(|e| ConversionError::MalformedMetadata(e.to_string()))?;
                meta.c
            }
            _ => Err(ConversionError::MalformedMetadata(
                "Metadata is not a MapJson type".to_string(),
            ))?,
        };
        let mut buf = BytesMut::with_capacity(64 * hex_group.len());

        for hex in hex_group {
            let b = hex::decode(hex)?;
            buf.extend(b);
        }

        let bytes = buf.freeze();
        let atala_object = AtalaObject::decode(bytes)?;
        let timestamp = context.timestamp.ok_or(ConversionError::MalformedMetadata(
            "Timestamp must be present in Cardano metadata".to_string(),
        ))? as i64;
        let timestamp: DateTime<Utc> =
            Utc.timestamp_opt(timestamp, 0).single().ok_or_else(|| {
                ConversionError::ProtoDecodeError(prost::DecodeError::new("timestamp is not valid"))
            })?;
        let block_metadata = BlockMetadata {
            cbt: timestamp,
            absn: context.tx_idx.ok_or(ConversionError::MalformedMetadata(
                "Transaction index must be present in Cardano metadata".to_string(),
            ))? as u32,
            block_number: context
                .block_number
                .ok_or(ConversionError::MalformedMetadata(
                    "Block number must be present in Cardano metadata".to_string(),
                ))?,
            slot_number: context.slot.ok_or(ConversionError::MalformedMetadata(
                "Slot number must be present in Cardano metadata".to_string(),
            ))?,
        };
        let published_atala_object = PublishedAtalaObject {
            block_metadata,
            atala_object,
        };
        Ok(published_atala_object)
    }
}

pub enum NetworkIdentifier {
    Mainnet,
}

impl NetworkIdentifier {
    fn magic_args(&self) -> MagicArg {
        let chain_magic = match self {
            NetworkIdentifier::Mainnet => MagicArg::from_str("mainnet"),
        };
        chain_magic.expect("The chain magic value cannot be parsed")
    }

    fn chain_wellknown_info(&self) -> ChainWellKnownInfo {
        match self {
            NetworkIdentifier::Mainnet => ChainWellKnownInfo::mainnet(),
        }
    }
}

pub struct OuraN2NSource<Store: DltCursorStore + Send + 'static> {
    with_utils: WithUtils<Config>,
    store: Store,
}

impl<Store: DltCursorStore + Send + Sync + 'static> OuraN2NSource<Store> {
    // FIXME: 71482683 was about the slot that first AtalaBlock was observed on mainnet.
    // How can we support multiple network and define genesis slot / block?
    pub fn since_genesis(store: Store, remote_addr: &str, chain: &NetworkIdentifier) -> Self {
        let intersect = oura::sources::IntersectArg::Point(PointArg(
            71482683,
            "f3fd56f7e390d4e45d06bb797d83b7814b1d32c2112bc997779e34de1579fa7d".to_string(),
        ));
        Self::new(store, remote_addr, chain, intersect)
    }

    pub async fn since_persisted_cursor_or_genesis(
        store: Store,
        remote_addr: &str,
        chain: &NetworkIdentifier,
    ) -> Result<Self, CursorStoreError> {
        let cursor = store.get_cursor().await?;
        match cursor {
            Some(cursor) => {
                let intersect = oura::sources::IntersectArg::Point(PointArg(
                    cursor.slot,
                    HexStr::from(Bytes::from(cursor.block_hash)).to_string(),
                ));
                Ok(Self::new(store, remote_addr, chain, intersect))
            }
            None => {
                log::info!("Persisted cursor not found, staring syncing from PRISM genesis block");
                Ok(Self::since_genesis(store, remote_addr, chain))
            }
        }
    }

    pub fn new(
        store: Store,
        remote_addr: &str,
        chain: &NetworkIdentifier,
        intersect: IntersectArg,
    ) -> Self {
        #[allow(deprecated)]
        let config = Config {
            address: AddressArg(oura::sources::BearerKind::Tcp, remote_addr.to_string()),
            magic: Some(NetworkIdentifier::Mainnet.magic_args()),
            since: None,
            intersect: Some(intersect),
            well_known: None,
            mapper: Default::default(),
            min_depth: 112,
            retry_policy: None,
            finalize: None,
        };
        let utils = Utils::new(chain.chain_wellknown_info());
        let with_utils = WithUtils::new(config, Arc::new(utils));
        Self { with_utils, store }
    }
}

impl<Store: DltCursorStore + Send + Sync + 'static> DltSource for OuraN2NSource<Store> {
    fn receiver(self) -> Result<Receiver<PublishedAtalaObject>, String> {
        let (event_tx, rx) = tokio::sync::mpsc::channel::<PublishedAtalaObject>(1024);
        let (cursor_tx, cursor_rx) = tokio::sync::watch::channel::<Option<DltCursor>>(None);

        let oura_stream_worker = OuraStreamWorker {
            with_utils: self.with_utils,
            cursor_tx,
            event_tx,
        };

        let cursor_persist_worker = CursorPersistWorker {
            cursor_rx,
            store: self.store,
        };

        // TODO: improve error propagation
        let handle_1 = oura_stream_worker.spawn();
        let handle_2 = cursor_persist_worker.spawn();

        Ok(rx)
    }
}

struct OuraStreamWorker {
    with_utils: WithUtils<Config>,
    cursor_tx: tokio::sync::watch::Sender<Option<DltCursor>>,
    event_tx: Sender<PublishedAtalaObject>,
}

impl OuraStreamWorker {
    fn spawn(self) -> std::thread::JoinHandle<Result<(), StdError>> {
        std::thread::spawn(move || loop {
            log::info!("Bootstraping oura pipeline thread");
            let (oura_handle, oura_rx) = self.with_utils.bootstrap().map_err(|e| e.to_string())?;
            let exit_err = self.stream_loop(oura_rx);
            log::error!("Oura pipeline terminated. Retry in 10 seconds");
            std::thread::sleep(std::time::Duration::from_secs(10));
        })
    }

    fn stream_loop(&self, receiver: StageReceiver) -> StdError {
        let timeout = std::time::Duration::from_secs(300);
        loop {
            let received_event = receiver.recv_timeout(timeout);
            let handle_result = match received_event {
                Ok(event) => {
                    self.persist_cursor(&event);
                    self.handle_atala_event(event.clone())
                }
                Err(e) => Err(e.into()),
            };
            if let Err(e) = handle_result {
                log::error!("Error handling event from oura source. {}", e);
                return e;
            }
        }
    }

    fn persist_cursor(&self, event: &Event) {
        let Some(slot) = event.context.slot else { return };
        let Some(block_hash_hex) = &event.context.block_hash else { return };
        let block_hash = HexStr::from_str(block_hash_hex)
            .unwrap_or_else(|_| panic!("Invalid hex string for block_hash on slot {}", slot))
            .as_bytes()
            .to_owned();
        let cursor = DltCursor { slot, block_hash };
        let _ = self.cursor_tx.send(Some(cursor));
    }

    fn handle_atala_event(&self, event: Event) -> Result<(), StdError> {
        let EventData::Metadata(meta) = event.data else { return Ok(()) };
        if meta.label != "21325" {
            return Ok(());
        }

        let context = event.context;
        log::info!(
            "Detect a new atala_block on slot ({}, {})",
            context.slot.unwrap_or_default(),
            context.block_hash.as_deref().unwrap_or_default(),
        );

        let parsed_atala_object = self::model::parse_oura_event(context, meta);
        match parsed_atala_object {
            Ok(atala_object) => self
                .event_tx
                .blocking_send(atala_object)
                .map_err(|e| e.to_string())?,
            Err(e) => {
                log::warn!("Unable to parse metadata into AtalaObject. ({})", e);
            }
        }

        Ok(())
    }
}

struct CursorPersistWorker<Store: DltCursorStore + Send + 'static> {
    cursor_rx: tokio::sync::watch::Receiver<Option<DltCursor>>,
    store: Store,
}

impl<Store: DltCursorStore + Send + Sync + 'static> CursorPersistWorker<Store> {
    fn spawn(mut self) -> JoinHandle<Result<(), StdError>> {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

                let recv_result = self.cursor_rx.changed().await;
                if let Err(e) = recv_result {
                    log::error!("Error getting cursor to persist: {}", e);
                }

                let cursor = self.cursor_rx.borrow_and_update().clone();
                let Some(cursor) = cursor else { continue };
                log::debug!(
                    "Persisting cursor on slot ({}, {})",
                    cursor.slot,
                    HexStr::from(cursor.block_hash.as_slice()).to_string(),
                );

                let persist_result = self.store.set_cursor(cursor).await;
                if let Err(e) = persist_result {
                    log::error!("Error persisting cursor: {}", e);
                }
            }

            log::info!("CursorPersistWorker terminated");
            Ok(())
        })
    }
}
