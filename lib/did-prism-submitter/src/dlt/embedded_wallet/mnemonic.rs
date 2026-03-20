//! BIP39 mnemonic parsing and seed derivation.

use bip39::Mnemonic;

use super::error::Error;

/// Parsed BIP39 mnemonic with seed derivation capability.
pub struct WalletMnemonic {
    mnemonic: Mnemonic,
}

impl WalletMnemonic {
    /// Parse a BIP39 mnemonic phrase.
    ///
    /// # Arguments
    /// * `phrase` - The mnemonic phrase (12, 15, 18, 21, or 24 words)
    /// * `password` - Optional password for seed derivation (BIP39 passphrase)
    ///
    /// # Errors
    /// Returns `Error::InvalidMnemonic` if the phrase is not valid BIP39.
    pub fn parse(phrase: &str, _password: Option<&str>) -> Result<Self, Error> {
        let mnemonic = Mnemonic::parse(phrase).map_err(|e| Error::InvalidMnemonic {
            reason: match e {
                bip39::Error::BadWordCount(_) => "invalid word count, expected 12, 15, 18, 21, or 24 words",
                bip39::Error::UnknownWord(_) => "contains unknown word not in BIP39 wordlist",
                bip39::Error::InvalidChecksum => "invalid checksum",
                bip39::Error::BadEntropyBitCount(_) => "invalid entropy bit count",
                bip39::Error::AmbiguousLanguages(_) => "ambiguous language detection",
            },
        })?;

        Ok(Self { mnemonic })
    }

    /// Derive the seed from the mnemonic with optional password.
    ///
    /// # Arguments
    /// * `password` - Optional BIP39 passphrase for additional security
    ///
    /// # Returns
    /// A 64-byte seed suitable for BIP32 key derivation.
    pub fn to_seed(&self, password: Option<&str>) -> [u8; 64] {
        self.mnemonic.to_seed(password.unwrap_or(""))
    }

    /// Get the mnemonic phrase as words.
    pub fn phrase(&self) -> String {
        self.mnemonic.words().collect::<Vec<_>>().join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_12_word_mnemonic() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let result = WalletMnemonic::parse(phrase, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_valid_24_word_mnemonic() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
        let result = WalletMnemonic::parse(phrase, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_invalid_word_count() {
        let phrase = "abandon abandon abandon";
        let result = WalletMnemonic::parse(phrase, None);
        assert!(result.is_err());
        if let Err(Error::InvalidMnemonic { reason }) = result {
            assert!(reason.contains("word count"));
        } else {
            panic!("expected InvalidMnemonic error");
        }
    }

    #[test]
    fn test_parse_invalid_word() {
        let phrase =
            "invalidword abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon";
        let result = WalletMnemonic::parse(phrase, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_to_seed_with_password() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let mnemonic = WalletMnemonic::parse(phrase, None).unwrap();

        let seed_no_password = mnemonic.to_seed(None);
        let seed_with_password = mnemonic.to_seed(Some("test"));

        // Password should result in different seed
        assert_ne!(seed_no_password, seed_with_password);
    }

    #[test]
    fn test_to_seed_length() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let mnemonic = WalletMnemonic::parse(phrase, None).unwrap();
        let seed = mnemonic.to_seed(None);

        // Seed should be 64 bytes
        assert_eq!(seed.len(), 64);
    }

    #[test]
    fn test_phrase_reconstruction() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let mnemonic = WalletMnemonic::parse(phrase, None).unwrap();
        let reconstructed = mnemonic.phrase();

        assert_eq!(phrase, reconstructed.as_str());
    }
}
