use std::str::FromStr;

use identus_apollo::hex::HexStr;
use identus_did_core::DidDocument;
use serde::{Deserialize, Serialize};

use crate::did::MidnightDid;

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, derive_more::Debug, derive_more::Display)]
#[display("{}", self.0)]
#[debug("{}", self.0)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "openapi", schema(value_type = String, example = "68656c6c6f20776f726c64"))]
pub struct ContractState(HexStr);

impl ContractState {
    pub fn inner(&self) -> &HexStr {
        &self.0
    }
}

impl<B: Into<HexStr>> From<B> for ContractState {
    fn from(value: B) -> Self {
        Self(value.into())
    }
}

impl FromStr for ContractState {
    type Err = identus_apollo::hex::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(HexStr::from_str(s)?))
    }
}

pub trait ContractStateDecoder {
    fn decode(
        &self,
        did: &MidnightDid,
        state: ContractState,
    ) -> Result<DidDocument, Box<dyn std::error::Error + Send + Sync>>;
}
