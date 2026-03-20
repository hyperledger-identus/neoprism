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
//! - Fetch UTXOs from Blockfrost API for transaction building
//! - Largest-first UTXO selection algorithm
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
//! use identus_did_prism_submitter::dlt::embedded_wallet::{Wallet, utxo::{BlockfrostUtxoConfig, fetch_utxos, select_largest_first}};
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
//! // Fetch UTXOs from Blockfrost
//! let config = BlockfrostUtxoConfig {
//!     api_url: "https://cardano-preview.blockfrost.io/api/v0".to_string(),
//!     project_id: "preview...".to_string(),
//! };
//! let address = wallet.payment_address(&config.api_url)?;
//! let utxos = fetch_utxos(&config, &address).await?;
//!
//! // Select UTXOs for an amount
//! let selected = select_largest_first(&mut utxos.clone(), 2_000_000)?;
//! ```

mod error;
mod key_derivation;
mod mnemonic;
mod utxo;
mod wallet;

pub use error::Error;
pub use utxo::{
    BlockfrostUtxoConfig, Utxo, create_blockfrost_api, fetch_utxos, has_sufficient_balance, select_largest_first,
    sort_by_value_descending, total_balance,
};
pub use wallet::Wallet;
