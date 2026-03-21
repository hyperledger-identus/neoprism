//! Error types for embedded wallet operations.

use blockfrost::BlockfrostError;
use pallas_txbuilder_v1::TxBuilderError as PallasTxBuilderError;

#[derive(Debug, derive_more::Display, derive_more::Error)]
pub enum Error {
    #[display("invalid mnemonic: {reason}")]
    InvalidMnemonic { reason: &'static str },

    #[display("insufficient UTXO balance: need {need} lovelace but have {available}")]
    InsufficientBalance { need: u64, available: u64 },

    #[display("blockfrost API error for address {address}: {source}")]
    Blockfrost { address: String, source: BlockfrostError },

    #[display("failed to build transaction: {source}")]
    TransactionBuild { source: PallasTxBuilderError },

    #[display("invalid address {address}: {reason}")]
    InvalidAddress { address: String, reason: String },

    #[display("insufficient UTXO value: need at least {need} lovelace but got {actual}")]
    InsufficientUtxoValue { need: u64, actual: u64 },

    #[display("no operations provided for PRISM transaction")]
    NoOperations,

    #[display("no destination address set for PRISM transaction")]
    MissingDestination,

    #[display("invalid transaction hash: {hash}")]
    InvalidTxHash { hash: String },

    #[display("transaction hash has invalid length: {hash}")]
    InvalidTxHashLength { hash: String },

    #[display("failed to encode metadata to CBOR: {encoding_error}")]
    InvalidMetadataCbor { encoding_error: String },
}
