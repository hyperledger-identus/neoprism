use std::rc::Rc;
use std::str::FromStr;
use std::sync::LazyLock;

use enum_dispatch::enum_dispatch;
use error::DidSyntaxError;
use identus_apollo::base64::Base64UrlStrNoPad;
use identus_apollo::hash::Sha256Digest;
use identus_apollo::hex::HexStr;
use identus_did_core::Did;
use regex::Regex;

use self::operation::{PublicKey, Service};
use crate::did::operation::StorageData;
use crate::prelude::*;
use crate::proto::node_api;
use crate::proto::prism::PrismOperation;
use crate::proto::prism::prism_operation::Operation;

pub mod did_doc;
pub mod error;
pub mod operation;

pub use error::Error;

static CANONICAL_SUFFIX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([0-9a-f]{64}$)").expect("CANONICAL_SUFFIX_RE regex is invalid"));
static LONG_FORM_SUFFIX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([0-9a-f]{64}):([A-Za-z0-9_-]+$)").expect("LONG_FORM_SUFFIX_RE regex is invalid"));

#[enum_dispatch(PrismDidOps)]
#[derive(Clone, PartialEq, Eq, Hash, derive_more::Debug, derive_more::Display)]
pub enum PrismDid {
    #[display("{_0}")]
    #[debug("{_0}")]
    Canonical(CanonicalPrismDid),
    #[display("{_0}")]
    #[debug("{_0}")]
    LongForm(LongFormPrismDid),
}

#[derive(Clone, PartialEq, Eq, Hash, derive_more::Debug, derive_more::Display)]
#[display("did:{}:{}", self.method(), self.suffix_hex())]
#[debug("did:{}:{}", self.method(), self.suffix_hex())]
pub struct CanonicalPrismDid {
    pub suffix: Sha256Digest,
}

#[derive(Clone, PartialEq, Eq, Hash, derive_more::Debug, derive_more::Display)]
#[display("did:{}:{}:{}", self.method(), self.suffix_hex(), self.encoded_state)]
#[debug("did:{}:{}:{}", self.method(), self.suffix_hex(), self.encoded_state)]
pub struct LongFormPrismDid {
    pub suffix: Sha256Digest,
    pub encoded_state: Base64UrlStrNoPad,
}

#[enum_dispatch]
pub trait PrismDidOps: std::fmt::Display {
    fn suffix(&self) -> &Sha256Digest;

    fn method(&self) -> &'static str {
        "prism"
    }

    fn suffix_hex(&self) -> HexStr {
        HexStr::from(self.suffix().as_bytes().to_owned())
    }

    fn into_canonical(self) -> CanonicalPrismDid;

    fn to_did(&self) -> Did {
        let s = self.to_string();
        Did::from_str(&s).expect("type implement PrismDidOps does not construct a valid DID syntax")
    }
}

impl PrismDidOps for CanonicalPrismDid {
    fn suffix(&self) -> &Sha256Digest {
        &self.suffix
    }

    fn into_canonical(self) -> CanonicalPrismDid {
        self
    }
}

impl PrismDidOps for LongFormPrismDid {
    fn suffix(&self) -> &Sha256Digest {
        &self.suffix
    }

    fn into_canonical(self) -> CanonicalPrismDid {
        CanonicalPrismDid { suffix: self.suffix }
    }
}

impl CanonicalPrismDid {
    pub fn from_operation(operation: &PrismOperation) -> Result<Self, Error> {
        Ok(LongFormPrismDid::from_operation(operation)?.into_canonical())
    }

    pub fn from_suffix_str(suffix: &str) -> Result<Self, DidSyntaxError> {
        let suffix = HexStr::from_str(suffix).map_err(|e| DidSyntaxError::DidSuffixInvalidStr {
            source: e,
            suffix: suffix.to_string(),
        })?;
        Self::from_suffix(suffix)
    }

    pub fn from_suffix(suffix: HexStr) -> Result<Self, DidSyntaxError> {
        let suffix = Sha256Digest::from_bytes(&suffix.to_bytes())
            .map_err(|e| DidSyntaxError::DidSuffixInvalidHex { source: e, suffix })?;
        Ok(Self { suffix })
    }
}

impl LongFormPrismDid {
    pub fn from_operation(operation: &PrismOperation) -> Result<Self, Error> {
        match operation.operation {
            Some(Operation::CreateDid(_)) => {
                let bytes = operation.encode_to_vec();
                let suffix = operation.operation_hash();
                let encoded_state = Base64UrlStrNoPad::from(bytes);
                Ok(Self { suffix, encoded_state })
            }
            None => Err(Error::OperationMissingFromPrismOperation),
            Some(_) => Err(Error::LongFormDidNotFromCreateOperation),
        }
    }

