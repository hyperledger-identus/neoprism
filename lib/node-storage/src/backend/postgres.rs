use identus_apollo::hash::Sha256Digest;
use identus_did_prism::dlt::{BlockNo, DltCursor, OperationMetadata, SlotNo};
use identus_did_prism::prelude::*;
use identus_did_prism::utils::paging::Paginated;
use identus_did_prism_indexer::repo::{
    DltCursorRepo, IndexedOperation, IndexedOperationRepo, IndexerStateRepo, RawOperationId, RawOperationRepo,
};
use lazybe::db::DbOps;
use lazybe::db::postgres::PostgresDbCtx;
use lazybe::filter::Filter;
use lazybe::page::PaginationInput;
use lazybe::sort::Sort;
use sqlx::PgPool;

use super::shared::parse_raw_operation;
use crate::snapshot::{DltCursorRecord, IndexedSsiRecord, IndexedVdrRecord, RawOperationRecord, StorageSnapshot};
use crate::{Error, entity};

#[derive(Debug, Clone)]
pub struct PostgresDb {
    pub pool: PgPool,
    db_ctx: PostgresDbCtx,
}

impl PostgresDb {
    pub async fn connect(db_url: &str) -> Result<Self, Error> {
        let pool = PgPool::connect(db_url).await?;
        Ok(Self {
            db_ctx: PostgresDbCtx,
            pool,
        })
    }

    pub async fn migrate(&self) -> Result<(), Error> {
        sqlx::migrate!("./migrations/postgres").run(&self.pool).await?;
        Ok(())
    }

