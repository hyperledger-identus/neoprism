use std::str::FromStr;
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{Error, InvalidUri};

static URI_FRAGMENT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([A-Za-z0-9\-._~!$&'()*+,;=:@/?]|%[0-9A-Fa-f]{2})*$").expect("URI regex is invalid")
});

#[derive(Clone, Serialize, Deserialize, derive_more::Debug, derive_more::Display)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "openapi", schema(value_type = String, example = "http://example.com"))]
#[cfg_attr(
    feature = "ts-types",
    derive(ts_rs::TS),
    ts(type = "string", export_to = "../../../bindings/ts-types/did_core_types.ts")
)]
#[debug("{}", self.0.to_string())]
#[display("{}", self.0.to_string())]
pub struct Uri(String);

impl FromStr for Uri {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if is_uri(s) {
            Ok(Uri(s.to_string()))
        } else {
            Err(Error::InvalidUri(InvalidUri { msg: "not a valid uri" }))
        }
    }
}

/// Check if the given string is a valid URI
///
/// # Example
/// ```
/// use identus_did_core::uri::is_uri;
/// assert_eq!(is_uri("http://example.com"), true);
/// assert_eq!(is_uri("ftps://example.com/help?q=example"), true);
/// assert_eq!(is_uri("urn:resource"), true);
/// assert_eq!(is_uri("did:web:example.com"), true);
/// assert_eq!(is_uri(""), false);
/// assert_eq!(is_uri("  "), false);
/// assert_eq!(is_uri("foo"), false);
/// assert_eq!(is_uri("hello world"), false);
/// ```
pub fn is_uri(s: &str) -> bool {
    let parsed = uriparse::URI::try_from(s);
    parsed.is_ok()
}

/// Check if the given string is a valid URI fragment.
///
/// # Example
/// ```
/// use identus_did_core::uri::is_uri_fragment;
/// assert_eq!(is_uri_fragment("hello"), true);
/// assert_eq!(is_uri_fragment("hello%20world"), true);
/// assert_eq!(is_uri_fragment("@123"), true);
/// assert_eq!(is_uri_fragment("+-*/"), true);
/// assert_eq!(is_uri_fragment(""), true);
/// assert_eq!(is_uri_fragment("hello world"), false);
/// assert_eq!(is_uri_fragment(" "), false);
/// assert_eq!(is_uri_fragment("hello%"), false);
/// assert_eq!(is_uri_fragment("hello%2"), false);
/// assert_eq!(is_uri_fragment("hello#"), false);
/// ```
pub fn is_uri_fragment(s: &str) -> bool {
    URI_FRAGMENT_RE.is_match(s)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_uri_from_str_valid() {
        let s = "http://example.com";
        let uri = Uri::from_str(s);
        assert!(uri.is_ok());
        assert_eq!(uri.unwrap().0, s);
    }

    #[test]
    fn test_uri_from_str_invalid() {
        let s = "not a uri";
        let uri = Uri::from_str(s);
        assert!(uri.is_err());
        if let Err(crate::Error::InvalidUri(_)) = uri {
            // expected
        } else {
            panic!("Expected InvalidUri error");
        }
    }
}
