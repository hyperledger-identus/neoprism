use identus_did_prism::utils::Location;

type StdError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, derive_more::Display, derive_more::Error)]
pub enum DltError {
    #[display("unable to initialize DLT source")]
    InitSource { source: StdError },
    #[display("timeout receiving event from DLT source {location}")]
    EventRecvTimeout { location: Location },
    #[display("event source has connectivity issue at {location}: {source}")]
    Connection { source: StdError, location: Location },
    #[display("handling DLT event failed {location}")]
    EventHandling { source: StdError, location: Location },
}

/// This is an internal error type that should be handled when streaming from DLT source.
#[allow(unused)]
#[derive(Debug, derive_more::Display, derive_more::Error)]
pub(crate) enum MetadataReadError {
    #[display("metadata is not a valid json on block {block_hash:?} tx {tx_idx:?}")]
    InvalidMetadataType {
        source: StdError,
        block_hash: Option<String>,
        tx_idx: Option<usize>,
    },
    #[display("failed to decode prism_block hex on block {block_hash:?} tx {tx_idx:?}")]
    PrismBlockHexDecode {
        source: identus_apollo::hex::Error,
        block_hash: Option<String>,
        tx_idx: Option<usize>,
    },
    #[display("failed to decode prism_block protobuf on block {block_hash:?} tx {tx_idx:?}")]
    PrismBlockProtoDecode {
        source: protobuf::Error,
        block_hash: Option<String>,
        tx_idx: Option<usize>,
    },
    #[display("timestamp {timestamp} on block {block_hash:?} tx {tx_idx:?} is invalid")]
    InvalidBlockTimestamp {
        block_hash: Option<String>,
        tx_idx: Option<usize>,
        timestamp: i64,
    },
    #[display("block property '{name}' is missing on block {block_hash:?} tx {tx_idx:?}")]
    MissingBlockProperty {
        block_hash: Option<String>,
        tx_idx: Option<usize>,
        name: &'static str,
    },
}
