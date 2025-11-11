use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::Utc;
use identus_apollo::hash::sha256;
use protobuf::Message;
use tokio::sync::mpsc;

use super::{BlockMetadata, BlockNo, PublishedPrismObject, SlotNo, TxId};
use crate::proto::prism::PrismObject;

pub struct InMemoryBlockchain {
    block_tx: mpsc::Sender<PublishedPrismObject>,
    block_rx: mpsc::Receiver<PublishedPrismObject>,
    block_counter: Arc<AtomicU64>,
}

impl InMemoryBlockchain {
    pub fn new() -> Self {
        let (block_tx, block_rx) = mpsc::channel::<PublishedPrismObject>(1024);

        Self {
            block_tx,
            block_rx,
            block_counter: Arc::new(AtomicU64::new(0)),
        }
    }

    pub async fn add_block(&self, prism_object: PrismObject) -> Result<TxId, String> {
        let slot = self.block_counter.fetch_add(1, Ordering::SeqCst);
        let block_number = slot; // For in-memory blockchain, use slot as block number

        let tx_id = Self::generate_tx_id(&prism_object, slot, 0);

        let published_prism_object = PublishedPrismObject {
            block_metadata: BlockMetadata {
                slot_number: SlotNo::from(slot),
                block_number: BlockNo::from(block_number),
                cbt: Utc::now(),
                absn: 0, // In-memory blocks contain a single PrismObject per block
            },
            prism_object,
        };

        self.block_tx
            .send(published_prism_object)
            .await
            .map_err(|e| format!("failed to send block to channel: {e}"))?;

        Ok(tx_id)
    }

    pub async fn into_block_receiver(self) -> mpsc::Receiver<PublishedPrismObject> {
        self.block_rx
    }

    fn generate_tx_id(prism_object: &PrismObject, slot: u64, tx_idx: u32) -> TxId {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&slot.to_le_bytes());
        bytes.extend_from_slice(&tx_idx.to_le_bytes());

        let obj_bytes = prism_object.write_to_bytes().unwrap_or_default();
        bytes.extend_from_slice(&obj_bytes);

        let hash = sha256(&bytes);
        TxId::from(hash)
    }
}

impl Default for InMemoryBlockchain {
    fn default() -> Self {
        Self::new()
    }
}