    pub fn operation(&self) -> Result<PrismOperation, Error> {
        let operation = PrismOperation::decode(self.encoded_state.to_bytes().as_slice()).map_err(|e| {
            DidSyntaxError::DidEncodedStateInvalidProto {
                source: e,
                did: self.to_string(),
            }
        })?;
        Ok(operation)
    }
}

impl FromStr for PrismDid {
    type Err = Error;

    /// # Example
    /// ```
    /// use std::str::FromStr;
    ///
    /// use identus_did_prism::did::PrismDid;
    ///
    /// let did = PrismDid::from_str("did:prism:1234567890abcdef");
    /// assert!(did.is_err());
    ///
    /// let did = PrismDid::from_str(
    ///     "did:prism:0000000000000000000000000000000000000000000000000000000000000000",
    /// )
    /// .unwrap();
    /// assert_eq!(
    ///     did.to_string(),
    ///     "did:prism:0000000000000000000000000000000000000000000000000000000000000000"
    /// );
    /// assert!(matches!(did, PrismDid::Canonical(_)));
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with("did:prism:") {
            Err(DidSyntaxError::DidSyntaxInvalid { did: s.to_string() })?
        }
        let (_, method_specific_id) = s.split_at("did:prism:".len());

        let canonical_match = CANONICAL_SUFFIX_RE.captures(method_specific_id);
        let long_form_match = LONG_FORM_SUFFIX_RE.captures(method_specific_id);

        match (canonical_match, long_form_match) {
            (None, Some(long_form_match)) => {
                let match_group_1 = long_form_match.get(1).expect("Regex did not match this group").as_str();
                let match_group_2 = long_form_match.get(2).expect("Regex did not match this group").as_str();
                let suffix: HexStr = match_group_1.parse().map_err(|e| DidSyntaxError::DidSuffixInvalidStr {
                    source: e,
                    suffix: match_group_1.to_string(),
                })?;
                let encoded_state: Base64UrlStrNoPad =
                    match_group_2
                        .parse()
                        .map_err(|e| DidSyntaxError::DidEncodedStateInvalidStr {
                            source: e,
                            encoded_state: match_group_2.to_string(),
                        })?;
                let operation = PrismOperation::decode(encoded_state.to_bytes().as_slice()).map_err(|e| {
                    DidSyntaxError::DidEncodedStateInvalidProto {
                        source: e,
                        did: s.to_string(),
                    }
                })?;
                let did = LongFormPrismDid::from_operation(&operation)?;
                if did.suffix_hex() == suffix {
                    Ok(did.into())
                } else {
                    Err(DidSyntaxError::DidSuffixEncodedStateUnmatched {
                        did: s.to_string(),
                        expected_did: did.into_canonical(),
                    })?
                }
            }
            (Some(canonical_match), None) => {
                let match_group_1 = canonical_match.get(1).expect("Regex did not match this group").as_str();
                let suffix: HexStr = match_group_1.parse().map_err(|e| DidSyntaxError::DidSuffixInvalidStr {
                    source: e,
                    suffix: match_group_1.to_string(),
                })?;
                let did = CanonicalPrismDid::from_suffix(suffix)?;
                Ok(did.into())
            }
            _ => Err(DidSyntaxError::DidSyntaxInvalid { did: s.to_string() })?,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DidState {
    pub did: CanonicalPrismDid,
    pub context: Vec<String>,
    pub last_operation_hash: Rc<Sha256Digest>,
    pub public_keys: Vec<PublicKey>,
    pub services: Vec<Service>,
    pub storage: Vec<StorageState>,
}

impl DidState {
    pub fn is_deactivated(&self) -> bool {
        self.public_keys.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageState {
    pub init_operation_hash: Rc<Sha256Digest>,
    pub last_operation_hash: Rc<Sha256Digest>,
    pub data: Rc<StorageData>,
}

impl From<DidState> for node_api::DIDData {
    fn from(value: DidState) -> Self {
        node_api::DIDData {
            id: value.did.to_string().replace("did:prism:", ""),
            public_keys: value.public_keys.into_iter().map(|i| i.orig).collect(),
            services: value.services.into_iter().map(|i| i.orig).collect(),
            context: value.context,
            storage_data: value.storage.into_iter().flat_map(|i| i.try_into().ok()).collect(),
            special_fields: Default::default(),
        }
    }
}

impl TryFrom<StorageState> for node_api::StorageData {
    type Error = &'static str;

    fn try_from(value: StorageState) -> Result<Self, Self::Error> {
        match value.data.as_ref() {
            StorageData::Bytes(items) => Ok(Self {
                init_operation_hash: value.init_operation_hash.to_vec(),
                prev_operation_hash: value.last_operation_hash.to_vec(),
                data: Some(node_api::storage_data::Data::Bytes(items.clone())),
                special_fields: Default::default(),
            }),
            _ => Err("only bytes is supported in DIDData"),
        }
    }
}
