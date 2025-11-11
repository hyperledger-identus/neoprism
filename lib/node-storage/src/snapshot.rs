use chrono::{DateTime, Utc};
use lazybe::uuid::Uuid;
use serde::{Deserialize, Serialize};

use crate::entity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSnapshot {
    pub raw_operations: Vec<RawOperationRecord>,
    pub indexed_ssi_operations: Vec<IndexedSsiRecord>,
    pub indexed_vdr_operations: Vec<IndexedVdrRecord>,
    pub dlt_cursor: Option<DltCursorRecord>,
}

impl StorageSnapshot {
    pub fn new() -> Self {
        Self {
            raw_operations: Vec::new(),
            indexed_ssi_operations: Vec::new(),
            indexed_vdr_operations: Vec::new(),
            dlt_cursor: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawOperationRecord {
    pub id: Uuid,
    pub signed_operation_data: Vec<u8>,
    pub slot: i64,
    pub block_number: i64,
    pub cbt: DateTime<Utc>,
    pub absn: i32,
    pub osn: i32,
    pub is_indexed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedSsiRecord {
    pub id: Uuid,
    pub raw_operation_id: Uuid,
    pub did: Vec<u8>,
    pub indexed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedVdrRecord {
    pub id: Uuid,
    pub raw_operation_id: Uuid,
    pub operation_hash: Vec<u8>,
    pub init_operation_hash: Vec<u8>,
    pub prev_operation_hash: Option<Vec<u8>>,
    pub did: Vec<u8>,
    pub indexed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DltCursorRecord {
    pub slot: i64,
    pub block_hash: Vec<u8>,
}

impl From<entity::RawOperation> for RawOperationRecord {
    fn from(value: entity::RawOperation) -> Self {
        Self {
            id: value.id,
            signed_operation_data: value.signed_operation_data,
            slot: value.slot,
            block_number: value.block_number,
            cbt: value.cbt,
            absn: value.absn,
            osn: value.osn,
            is_indexed: value.is_indexed,
        }
    }
}

impl From<entity::IndexedSsiOperation> for IndexedSsiRecord {
    fn from(value: entity::IndexedSsiOperation) -> Self {
        Self {
            id: value.id,
            raw_operation_id: value.raw_operation_id,
            did: value.did.into_bytes(),
            indexed_at: value.indexed_at,
        }
    }
}

impl From<entity::IndexedVdrOperation> for IndexedVdrRecord {
    fn from(value: entity::IndexedVdrOperation) -> Self {
        Self {
            id: value.id,
            raw_operation_id: value.raw_operation_id,
            operation_hash: value.operation_hash,
            init_operation_hash: value.init_operation_hash,
            prev_operation_hash: value.prev_operation_hash,
            did: value.did.into_bytes(),
            indexed_at: value.indexed_at,
        }
    }
}

impl From<entity::DltCursor> for DltCursorRecord {
    fn from(value: entity::DltCursor) -> Self {
        Self {
            slot: value.slot,
            block_hash: value.block_hash,
        }
    }
}
