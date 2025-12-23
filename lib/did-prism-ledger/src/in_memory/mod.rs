mod sink;
mod source;

use std::sync::Arc;

use chrono::Utc;
use identus_did_prism::dlt::{BlockMetadata, BlockNo, PublishedPrismObject, SlotNo, TxId};
use identus_did_prism::proto::prism::PrismObject;
use identus_did_prism_submitter::DltSink;
pub use sink::InMemoryDltSink;
pub use source::InMemoryDltSource;
use tokio::sync::mpsc;

const BUFFER_SIZE: usize = 1024;

pub fn create_ledger() -> (InMemoryDltSource, Arc<dyn DltSink + Send + Sync>) {
    let (block_tx, block_rx) = mpsc::channel::<PublishedPrismObject>(BUFFER_SIZE);
    let (object_tx, mut object_rx) = mpsc::channel::<(PrismObject, TxId)>(BUFFER_SIZE);

    tokio::spawn(async move {
        let mut block_count = 0;
        while let Some((prism_object, tx_id)) = object_rx.recv().await {
            let slot = block_count;
            let block_number = slot; // For in-memory blockchain, use slot as block number
            let published_prism_object = PublishedPrismObject {
                block_metadata: BlockMetadata {
                    slot_number: SlotNo::from(slot),
                    block_number: BlockNo::from(block_number),
                    tx_id,
                    cbt: Utc::now(),
                    absn: 0, // In-memory blocks contain a single PrismObject per block
                },
                prism_object,
            };
            if let Err(e) = block_tx.send(published_prism_object).await {
                tracing::error!(error = ?e, "failed to send published object to block receiver");
                break;
            }
            block_count += 1;
        }
    });

    let source = InMemoryDltSource::new(block_rx);
    let sink = Arc::new(InMemoryDltSink::new(object_tx));
    (source, sink)
}
