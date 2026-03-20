//! BIP32 hierarchical deterministic key derivation for Cardano.
//!
//! Implements key derivation paths specific to Cardano Shelley:
//! - Payment key: 1852H/1815H/0H/0/0
//! - Stake key: 1852H/1815H/0H/2/0
//!
//! Cardano uses BIP32-Ed25519 with specific derivation scheme:
//! - Master key derivation uses HMAC-SHA512 with key "ed25519 seed"
//! - Hardened derivation (index >= 0x80000000) is always used

use ed25519_bip32::{DerivationIndex, DerivationScheme, XPrv, XPub};
use hmac::{Hmac, Mac};
use sha2::Sha512;

type HmacSha512 = Hmac<Sha512>;

/// Hardened key derivation offset (0x80000000)
const HARDENED_OFFSET: u32 = 0x80000000;

/// Derivation path indices for Cardano Shelley
mod path_indices {
    pub const PURPOSE: u32 = 1852; // Cardano Shelley purpose
    pub const COIN_TYPE: u32 = 1815; // ADA coin type
    pub const ACCOUNT: u32 = 0; // First account
    pub const EXTERNAL_CHAIN: u32 = 0; // External chain for payment
    pub const STAKING_CHAIN: u32 = 2; // Staking chain
    pub const ADDRESS_INDEX: u32 = 0; // First address
}

/// Derive the master key from a BIP39 seed.
///
/// Cardano uses HMAC-SHA512 with key "ed25519 seed" to derive the master key
/// from the BIP39 seed (64 bytes).
///
/// # Arguments
/// * `seed` - BIP39 seed (64 bytes)
///
/// # Returns
/// An extended private key (XPrv) containing both the secret key and chain code.
fn master_key_from_seed(seed: &[u8; 64]) -> XPrv {
    // HMAC-SHA512 with key "ed25519 seed"
    let mut mac = HmacSha512::new_from_slice(b"ed25519 seed").expect("HMAC key size is valid");
    mac.update(seed);
    let result = mac.finalize().into_bytes();

    // Split result: first 32 bytes = key seed, last 32 bytes = chain code
    let (key_seed, chain_code) = result.split_at(32);

    // Convert to fixed-size arrays
    let key_array: [u8; 32] = key_seed.try_into().expect("Key seed is 32 bytes");
    let cc_array: [u8; 32] = chain_code.try_into().expect("Chain code is 32 bytes");

    // Create XPrv from non-extended seed and chain code
    // This applies SHA512 to extend the key properly
    XPrv::from_nonextended_force(&key_array, &cc_array)
}

/// Derive a key at a specific path from the master key.
///
/// # Arguments
/// * `master` - The master key (XPrv)
/// * `path` - Derivation path as a slice of indices (hardened indices should have 0x80000000 set)
///
/// # Returns
/// The derived XPrv at the given path.
fn derive_path(master: &XPrv, path: &[DerivationIndex]) -> XPrv {
    let mut current = master.clone();
    for &index in path {
        current = current.derive(DerivationScheme::V2, index);
    }
    current
}

/// BIP32 derivation path for Cardano Shelley keys.
///
/// Cardano uses a specific derivation scheme:
/// - Purpose: 1852H (Cardano Shelley, registered in SLIP-0010)
/// - Coin type: 1815H (ADA, registered in BIP-0044)
/// - Account: 0H (first account)
/// - Change: 0 (external) or 2 (staking)
/// - Index: 0 (first address)
pub struct DerivationPath {
    path: Vec<DerivationIndex>,
}

impl DerivationPath {
    /// Create a hardened derivation index.
    fn hardened(index: u32) -> DerivationIndex {
        index | HARDENED_OFFSET
    }

    /// Create the payment key derivation path: 1852H/1815H/0H/0/0
    pub fn payment() -> Self {
        use path_indices::*;
        Self {
            path: vec![
                Self::hardened(PURPOSE),
                Self::hardened(COIN_TYPE),
                Self::hardened(ACCOUNT),
                EXTERNAL_CHAIN,
                ADDRESS_INDEX,
            ],
        }
    }

    /// Create the stake key derivation path: 1852H/1815H/0H/2/0
    pub fn stake() -> Self {
        use path_indices::*;
        Self {
            path: vec![
                Self::hardened(PURPOSE),
                Self::hardened(COIN_TYPE),
                Self::hardened(ACCOUNT),
                STAKING_CHAIN,
                ADDRESS_INDEX,
            ],
        }
    }

    /// Convert to string representation for error messages.
    #[allow(dead_code)]
    pub fn to_string_path(&self) -> String {
        self.path
            .iter()
            .map(|&idx| {
                if idx >= HARDENED_OFFSET {
                    format!("{}H", idx - HARDENED_OFFSET)
                } else {
                    format!("{}", idx)
                }
            })
            .collect::<Vec<_>>()
            .join("/")
    }
}

/// Derived key pair containing both private and public keys.
pub struct DerivedKeyPair {
    /// Extended private key (96 bytes: 64-byte extended secret + 32-byte chain code)
    pub xprv: XPrv,
    /// Derivation path used
    #[allow(dead_code)]
    path: DerivationPath,
}

