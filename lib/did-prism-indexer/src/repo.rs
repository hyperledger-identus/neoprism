use std::sync::Arc;

use identus_apollo::hash::Sha256Digest;
use identus_did_prism::did::CanonicalPrismDid;
use identus_did_prism::dlt::{BlockNo, DltCursor, OperationMetadata, SlotNo};
use identus_did_prism::prelude::*;
use identus_did_prism::utils::paging::Paginated;
use uuid::Uuid;

#[derive(Clone, Debug, Copy, derive_more::From, derive_more::Into, derive_more::AsRef)]
pub struct RawOperationId(Uuid);

pub enum IndexedOperation {
    Ssi {
        raw_operation_id: RawOperationId,
        did: CanonicalPrismDid,
    },
    Vdr {
        raw_operation_id: RawOperationId,
        operation_hash: Vec<u8>,
        init_operation_hash: Vec<u8>,
        prev_operation_hash: Option<Vec<u8>>,
        did: CanonicalPrismDid,
    },
    Ignored {
        raw_operation_id: RawOperationId,
    },
}

impl IndexedOperation {
    pub fn raw_operation_id(&self) -> &RawOperationId {
        match self {
            IndexedOperation::Ssi { raw_operation_id, .. } => raw_operation_id,
            IndexedOperation::Vdr { raw_operation_id, .. } => raw_operation_id,
            IndexedOperation::Ignored { raw_operation_id } => raw_operation_id,
        }
    }
}

#[async_trait::async_trait]
pub trait RawOperationRepo {
    type Error: std::error::Error;

    async fn get_raw_operations_unindexed(
        &self,
    ) -> Result<Vec<(RawOperationId, OperationMetadata, SignedPrismOperation)>, Self::Error>;

    async fn get_raw_operations_by_did(
        &self,
        did: &CanonicalPrismDid,
    ) -> Result<Vec<(RawOperationId, OperationMetadata, SignedPrismOperation)>, Self::Error>;

    async fn get_raw_operation_vdr_by_operation_hash(
        &self,
        operation_hash: &Sha256Digest,
    ) -> Result<Option<(RawOperationId, OperationMetadata, SignedPrismOperation)>, Self::Error>;

    async fn insert_raw_operations(
        &self,
        operations: Vec<(OperationMetadata, SignedPrismOperation)>,
    ) -> Result<(), Self::Error>;
}

#[async_trait::async_trait]
pub trait IndexedOperationRepo {
    type Error: std::error::Error;

    async fn insert_indexed_operations(&self, operations: Vec<IndexedOperation>) -> Result<(), Self::Error>;
}

#[async_trait::async_trait]
pub trait IndexerStateRepo {
    type Error: std::error::Error;

    async fn get_last_indexed_block(&self) -> Result<Option<(SlotNo, BlockNo)>, Self::Error>;

    async fn get_all_dids(&self, page: u32, page_size: u32) -> Result<Paginated<CanonicalPrismDid>, Self::Error>;

    async fn get_did_by_vdr_entry(
        &self,
        operation_hash: &Sha256Digest,
    ) -> Result<Option<CanonicalPrismDid>, Self::Error>;
}

#[async_trait::async_trait]
pub trait DltCursorRepo {
    type Error: std::error::Error;

    async fn set_cursor(&self, cursor: DltCursor) -> Result<(), Self::Error>;
    async fn get_cursor(&self) -> Result<Option<DltCursor>, Self::Error>;
}

#[async_trait::async_trait]
impl<T> RawOperationRepo for Arc<T>
where
    T: RawOperationRepo + Send + Sync + ?Sized,
    <T as RawOperationRepo>::Error: Send + Sync,
{
    type Error = T::Error;

    async fn get_raw_operations_unindexed(
        &self,
    ) -> Result<Vec<(RawOperationId, OperationMetadata, SignedPrismOperation)>, Self::Error> {
        self.as_ref().get_raw_operations_unindexed().await
    }

    async fn get_raw_operations_by_did(
        &self,
        did: &CanonicalPrismDid,
    ) -> Result<Vec<(RawOperationId, OperationMetadata, SignedPrismOperation)>, Self::Error> {
        self.as_ref().get_raw_operations_by_did(did).await
    }

    async fn get_raw_operation_vdr_by_operation_hash(
        &self,
        operation_hash: &Sha256Digest,
    ) -> Result<Option<(RawOperationId, OperationMetadata, SignedPrismOperation)>, Self::Error> {
        self.as_ref()
            .get_raw_operation_vdr_by_operation_hash(operation_hash)
            .await
    }

    async fn insert_raw_operations(
        &self,
        operations: Vec<(OperationMetadata, SignedPrismOperation)>,
    ) -> Result<(), Self::Error> {
        self.as_ref().insert_raw_operations(operations).await
    }
}

#[async_trait::async_trait]
impl<T> IndexedOperationRepo for Arc<T>
where
    T: IndexedOperationRepo + Send + Sync + ?Sized,
    <T as IndexedOperationRepo>::Error: Send + Sync,
{
    type Error = T::Error;

    async fn insert_indexed_operations(&self, operations: Vec<IndexedOperation>) -> Result<(), Self::Error> {
        self.as_ref().insert_indexed_operations(operations).await
    }
}

#[async_trait::async_trait]
impl<T> IndexerStateRepo for Arc<T>
where
    T: IndexerStateRepo + Send + Sync + ?Sized,
    <T as IndexerStateRepo>::Error: Send + Sync,
{
    type Error = T::Error;

    async fn get_last_indexed_block(&self) -> Result<Option<(SlotNo, BlockNo)>, Self::Error> {
        self.as_ref().get_last_indexed_block().await
    }

    async fn get_all_dids(&self, page: u32, page_size: u32) -> Result<Paginated<CanonicalPrismDid>, Self::Error> {
        self.as_ref().get_all_dids(page, page_size).await
    }

    async fn get_did_by_vdr_entry(
        &self,
        operation_hash: &Sha256Digest,
    ) -> Result<Option<CanonicalPrismDid>, Self::Error> {
        self.as_ref().get_did_by_vdr_entry(operation_hash).await
    }
}

#[async_trait::async_trait]
impl<T> DltCursorRepo for Arc<T>
where
    T: DltCursorRepo + Send + Sync + ?Sized,
    <T as DltCursorRepo>::Error: Send + Sync,
{
    type Error = T::Error;

    async fn set_cursor(&self, cursor: DltCursor) -> Result<(), Self::Error> {
        self.as_ref().set_cursor(cursor).await
    }

    async fn get_cursor(&self) -> Result<Option<DltCursor>, Self::Error> {
        self.as_ref().get_cursor().await
    }
}
