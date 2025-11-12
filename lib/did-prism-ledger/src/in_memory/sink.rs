use std::sync::atomic::AtomicU64;

use identus_apollo::hash::sha256;
use identus_did_prism::dlt::TxId;
use identus_did_prism::prelude::SignedPrismOperation;
use identus_did_prism::proto::prism::{PrismBlock, PrismObject};
use identus_did_prism_submitter::DltSink;
use tokio::sync::mpsc;

pub struct InMemoryDltSink {
    block_tx: mpsc::Sender<PrismObject>,
    count: AtomicU64,
}

impl InMemoryDltSink {
    pub fn new(block_tx: mpsc::Sender<PrismObject>) -> Self {
        Self {
            block_tx,
            count: AtomicU64::new(0),
        }
    }
}

#[async_trait::async_trait]
impl DltSink for InMemoryDltSink {
    async fn publish_operations(&self, operations: Vec<SignedPrismOperation>) -> Result<TxId, String> {
        let prism_object = PrismObject {
            block_content: Some(PrismBlock {
                operations,
                special_fields: Default::default(),
            })
            .into(),
            special_fields: Default::default(),
        };

        let count = self.count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let tx_id = TxId::from(sha256(count.to_le_bytes()));
        self.block_tx
            .send(prism_object)
            .await
            .map_err(|e| e.to_string())
            .map(|_| tx_id)
    }
}
