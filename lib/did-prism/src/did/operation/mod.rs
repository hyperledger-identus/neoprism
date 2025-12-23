mod ssi;
mod storage;

use std::str::FromStr;

use identus_apollo::hash::Sha256Digest;
use identus_apollo::hex::HexStr;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
pub use ssi::*;
pub use storage::*;

use crate::prelude::SignedPrismOperation;
use crate::proto::MessageExt;

#[derive(Debug, Clone)]
pub struct OperationParameters {
    pub max_services: usize,
    pub max_public_keys: usize,
    pub max_id_size: usize,
    pub max_type_size: usize,
    pub max_service_endpoint_size: usize,
}

impl OperationParameters {
    pub fn v1() -> Self {
        Self {
            max_services: 50,
            max_public_keys: 50,
            max_id_size: 50,
            max_type_size: 100,
            max_service_endpoint_size: 300,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, derive_more::From, derive_more::Into)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "openapi", schema(description = "A hexadecimal string representing a SignedPrismOperation", value_type = String, example = "0a086d61737465722d30124630440220442eec28ec60464acd8df155e73f88a1c7faf4549975582ff0601449525aba31022019257250071818066b377b83a8b1765df1b7dc21d9bccfc7d5da036801d3ba0e1a420a400a3e123c0a086d61737465722d3010014a2e0a09736563703235366b3112210398e61c14328a6a844eec6dc084b825ae8525f10204e9244aaf61260bd221a457"))]
pub struct SignedPrismOperationHexStr(
    #[serde(
        serialize_with = "SignedPrismOperationHexStr::serialize",
        deserialize_with = "SignedPrismOperationHexStr::deserialize"
    )]
    SignedPrismOperation,
);

impl SignedPrismOperationHexStr {
    fn serialize<S>(op: &SignedPrismOperation, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let hex_str = HexStr::from(&op.encode_to_vec());
        serializer.serialize_str(&hex_str.to_string())
    }

    fn deserialize<'de, D>(deserializer: D) -> Result<SignedPrismOperation, D::Error>
    where
        D: Deserializer<'de>,
    {
        let hex_str = String::deserialize(deserializer)?;
        let bytes = HexStr::from_str(&hex_str)
            .map_err(|e| serde::de::Error::custom(format!("Value is not a valid hex: {e}")))?;
        let op = SignedPrismOperation::decode(&bytes.to_bytes())
            .map_err(|e| serde::de::Error::custom(format!("Value cannot be decoded to SignedPrismOperation: {e}")))?;
        Ok(op)
    }
}

#[derive(
    Clone, PartialEq, Eq, Hash, Serialize, Deserialize, derive_more::Debug, derive_more::Display, derive_more::From,
)]
#[display("{}", identus_apollo::hex::HexStr::from(self.0.as_bytes()))]
#[debug("{}", identus_apollo::hex::HexStr::from(self.0.as_bytes()))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "openapi", schema(value_type = String, example = "a1b2c3d4e5f6789012345678901234567890123456789012345678901234abcd"))]
pub struct OperationId(
    #[serde(
        serialize_with = "OperationId::serialize",
        deserialize_with = "OperationId::deserialize"
    )]
    Sha256Digest,
);

impl OperationId {
    /// Create an OperationId from raw bytes (must be 32 bytes)
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, identus_apollo::hash::Error> {
        Sha256Digest::from_bytes(bytes).map(Self)
    }

    /// Convert OperationId to vector of bytes
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    /// Get reference to underlying bytes
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    fn serialize<S>(bytes: &Sha256Digest, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let hex_str = HexStr::from(bytes.as_bytes());
        serializer.serialize_str(&hex_str.to_string())
    }

    fn deserialize<'de, D>(deserializer: D) -> Result<Sha256Digest, D::Error>
    where
        D: Deserializer<'de>,
    {
        let hex_str = String::deserialize(deserializer)?;
        let bytes = HexStr::from_str(&hex_str)
            .map_err(|e| serde::de::Error::custom(format!("value is not a valid hex: {e}")))?;
        let digest = Sha256Digest::from_bytes(&bytes.to_bytes())
            .map_err(|e| serde::de::Error::custom(format!("value is not a valid digest: {e}")))?;
        Ok(digest)
    }
}

impl std::str::FromStr for OperationId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = HexStr::from_str(s).map_err(|e| format!("invalid hex string: {}", e))?;
        Self::from_bytes(&bytes.to_bytes()).map_err(|e| format!("invalid operation id: {}", e))
    }
}
