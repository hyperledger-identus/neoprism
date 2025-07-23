use std::str::FromStr;

use chrono::{DateTime, Utc};
use identus_apollo::hash::Sha256Digest;
use identus_apollo::hex::HexStr;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use strum::VariantArray;

use crate::proto::prism::PrismObject;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DltCursor {
    pub slot: u64,
    pub block_hash: Vec<u8>,
    pub cbt: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockMetadata {
    /// Cardano slot number
    pub slot_number: u64,
    /// Cardano block number
    pub block_number: u64,
    /// Cardano block timestamp
    pub cbt: DateTime<Utc>,
    /// PrismBlock seqeuence number
    ///
    /// This is used to order PrismBlock within the same Cardano block
    pub absn: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationMetadata {
    /// PrismBlock metadata
    pub block_metadata: BlockMetadata,
    /// Operation sequence number
    ///
    /// This is used to order PrismOperation within the same PrismBlock
    pub osn: u32,
}

impl OperationMetadata {
    pub fn compare_time_asc(a: &Self, b: &Self) -> std::cmp::Ordering {
        let a_tup = (a.block_metadata.block_number, a.block_metadata.absn, a.osn);
        let b_tup = (b.block_metadata.block_number, b.block_metadata.absn, b.osn);
        a_tup.cmp(&b_tup)
    }

    pub fn compare_time_desc(a: &Self, b: &Self) -> std::cmp::Ordering {
        Self::compare_time_asc(b, a)
    }
}

#[derive(Debug, Clone)]
pub struct PublishedPrismObject {
    pub block_metadata: BlockMetadata,
    pub prism_object: PrismObject,
}

#[derive(
    Clone, PartialEq, Eq, Hash, Serialize, Deserialize, derive_more::Debug, derive_more::Display, derive_more::From,
)]
#[display("{}", identus_apollo::hex::HexStr::from(self.0.as_bytes()))]
#[debug("{}", identus_apollo::hex::HexStr::from(self.0.as_bytes()))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "openapi", schema(value_type = String, example = "5ab0cf7e4c7cd4b63ba84a4fe299409be12ba85607cb6d1a149e80bc2eac070d"))]
pub struct TxId(#[serde(serialize_with = "TxId::serialize", deserialize_with = "TxId::deserialize")] Sha256Digest);

impl TxId {
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
            .map_err(|e| serde::de::Error::custom(format!("Value is not a valid hex: {e}")))?;
        let digest = Sha256Digest::from_bytes(&bytes.to_bytes())
            .map_err(|e| serde::de::Error::custom(format!("Value is not a valid digest: {e}")))?;
        Ok(digest)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display, strum::EnumString, strum::VariantArray)]
pub enum NetworkIdentifier {
    #[strum(serialize = "mainnet")]
    Mainnet,
    #[strum(serialize = "preprod")]
    Preprod,
    #[strum(serialize = "preview")]
    Preview,
}

impl NetworkIdentifier {
    pub fn variants() -> &'static [Self] {
        Self::VARIANTS
    }
}
