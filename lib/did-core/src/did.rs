use std::str::FromStr;

use identus_did::{Did as SdkDid, DidUrl as SdkDidUrl};
use serde::{Deserialize, Serialize};

use crate::{Error, InvalidDid};

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
pub struct Did(#[cfg_attr(feature = "ts-types", ts(type = "string"))] SdkDid);

#[derive(Clone, Serialize, Deserialize, derive_more::Debug, derive_more::Display)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "openapi", schema(value_type = String, example = "did:example:123456789abcdefghi#key-1?service=abc"))]
#[display("{}", self.0.to_string())]
#[debug("{}", self.0.to_string())]
pub struct DidUrl(SdkDidUrl);

impl Did {
    pub fn to_did_url(&self) -> DidUrl {
        DidUrl::from_str(&self.to_string()).unwrap()
    }
}

impl DidUrl {
    pub fn to_did(&self) -> Did {
        Did(self.0.to_did())
    }
}

impl FromStr for Did {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let did_url = SdkDidUrl::parse(s).map_err(InvalidDid::from)?;
        if !did_url.path().is_empty() {
            Err(InvalidDid::new("DID cannot contain path segment(s)"))?;
        }
        if did_url.query().is_some() {
            Err(InvalidDid::new("DID cannot contain query"))?;
        }
        if did_url.fragment().is_some() {
            Err(InvalidDid::new("DID cannot contain fragment"))?;
        }
        Ok(Self(SdkDid::parse(s).map_err(InvalidDid::from)?))
    }
}

impl FromStr for DidUrl {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(SdkDidUrl::parse(s).map_err(InvalidDid::from)?))
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
        self.0.method_specific_id()
    }
}

impl DidOps for DidUrl {
    fn method(&self) -> &str {
        self.0.method()
    }

    fn method_id(&self) -> &str {
        self.0.method_specific_id()
    }
}

impl DidUrlOps for DidUrl {
    fn fragment(&self) -> Option<&str> {
        self.0.fragment()
    }

    fn path(&self) -> Option<&str> {
        let path = self.0.path();
        (!path.is_empty()).then_some(path)
    }

    fn query(&self) -> Option<&str> {
        self.0.query()
    }
}
