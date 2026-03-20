//! Embedded wallet for Cardano Shelley key derivation.
//!
//! This module provides BIP39 mnemonic parsing and BIP32 hierarchical
//! deterministic key derivation for Cardano Shelley addresses.
//!
//! # Features
//!
//! - Parse BIP39 mnemonic phrases (12, 15, 18, 21, or 24 words)
//! - Derive payment and stake keys at Cardano Shelley paths
//! - Support for BIP39 passphrase (optional password)
//!
//! # Derivation Paths
//!
//! Cardano Shelley uses the following derivation paths:
//! - Payment: 1852H/1815H/0H/0/0
//! - Stake:   1852H/1815H/0H/2/0
//!
//! Where:
//! - 1852H is the purpose (Cardano Shelley)
//! - 1815H is the coin type (ADA)
//! - 0H is the account index
//! - 0/2 is the change type (external/staking)
//! - 0 is the address index
//!
//! # Example
//!
//! ```ignore
//! use identus_did_prism_submitter::dlt::embedded_wallet::Wallet;
//!
//! let wallet = Wallet::from_mnemonic(
//!     "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
//!     None
//! )?;
//!
//! // Get public keys for address generation
//! let payment_pubkey = wallet.payment_public_key()?;
//! let stake_pubkey = wallet.stake_public_key()?;
//!
//! // Get private keys for signing
//! let payment_privkey = wallet.payment_private_key()?;
//! ```

mod error;
mod key_derivation;
mod mnemonic;
mod wallet;

pub use error::Error;
pub use wallet::Wallet;
