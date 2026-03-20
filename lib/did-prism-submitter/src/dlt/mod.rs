//! DLT submission implementations.
//!
//! Features `cardano-wallet` and `embedded-wallet` provide alternative submitter
//! implementations and are mutually exclusive at runtime. Only one should be
//! enabled when building the node binary.

#[cfg(feature = "cardano-wallet")]
pub mod cardano_wallet;

// TODO: US-008 will add embedded_wallet module
// #[cfg(feature = "embedded-wallet")]
// pub mod embedded_wallet;
