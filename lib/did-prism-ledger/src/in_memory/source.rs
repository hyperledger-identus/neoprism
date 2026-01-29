use identus_did_prism::dlt::{DltCursor, PublishedPrismObject};
use identus_did_prism_indexer::DltSource;
use tokio::sync::{mpsc, watch};

pub struct InMemoryDltSource {
    block_rx: mpsc::Receiver<PublishedPrismObject>,
    sync_cursor_tx: watch::Sender<Option<DltCursor>>,
}

impl InMemoryDltSource {
    pub fn new(block_rx: mpsc::Receiver<PublishedPrismObject>) -> Self {
        let (sync_cursor_tx, _) = watch::channel::<Option<DltCursor>>(None);
        Self {
            block_rx,
            sync_cursor_tx,
        }
    }
}

impl DltSource for InMemoryDltSource {
    fn sync_cursor(&self) -> watch::Receiver<Option<DltCursor>> {
        self.sync_cursor_tx.subscribe()
    }

    fn into_stream(self) -> Result<mpsc::Receiver<PublishedPrismObject>, String> {
        let (event_tx, event_rx) = mpsc::channel::<PublishedPrismObject>(1024);
        let sync_cursor_tx = self.sync_cursor_tx;

        tokio::spawn(async move {
            let mut block_rx = self.block_rx;

            while let Some(published_object) = block_rx.recv().await {
                // Update cursor based on the block metadata
                let cursor = DltCursor {
                    slot: published_object.block_metadata.slot_number.into(),
                    // Generate synthetic block_hash from block_number for in-memory ledger
                    block_hash: published_object
                        .block_metadata
                        .block_number
                        .inner()
                        .to_le_bytes()
                        .to_vec(),
                    cbt: Some(published_object.block_metadata.cbt),
                    blockfrost_page: None,
                };
                let _ = sync_cursor_tx.send(Some(cursor));

                // Send the published object downstream
                if event_tx.send(published_object).await.is_err() {
                    tracing::warn!("InMemoryDltSource: event receiver closed");
                    break;
                }
            }
        });

        Ok(event_rx)
    }
}
