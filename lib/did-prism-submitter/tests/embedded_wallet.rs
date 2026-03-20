//! Integration tests for embedded wallet key derivation.
//!
//! These tests verify the BIP39 mnemonic parsing and BIP32 key derivation
//! for Cardano Shelley addresses.

#![cfg(feature = "embedded-wallet")]

use identus_did_prism_submitter::dlt::embedded_wallet::{Error, Wallet};

/// Test vector: known mnemonic with expected keys
/// The mnemonic is a standard BIP39 test vector, and the keys are derived
/// following Cardano Shelley derivation paths.
fn test_mnemonic() -> &'static str {
    // Standard BIP39 test vector (12 words)
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
}

#[test]
fn test_wallet_creation() {
    let result = Wallet::from_mnemonic(test_mnemonic(), None);
    assert!(result.is_ok());
    let wallet = result.unwrap();
    assert_eq!(wallet.phrase(), test_mnemonic());
}

#[test]
fn test_wallet_with_password() {
    let wallet_no_pass = Wallet::from_mnemonic(test_mnemonic(), None).unwrap();
    let wallet_with_pass = Wallet::from_mnemonic(test_mnemonic(), Some("password123")).unwrap();

    // Different passwords should result in different keys
    let key1 = wallet_no_pass.payment_public_key();
    let key2 = wallet_with_pass.payment_public_key();
    assert_ne!(key1, key2, "Keys should differ with different passwords");
}

#[test]
fn test_payment_key_length() {
    let wallet = Wallet::from_mnemonic(test_mnemonic(), None).unwrap();
    let pubkey = wallet.payment_public_key();
    assert_eq!(pubkey.len(), 32, "Payment public key should be 32 bytes");
}

#[test]
fn test_stake_key_length() {
    let wallet = Wallet::from_mnemonic(test_mnemonic(), None).unwrap();
    let pubkey = wallet.stake_public_key();
    assert_eq!(pubkey.len(), 32, "Stake public key should be 32 bytes");
}

#[test]
fn test_payment_and_stake_keys_differ() {
    let wallet = Wallet::from_mnemonic(test_mnemonic(), None).unwrap();

    let payment_key = wallet.payment_public_key();
    let stake_key = wallet.stake_public_key();

    assert_ne!(payment_key, stake_key, "Payment and stake keys should be different");
}

#[test]
fn test_deterministic_derivation() {
    // Create two wallets from the same mnemonic
    let wallet1 = Wallet::from_mnemonic(test_mnemonic(), None).unwrap();
    let wallet2 = Wallet::from_mnemonic(test_mnemonic(), None).unwrap();

    // All derived keys should be identical
    assert_eq!(wallet1.payment_public_key(), wallet2.payment_public_key());
    assert_eq!(wallet1.stake_public_key(), wallet2.stake_public_key());
    assert_eq!(wallet1.payment_private_key(), wallet2.payment_private_key());
    assert_eq!(wallet1.stake_private_key(), wallet2.stake_private_key());
}

#[test]
fn test_invalid_mnemonic_word_count() {
    let result = Wallet::from_mnemonic("abandon abandon abandon", None);
    assert!(matches!(result, Err(Error::InvalidMnemonic { .. })));
}

#[test]
fn test_invalid_mnemonic_unknown_word() {
    let result = Wallet::from_mnemonic(
        "notaword abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon",
        None,
    );
    assert!(matches!(result, Err(Error::InvalidMnemonic { .. })));
}

#[test]
fn test_invalid_mnemonic_checksum() {
    // Change last word to break checksum
    let result = Wallet::from_mnemonic(
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon",
        None,
    );
    assert!(matches!(result, Err(Error::InvalidMnemonic { .. })));
}

#[test]
fn test_24_word_mnemonic() {
    let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
    let wallet = Wallet::from_mnemonic(phrase, None);
    assert!(wallet.is_ok());

    let wallet = wallet.unwrap();
    assert!(wallet.payment_public_key().len() == 32);
    assert!(wallet.stake_public_key().len() == 32);
}

#[test]
fn test_hex_encoding() {
    let wallet = Wallet::from_mnemonic(test_mnemonic(), None).unwrap();

    let pubkey_hex = wallet.payment_public_key_hex();
    let pubkey_bytes = wallet.payment_public_key();

    // Hex string should be 64 characters (32 bytes * 2)
    assert_eq!(pubkey_hex.len(), 64);

    // Decode and verify
    let decoded = hex::decode(&pubkey_hex).unwrap();
    assert_eq!(decoded.as_slice(), pubkey_bytes);
}

#[test]
fn test_private_key_length() {
    let wallet = Wallet::from_mnemonic(test_mnemonic(), None).unwrap();

    let payment_priv = wallet.payment_private_key();
    let stake_priv = wallet.stake_private_key();

    assert_eq!(payment_priv.len(), 32);
    assert_eq!(stake_priv.len(), 32);
}
