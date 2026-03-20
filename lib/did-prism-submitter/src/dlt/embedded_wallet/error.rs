//! Error types for embedded wallet operations.

use blockfrost::BlockfrostError;

#[derive(Debug, derive_more::Display, derive_more::Error, derive_more::From)]
pub enum Error {
    #[display("invalid mnemonic: {reason}")]
    InvalidMnemonic { reason: &'static str },

    #[display("insufficient UTXO balance: need {need} lovelace but have {available}")]
    InsufficientBalance { need: u64, available: u64 },

    #[display("blockfrost API error for address {address}: {source}")]
    Blockfrost {
        address: String,
        #[from]
        source: BlockfrostError,
    },
}
