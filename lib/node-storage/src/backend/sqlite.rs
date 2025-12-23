use std::str::FromStr;

use identus_apollo::hash::Sha256Digest;
use identus_apollo::hex::HexStr;
use identus_did_prism::did::operation::OperationId;
use identus_did_prism::dlt::{BlockNo, DltCursor, OperationMetadata, SlotNo, TxId};
use identus_did_prism::prelude::*;
use identus_did_prism::utils::paging::Paginated;
use identus_did_prism_indexer::repo::{
    DltCursorRepo, IndexedOperation, IndexedOperationRepo, IndexerStateRepo, RawOperationRecord, RawOperationRepo,
};
use lazybe::db::DbOps;
use lazybe::db::sqlite::SqliteDbCtx;
use lazybe::filter::Filter;
use lazybe::page::PaginationInput;
use lazybe::sort::Sort;
use lazybe::uuid::Uuid;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

use super::shared::parse_raw_operation;
use crate::entity::DidSuffix;
use crate::{Error, entity};

#[derive(Debug, Clone)]
pub struct SqliteDb {
    pub pool: SqlitePool,
    db_ctx: SqliteDbCtx,
}

impl SqliteDb {
    pub async fn connect(db_url: &str) -> Result<Self, Error> {
        let options = SqliteConnectOptions::from_str(db_url)?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;
        Ok(Self {
            db_ctx: SqliteDbCtx,
            pool,
        })
    }

