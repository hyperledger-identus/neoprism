//! Blockfrost UTXO query and selection for embedded wallet.
//!
//! This module provides functionality to:
//! - Fetch UTXOs from Blockfrost API for a given address
//! - Parse Blockfrost responses into internal UTXO representation
//! - Select UTXOs using a largest-first algorithm for transaction building

use blockfrost::{BlockFrostSettings, BlockfrostAPI, Pagination};

use super::error::Error;

/// Configuration for Blockfrost UTXO queries.
#[derive(Debug, Clone)]
pub struct BlockfrostUtxoConfig {
    /// Blockfrost API URL (e.g., "https://cardano-preview.blockfrost.io/api/v0")
    pub api_url: String,
    /// Blockfrost project ID (API key)
    pub project_id: String,
}

/// A single UTXO (unspent transaction output).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Utxo {
    /// Transaction hash (hex-encoded)
    pub tx_hash: String,
    /// Output index within the transaction
    pub output_index: u32,
    /// ADA amount in lovelace (1 ADA = 1,000,000 lovelace)
    pub ada: u64,
}

impl Utxo {
    /// Calculate the total ADA value of this UTXO.
    pub fn value(&self) -> u64 {
        self.ada
    }
}

/// Create a new BlockfrostAPI instance with the given config.
pub fn create_blockfrost_api(config: &BlockfrostUtxoConfig) -> BlockfrostAPI {
    let mut settings = BlockFrostSettings::default();
    settings.base_url = Some(config.api_url.clone());
    BlockfrostAPI::new(&config.project_id, settings)
}

/// Fetch UTXOs for a given address from Blockfrost API.
///
/// # Arguments
/// * `config` - Blockfrost configuration (API URL and project ID)
/// * `address` - Cardano address to fetch UTXOs for
///
/// # Returns
/// A vector of UTXOs sorted by value descending (largest first).
///
/// # Errors
/// Returns `Error::Blockfrost` if the API call fails.
pub async fn fetch_utxos(config: &BlockfrostUtxoConfig, address: &str) -> Result<Vec<Utxo>, Error> {
    let api = create_blockfrost_api(config);
    fetch_utxos_with_api(&api, address).await
}

/// Fetch UTXOs using an existing BlockfrostAPI instance.
///
/// This is useful for testing or when you need to share an API instance.
pub async fn fetch_utxos_with_api(api: &BlockfrostAPI, address: &str) -> Result<Vec<Utxo>, Error> {
    let pagination = Pagination::all();

    // Blockfrost returns all UTXOs for an address (pagination handled internally)
    let utxo_inputs = api
        .addresses_utxos(address, pagination)
        .await
        .map_err(|source| Error::Blockfrost {
            address: address.to_string(),
            source,
        })?;

    let mut utxos = Vec::new();
    for input in utxo_inputs {
        let utxo = parse_utxo_from_input(input)?;
        utxos.push(utxo);
    }

    sort_by_value_descending(&mut utxos);
    Ok(utxos)
}

/// Parse a Blockfrost UTXO input into our internal representation.
fn parse_utxo_from_input(input: blockfrost_openapi::models::AddressUtxoContentInner) -> Result<Utxo, Error> {
    let output_index = input.output_index as u32;
    let tx_hash = input.tx_hash;

    // Extract ADA amount from the amount array
    // Blockfrost returns amounts as [{"unit": "lovelace", "quantity": "1000000"}, ...]
    let ada = extract_ada_amount(&input.amount);

    Ok(Utxo {
        tx_hash,
        output_index,
        ada,
    })
}

/// Extract the ADA (lovelace) amount from Blockfrost amount array.
fn extract_ada_amount(amounts: &[blockfrost_openapi::models::TxContentOutputAmountInner]) -> u64 {
    for amount in amounts {
        if amount.unit == "lovelace" {
            return amount.quantity.parse().unwrap_or(0);
        }
    }
    0
}

/// Sort UTXOs by value in descending order (largest first).
pub fn sort_by_value_descending(utxos: &mut [Utxo]) {
    utxos.sort_by_key(|b| std::cmp::Reverse(b.ada));
}

/// Select UTXOs using a largest-first algorithm.
///
/// This algorithm sorts UTXOs by value descending and selects them one by one
/// until the required amount is met. This minimizes the number of UTXOs used
/// and is suitable for most transaction scenarios.
///
/// # Arguments
/// * `utxos` - Available UTXOs (will be sorted in place)
/// * `amount_needed` - The minimum ADA required in lovelace
///
/// # Returns
/// Selected UTXOs that satisfy the amount requirement.
///
/// # Errors
/// Returns `Error::InsufficientBalance` if the total available UTXOs
/// don't meet the required amount.
pub fn select_largest_first(utxos: &mut [Utxo], amount_needed: u64) -> Result<Vec<Utxo>, Error> {
    sort_by_value_descending(utxos);

    let total_available: u64 = utxos.iter().map(|u| u.ada).sum();

    if total_available < amount_needed {
        return Err(Error::InsufficientBalance {
            need: amount_needed,
            available: total_available,
        });
    }

    let mut selected = Vec::new();
    let mut accumulated: u64 = 0;

    for utxo in utxos.iter() {
        selected.push(utxo.clone());
        accumulated += utxo.ada;

        if accumulated >= amount_needed {
            break;
        }
    }

    Ok(selected)
}

