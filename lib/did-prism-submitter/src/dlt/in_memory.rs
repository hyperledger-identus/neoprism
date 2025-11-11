use identus_did_prism::dlt::in_memory::InMemoryBlockchain;
use identus_did_prism::dlt::TxId;
use identus_did_prism::prelude::SignedPrismOperation;
use identus_did_prism::proto::prism::{PrismBlock, PrismObject};

use crate::DltSink;

/// In-memory DLT sink for testing.
pub struct InMemoryDltSink {
    blockchain: InMemoryBlockchain,
}

impl InMemoryDltSink {
    pub fn new(blockchain: InMemoryBlockchain) -> Self {
        Self { blockchain }
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

        self.blockchain.add_block(prism_object)
    }
}