    pub async fn migrate(&self) -> Result<(), Error> {
        sqlx::migrate!("./migrations/sqlite").run(&self.pool).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl IndexerStateRepo for SqliteDb {
    type Error = Error;

    async fn get_last_indexed_block(&self) -> Result<Option<(SlotNo, BlockNo)>, Self::Error> {
        let rows: Vec<entity::RawOperation> = sqlx::query_as(
            r#"
SELECT *
FROM raw_operation
WHERE
    is_indexed = true AND block_number NOT IN (
        SELECT DISTINCT block_number FROM raw_operation
        WHERE is_indexed = false
    )
ORDER BY block_number DESC
LIMIT 1
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let last_op = rows.into_iter().next();
        Ok(last_op.map(|op| {
            let slot: SlotNo = u64::try_from(op.slot).expect("slot_number does not fit in u64").into();
            let block: BlockNo = u64::try_from(op.block_number)
                .expect("block_number does not fit in u64")
                .into();
            (slot, block)
        }))
    }

    async fn get_all_dids(&self, page: u32, page_size: u32) -> Result<Paginated<CanonicalPrismDid>, Self::Error> {
        let mut tx = self.pool.begin().await?;
        let did_page = self
            .db_ctx
            .list::<entity::DidStats>(
                &mut tx,
                Filter::empty(),
                Sort::new([
                    entity::DidStatsSort::first_slot().desc(),
                    entity::DidStatsSort::did().asc(),
                ]),
                Some(PaginationInput { page, limit: page_size }),
            )
            .await?;
        tx.commit().await?;

        let items = did_page
            .data
            .into_iter()
            .map(|stats| stats.did.try_into())
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Paginated {
            items,
            current_page: did_page.page,
            page_size: did_page.page_size,
            total_items: did_page.total_records,
        })
    }

    async fn get_did_by_vdr_entry(
        &self,
        operation_hash: &Sha256Digest,
    ) -> Result<Option<CanonicalPrismDid>, Self::Error> {
        let mut tx = self.pool.begin().await?;
        let result = self
            .db_ctx
            .list::<entity::IndexedVdrOperation>(
                &mut tx,
                Filter::all([entity::IndexedVdrOperationFilter::init_operation_hash().eq(operation_hash.to_vec())]),
                Sort::new([]),
                Some(PaginationInput { page: 0, limit: 1 }),
            )
            .await?;
        tx.commit().await?;

        let did = result.data.into_iter().next().map(|i| i.did.try_into()).transpose()?;
        Ok(did)
    }
}

#[async_trait::async_trait]
impl RawOperationRepo for SqliteDb {
    type Error = Error;

    async fn get_raw_operations_unindexed(&self) -> Result<Vec<RawOperationRecord>, Self::Error> {
        let mut tx = self.pool.begin().await?;
        let result = self
            .db_ctx
            .list::<entity::RawOperation>(
                &mut tx,
                Filter::all([entity::RawOperationFilter::is_indexed().eq(false)]),
                Sort::new([
                    entity::RawOperationSort::block_number().asc(),
                    entity::RawOperationSort::absn().asc(),
                    entity::RawOperationSort::osn().asc(),
                ]),
                Some(PaginationInput { page: 0, limit: 200 }),
            )
            .await?
            .data
            .into_iter()
            .map(parse_raw_operation)
            .collect::<Result<Vec<_>, _>>()?;
        tx.commit().await?;
        Ok(result)
    }

    async fn get_raw_operations_by_did(&self, did: &CanonicalPrismDid) -> Result<Vec<RawOperationRecord>, Self::Error> {
        let suffix_bytes = did.suffix().to_vec();
        let mut tx = self.pool.begin().await?;
        let result = self
            .db_ctx
            .list::<entity::RawOperationByDid>(
                &mut tx,
                Filter::all([entity::RawOperationByDidFilter::did().eq(suffix_bytes.into())]),
                Sort::empty(),
                None,
            )
            .await?
            .data
            .into_iter()
            .map(|i| parse_raw_operation(i.into()))
            .collect::<Result<Vec<_>, _>>()?;
        tx.commit().await?;
        Ok(result)
    }

    async fn get_raw_operation_vdr_by_operation_hash(
        &self,
        operation_hash: &Sha256Digest,
    ) -> Result<Option<RawOperationRecord>, Self::Error> {
        let mut tx = self.pool.begin().await?;
        let vdr_operation = self
            .db_ctx
            .list::<entity::IndexedVdrOperation>(
                &mut tx,
                Filter::all([entity::IndexedVdrOperationFilter::operation_hash().eq(operation_hash.to_vec())]),
                Sort::empty(),
                Some(PaginationInput { page: 0, limit: 1 }),
            )
            .await?
            .data
            .into_iter()
            .next();

        let result = match vdr_operation {
            None => None,
            Some(op) => {
                let raw_op =
                    sqlx::query_as::<_, entity::RawOperation>(r#"SELECT * FROM raw_operation WHERE id = ?1 LIMIT 1"#)
                        .bind(op.raw_operation_id)
                        .fetch_optional(&mut *tx)
                        .await?;

                raw_op.map(parse_raw_operation).transpose()?
            }
        };

        tx.commit().await?;
        Ok(result)
    }

    async fn get_raw_operations_by_tx_id(
        &self,
        tx_id: &TxId,
    ) -> Result<Vec<(RawOperationRecord, CanonicalPrismDid)>, Self::Error> {
        let mut tx = self.pool.begin().await?;
        let result = self
            .db_ctx
            .list::<entity::RawOperationByDid>(
                &mut tx,
                Filter::all([entity::RawOperationByDidFilter::tx_hash().eq(tx_id.to_vec())]),
                Sort::new([
                    entity::RawOperationByDidSort::block_number().asc(),
                    entity::RawOperationByDidSort::absn().asc(),
                    entity::RawOperationByDidSort::osn().asc(),
                ]),
                None,
            )
            .await?
            .data
            .into_iter()
            .map(|ro| {
                let did_suffix = HexStr::from(ro.did.as_bytes());
                parse_raw_operation(ro.into()).and_then(|i| {
                    CanonicalPrismDid::from_suffix(did_suffix)
                        .map_err(|e| e.into())
                        .map(|j| (i, j))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        tx.commit().await?;
        Ok(result)
    }

    async fn get_raw_operation_by_operation_id(
        &self,
        operation_id: &OperationId,
    ) -> Result<Option<(RawOperationRecord, CanonicalPrismDid)>, Self::Error> {
        let mut tx = self.pool.begin().await?;

        let result = self
            .db_ctx
            .list::<entity::RawOperationByDid>(
                &mut tx,
                Filter::all([entity::RawOperationByDidFilter::operation_id().eq(operation_id.to_vec())]),
                Sort::empty(),
                Some(PaginationInput { page: 0, limit: 1 }),
            )
            .await?
            .data
            .into_iter()
            .next()
            .map(|ro| -> Result<(RawOperationRecord, CanonicalPrismDid), Error> {
                let did_suffix = HexStr::from(ro.did.as_bytes());
                let record = parse_raw_operation(ro.into())?;
                let did = CanonicalPrismDid::from_suffix(did_suffix)?;
                Ok((record, did))
            })
            .transpose()?;

        tx.commit().await?;
        Ok(result)
    }

    async fn insert_raw_operations(
        &self,
        operations: Vec<(OperationMetadata, SignedPrismOperation)>,
    ) -> Result<(), Self::Error> {
        let mut tx = self.pool.begin().await?;
        for (metadata, signed_operation) in operations {
            let slot: i64 = metadata
                .block_metadata
                .slot_number
                .inner()
                .try_into()
                .expect("slot_number does not fit in i64");
            let block_number: i64 = metadata
                .block_metadata
                .block_number
                .inner()
                .try_into()
                .expect("block_number does not fit in i64");
            let absn: i32 = metadata
                .block_metadata
                .absn
                .try_into()
                .expect("absn does not fit in i32");
            let osn: i32 = metadata.osn.try_into().expect("osn does not fit in i32");

            let operation_id = signed_operation.operation_id();
            sqlx::query(
                r#"
INSERT INTO raw_operation (id, signed_operation_data, slot, block_number, cbt, absn, osn, tx_hash, operation_id, is_indexed)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0)
ON CONFLICT(block_number, absn, osn) DO NOTHING
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(signed_operation.encode_to_vec())
            .bind(slot)
            .bind(block_number)
            .bind(metadata.block_metadata.cbt)
            .bind(absn)
            .bind(osn)
            .bind(metadata.block_metadata.tx_id.to_vec())
            .bind(operation_id.to_vec())
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl IndexedOperationRepo for SqliteDb {
    type Error = Error;

    async fn insert_indexed_operations(&self, operations: Vec<IndexedOperation>) -> Result<(), Self::Error> {
        let mut tx = self.pool.begin().await?;
        for op in operations {
            let raw_uuid = *op.raw_operation_id().as_ref();
            sqlx::query("UPDATE raw_operation SET is_indexed = 1 WHERE id = ?1")
                .bind(raw_uuid)
                .execute(&mut *tx)
                .await?;

            match op {
                IndexedOperation::Ssi { did, .. } => {
                    let did_bytes = DidSuffix::from(did).into_bytes();
                    sqlx::query(
                        r#"
INSERT INTO indexed_ssi_operation (raw_operation_id, did, indexed_at)
VALUES (?1, ?2, datetime('now'))
                        "#,
                    )
                    .bind(raw_uuid)
                    .bind(did_bytes)
                    .execute(&mut *tx)
                    .await?;
                }
                IndexedOperation::Vdr {
                    operation_hash,
                    init_operation_hash,
                    prev_operation_hash,
                    did,
                    ..
                } => {
                    let did_bytes = DidSuffix::from(did).into_bytes();
                    sqlx::query(
                        r#"
INSERT INTO indexed_vdr_operation (raw_operation_id, operation_hash, init_operation_hash, prev_operation_hash, did, indexed_at)
VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))
                        "#,
                    )
                    .bind(raw_uuid)
                    .bind(operation_hash)
                    .bind(init_operation_hash)
                    .bind(prev_operation_hash)
                    .bind(did_bytes)
                    .execute(&mut *tx)
                    .await?;
                }
                IndexedOperation::Ignored { .. } => (),
            };
        }
        tx.commit().await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl DltCursorRepo for SqliteDb {
    type Error = Error;

    async fn get_cursor(&self) -> Result<Option<DltCursor>, Self::Error> {
        let mut tx = self.pool.begin().await?;
        let result = self
            .db_ctx
            .list::<entity::DltCursor>(&mut tx, Filter::empty(), Sort::empty(), None)
            .await?
            .data
            .into_iter()
            .next()
            .map(|model| DltCursor {
                slot: model.slot as u64,
                block_hash: model.block_hash,
                cbt: None,
            });
        tx.commit().await?;
        Ok(result)
    }

    async fn set_cursor(&self, cursor: DltCursor) -> Result<(), Self::Error> {
        let mut tx = self.pool.begin().await?;
        let cursors = self
            .db_ctx
            .list::<entity::DltCursor>(&mut tx, Filter::empty(), Sort::empty(), None)
            .await?
            .data;
        for c in cursors {
            self.db_ctx.delete::<entity::DltCursor>(&mut tx, c.id).await?;
        }
        self.db_ctx
            .create::<entity::DltCursor>(
                &mut tx,
                entity::CreateDltCursor {
                    slot: cursor.slot as i64,
                    block_hash: cursor.block_hash,
                },
            )
            .await?;
        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use identus_apollo::hash::sha256;
    use identus_did_prism::dlt::{BlockMetadata, OperationMetadata, TxId};
    use tempfile::TempDir;

    use super::*;

    fn dummy_metadata(block: u64, absn: u32, osn: u32) -> OperationMetadata {
        OperationMetadata {
            block_metadata: BlockMetadata {
                slot_number: block.into(),
                block_number: block.into(),
                cbt: Utc.timestamp_opt(0, 0).single().expect("failed to build timestamp"),
                absn,
                tx_id: TxId::from(sha256(block.to_le_bytes())),
            },
            osn,
        }
    }

    async fn setup_db() -> (TempDir, SqliteDb) {
        let dir = tempfile::tempdir().expect("temp dir");
        let db_path = dir.path().join("db.sqlite");
        let url = format!("sqlite://{}", db_path.display());
        let db = SqliteDb::connect(&url).await.expect("connect sqlite");
        db.migrate().await.expect("migrate sqlite");
        (dir, db)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn insert_raw_operations_is_idempotent_on_absn_triplet() {
        let (_tmp_dir, db) = setup_db().await;
        let metadata = dummy_metadata(42, 1, 7);
        let operation = SignedPrismOperation::default();

        db.insert_raw_operations(vec![(metadata.clone(), operation.clone())])
            .await
            .expect("first insert succeeds");
        db.insert_raw_operations(vec![(metadata, operation)])
            .await
            .expect("duplicate insert ignored");

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM raw_operation")
            .fetch_one(&db.pool)
            .await
            .expect("count rows");
        assert_eq!(count, 1);
    }
}
