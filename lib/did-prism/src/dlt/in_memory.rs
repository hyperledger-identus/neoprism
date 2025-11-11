use chrono::Utc;
use tokio::sync::mpsc;

use super::{BlockMetadata, BlockNo, PublishedPrismObject, SlotNo};
use crate::proto::prism::PrismObject;

pub struct InMemoryBlockchain;

const BUFFER_SIZE: usize = 1024;

impl InMemoryBlockchain {
    pub fn new_tx_rx() -> (mpsc::Receiver<PublishedPrismObject>, mpsc::Sender<PrismObject>) {
        let (block_tx, block_rx) = mpsc::channel::<PublishedPrismObject>(BUFFER_SIZE);
        let (object_tx, mut object_rx) = mpsc::channel::<PrismObject>(BUFFER_SIZE);

        tokio::spawn(async move {
            let mut block_count = 0;
            while let Some(prism_object) = object_rx.recv().await {
                let slot = block_count;
                let block_number = slot; // For in-memory blockchain, use slot as block number

                let published_prism_object = PublishedPrismObject {
                    block_metadata: BlockMetadata {
                        slot_number: SlotNo::from(slot),
                        block_number: BlockNo::from(block_number),
                        cbt: Utc::now(),
                        absn: 0, // In-memory blocks contain a single PrismObject per block
                    },
                    prism_object,
                };
                if let Err(e) = block_tx.send(published_prism_object).await {
                    tracing::error!("{:?}", e);
                    break;
                }
                block_count += 1;
            }
        });

        (block_rx, object_tx)
    }
}
