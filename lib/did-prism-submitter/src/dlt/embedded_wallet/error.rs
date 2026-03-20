//! Error types for embedded wallet key derivation.

#[derive(Debug, derive_more::Display, derive_more::Error)]
pub enum Error {
    #[display("invalid mnemonic: {reason}")]
    InvalidMnemonic { reason: &'static str },
}
