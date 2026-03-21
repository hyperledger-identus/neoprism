//! Transaction builder for Cardano transactions with PRISM metadata.
//!
//! This module provides functionality to:
//! - Build Cardano transactions using pallas-txbuilder
//! - Include PRISM operations as transaction metadata
//! - Handle UTXO selection, fee calculation, and change outputs
//!
//! # Example
//!
//! ```ignore
//! use identus_did_prism_submitter::dlt::embedded_wallet::{
//!     tx_builder::{TransactionBuilder, PrismTransactionBuilder},
//!     Wallet, Utxo, BlockfrostUtxoConfig, fetch_utxos,
//! };
//!
//! // Create wallet from mnemonic
//! let wallet = Wallet::from_mnemonic(mnemonic, None)?;
//!
//! // Fetch UTXOs
//! let config = BlockfrostUtxoConfig { ... };
//! let utxos = fetch_utxos(&config, &wallet.payment_address(&network)?).await?;
//!
//! // Build transaction with PRISM metadata
//! let builder = PrismTransactionBuilder::new(
//!     wallet.payment_address(&network)?,
//!     wallet.stake_address(&network)?,
//!     network_id,
//! );
//! let tx = builder
//!     .utxos(utxos)
//!     .operations(operations)
//!     .output(destination_address, 2_000_000)?
//!     .build()?;
//! ```

use identus_did_prism::prelude::MessageExt;
use identus_did_prism::proto::prism::{PrismBlock, PrismObject};
use pallas_addresses_v1::Address as PallasAddress;
use pallas_codec_v1::minicbor;
use pallas_codec_v1::utils::{Bytes as PallasBytes, Int as PallasInt, KeyValuePairs};
use pallas_crypto_v1::hash::Hash;
use pallas_primitives_v1::alonzo::Metadatum;
use pallas_txbuilder_v1::{BuildConway, Input, Output, StagingTransaction};

use super::error::Error;
use super::utxo::Utxo;

/// Minimum ADA for a transaction output without tokens (in lovelace).
/// This is the Cardano protocol parameter MINIMUM_UTXO_VALUE.
pub const MINIMUM_UTXO_LOVELACE: u64 = 1_000_000;

/// Typical fee estimation for a simple transaction (in lovelace).
/// This is an upper bound for basic transaction fees.
pub const ESTIMATED_FEE_LOVELACE: u64 = 200_000;

/// PRISM metadata label used for PRISM operations.
/// This label is registered for PRISM protocol use on Cardano.
pub const PRISM_METADATA_LABEL: u64 = 21325;

/// Version of the PRISM metadata format.
pub const PRISM_METADATA_VERSION: u64 = 1;

/// Transaction builder for Cardano transactions.
///
/// This builder provides a high-level interface for constructing
/// Cardano transactions using pallas-txbuilder.
#[derive(Debug, Clone)]
pub struct TransactionBuilder {
    inputs: Vec<Input>,
    outputs: Vec<Output>,
    network_id: u8,
    change_address: Option<PallasAddress>,
    ttl: Option<u64>,
    valid_from_slot: Option<u64>,
}

impl TransactionBuilder {
    /// Create a new transaction builder.
    pub fn new() -> Self {
        Self {
            inputs: Vec::new(),
            outputs: Vec::new(),
            network_id: 0, // Default to testnet
            change_address: None,
            ttl: None,
            valid_from_slot: None,
        }
    }

    /// Set the network ID.
    ///
    /// - 0 = Testnet
    /// - 1 = Mainnet
    pub fn network_id(mut self, network_id: u8) -> Self {
        self.network_id = network_id;
        self
    }

    /// Set the change address for any remaining balance.
    pub fn change_address(mut self, address: PallasAddress) -> Self {
        self.change_address = Some(address);
        self
    }

    /// Set the transaction time-to-live (slot number).
    pub fn ttl(mut self, slot: u64) -> Self {
        self.ttl = Some(slot);
        self
    }

    /// Set the valid from slot.
    pub fn valid_from_slot(mut self, slot: u64) -> Self {
        self.valid_from_slot = Some(slot);
        self
    }

