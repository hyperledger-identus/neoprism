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
    use std::fs;
    use std::path::Path;

    use super::*;

    fn assert_backend<T: StorageBackend>() {}

    #[test]
    fn postgres_backend_implements_storage_backend() {
        assert_backend::<PostgresDb>();
    }

    #[test]
    fn sqlite_and_postgres_migrations_are_in_sync() {
        fn collect(dir: &str) -> Vec<String> {
            let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
            let mut names = fs::read_dir(&manifest_dir)
                .unwrap_or_else(|_| panic!("failed to read {}", manifest_dir.display()))
                .filter_map(|entry| {
                    entry.ok().and_then(|e| {
                        let file_name = e.file_name();
                        let name = file_name.to_string_lossy().to_string();
                        if name.ends_with(".sql") { Some(name) } else { None }
                    })
                })
                .collect::<Vec<_>>();
            names.sort();
            names
        }

        let postgres = collect("migrations/postgres");
        let sqlite = collect("migrations/sqlite");
        assert_eq!(
            postgres, sqlite,
            "Postgres and SQLite migrations differ: {:?} vs {:?}",
            postgres, sqlite
        );
    }
}
