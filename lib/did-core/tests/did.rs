use std::str::FromStr;

use identus_did_core::{Did, DidOps, DidUrl, DidUrlOps};

#[test]
fn parse_did() {
    let did: Did = "did:example:abcdefghi".parse().unwrap();
    assert_eq!(did.to_string(), "did:example:abcdefghi");
    assert_eq!(did.method(), "example");
    assert_eq!(did.method_id(), "abcdefghi");

    let did: Did = "did:prism:9bf36a6dd4090ad66e359a0c041e25662c3f84c00467e9a61eeba68477c8a595"
        .parse()
        .unwrap();
    assert_eq!(
        did.to_string(),
        "did:prism:9bf36a6dd4090ad66e359a0c041e25662c3f84c00467e9a61eeba68477c8a595"
    );
    assert_eq!(did.method(), "prism");
    assert_eq!(
        did.method_id(),
        "9bf36a6dd4090ad66e359a0c041e25662c3f84c00467e9a61eeba68477c8a595"
    );
}

#[test]
fn parse_did_fail() {
    assert!(Did::from_str("did").is_err());
    assert!(Did::from_str("did:").is_err());
    assert!(Did::from_str("did::").is_err());
    assert!(Did::from_str("did:example").is_err());
    assert!(Did::from_str("did:example:").is_err());
    assert!(Did::from_str("did:_______:abcdefghi").is_err());
    assert!(Did::from_str("did:example:abcdefghi?service=abc").is_err());
    assert!(Did::from_str("did:example:abcdefghi#key-1").is_err());
}

#[test]
fn parse_did_url() {
    let did: DidUrl = "did:example:abcdefghi".parse().unwrap();
    assert_eq!(did.to_string(), "did:example:abcdefghi");

    let did: DidUrl = "did:prism:9bf36a6dd4090ad66e359a0c041e25662c3f84c00467e9a61eeba68477c8a595"
        .parse()
        .unwrap();
    assert_eq!(
        did.to_string(),
        "did:prism:9bf36a6dd4090ad66e359a0c041e25662c3f84c00467e9a61eeba68477c8a595"
    );
}

#[test]
fn did_to_did_url() {
    let did: Did = "did:example:abcdefghi".parse().unwrap();
    let did_url = did.to_did_url();
    assert_eq!(did_url.to_string(), "did:example:abcdefghi");
}

#[test]
fn did_url_did_ops_method_and_method_id() {
    let did_url: DidUrl = "did:example:abcdefghi".parse().unwrap();
    assert_eq!(did_url.method(), "example");
    assert_eq!(did_url.method_id(), "abcdefghi");

    let did_url: DidUrl = "did:prism:9bf36a6dd4090ad66e359a0c041e25662c3f84c00467e9a61eeba68477c8a595"
        .parse()
        .unwrap();
    assert_eq!(did_url.method(), "prism");
    assert_eq!(
        did_url.method_id(),
        "9bf36a6dd4090ad66e359a0c041e25662c3f84c00467e9a61eeba68477c8a595"
    );
}

#[test]
fn did_url_fragment_query_and_path() {
    let did_url: DidUrl = "did:example:abcdefghi#key-1".parse().unwrap();
    assert_eq!(did_url.fragment(), Some("key-1"));
    assert_eq!(did_url.path(), None);
    assert_eq!(did_url.query(), None);

    let did_url: DidUrl = "did:example:abcdefghi?service=abc".parse().unwrap();
    assert_eq!(did_url.fragment(), None);
    assert_eq!(did_url.query(), Some("service=abc"));
}

#[test]
fn parse_did_rejects_path() {
    // A DID URL with a path should be rejected by Did::from_str
    let result = Did::from_str("did:example:abcdefghi/some/path");
    assert!(result.is_err(), "expected DID with path to be rejected");
}

#[test]
fn did_url_to_did_strips_components() {
    let did_url: DidUrl = "did:example:abcdefghi/some/path?service=abc#key-1".parse().unwrap();
    // Confirm the URL actually carries all three components before stripping
    assert_eq!(did_url.path(), Some("/some/path"));
    assert_eq!(did_url.query(), Some("service=abc"));
    assert_eq!(did_url.fragment(), Some("key-1"));

    let did = did_url.to_did();
    assert_eq!(did.to_string(), "did:example:abcdefghi");
    assert_eq!(did.method(), "example");
    assert_eq!(did.method_id(), "abcdefghi");
}