    /// Add a UTXO as an input to the transaction.
    ///
    /// # Arguments
    /// * `utxo` - The UTXO to add as input
    ///
    /// # Returns
    /// A new TransactionBuilder with the input added.
    pub fn add_input(mut self, utxo: &Utxo) -> Result<Self, Error> {
        let tx_hash: [u8; 32] = hex::decode(&utxo.tx_hash)
            .map_err(|_| Error::InvalidTxHash {
                hash: utxo.tx_hash.clone(),
            })?
            .try_into()
            .map_err(|_| Error::InvalidTxHashLength {
                hash: utxo.tx_hash.clone(),
            })?;

        let input = Input::new(Hash::from(tx_hash), utxo.output_index as u64);
        self.inputs.push(input);
        Ok(self)
    }

    /// Add multiple UTXOs as inputs.
    pub fn add_inputs(mut self, utxos: &[Utxo]) -> Result<Self, Error> {
        for utxo in utxos {
            self = self.add_input(utxo)?;
        }
        Ok(self)
    }

    /// Add an output to the transaction.
    ///
    /// # Arguments
    /// * `address` - The destination address
    /// * `lovelace` - Amount in lovelace
    ///
    /// # Returns
    /// A new TransactionBuilder with the output added.
    pub fn add_output(mut self, address: &str, lovelace: u64) -> Result<Self, Error> {
        let pallas_addr = PallasAddress::from_bech32(address).map_err(|e| Error::InvalidAddress {
            address: address.to_string(),
            reason: e.to_string(),
        })?;

        let output = Output::new(pallas_addr, lovelace);
        self.outputs.push(output);
        Ok(self)
    }

    /// Build the transaction into a staging transaction.
    ///
    /// This creates a `StagingTransaction` that can be further modified
    /// or built into a final transaction.
    pub fn build_staging(self) -> Result<StagingTransaction, Error> {
        let mut staging = StagingTransaction::new().network_id(self.network_id);

        // Set validity window if provided
        if let Some(valid_from) = self.valid_from_slot {
            staging = staging.valid_from_slot(valid_from);
        }
        if let Some(ttl) = self.ttl {
            staging = staging.invalid_from_slot(ttl);
        }

        // Add inputs
        for input in self.inputs {
            staging = staging.input(input);
        }

        // Add outputs
        for output in self.outputs {
            staging = staging.output(output);
        }

        // Add change address if set
        if let Some(change_addr) = self.change_address {
            staging = staging.change_address(change_addr);
        }

        Ok(staging)
    }

    /// Build and finalize the transaction.
    ///
    /// This creates a fully built transaction ready for signing.
    pub fn build(self) -> Result<pallas_txbuilder_v1::BuiltTransaction, Error> {
        let staging = self.build_staging()?;
        staging
            .build_conway_raw()
            .map_err(|e| Error::TransactionBuild { source: e })
    }
}

impl Default for TransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Transaction builder specifically for PRISM operations.
///
/// This builder extends the basic transaction builder with PRISM-specific
/// functionality, including metadata encoding and validation.
#[derive(Debug, Clone)]
pub struct PrismTransactionBuilder {
    builder: TransactionBuilder,
    operations: Vec<identus_did_prism::prelude::SignedPrismOperation>,
    destination_address: Option<String>,
    destination_amount: Option<u64>,
}

impl PrismTransactionBuilder {
    /// Create a new PRISM transaction builder.
    ///
    /// # Arguments
    /// * `payment_address` - The wallet's payment address for change
    /// * `stake_address` - The wallet's stake address
    /// * `network_id` - Network ID (0 = testnet, 1 = mainnet)
    pub fn new(_payment_address: String, _stake_address: String, network_id: u8) -> Self {
        let builder = TransactionBuilder::new().network_id(network_id);

        Self {
            builder,
            operations: Vec::new(),
            destination_address: None,
            destination_amount: None,
        }
    }

    /// Set the UTXOs to use as inputs.
    ///
    /// # Arguments
    /// * `utxos` - Selected UTXOs (from Blockfrost or other source)
    pub fn utxos(mut self, utxos: Vec<Utxo>) -> Result<Self, Error> {
        self.builder = self.builder.add_inputs(&utxos)?;
        Ok(self)
    }

    /// Set the operations to include in the transaction metadata.
    pub fn operations(mut self, operations: Vec<identus_did_prism::prelude::SignedPrismOperation>) -> Self {
        self.operations = operations;
        self
    }