/// Get total ADA balance from a list of UTXOs.
pub fn total_balance(utxos: &[Utxo]) -> u64 {
    utxos.iter().map(|u| u.ada).sum()
}

/// Check if the UTXOs have sufficient balance for an amount.
pub fn has_sufficient_balance(utxos: &[Utxo], amount: u64) -> bool {
    total_balance(utxos) >= amount
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_utxo(tx_hash: &str, output_index: u32, ada: u64) -> Utxo {
        Utxo {
            tx_hash: tx_hash.to_string(),
            output_index,
            ada,
        }
    }

    #[test]
    fn test_select_largest_first_single_utxo_sufficient() {
        let mut utxos = vec![make_utxo("tx1", 0, 5_000_000)];

        let result = select_largest_first(&mut utxos, 1_000_000);

        assert!(result.is_ok());
        let selected = result.unwrap();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].ada, 5_000_000);
    }

    #[test]
    fn test_select_largest_first_multiple_utxos() {
        // UTXOs: 3 ADA, 2 ADA, 1 ADA
        let mut utxos = vec![
            make_utxo("tx1", 0, 3_000_000),
            make_utxo("tx2", 0, 2_000_000),
            make_utxo("tx3", 0, 1_000_000),
        ];

        // Need 4 ADA - should select 3 ADA and 2 ADA
        let result = select_largest_first(&mut utxos, 4_000_000);

        assert!(result.is_ok());
        let selected = result.unwrap();
        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].ada, 3_000_000); // largest first
        assert_eq!(selected[1].ada, 2_000_000);
    }

    #[test]
    fn test_select_largest_first_insufficient_balance() {
        let mut utxos = vec![make_utxo("tx1", 0, 1_000_000), make_utxo("tx2", 0, 500_000)];

        let result = select_largest_first(&mut utxos, 2_000_000);

        assert!(result.is_err());
        if let Err(Error::InsufficientBalance { need, available }) = result {
            assert_eq!(need, 2_000_000);
            assert_eq!(available, 1_500_000);
        } else {
            panic!("expected InsufficientBalance error");
        }
    }

    #[test]
    fn test_select_largest_first_exact_match() {
        let mut utxos = vec![make_utxo("tx1", 0, 2_000_000), make_utxo("tx2", 0, 3_000_000)];

        let result = select_largest_first(&mut utxos, 5_000_000);

        assert!(result.is_ok());
        let selected = result.unwrap();
        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].ada, 3_000_000); // largest first
        assert_eq!(selected[1].ada, 2_000_000);
    }

    #[test]
    fn test_select_largest_first_empty_utxos() {
        let mut utxos: Vec<Utxo> = vec![];

        let result = select_largest_first(&mut utxos, 1_000_000);

        assert!(result.is_err());
        if let Err(Error::InsufficientBalance { need, available }) = result {
            assert_eq!(need, 1_000_000);
            assert_eq!(available, 0);
        }
    }

    #[test]
    fn test_sort_by_value_descending() {
        let mut utxos = vec![
            make_utxo("tx1", 0, 1_000_000),
            make_utxo("tx2", 0, 5_000_000),
            make_utxo("tx3", 0, 3_000_000),
        ];

        sort_by_value_descending(&mut utxos);

        assert_eq!(utxos[0].ada, 5_000_000);
        assert_eq!(utxos[1].ada, 3_000_000);
        assert_eq!(utxos[2].ada, 1_000_000);
    }

    #[test]
    fn test_total_balance() {
        let utxos = vec![
            make_utxo("tx1", 0, 1_000_000),
            make_utxo("tx2", 0, 2_000_000),
            make_utxo("tx3", 0, 3_000_000),
        ];

        assert_eq!(total_balance(&utxos), 6_000_000);
    }

    #[test]
    fn test_total_balance_empty() {
        let utxos: Vec<Utxo> = vec![];
        assert_eq!(total_balance(&utxos), 0);
    }

    #[test]
    fn test_has_sufficient_balance_true() {
        let utxos = vec![make_utxo("tx1", 0, 2_000_000), make_utxo("tx2", 0, 3_000_000)];

        assert!(has_sufficient_balance(&utxos, 4_000_000));
        assert!(has_sufficient_balance(&utxos, 5_000_000));
        assert!(has_sufficient_balance(&utxos, 0));
    }

    #[test]
    fn test_has_sufficient_balance_false() {
        let utxos = vec![make_utxo("tx1", 0, 1_000_000), make_utxo("tx2", 0, 500_000)];

        assert!(!has_sufficient_balance(&utxos, 2_000_000));
    }

    #[test]
    fn test_utxo_value() {
        let utxo = make_utxo("tx1", 0, 3_000_000);
        assert_eq!(utxo.value(), 3_000_000);
    }

    #[test]
    fn test_select_largest_first_preserves_order() {
        // When there's a tie in value, original order should be preserved
        let mut utxos = vec![
            make_utxo("tx1", 0, 2_000_000),
            make_utxo("tx2", 0, 2_000_000),
            make_utxo("tx3", 0, 2_000_000),
        ];

        let result = select_largest_first(&mut utxos, 2_000_000);

        assert!(result.is_ok());
        let selected = result.unwrap();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].tx_hash, "tx1");
    }
}
