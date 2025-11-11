use identus_did_prism::did::Error as DidError;
use identus_did_prism::did::error::DidSyntaxError;
use identus_did_prism_indexer::repo::{DltCursorRepo, IndexedOperationRepo, IndexerStateRepo, RawOperationRepo};

pub mod backend;
mod entity;

pub use backend::postgres::PostgresDb;
#[cfg(feature = "sqlite-storage")]
pub use backend::sqlite::SqliteDb;

pub trait StorageBackend:
    RawOperationRepo<Error = Error>
    + IndexedOperationRepo<Error = Error>
    + IndexerStateRepo<Error = Error>
    + DltCursorRepo<Error = Error>
    + Send
    + Sync
    + 'static
{
}

impl<T> StorageBackend for T where
    T: RawOperationRepo<Error = Error>
        + IndexedOperationRepo<Error = Error>
        + IndexerStateRepo<Error = Error>
        + DltCursorRepo<Error = Error>
        + Send
        + Sync
        + 'static
{
}

#[derive(Debug, derive_more::From, derive_more::Display, derive_more::Error)]
pub enum Error {
    #[from]
    #[display("database connection error")]
    Db { source: sqlx::Error },
    #[from]
    #[display("database migration error")]
    DbMigration { source: sqlx::migrate::MigrateError },
    #[display("unable to decode to protobuf message into type {target_type} from stored data")]
    ProtobufDecode {
        source: protobuf::Error,
        target_type: &'static str,
    },
    #[from]
    #[display("failed to compute did index from signed-prism-operation")]
    DidIndexFromSignedPrismOperation { source: DidError },
    #[from]
    #[display("failed to decode did from stored data")]
    DidDecode { source: DidSyntaxError },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_backend<T: StorageBackend>() {}

    #[test]
    fn postgres_backend_implements_storage_backend() {
        assert_backend::<PostgresDb>();
    }
}