    /// Set the destination output.
    ///
    /// # Arguments
    /// * `address` - Destination address
    /// * `lovelace` - Amount in lovelace
    pub fn output(mut self, address: &str, lovelace: u64) -> Result<Self, Error> {
        // Validate minimum ADA requirement
        if lovelace < MINIMUM_UTXO_LOVELACE {
            return Err(Error::InsufficientUtxoValue {
                need: MINIMUM_UTXO_LOVELACE,
                actual: lovelace,
            });
        }

        self.destination_address = Some(address.to_string());
        self.destination_amount = Some(lovelace);
        self.builder = self.builder.add_output(address, lovelace)?;
        Ok(self)
    }

    /// Set the change address.
    pub fn change_address(mut self, address: &str) -> Result<Self, Error> {
        let pallas_addr = PallasAddress::from_bech32(address).map_err(|e| Error::InvalidAddress {
            address: address.to_string(),
            reason: e.to_string(),
        })?;

        self.builder = self.builder.change_address(pallas_addr);
        Ok(self)
    }

    /// Set the transaction validity window.
    pub fn validity_window(mut self, valid_from: u64, ttl: u64) -> Self {
        self.builder = self.builder.valid_from_slot(valid_from).ttl(ttl);
        self
    }

    /// Calculate the total input value from UTXOs.
    ///
    /// This is useful for determining the change amount.
    pub fn total_input_value(utxos: &[Utxo]) -> u64 {
        utxos.iter().map(|u| u.ada).sum()
    }

    /// Calculate the total output value.
    ///
    /// This includes the destination output and any other outputs.
    pub fn total_output_value(destination_amount: u64) -> u64 {
        destination_amount + ESTIMATED_FEE_LOVELACE
    }

    /// Calculate the estimated change amount.
    pub fn calculate_change(input_value: u64, output_value: u64) -> u64 {
        input_value.saturating_sub(output_value)
    }

    /// Build the PRISM transaction.
    ///
    /// This creates a fully built transaction with PRISM metadata included.
    ///
    /// # Returns
    /// A built transaction ready for signing.
    ///
    /// # Errors
    /// Returns an error if:
    /// - No operations are provided
    /// - No destination is set
    /// - Transaction building fails
    pub fn build(self) -> Result<pallas_txbuilder_v1::BuiltTransaction, Error> {
        if self.operations.is_empty() {
            return Err(Error::NoOperations);
        }

        let Some(_dest_address) = &self.destination_address else {
            return Err(Error::MissingDestination);
        };

        // Encode PRISM operations as metadata
        let metadata = encode_prism_metadata(&self.operations)?;

        // Build staging transaction
        let mut staging = self.builder.build_staging()?;

        // Add PRISM metadata to auxiliary data
        let metadata_bytes = encode_metadata_to_cbor(&metadata)?;
        staging = staging.add_auxiliary_data(metadata_bytes);

        // Build the final transaction
        staging
            .build_conway_raw()
            .map_err(|e| Error::TransactionBuild { source: e })
    }
}

/// Encode PRISM operations as metadata using the PRISM format.
///
/// This matches the format used by the cardano-wallet sink:
/// ```json
/// {
///   "21325": {
///     "map": [
///       { "k": { "string": "v" }, "v": { "int": 1 } },
///       { "k": { "string": "c" }, "v": { "list": [...] } }
///     ]
///   }
/// }
/// ```
pub fn encode_prism_metadata(
    operations: &[identus_did_prism::prelude::SignedPrismOperation],
) -> Result<Metadatum, Error> {
    // Create PRISM object with operations
    let prism_object = PrismObject {
        block_content: Some(PrismBlock {
            operations: operations.to_vec(),
            special_fields: Default::default(),
        })
        .into(),
        special_fields: Default::default(),
    };

    // Encode to bytes
    let bytes = prism_object.encode_to_vec();

    // Split into 64-byte chunks and encode each as raw bytes
    // The indexer (oura) expects array of raw byte chunks
    let chunks: Vec<Metadatum> = bytes
        .chunks(64)
        .map(|chunk| Metadatum::Bytes(PallasBytes::from(chunk.to_vec())))
        .collect();

    // Build the PRISM metadata structure
    let inner_map: KeyValuePairs<Metadatum, Metadatum> = vec![
        (
            Metadatum::Text("v".to_string()),
            Metadatum::Int(PallasInt::from(PRISM_METADATA_VERSION as i64)),
        ),
        (Metadatum::Text("c".to_string()), Metadatum::Array(chunks)),
    ]
    .into();

    let metadata: KeyValuePairs<Metadatum, Metadatum> = vec![(
        Metadatum::Int(PallasInt::from(PRISM_METADATA_LABEL as i64)),
        Metadatum::Map(inner_map),
    )]
    .into();

    Ok(Metadatum::Map(metadata))
}

