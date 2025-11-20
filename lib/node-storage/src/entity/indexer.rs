use chrono::{DateTime, Utc};
use lazybe::macros::Entity;
use lazybe::uuid::Uuid;

use crate::entity::DidSuffix;

#[derive(Entity, sqlx::FromRow)]
#[lazybe(table = "dlt_cursor")]
pub struct DltCursor {
    #[lazybe(primary_key)]
    pub id: Uuid,
    pub slot: i64,
    pub block_hash: Vec<u8>,
}

#[derive(Entity, sqlx::FromRow)]
#[lazybe(table = "raw_operation")]
#[allow(unused)]
pub struct RawOperation {
    #[lazybe(primary_key)]
    pub id: Uuid,
    pub signed_operation_data: Vec<u8>,
    pub slot: i64,
    pub block_number: i64,
    pub cbt: DateTime<Utc>,
    pub absn: i32,
    pub osn: i32,
    pub is_indexed: bool,
}

#[derive(Entity, sqlx::FromRow)]
#[lazybe(table = "indexed_ssi_operation")]
#[allow(unused)]
pub struct IndexedSsiOperation {
    #[lazybe(primary_key)]
    pub id: Uuid,
    pub raw_operation_id: Uuid,
    pub did: DidSuffix,
    #[lazybe(created_at)]
    pub indexed_at: DateTime<Utc>,
}

#[derive(Entity, sqlx::FromRow)]
#[lazybe(table = "indexed_vdr_operation")]
#[allow(unused)]
pub struct IndexedVdrOperation {
    #[lazybe(primary_key)]
    pub id: Uuid,
    pub raw_operation_id: Uuid,
    pub operation_hash: Vec<u8>,
    pub init_operation_hash: Vec<u8>,
    pub prev_operation_hash: Option<Vec<u8>>,
    pub did: DidSuffix,
    #[lazybe(created_at)]
    pub indexed_at: DateTime<Utc>,
}

#[derive(Entity)]
#[lazybe(table = "did_stats")]
#[allow(unused)]
pub struct DidStats {
    #[lazybe(primary_key)]
    pub did: DidSuffix,
    pub operation_count: i64,
    pub last_block: i64,
    pub last_slot: i64,
    pub last_cbt: DateTime<Utc>,
    pub first_block: i64,
    pub first_slot: i64,
    pub first_cbt: DateTime<Utc>,
}

#[derive(Entity)]
#[lazybe(table = "raw_operation_by_did")]
#[allow(unused)]
pub struct RawOperationByDid {
    #[lazybe(primary_key)]
    pub id: Uuid,
    pub signed_operation_data: Vec<u8>,
    pub slot: i64,
    pub block_number: i64,
    pub cbt: DateTime<Utc>,
    pub absn: i32,
    pub osn: i32,
    pub is_indexed: bool,
    pub did: DidSuffix,
}

impl From<RawOperationByDid> for RawOperation {
    fn from(value: RawOperationByDid) -> Self {
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
