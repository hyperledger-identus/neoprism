use identus_apollo::hex::HexStr;
use identus_did_prism::did::CanonicalPrismDid;
use lazybe::macros::Newtype;
use serde::{Deserialize, Serialize};

mod indexer;

pub use indexer::*;

#[derive(Debug, Clone, Serialize, Deserialize, Newtype, derive_more::From)]
pub struct DidSuffix(Vec<u8>);

impl From<CanonicalPrismDid> for DidSuffix {
    fn from(value: CanonicalPrismDid) -> Self {
        value.suffix.to_vec().into()
    }
}

impl TryFrom<DidSuffix> for CanonicalPrismDid {
    type Error = crate::Error;

    fn try_from(value: DidSuffix) -> Result<Self, Self::Error> {
        let suffix = HexStr::from(value.0);
        let did = CanonicalPrismDid::from_suffix(suffix)?;
        Ok(did)
    }
}

impl DidSuffix {
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}