/// Encode metadata to CBOR bytes for inclusion in transaction.
pub fn encode_metadata_to_cbor(metadata: &Metadatum) -> Result<Vec<u8>, Error> {
    minicbor::to_vec(metadata).map_err(|e| Error::InvalidMetadataCbor {
        encoding_error: e.to_string(),
    })
}

/// Check if output meets minimum ADA requirement.
pub fn validate_output_minimum_ada(lovelace: u64) -> bool {
    lovelace >= MINIMUM_UTXO_LOVELACE
}

/// Estimate transaction fee based on inputs and outputs.
///
/// This is a rough estimation. The actual fee depends on the transaction
/// size and current protocol parameters.
pub fn estimate_transaction_fee(num_inputs: usize, num_outputs: usize) -> u64 {
    // Base fee + per-input fee + per-output fee
    let base = 155_381;
    let per_input = 10_397;
    let per_output = 65_965;

    base + (num_inputs as u64 * per_input) + (num_outputs as u64 * per_output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimum_utxo_constant() {
        assert_eq!(MINIMUM_UTXO_LOVELACE, 1_000_000);
    }

    #[test]
    fn test_estimated_fee_constant() {
        assert_eq!(ESTIMATED_FEE_LOVELACE, 200_000);
    }

    #[test]
    fn test_prism_metadata_label() {
        assert_eq!(PRISM_METADATA_LABEL, 21325);
    }

    #[test]
    fn test_prism_metadata_version() {
        assert_eq!(PRISM_METADATA_VERSION, 1);
    }

    #[test]
    fn test_transaction_builder_new() {
        let builder = TransactionBuilder::new();
        assert!(builder.inputs.is_empty());
        assert!(builder.outputs.is_empty());
        assert_eq!(builder.network_id, 0);
        assert!(builder.change_address.is_none());
    }

    #[test]
    fn test_transaction_builder_network_id() {
        let builder = TransactionBuilder::new().network_id(1);
        assert_eq!(builder.network_id, 1);
    }

    #[test]
    fn test_transaction_builder_default() {
        let _ = TransactionBuilder::default();
    }

    #[test]
    fn test_prism_transaction_builder_new() {
        let builder = PrismTransactionBuilder::new(
            "addr_test1qr73wchgpz5gny0q56vq4eplexk80my3qr3qn8lw2vpnv6ks5mqzf9ncx4h8cc4x7tqvh6hyvsew0m80vjr6n5t0fqu8xg9q2q5g80".to_string(),
            "stake_test1uzlq3ksrm8cteappe9r4qe5kwpkqw8x5c5v5y8xz5hq6ks5mqzf9nc".to_string(),
            0,
        );
        assert!(builder.operations.is_empty());
        assert!(builder.destination_address.is_none());
    }

    #[test]
    fn test_total_input_value() {
        let utxos = vec![
            Utxo {
                tx_hash: "tx1".to_string(),
                output_index: 0,
                ada: 3_000_000,
            },
            Utxo {
                tx_hash: "tx2".to_string(),
                output_index: 1,
                ada: 2_000_000,
            },
        ];

        assert_eq!(PrismTransactionBuilder::total_input_value(&utxos), 5_000_000);
    }

    #[test]
    fn test_total_input_value_empty() {
        assert_eq!(PrismTransactionBuilder::total_input_value(&[]), 0);
    }

    #[test]
    fn test_total_output_value() {
        assert_eq!(PrismTransactionBuilder::total_output_value(1_000_000), 1_200_000);
    }

    #[test]
    fn test_calculate_change_sufficient() {
        assert_eq!(
            PrismTransactionBuilder::calculate_change(5_000_000, 1_000_000),
            4_000_000
        );
    }

    #[test]
    fn test_calculate_change_insufficient() {
        assert_eq!(PrismTransactionBuilder::calculate_change(1_000_000, 5_000_000), 0);
    }

    #[test]
    fn test_encode_metadata_to_cbor_empty_array() {
        let bytes = encode_metadata_to_cbor(&Metadatum::Array(vec![])).expect("encoding should succeed");
        // CBOR empty array encoding: 0x80
        assert_eq!(bytes, &[0x80]);
    }
}
