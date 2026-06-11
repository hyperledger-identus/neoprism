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
                blockfrost_page: model.blockfrost_page.map(|p| p as u32),
            });
        tx.commit().await?;
        Ok(result)
    }

    async fn set_cursor(&self, cursor: DltCursor) -> Result<(), Self::Error> {
        let mut tx = self.pool.begin().await?;
        // Use raw SQL here: lazybe/sea_query encodes a `uuid::Uuid` as a string
        // for the `id = ?` comparison, but the column is `BLOB` (16 bytes), so the
        // per-row delete never matches and stale cursors pile up. The table holds
        // at most one cursor, so a single DELETE clears all rows regardless of id.
        sqlx::query("DELETE FROM dlt_cursor").execute(&mut *tx).await?;
        self.db_ctx
            .create::<entity::DltCursor>(
                &mut tx,
                entity::CreateDltCursor {
                    slot: cursor.slot as i64,
                    block_hash: cursor.block_hash,
                    blockfrost_page: cursor.blockfrost_page.map(|p| p as i64),
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
    use identus_apollo::crypto::secp256k1::Secp256k1PrivateKey;
    use identus_apollo::hash::sha256;
    use identus_did_prism::did::CanonicalPrismDid;
    use identus_did_prism::dlt::{BlockMetadata, DltCursor, OperationMetadata, TxId};
    use identus_did_prism::prelude::*;
    use identus_did_prism::proto;
    use identus_did_prism_indexer::repo::{
        DltCursorRepo, IndexedOperation, IndexedOperationRepo, IndexerStateRepo, RawOperationRepo,
    };
    use tempfile::TempDir;

    use super::*;

    const MASTER_KEY: [u8; 32] = [1; 32];
    const MASTER_KEY_NAME: &str = "master-0";

    fn new_create_did_signed_operation() -> SignedPrismOperation {
        let master_sk = Secp256k1PrivateKey::from_slice(&MASTER_KEY).unwrap();
        let pk = master_sk.to_public_key();
        let public_key = proto::prism_ssi::PublicKey {
            id: MASTER_KEY_NAME.to_string(),
            usage: proto::prism_ssi::KeyUsage::MASTER_KEY.into(),
            key_data: Some(proto::prism_ssi::public_key::Key_data::CompressedEcKeyData(
                proto::prism_ssi::CompressedECKeyData {
                    curve: "secp256k1".to_string(),
                    data: pk.encode_compressed().into(),
                    special_fields: Default::default(),
                },
            )),
            special_fields: Default::default(),
        };
        let operation_inner = proto::prism::prism_operation::Operation::CreateDid(proto::prism_ssi::ProtoCreateDID {
            did_data: protobuf::MessageField(Some(
                proto::prism_ssi::proto_create_did::DIDCreationData {
                    public_keys: vec![public_key],
                    services: vec![],
                    context: vec![],
                    special_fields: Default::default(),
                }
                .into(),
            )),
            special_fields: Default::default(),
        });
        let operation = proto::prism::PrismOperation {
            operation: Some(operation_inner),
            special_fields: Default::default(),
        };
        SignedPrismOperation {
            signed_with: MASTER_KEY_NAME.to_string(),
            signature: master_sk.sign(&operation.encode_to_vec()),
            operation: Some(operation).into(),
            special_fields: Default::default(),
        }
    }

    /// Derive the canonical DID from the inner PrismOperation of a signed operation.
    fn did_from_signed_op(op: &SignedPrismOperation) -> CanonicalPrismDid {
        let prism_op = op.operation.as_ref().expect("operation present");
        CanonicalPrismDid::from_operation(prism_op).expect("did from operation")
    }

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

    /// Helper: insert a single raw operation and return its record.
    async fn insert_one(db: &SqliteDb, block: u64, absn: u32, osn: u32) -> RawOperationRecord {
        let metadata = dummy_metadata(block, absn, osn);
        let operation = new_create_did_signed_operation();
        db.insert_raw_operations(vec![(metadata, operation)])
            .await
            .expect("insert");
        db.get_raw_operations_unindexed()
            .await
            .expect("fetch unindexed")
            .remove(0)
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

    // ── RawOperationRepo: get_raw_operations_unindexed ──

    #[tokio::test(flavor = "multi_thread")]
    async fn get_raw_operations_unindexed_returns_only_unindexed() {
        let (_tmp_dir, db) = setup_db().await;

        // Initially empty
        let result = db.get_raw_operations_unindexed().await.expect("fetch");
        assert!(result.is_empty());

        // Insert two operations
        let op = new_create_did_signed_operation();
        db.insert_raw_operations(vec![
            (dummy_metadata(1, 0, 0), op.clone()),
            (dummy_metadata(2, 0, 0), op.clone()),
        ])
        .await
        .expect("insert");

        let unindexed = db.get_raw_operations_unindexed().await.expect("fetch");
        assert_eq!(unindexed.len(), 2);

        // Mark first as indexed
        sqlx::query("UPDATE raw_operation SET is_indexed = 1 WHERE block_number = 1")
            .execute(&db.pool)
            .await
            .expect("update");

        let unindexed = db.get_raw_operations_unindexed().await.expect("fetch");
        assert_eq!(unindexed.len(), 1);
        assert_eq!(unindexed[0].metadata.block_metadata.block_number.inner(), 2);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn get_raw_operations_unindexed_ordered_by_block_absn_osn() {
        let (_tmp_dir, db) = setup_db().await;
        let op = new_create_did_signed_operation();

        db.insert_raw_operations(vec![
            (dummy_metadata(3, 0, 1), op.clone()),
            (dummy_metadata(1, 0, 0), op.clone()),
            (dummy_metadata(1, 1, 0), op.clone()),
        ])
        .await
        .expect("insert");

        let result = db.get_raw_operations_unindexed().await.expect("fetch");
        assert_eq!(result.len(), 3);
        // Ordered by (block_number ASC, absn ASC, osn ASC)
        assert_eq!(result[0].metadata.block_metadata.block_number.inner(), 1);
        assert_eq!(result[0].metadata.block_metadata.absn, 0);
        assert_eq!(result[0].metadata.osn, 0);
        assert_eq!(result[1].metadata.block_metadata.block_number.inner(), 1);
        assert_eq!(result[1].metadata.block_metadata.absn, 1);
        assert_eq!(result[2].metadata.block_metadata.block_number.inner(), 3);
    }

    // ── RawOperationRepo: get_raw_operations_by_did ──

    #[tokio::test(flavor = "multi_thread")]
    async fn get_raw_operations_by_did_returns_operations_for_did() {
        let (_tmp_dir, db) = setup_db().await;
        let rec = insert_one(&db, 10, 0, 0).await;

        let did = did_from_signed_op(&rec.signed_operation);

        // Index as SSI so the view has the DID mapping
        db.insert_indexed_operations(vec![IndexedOperation::Ssi {
            raw_operation_id: rec.id,
            did: did.clone(),
        }])
        .await
        .expect("index");

        let ops = db.get_raw_operations_by_did(&did).await.expect("fetch by did");
        assert_eq!(ops.len(), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn get_raw_operations_by_did_returns_empty_for_unknown_did() {
        let (_tmp_dir, db) = setup_db().await;
        let rec = insert_one(&db, 10, 0, 0).await;
        let did = did_from_signed_op(&rec.signed_operation);

        // No indexing → view has no DID mapping
        let ops = db.get_raw_operations_by_did(&did).await.expect("fetch by did");
        assert!(ops.is_empty());
    }

    // ── RawOperationRepo: get_raw_operation_vdr_by_operation_hash ──

    #[tokio::test(flavor = "multi_thread")]
    async fn get_raw_operation_vdr_by_operation_hash_returns_none_when_not_found() {
        let (_tmp_dir, db) = setup_db().await;
        let hash = sha256([0u8; 32]);
        let result = db.get_raw_operation_vdr_by_operation_hash(&hash).await.expect("fetch");
        assert!(result.is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn get_raw_operation_vdr_by_operation_hash_returns_record() {
        let (_tmp_dir, db) = setup_db().await;
        let rec = insert_one(&db, 10, 0, 0).await;
        let did = did_from_signed_op(&rec.signed_operation);

        let operation_hash = sha256([1u8; 32]);
        let init_operation_hash = sha256([2u8; 32]);

        db.insert_indexed_operations(vec![IndexedOperation::Vdr {
            raw_operation_id: rec.id,
            operation_hash: operation_hash.to_vec(),
            init_operation_hash: init_operation_hash.to_vec(),
            prev_operation_hash: None,
            did: did.clone(),
        }])
        .await
        .expect("index");

        let result = db
            .get_raw_operation_vdr_by_operation_hash(&operation_hash)
            .await
            .expect("fetch");
        assert!(result.is_some());
    }

    // ── RawOperationRepo: get_raw_operations_by_tx_id ──

    #[tokio::test(flavor = "multi_thread")]
    async fn get_raw_operations_by_tx_id_returns_empty_for_unknown() {
        let (_tmp_dir, db) = setup_db().await;
        let tx_id = TxId::from(sha256([99u8; 32]));
        let result = db.get_raw_operations_by_tx_id(&tx_id).await.expect("fetch");
        assert!(result.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn get_raw_operations_by_tx_id_returns_matching_records() {
        let (_tmp_dir, db) = setup_db().await;
        let rec = insert_one(&db, 10, 0, 0).await;
        let did = did_from_signed_op(&rec.signed_operation);
        let tx_id = rec.metadata.block_metadata.tx_id.clone();

        // Index as SSI so the view maps DID
        db.insert_indexed_operations(vec![IndexedOperation::Ssi {
            raw_operation_id: rec.id,
            did: did.clone(),
        }])
        .await
        .expect("index");

        let result = db.get_raw_operations_by_tx_id(&tx_id).await.expect("fetch");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, did);
    }

    // ── RawOperationRepo: get_raw_operation_by_operation_id ──

    #[tokio::test(flavor = "multi_thread")]
    async fn get_raw_operation_by_operation_id_returns_none_for_unknown() {
        let (_tmp_dir, db) = setup_db().await;
        let op_id = identus_did_prism::did::operation::OperationId::from(sha256([0u8; 32]));
        let result = db.get_raw_operation_by_operation_id(&op_id).await.expect("fetch");
        assert!(result.is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn get_raw_operation_by_operation_id_returns_matching_record() {
        let (_tmp_dir, db) = setup_db().await;
        let rec = insert_one(&db, 10, 0, 0).await;
        let did = did_from_signed_op(&rec.signed_operation);
        let op_id = rec.signed_operation.operation_id();

        // Index as SSI so the view maps DID
        db.insert_indexed_operations(vec![IndexedOperation::Ssi {
            raw_operation_id: rec.id,
            did: did.clone(),
        }])
        .await
        .expect("index");

        let result = db.get_raw_operation_by_operation_id(&op_id).await.expect("fetch");
        assert!(result.is_some());
        let (fetched_rec, fetched_did) = result.unwrap();
        assert_eq!(fetched_did, did);
        assert_eq!(fetched_rec.id.as_ref(), rec.id.as_ref());
    }

    // ── IndexerStateRepo: get_last_indexed_block ──

    #[tokio::test(flavor = "multi_thread")]
    async fn get_last_indexed_block_returns_none_when_empty() {
        let (_tmp_dir, db) = setup_db().await;
        let result = db.get_last_indexed_block().await.expect("fetch");
        assert!(result.is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn get_last_indexed_block_returns_highest_fully_indexed_block() {
        let (_tmp_dir, db) = setup_db().await;
        let op = new_create_did_signed_operation();

        db.insert_raw_operations(vec![
            (dummy_metadata(1, 0, 0), op.clone()),
            (dummy_metadata(2, 0, 0), op.clone()),
            (dummy_metadata(3, 0, 0), op.clone()),
        ])
        .await
        .expect("insert");

        // Index blocks 1 and 3 (skip 2)
        let unindexed = db.get_raw_operations_unindexed().await.expect("fetch");
        let did = did_from_signed_op(&unindexed[0].signed_operation);
        for rec in &unindexed {
            if rec.metadata.block_metadata.block_number.inner() == 2u64 {
                continue;
            }
            db.insert_indexed_operations(vec![IndexedOperation::Ssi {
                raw_operation_id: rec.id,
                did: did.clone(),
            }])
            .await
            .expect("index");
        }

        // The query excludes block_numbers that have ANY unindexed operations.
        // Block 2 has unindexed ops, so it is excluded. Blocks 1 and 3 are fully
        // indexed, so the highest is block 3.
        let result = db.get_last_indexed_block().await.expect("fetch");
        assert_eq!(result, Some((3u64.into(), 3u64.into())));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn get_last_indexed_block_returns_highest_when_all_indexed() {
        let (_tmp_dir, db) = setup_db().await;
        let op = new_create_did_signed_operation();

        db.insert_raw_operations(vec![
            (dummy_metadata(5, 0, 0), op.clone()),
            (dummy_metadata(10, 0, 0), op.clone()),
        ])
        .await
        .expect("insert");

        let unindexed = db.get_raw_operations_unindexed().await.expect("fetch");
        let did = did_from_signed_op(&unindexed[0].signed_operation);
        for rec in &unindexed {
            db.insert_indexed_operations(vec![IndexedOperation::Ssi {
                raw_operation_id: rec.id,
                did: did.clone(),
            }])
            .await
            .expect("index");
        }

        let result = db.get_last_indexed_block().await.expect("fetch");
        assert_eq!(result, Some((10u64.into(), 10u64.into())));
    }

    // ── IndexerStateRepo: get_all_dids ──

    #[tokio::test(flavor = "multi_thread")]
    async fn get_all_dids_returns_empty_when_no_operations() {
        let (_tmp_dir, db) = setup_db().await;
        let result = db.get_all_dids(0, 10).await.expect("fetch");
        assert!(result.items.is_empty());
        assert_eq!(result.total_items, 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn get_all_dids_returns_indexed_dids() {
        let (_tmp_dir, db) = setup_db().await;
        let rec = insert_one(&db, 10, 0, 0).await;
        let did = did_from_signed_op(&rec.signed_operation);

        db.insert_indexed_operations(vec![IndexedOperation::Ssi {
            raw_operation_id: rec.id,
            did: did.clone(),
        }])
        .await
        .expect("index");

        let result = db.get_all_dids(0, 10).await.expect("fetch");
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.total_items, 1);
        assert_eq!(result.items[0], did);
    }

    // ── IndexerStateRepo: get_did_by_vdr_entry ──

    #[tokio::test(flavor = "multi_thread")]
    async fn get_did_by_vdr_entry_returns_none_when_not_found() {
        let (_tmp_dir, db) = setup_db().await;
        let hash = sha256([0u8; 32]);
        let result = db.get_did_by_vdr_entry(&hash).await.expect("fetch");
        assert!(result.is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn get_did_by_vdr_entry_returns_did_for_init_hash() {
        let (_tmp_dir, db) = setup_db().await;
        let rec = insert_one(&db, 10, 0, 0).await;
        let did = did_from_signed_op(&rec.signed_operation);
        let init_hash = sha256([42u8; 32]);

        db.insert_indexed_operations(vec![IndexedOperation::Vdr {
            raw_operation_id: rec.id,
            operation_hash: sha256([1u8; 32]).to_vec(),
            init_operation_hash: init_hash.to_vec(),
            prev_operation_hash: None,
            did: did.clone(),
        }])
        .await
        .expect("index");

        let result = db.get_did_by_vdr_entry(&init_hash).await.expect("fetch");
        assert_eq!(result, Some(did));
    }

    // ── IndexedOperationRepo: insert_indexed_operations ──

    #[tokio::test(flavor = "multi_thread")]
    async fn insert_indexed_operations_ssi_marks_as_indexed() {
        let (_tmp_dir, db) = setup_db().await;
        let rec = insert_one(&db, 10, 0, 0).await;
        let did = did_from_signed_op(&rec.signed_operation);

        db.insert_indexed_operations(vec![IndexedOperation::Ssi {
            raw_operation_id: rec.id,
            did,
        }])
        .await
        .expect("index");

        // Should be marked as indexed now
        let unindexed = db.get_raw_operations_unindexed().await.expect("fetch");
        assert!(unindexed.is_empty());

        // SSI entry should exist
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM indexed_ssi_operation")
            .fetch_one(&db.pool)
            .await
            .expect("count");
        assert_eq!(count, 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn insert_indexed_operations_vdr_inserts_vdr_record() {
        let (_tmp_dir, db) = setup_db().await;
        let rec = insert_one(&db, 10, 0, 0).await;
        let did = did_from_signed_op(&rec.signed_operation);

        db.insert_indexed_operations(vec![IndexedOperation::Vdr {
            raw_operation_id: rec.id,
            operation_hash: sha256([1u8; 32]).to_vec(),
            init_operation_hash: sha256([2u8; 32]).to_vec(),
            prev_operation_hash: Some(sha256([3u8; 32]).to_vec()),
            did,
        }])
        .await
        .expect("index");

        let unindexed = db.get_raw_operations_unindexed().await.expect("fetch");
        assert!(unindexed.is_empty());

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM indexed_vdr_operation")
            .fetch_one(&db.pool)
            .await
            .expect("count");
        assert_eq!(count, 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn insert_indexed_operations_ignored_marks_as_indexed_without_inserting() {
        let (_tmp_dir, db) = setup_db().await;
        let rec = insert_one(&db, 10, 0, 0).await;

        db.insert_indexed_operations(vec![IndexedOperation::Ignored {
            raw_operation_id: rec.id,
        }])
        .await
        .expect("index");

        // Marked as indexed
        let unindexed = db.get_raw_operations_unindexed().await.expect("fetch");
        assert!(unindexed.is_empty());

        // No SSI or VDR entries
        let ssi_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM indexed_ssi_operation")
            .fetch_one(&db.pool)
            .await
            .expect("count");
        let vdr_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM indexed_vdr_operation")
            .fetch_one(&db.pool)
            .await
            .expect("count");
        assert_eq!(ssi_count, 0);
        assert_eq!(vdr_count, 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn insert_indexed_operations_vdr_with_no_prev_hash() {
        let (_tmp_dir, db) = setup_db().await;
        let rec = insert_one(&db, 10, 0, 0).await;
        let did = did_from_signed_op(&rec.signed_operation);

        db.insert_indexed_operations(vec![IndexedOperation::Vdr {
            raw_operation_id: rec.id,
            operation_hash: sha256([1u8; 32]).to_vec(),
            init_operation_hash: sha256([2u8; 32]).to_vec(),
            prev_operation_hash: None,
            did,
        }])
        .await
        .expect("index");

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM indexed_vdr_operation")
            .fetch_one(&db.pool)
            .await
            .expect("count");
        assert_eq!(count, 1);
    }

    // ── DltCursorRepo: get_cursor / set_cursor ──

    #[tokio::test(flavor = "multi_thread")]
    async fn get_cursor_returns_none_initially() {
        let (_tmp_dir, db) = setup_db().await;
        let result = db.get_cursor().await.expect("fetch");
        assert!(result.is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn set_and_get_cursor_roundtrip() {
        let (_tmp_dir, db) = setup_db().await;
        let cursor = DltCursor {
            slot: 42,
            block_hash: vec![1, 2, 3],
            cbt: None,
            blockfrost_page: None,
        };
        db.set_cursor(cursor.clone()).await.expect("set");
        let result = db.get_cursor().await.expect("get");
        assert_eq!(result, Some(cursor));
    }

    // Regression test: set_cursor must replace any prior cursor (not leave stale
    // rows behind), so the indexer resumes from the latest slot on restart.
    // The original bug: lazybe/sea_query encoded the uuid id as a string while
    // the column is BLOB, so the per-row delete never matched (see set_cursor).
    #[tokio::test(flavor = "multi_thread")]
    async fn set_cursor_replaces_previous_cursor() {
        let (_tmp_dir, db) = setup_db().await;

        db.set_cursor(DltCursor {
            slot: 1,
            block_hash: vec![1],
            cbt: None,
            blockfrost_page: None,
        })
        .await
        .expect("set 1");

        db.set_cursor(DltCursor {
            slot: 99,
            block_hash: vec![2],
            cbt: None,
            blockfrost_page: Some(5),
        })
        .await
        .expect("set 2");

        // get_cursor returns the latest cursor, not the first.
        let result = db.get_cursor().await.expect("get");
        let cursor = result.expect("cursor exists");
        assert_eq!(cursor.slot, 99);
        assert_eq!(cursor.block_hash, vec![2]);
        assert_eq!(cursor.blockfrost_page, Some(5));

        // Only one cursor row remains — the previous one was deleted.
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM dlt_cursor")
            .fetch_one(&db.pool)
            .await
            .expect("count");
        assert_eq!(count, 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn set_cursor_with_blockfrost_page_roundtrip() {
        let (_tmp_dir, db) = setup_db().await;
        let cursor = DltCursor {
            slot: 100,
            block_hash: vec![0xAA, 0xBB],
            cbt: None,
            blockfrost_page: Some(3),
        };
        db.set_cursor(cursor.clone()).await.expect("set");
        let result = db.get_cursor().await.expect("get");
        assert_eq!(result, Some(cursor));
    }
}
