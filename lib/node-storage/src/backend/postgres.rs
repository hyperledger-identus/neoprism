use identus_apollo::hash::Sha256Digest;
use identus_did_prism::dlt::{BlockNo, DltCursor, OperationMetadata, SlotNo, TxId};
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

    async fn get_raw_operations_by_tx_id(
        &self,
        tx_id: &TxId,
    ) -> Result<Vec<(RawOperationId, OperationMetadata, SignedPrismOperation)>, Self::Error> {
        let mut tx = self.pool.begin().await?;
        let result = self
            .db_ctx
            .list::<entity::RawOperation>(
                &mut tx,
                Filter::all([entity::RawOperationFilter::tx_hash().eq(tx_id.to_vec())]),
                Sort::new([
                    entity::RawOperationSort::block_number().asc(),
                    entity::RawOperationSort::absn().asc(),
                    entity::RawOperationSort::osn().asc(),
                ]),
                None,
            )
            .await?
            .data
            .into_iter()
            .map(parse_raw_operation)
            .collect::<Result<Vec<_>, _>>()?;
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
                tx_hash: metadata.block_metadata.tx_id.to_vec(),
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
