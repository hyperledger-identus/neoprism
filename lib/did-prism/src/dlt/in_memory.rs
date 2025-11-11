use std::sync::{Arc, RwLock};

use chrono::{DateTime, Duration, Utc};
use identus_apollo::hash::sha256;
use protobuf::Message;

use super::{BlockNo, SlotNo, TxId};
use crate::proto::prism::PrismObject;

#[derive(Clone)]
pub struct InMemoryBlockchain {
    inner: Arc<RwLock<InMemoryBlockchainInner>>,
}

struct InMemoryBlockchainInner {
    /// Blocks stored in chronological order
    blocks: Vec<InMemoryBlock>,
    /// Genesis timestamp for calculating block times
    genesis_time: DateTime<Utc>,
    /// Duration of each slot in seconds
    slot_duration_secs: u64,
}

#[derive(Debug, Clone)]
struct InMemoryBlock {
    slot_no: SlotNo,
    block_no: BlockNo,
    timestamp: DateTime<Utc>,
    transactions: Vec<InMemoryTransaction>,
}

#[derive(Debug, Clone)]
struct InMemoryTransaction {
    prism_object: PrismObject,
}

impl InMemoryBlockchain {
    pub fn new() -> Self {
        let genesis_time = DateTime::UNIX_EPOCH;
        let slot_duration_secs = 1;
        Self {
            inner: Arc::new(RwLock::new(InMemoryBlockchainInner {
                blocks: Vec::new(),
                genesis_time,
                slot_duration_secs,
            })),
        }
    }

    /// Adds a new block containing the given PRISM object and returns its transaction ID.
    pub fn add_block(&self, prism_object: PrismObject) -> Result<TxId, String> {
        let mut inner = self
            .inner
            .write()
            .map_err(|e| format!("failed to acquire write lock: {e}"))?;

        let slot_no = SlotNo::from(inner.blocks.len() as u64);
        let block_no = BlockNo::from(inner.blocks.len() as u64);
        let timestamp = inner.calculate_timestamp(inner.blocks.len() as u64);

        let tx_id = Self::generate_tx_id(&prism_object, inner.blocks.len() as u64, 0);
        let transaction = InMemoryTransaction { prism_object };
        let block = InMemoryBlock {
            slot_no,
            block_no,
            timestamp,
            transactions: vec![transaction],
        };

        inner.blocks.push(block);

        Ok(tx_id)
    }

    /// Advance to the next slot without adding a block (empty slot)
    pub fn advance_slot(&self) -> Result<(), String> {
        let mut inner = self
            .inner
            .write()
            .map_err(|e| format!("failed to acquire write lock: {e}"))?;

        let slot_no = SlotNo::from(inner.blocks.len() as u64);
        let block_no = BlockNo::from(inner.blocks.len() as u64);
        let timestamp = inner.calculate_timestamp(inner.blocks.len() as u64);

        let block = InMemoryBlock {
            slot_no,
            block_no,
            timestamp,
            transactions: Vec::new(),
        };

        inner.blocks.push(block);

        Ok(())
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

impl InMemoryBlockchainInner {
    fn calculate_timestamp(&self, slot: u64) -> DateTime<Utc> {
        self.genesis_time + Duration::seconds((slot * self.slot_duration_secs) as i64)
    }
}

impl Default for InMemoryBlockchain {
    fn default() -> Self {
        Self::new()
    }
}
