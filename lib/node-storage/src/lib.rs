use identus_did_prism::did::Error as DidError;
use identus_did_prism::did::error::DidSyntaxError;

mod entity;
mod pg;

pub use pg::PostgresDb;

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
    #[display("cannot compute did index from signed-prism-operation")]
    DidIndexFromSignedPrismOperation { source: DidError },
    #[from]
    #[display("cannot decode did from stored data")]
    DidDecode { source: DidSyntaxError },
}
