use ring::digest;

#[derive(Debug, derive_more::Display, derive_more::Error)]
pub enum Error {
    #[display("hash operation encountered invalid input size")]
    InvalidByteSize {
        type_name: &'static str,
        expected: usize,
        actual: usize,
    },
}

#[derive(Clone, PartialEq, Eq, Hash, derive_more::From, derive_more::Debug)]
#[debug("sha256-{}", crate::hex::HexStr::from(_0))]
pub struct Sha256Digest([u8; 32]);

impl Sha256Digest {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn as_array(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    /// Convert bytes to the digest.
    /// This only validate and wrap the raw bytes, it does not hash.
    ///
    /// # Example
    ///
    /// ```
    /// use identus_apollo::hash::Sha256Digest;
    /// let digest = Sha256Digest::from_bytes(&vec![42u8; 32]).unwrap();
    /// assert_eq!(digest.as_bytes(), vec![42u8; 32]);
    ///
    /// let digest = Sha256Digest::from_bytes(&vec![42u8; 31]);
    /// assert!(digest.is_err());
    /// ```
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != 32 {
            Err(Error::InvalidByteSize {
                type_name: std::any::type_name::<Self>(),
                expected: 32,
                actual: bytes.len(),
            })?
        }

        let mut digest = [0u8; 32];
        digest.copy_from_slice(bytes);
        Ok(Self(digest))
    }
}

pub fn sha256<B: AsRef<[u8]>>(bytes: B) -> Sha256Digest {
    let digest = digest::digest(&digest::SHA256, bytes.as_ref());
    let digest: [u8; 32] = digest
        .as_ref()
        .try_into()
        .expect("The digest must have length of 32 bytes");
    Sha256Digest(digest)
}
