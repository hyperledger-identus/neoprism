use crate::base64::Base64UrlStrNoPad;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Jwk {
    pub kty: String,
    pub crv: String,
    pub x: Option<Base64UrlStrNoPad>,
    pub y: Option<Base64UrlStrNoPad>,
}

pub trait EncodeJwk {
    fn encode_jwk(&self) -> Jwk;
}

#[cfg(any(feature = "ed25519", feature = "secp256k1", feature = "x25519"))]
fn from_sdk(jwk: identus_crypto::jwk::PublicKeyJwk) -> Jwk {
    Jwk {
        kty: jwk.kty().to_string(),
        crv: jwk.crv().to_string(),
        x: Some(Base64UrlStrNoPad::from(jwk.x().to_bytes())),
        y: jwk.y().map(|coordinate| Base64UrlStrNoPad::from(coordinate.to_bytes())),
    }
}

#[cfg(feature = "ed25519")]
impl EncodeJwk for identus_crypto::crypto::ed25519::Ed25519PublicKey {
    fn encode_jwk(&self) -> Jwk {
        from_sdk(identus_crypto::EncodeJwk::encode_jwk(self))
    }
}

#[cfg(feature = "secp256k1")]
impl EncodeJwk for identus_crypto::crypto::secp256k1::Secp256k1PublicKey {
    fn encode_jwk(&self) -> Jwk {
        from_sdk(identus_crypto::EncodeJwk::encode_jwk(self))
    }
}

#[cfg(feature = "x25519")]
impl EncodeJwk for identus_crypto::crypto::x25519::X25519PublicKey {
    fn encode_jwk(&self) -> Jwk {
        from_sdk(identus_crypto::EncodeJwk::encode_jwk(self))
    }
}
