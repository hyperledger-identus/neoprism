//! Blockfrost DLT source implementation.
//!
//! This module provides a DLT source that fetches PRISM data from the Blockfrost API.
//! It follows the same pattern as DbSyncSource, using REST API polling.
//!
//! TODO: Implement actual Blockfrost API calls in the stream_loop method.

use identus_did_prism::dlt::{DltCursor, PublishedPrismObject};
use identus_did_prism::location;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;

use crate::DltSource;
use crate::dlt::common::CursorPersistWorker;
use crate::dlt::error::DltError;
use crate::repo::DltCursorRepo;

mod models {
    use identus_did_prism::dlt::{BlockMetadata, PublishedPrismObject};

    use crate::dlt::error::MetadataReadError;

    // Placeholder struct for Blockfrost block data
    #[derive(Debug, Clone)]
    pub struct BlockfrostBlock {
        pub slot: u64,
        pub hash: String,
        pub height: u64,
        pub time: i64,
    }

    // Placeholder struct for Blockfrost transaction metadata
    #[derive(Debug, Clone)]
    pub struct BlockfrostMetadata {
        pub tx_hash: String,
        pub label: String,
        pub json_metadata: serde_json::Value,
    }

    pub fn parse_blockfrost_metadata(
        _block: BlockfrostBlock,
        _metadata: BlockfrostMetadata,
    ) -> Result<PublishedPrismObject, MetadataReadError> {
        todo!("Parse Blockfrost metadata into PublishedPrismObject")
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
