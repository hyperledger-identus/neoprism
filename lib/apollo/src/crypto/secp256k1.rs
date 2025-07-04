use std::fmt::Debug;

use k256::Secp256k1;
use k256::ecdsa::signature::{SignerMut, Verifier};
use k256::elliptic_curve::sec1::{EncodedPoint, ToEncodedPoint};

use super::{EncodeArray, EncodeVec, Error, Verifiable};
use crate::base64::Base64UrlStrNoPad;
use crate::jwk::{EncodeJwk, Jwk};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Secp256k1PublicKey(k256::PublicKey);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Secp256k1PrivateKey(k256::SecretKey);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CurvePoint {
    pub x: [u8; 32],
    pub y: [u8; 32],
}

impl EncodeVec for Secp256k1PublicKey {
    fn encode_vec(&self) -> Vec<u8> {
        self.encode_compressed().into()
    }
}

impl EncodeArray<33> for Secp256k1PublicKey {
    fn encode_array(&self) -> [u8; 33] {
        self.encode_compressed()
    }
}

impl EncodeArray<65> for Secp256k1PublicKey {
    fn encode_array(&self) -> [u8; 65] {
        self.encode_uncompressed()
    }
}

impl Verifiable for Secp256k1PublicKey {
    /// In the old days of PRISM node implementation, the signature is signed using bouncycastle / bitcoinj which has some issue with signature verification.
    /// This make some signed operation from JVM PRISM node not verifiable in the rust library.
    ///
    /// https://github.com/hyperledger/identus-apollo/blob/6b331d9ea1432ada4c1124af95a671d0c38bd9e2/apollo/src/jvmMain/kotlin/org/hyperledger/identus/apollo/secp256k1/Secp256k1Lib.kt#L58
    fn verify(&self, message: &[u8], signature: &[u8]) -> bool {
        let verifying_key: k256::ecdsa::VerifyingKey = self.0.into();

        let Ok(signature) = k256::ecdsa::Signature::from_der(signature) else {
            return false;
        };

        // verify using the vanilla verification from the library
        if verifying_key.verify(message, &signature).is_ok() {
            return true;
        };

        // verify using normalized signature
        let Some(normalized_signature) = signature.normalize_s() else {
            return false;
        };
        if verifying_key.verify(message, &normalized_signature).is_ok() {
            return true;
        };

        // verify using transcoded signature
        let transcoded_signature_bytes = transcode_signature_to_bitcoin(&normalized_signature.to_bytes());
        let Ok(transcoded_signature) = k256::ecdsa::Signature::from_der(&transcoded_signature_bytes) else {
            return false;
        };

        verifying_key.verify(message, &transcoded_signature).is_ok()
    }
}

impl Secp256k1PublicKey {
    pub fn from_slice(slice: &[u8]) -> Result<Secp256k1PublicKey, Error> {
        Ok(Secp256k1PublicKey(k256::PublicKey::from_sec1_bytes(slice)?))
    }

    pub fn encode_uncompressed(&self) -> [u8; 65] {
        let bytes: EncodedPoint<Secp256k1> = self.0.to_encoded_point(false);
        let bytes = bytes.to_bytes();
        let Some((chunk, _)) = bytes.split_first_chunk::<65>() else {
            unreachable!("EncodedPoint::to_bytes() must return a single chunk");
        };
        chunk.to_owned()
    }

    pub fn encode_compressed(&self) -> [u8; 33] {
        let bytes: EncodedPoint<Secp256k1> = self.0.to_encoded_point(true);
        let bytes = bytes.to_bytes();
        let Some((chunk, _)) = bytes.split_first_chunk::<33>() else {
            unreachable!("EncodedPoint::to_bytes() must return a single chunk");
        };
        chunk.to_owned()
    }

    pub fn curve_point(&self) -> CurvePoint {
        let uncompressed = self.encode_uncompressed();
        let (_, xy) = uncompressed.rsplit_array_ref::<64>();
        let (x, _) = xy.split_array_ref::<32>();
        let (_, y) = xy.rsplit_array_ref::<32>();
        CurvePoint {
            x: x.to_owned(),
            y: y.to_owned(),
        }
    }
}

impl Secp256k1PrivateKey {
    pub fn from_slice(slice: &[u8]) -> Result<Self, Error> {
        let sk = k256::SecretKey::from_slice(slice)?;
        Ok(Self(sk))
    }

    pub fn to_public_key(&self) -> Secp256k1PublicKey {
        Secp256k1PublicKey(self.0.public_key())
    }

    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        let mut signing_key = k256::ecdsa::SigningKey::from(&self.0);
        let signature: k256::ecdsa::Signature = signing_key.sign(message);
        signature.to_der().to_bytes().to_vec()
    }
}

/// https://github.com/hyperledger/identus-apollo/blob/6b331d9ea1432ada4c1124af95a671d0c38bd9e2/apollo/src/jvmMain/kotlin/org/hyperledger/identus/apollo/secp256k1/Secp256k1Lib.kt#L80
fn transcode_signature_to_bitcoin(sig: &[u8]) -> Vec<u8> {
    let raw_len = sig.len() / 2;
    let (r, s) = sig.split_at(raw_len);
    let r_rev = r.iter().rev();
    let s_rev = s.iter().rev();
    r_rev.chain(s_rev).cloned().collect()
}

impl EncodeJwk for Secp256k1PublicKey {
    fn encode_jwk(&self) -> Jwk {
        let point = self.curve_point();
        Jwk {
            kty: "EC".to_string(),
            crv: "secp256k1".to_string(),
            x: Some(Base64UrlStrNoPad::from(point.x)),
            y: Some(Base64UrlStrNoPad::from(point.y)),
        }
    }
}