    pub async fn export_snapshot(&self) -> Result<StorageSnapshot, Error> {
        let raw_operations = sqlx::query_as::<_, entity::RawOperation>(
            r#"SELECT * FROM raw_operation ORDER BY block_number, absn, osn"#,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(RawOperationRecord::from)
        .collect();

        let indexed_ssi_operations = sqlx::query_as::<_, entity::IndexedSsiOperation>(
            r#"SELECT * FROM indexed_ssi_operation ORDER BY indexed_at"#,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(IndexedSsiRecord::from)
        .collect();

        let indexed_vdr_operations = sqlx::query_as::<_, entity::IndexedVdrOperation>(
            r#"SELECT * FROM indexed_vdr_operation ORDER BY indexed_at"#,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(IndexedVdrRecord::from)
        .collect();

        let dlt_cursor = sqlx::query_as::<_, entity::DltCursor>(r#"SELECT * FROM dlt_cursor LIMIT 1"#)
            .fetch_optional(&self.pool)
            .await?
            .map(DltCursorRecord::from);

        Ok(StorageSnapshot {
            raw_operations,
            indexed_ssi_operations,
            indexed_vdr_operations,
            dlt_cursor,
        })
    }

    pub async fn import_snapshot(&self, snapshot: &StorageSnapshot) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM indexed_vdr_operation")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM indexed_ssi_operation")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM raw_operation").execute(&mut *tx).await?;
        sqlx::query("DELETE FROM dlt_cursor").execute(&mut *tx).await?;

        for op in &snapshot.raw_operations {
            sqlx::query(
                r#"
INSERT INTO raw_operation (id, signed_operation_data, slot, block_number, cbt, absn, osn, is_indexed)
VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
                "#,
            )
            .bind(op.id)
            .bind(&op.signed_operation_data)
            .bind(op.slot)
            .bind(op.block_number)
            .bind(op.cbt)
            .bind(op.absn)
            .bind(op.osn)
            .bind(op.is_indexed)
            .execute(&mut *tx)
            .await?;
        }

        for entry in &snapshot.indexed_ssi_operations {
            sqlx::query(
                r#"
INSERT INTO indexed_ssi_operation (id, raw_operation_id, did, indexed_at)
VALUES ($1,$2,$3,$4)
                "#,
            )
            .bind(entry.id)
            .bind(entry.raw_operation_id)
            .bind(&entry.did)
            .bind(entry.indexed_at)
            .execute(&mut *tx)
            .await?;
        }

        for entry in &snapshot.indexed_vdr_operations {
            sqlx::query(
                r#"
INSERT INTO indexed_vdr_operation (id, raw_operation_id, operation_hash, init_operation_hash, prev_operation_hash, did, indexed_at)
VALUES ($1,$2,$3,$4,$5,$6,$7)
                "#,
            )
            .bind(entry.id)
            .bind(entry.raw_operation_id)
            .bind(&entry.operation_hash)
            .bind(&entry.init_operation_hash)
            .bind(&entry.prev_operation_hash)
            .bind(&entry.did)
            .bind(entry.indexed_at)
            .execute(&mut *tx)
            .await?;
        }

        if let Some(cursor) = &snapshot.dlt_cursor {
            sqlx::query(
                r#"
INSERT INTO dlt_cursor (slot, block_hash)
VALUES ($1,$2)
                "#,
            )
            .bind(cursor.slot)
            .bind(&cursor.block_hash)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl IndexerStateRepo for PostgresDb {
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
impl RawOperationRepo for PostgresDb {
    type Error = Error;

    async fn get_raw_operations_unindexed(
        &self,
    ) -> Result<Vec<(RawOperationId, OperationMetadata, SignedPrismOperation)>, Self::Error> {
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

    async fn get_raw_operations_by_did(
        &self,
        did: &CanonicalPrismDid,
    ) -> Result<Vec<(RawOperationId, OperationMetadata, SignedPrismOperation)>, Self::Error> {
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
    ) -> Result<Option<(RawOperationId, OperationMetadata, SignedPrismOperation)>, Self::Error> {
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
            Some(op) => self
                .db_ctx
                .get::<entity::RawOperation>(&mut tx, op.raw_operation_id)
                .await?
                .map(parse_raw_operation)
                .transpose()?,
        };

        tx.commit().await?;
        Ok(result)
    }

    async fn insert_raw_operations(
        &self,
        operations: Vec<(OperationMetadata, SignedPrismOperation)>,
    ) -> Result<(), Self::Error> {
        let mut tx = self.pool.begin().await?;
        for (metadata, signed_operation) in operations {
            let create_op = entity::CreateRawOperation {
                signed_operation_data: signed_operation.encode_to_vec(),
                slot: metadata
                    .block_metadata
                    .slot_number
                    .inner()
                    .try_into()
                    .expect("slot_number does not fit in i64"),
                block_number: metadata
                    .block_metadata
                    .block_number
                    .inner()
                    .try_into()
                    .expect("block_number does not fit in i64"),
                cbt: metadata.block_metadata.cbt,
                absn: metadata
                    .block_metadata
                    .absn
                    .try_into()
                    .expect("absn does not fit in i32"),
                osn: metadata.osn.try_into().expect("osn does not fit in i32"),
                is_indexed: false,
            };
            self.db_ctx.create::<entity::RawOperation>(&mut tx, create_op).await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl IndexedOperationRepo for PostgresDb {
    type Error = Error;

    async fn insert_indexed_operations(&self, operations: Vec<IndexedOperation>) -> Result<(), Self::Error> {
        let mut tx = self.pool.begin().await?;
        for op in operations {
            // mark raw_operation as indexed
            self.db_ctx
                .update::<entity::RawOperation>(
                    &mut tx,
                    *op.raw_operation_id().as_ref(),
                    entity::UpdateRawOperation {
                        is_indexed: Some(true),
                        ..Default::default()
                    },
                )
                .await?;

            // write to indexing table
            match op {
                IndexedOperation::Ssi { raw_operation_id, did } => {
                    self.db_ctx
                        .create::<entity::IndexedSsiOperation>(
                            &mut tx,
                            entity::CreateIndexedSsiOperation {
                                raw_operation_id: raw_operation_id.into(),
                                did: did.into(),
                            },
                        )
                        .await?;
                }
                IndexedOperation::Vdr {
                    raw_operation_id,
                    operation_hash,
                    init_operation_hash,
                    prev_operation_hash,
                    did,
                } => {
                    self.db_ctx
                        .create::<entity::IndexedVdrOperation>(
                            &mut tx,
                            entity::CreateIndexedVdrOperation {
                                raw_operation_id: raw_operation_id.into(),
                                operation_hash,
                                init_operation_hash,
                                prev_operation_hash,
                                did: did.into(),
                            },
                        )
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
impl DltCursorRepo for PostgresDb {
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