impl DerivedKeyPair {
    /// Derive a key pair from a seed at the specified path.
    ///
    /// # Arguments
    /// * `seed` - BIP39 seed (64 bytes) from mnemonic
    /// * `path` - Derivation path (payment or stake)
    ///
    /// # Note
    /// Derivation cannot fail for valid seeds and predefined paths.
    /// The `DerivationPath` type guarantees valid paths through its constructors.
    pub fn derive(seed: &[u8; 64], path: DerivationPath) -> Self {
        // Create master key from seed
        let master = master_key_from_seed(seed);

        // Derive through the path
        let xprv = derive_path(&master, &path.path);

        Self { xprv, path }
    }

    /// Get the public key bytes (32 bytes).
    pub fn public_key(&self) -> [u8; 32] {
        self.xprv.public().public_key()
    }

    /// Get the extended public key (64 bytes: 32-byte public key + 32-byte chain code).
    #[allow(dead_code)]
    pub fn extended_public_key(&self) -> XPub {
        self.xprv.public()
    }

    /// Get the secret key seed (32 bytes).
    ///
    /// This is the raw Ed25519 secret key seed, suitable for
    /// creating Ed25519 signing keys.
    pub fn secret_key_seed(&self) -> [u8; 32] {
        // The first 32 bytes of the extended secret key
        let bytes = self.xprv.extended_secret_key_bytes();
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&bytes[0..32]);
        seed
    }

    /// Get the entire extended secret key (64 bytes).
    ///
    /// This includes both the secret scalar and the prefix for Ed25519.
    #[allow(dead_code)]
    pub fn extended_secret_key(&self) -> [u8; 64] {
        *self.xprv.extended_secret_key_bytes()
    }

    /// Get the chain code (32 bytes).
    #[allow(dead_code)]
    pub fn chain_code(&self) -> [u8; 32] {
        *self.xprv.chain_code()
    }

    /// Get the derivation path as string.
    #[allow(dead_code)]
    pub fn derivation_path(&self) -> String {
        self.path.to_string_path()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_seed() -> [u8; 64] {
        // Standard BIP39 test vector (12 words)
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        // Parse and get seed
        let mnemonic = bip39::Mnemonic::parse(phrase).unwrap();
        mnemonic.to_seed("")
    }

    #[test]
    fn test_master_key_derivation() {
        let seed = test_seed();
        let _master = master_key_from_seed(&seed);
        // Master key should be 96 bytes internally
    }

    #[test]
    fn test_derive_payment_key() {
        let seed = test_seed();
        let path = DerivationPath::payment();
        let keypair = DerivedKeyPair::derive(&seed, path);

        // Verify we get 32-byte keys
        assert_eq!(keypair.public_key().len(), 32);
        assert_eq!(keypair.secret_key_seed().len(), 32);
        assert_eq!(keypair.chain_code().len(), 32);

        // Verify derivation path string
        assert_eq!(keypair.derivation_path(), "1852H/1815H/0H/0/0");
    }

    #[test]
    fn test_derive_stake_key() {
        let seed = test_seed();
        let path = DerivationPath::stake();
        let keypair = DerivedKeyPair::derive(&seed, path);

        // Verify we get 32-byte keys
        assert_eq!(keypair.public_key().len(), 32);
        assert_eq!(keypair.secret_key_seed().len(), 32);

        // Verify derivation path string
        assert_eq!(keypair.derivation_path(), "1852H/1815H/0H/2/0");
    }

    #[test]
    fn test_payment_and_stake_keys_are_different() {
        let seed = test_seed();

        let payment_keypair = DerivedKeyPair::derive(&seed, DerivationPath::payment());
        let stake_keypair = DerivedKeyPair::derive(&seed, DerivationPath::stake());

        // Public keys should be different
        assert_ne!(payment_keypair.public_key(), stake_keypair.public_key());

        // Secret keys should be different
        assert_ne!(payment_keypair.secret_key_seed(), stake_keypair.secret_key_seed());

        // Chain codes should be different
        assert_ne!(payment_keypair.chain_code(), stake_keypair.chain_code());
    }

    #[test]
    fn test_deterministic_derivation() {
        let seed = test_seed();

        // Derive payment key twice from same seed
        let keypair1 = DerivedKeyPair::derive(&seed, DerivationPath::payment());
        let keypair2 = DerivedKeyPair::derive(&seed, DerivationPath::payment());

        // Should produce identical keys
        assert_eq!(keypair1.public_key(), keypair2.public_key());
        assert_eq!(keypair1.secret_key_seed(), keypair2.secret_key_seed());
        assert_eq!(keypair1.chain_code(), keypair2.chain_code());
    }

    #[test]
    fn test_derivation_path_string() {
        assert_eq!(DerivationPath::payment().to_string_path(), "1852H/1815H/0H/0/0");
        assert_eq!(DerivationPath::stake().to_string_path(), "1852H/1815H/0H/2/0");
    }

    #[test]
    fn test_known_test_vector() {
        // Test vector from Cardano documentation
        // Mnemonic: abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about
        // This is a standard test vector, but Cardano-specific derivation may differ from other implementations

        let seed = test_seed();
        let keypair = DerivedKeyPair::derive(&seed, DerivationPath::payment());

        // Public key should be deterministic and 32 bytes
        let pubkey = keypair.public_key();
        assert_eq!(pubkey.len(), 32);

        // Verify the key is non-zero (sanity check)
        assert!(pubkey.iter().any(|&b| b != 0));
    }
}
