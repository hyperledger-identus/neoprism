//! Wallet implementation combining BIP39 mnemonic parsing and BIP32 key derivation.
//!
//! Provides a high-level API for Cardano Shelley wallet operations.

use ed25519_bip32::Signature;

use super::error::Error;
use super::key_derivation::{DerivationPath, DerivedKeyPair};
use super::mnemonic::WalletMnemonic;

/// Embedded wallet for Cardano Shelley key derivation.
///
/// This wallet derives Ed25519 key pairs from a BIP39 mnemonic phrase
/// using BIP32 hierarchical deterministic derivation.
///
/// # Example
///
/// ```ignore
/// use identus_did_prism_submitter::dlt::embedded_wallet::Wallet;
///
/// let wallet = Wallet::from_mnemonic(
///     "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
///     None
/// )?;
///
/// let payment_pubkey = wallet.payment_public_key()?;
/// let stake_pubkey = wallet.stake_public_key()?;
/// ```
pub struct Wallet {
    mnemonic: WalletMnemonic,
    password: Option<String>,
}

impl Wallet {
    /// Create a wallet from a BIP39 mnemonic phrase.
    ///
    /// # Arguments
    /// * `phrase` - The mnemonic phrase (12, 15, 18, 21, or 24 words)
    /// * `password` - Optional BIP39 passphrase for additional security
    ///
    /// # Errors
    /// Returns `Error::InvalidMnemonic` if the phrase is not valid BIP39.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let wallet = Wallet::from_mnemonic(
    ///     "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    ///     None
    /// )?;
    /// ```
    pub fn from_mnemonic(phrase: &str, password: Option<&str>) -> Result<Self, Error> {
        let mnemonic = WalletMnemonic::parse(phrase, password)?;
        Ok(Self {
            mnemonic,
            password: password.map(String::from),
        })
    }

    /// Get the underlying seed for key derivation.
    fn get_seed(&self) -> [u8; 64] {
        self.mnemonic.to_seed(self.password.as_deref())
    }

    /// Get the payment key pair for the wallet.
    ///
    /// Derives the key at path: 1852H/1815H/0H/0/0
    fn payment_keypair(&self) -> DerivedKeyPair {
        let seed = self.get_seed();
        DerivedKeyPair::derive(&seed, DerivationPath::payment())
    }

    /// Get the stake key pair for the wallet.
    ///
    /// Derives the key at path: 1852H/1815H/0H/2/0
    fn stake_keypair(&self) -> DerivedKeyPair {
        let seed = self.get_seed();
        DerivedKeyPair::derive(&seed, DerivationPath::stake())
    }

    /// Get the payment public key (32 bytes).
    ///
    /// This key is used for generating the payment address.
    pub fn payment_public_key(&self) -> [u8; 32] {
        self.payment_keypair().public_key()
    }

    /// Get the stake public key (32 bytes).
    ///
    /// This key is used for generating the stake address.
    pub fn stake_public_key(&self) -> [u8; 32] {
        self.stake_keypair().public_key()
    }

    /// Get the payment public key as a hex string.
    pub fn payment_public_key_hex(&self) -> String {
        hex::encode(self.payment_public_key())
    }

    /// Get the stake public key as a hex string.
    pub fn stake_public_key_hex(&self) -> String {
        hex::encode(self.stake_public_key())
    }

    /// Get the payment private key seed (32 bytes).
    ///
    /// This is the raw Ed25519 secret key seed, suitable for
    /// creating Ed25519 signing keys for transaction signing.
    pub fn payment_private_key(&self) -> [u8; 32] {
        self.payment_keypair().secret_key_seed()
    }

    /// Get the stake private key seed (32 bytes).
    ///
    /// This is the raw Ed25519 secret key seed, suitable for
    /// creating Ed25519 signing keys for staking operations.
    pub fn stake_private_key(&self) -> [u8; 32] {
        self.stake_keypair().secret_key_seed()
    }

    /// Sign a message with the payment private key.
    ///
    /// # Arguments
    /// * `message` - The message to sign
    ///
    /// # Returns
    /// An Ed25519 signature (64 bytes).
    pub fn sign_with_payment_key(&self, message: &[u8]) -> Signature<Vec<u8>> {
        let keypair = self.payment_keypair();
        keypair.xprv.sign(message)
    }

    /// Sign a message with the stake private key.
    ///
    /// # Arguments
    /// * `message` - The message to sign
    ///
    /// # Returns
    /// An Ed25519 signature (64 bytes).
    pub fn sign_with_stake_key(&self, message: &[u8]) -> Signature<Vec<u8>> {
        let keypair = self.stake_keypair();
        keypair.xprv.sign(message)
    }

    /// Get the mnemonic phrase.
    pub fn phrase(&self) -> String {
        self.mnemonic.phrase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_phrase() -> &'static str {
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
    }

    #[test]
    fn test_wallet_from_mnemonic() {
        let wallet = Wallet::from_mnemonic(test_phrase(), None).unwrap();
        assert!(wallet.phrase().contains("abandon"));
    }

    #[test]
    fn test_wallet_with_password() {
        let wallet1 = Wallet::from_mnemonic(test_phrase(), None).unwrap();
        let wallet2 = Wallet::from_mnemonic(test_phrase(), Some("password")).unwrap();

        // Different passwords should result in different keys
        let pubkey1 = wallet1.payment_public_key();
        let pubkey2 = wallet2.payment_public_key();
        assert_ne!(pubkey1, pubkey2);
    }

    #[test]
    fn test_wallet_payment_public_key() {
        let wallet = Wallet::from_mnemonic(test_phrase(), None).unwrap();
        let pubkey = wallet.payment_public_key();
        assert_eq!(pubkey.len(), 32);
    }

    #[test]
    fn test_wallet_stake_public_key() {
        let wallet = Wallet::from_mnemonic(test_phrase(), None).unwrap();
        let pubkey = wallet.stake_public_key();
        assert_eq!(pubkey.len(), 32);
    }

    #[test]
    fn test_wallet_payment_and_stake_keys_differ() {
        let wallet = Wallet::from_mnemonic(test_phrase(), None).unwrap();
        let payment_key = wallet.payment_public_key();
        let stake_key = wallet.stake_public_key();
        assert_ne!(payment_key, stake_key);
    }

    #[test]
    fn test_wallet_deterministic_keys() {
        let wallet1 = Wallet::from_mnemonic(test_phrase(), None).unwrap();
        let wallet2 = Wallet::from_mnemonic(test_phrase(), None).unwrap();

        let payment1 = wallet1.payment_public_key();
        let payment2 = wallet2.payment_public_key();
        assert_eq!(payment1, payment2);

        let stake1 = wallet1.stake_public_key();
        let stake2 = wallet2.stake_public_key();
        assert_eq!(stake1, stake2);
    }

    #[test]
    fn test_wallet_invalid_mnemonic() {
        let result = Wallet::from_mnemonic("invalid mnemonic phrase", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_wallet_private_key_seed_length() {
        let wallet = Wallet::from_mnemonic(test_phrase(), None).unwrap();
        let private_key = wallet.payment_private_key();
        assert_eq!(private_key.len(), 32);
    }

    #[test]
    fn test_wallet_sign_message() {
        let wallet = Wallet::from_mnemonic(test_phrase(), None).unwrap();
        let message = b"test message";

        let signature = wallet.sign_with_payment_key(message);
        // Signature should be 64 bytes
        assert_eq!(signature.as_ref().len(), 64);
    }
}
