use identus_apollo::hex::HexStr;
use identus_did_core::DidDocument;
use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    derive_more::Debug,
    derive_more::Display,
    derive_more::From,
    derive_more::Into,
)]
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

pub trait ContractStateDecoder {
    fn decode(&self, state: ContractState) -> DidDocument;
}
