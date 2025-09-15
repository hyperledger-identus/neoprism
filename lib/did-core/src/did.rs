use std::str::FromStr;

use identity_did::DID;
use serde::{Deserialize, Serialize};

use crate::InvalidDid;

#[derive(Clone, Serialize, Deserialize, derive_more::Debug, derive_more::Display)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "openapi", schema(value_type = String, example = "did:example:123456789abcdefghi"))]
#[cfg_attr(
    feature = "ts-types",
    derive(ts_rs::TS),
    ts(type = "string", export_to = "../../../bindings/ts-types/did_core_types.ts")
)]
#[debug("{}", self.0.to_string())]
#[display("{}", self.0.to_string())]
pub struct Did(#[cfg_attr(feature = "ts-types", ts(type = "string"))] identity_did::CoreDID);

#[derive(Clone, Serialize, Deserialize, derive_more::Debug, derive_more::Display)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "openapi", schema(value_type = String, example = "did:example:123456789abcdefghi#key-1?service=abc"))]
#[display("{}", self.0.to_string())]
#[debug("{}", self.0.to_string())]
pub struct DidUrl(identity_did::DIDUrl);

impl Did {
    pub fn to_did_url(&self) -> DidUrl {
        DidUrl::from_str(&self.to_string()).unwrap()
    }
}

impl DidUrl {
    pub fn to_did(&self) -> Did {
        let mut did_url = self.0.clone();
        did_url.set_fragment(None).unwrap();
        did_url.set_path(None).unwrap();
        did_url.set_query(None).unwrap();
        Did(did_url.did().clone())
    }
}

impl FromStr for Did {
    type Err = InvalidDid;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let did_url = DidUrl::from_str(s)?;
        if did_url.path().is_some() {
            Err(identity_did::Error::Other("DID cannot contain path segment(s)"))?;
        }
        if did_url.query().is_some() {
            Err(identity_did::Error::Other("DID cannot contain query"))?;
        }
        if did_url.fragment().is_some() {
            Err(identity_did::Error::Other("DID cannot contain fragment"))?;
        }
        Ok(did_url.to_did())
    }
}

impl FromStr for DidUrl {
    type Err = InvalidDid;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(identity_did::DIDUrl::parse(s)?))
    }
}

pub trait DidOps: std::fmt::Display {
    fn method(&self) -> &str;
    fn method_id(&self) -> &str;
}

pub trait DidUrlOps: DidOps + std::fmt::Display {
    fn fragment(&self) -> Option<&str>;
    fn path(&self) -> Option<&str>;
    fn query(&self) -> Option<&str>;
}

impl DidOps for Did {
    fn method(&self) -> &str {
        self.0.method()
    }

    fn method_id(&self) -> &str {
        self.0.method_id()
    }
}

impl DidOps for DidUrl {
    fn method(&self) -> &str {
        self.0.did().method()
    }

    fn method_id(&self) -> &str {
        self.0.did().method_id()
    }
}

impl DidUrlOps for DidUrl {
    fn fragment(&self) -> Option<&str> {
        self.0.fragment()
    }

    fn path(&self) -> Option<&str> {
        self.0.path()
    }

    fn query(&self) -> Option<&str> {
        self.0.query()
    }
}
