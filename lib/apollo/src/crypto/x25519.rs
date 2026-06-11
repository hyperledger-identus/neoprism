use super::{EncodeArray, EncodeVec, Error};
use crate::base64::Base64UrlStrNoPad;
use crate::jwk::{EncodeJwk, Jwk};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct X25519PublicKey(x25519_dalek::PublicKey);

impl X25519PublicKey {
    pub fn from_slice(slice: &[u8]) -> Result<X25519PublicKey, Error> {
        if slice.len() != 32 {
            return Err(Error::InvalidKeySize {
                expected: 32,
                actual: slice.len(),
                key_type: std::any::type_name::<X25519PublicKey>(),
            });
        }
        let key = x25519_dalek::PublicKey::from(<[u8; 32]>::try_from(slice).expect("checked above"));
        Ok(X25519PublicKey(key))
    }
}

impl EncodeVec for X25519PublicKey {
    fn encode_vec(&self) -> Vec<u8> {
        self.0.as_bytes().to_vec()
    }
}

impl EncodeArray<32> for X25519PublicKey {
    fn encode_array(&self) -> [u8; 32] {
        self.0.to_bytes()
    }
}

impl EncodeJwk for X25519PublicKey {
    fn encode_jwk(&self) -> Jwk {
        let x = self.encode_array();
        Jwk {
            kty: "OKP".to_string(),
            crv: "X25519".to_string(),
            x: Some(Base64UrlStrNoPad::from(x)),
            y: None,
        }
    }
}
