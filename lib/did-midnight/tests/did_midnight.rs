use std::str::FromStr;

use identus_did_midnight::did::{MidnightContractAddress, MidnightDid, MidnightNetwork};
use identus_did_midnight::error::Error;

#[test]
fn test_midnight_network_from_str() {
    assert_eq!(MidnightNetwork::from_str("mainnet").unwrap(), MidnightNetwork::Mainnet);
    assert_eq!(MidnightNetwork::from_str("testnet").unwrap(), MidnightNetwork::Testnet);
    assert_eq!(MidnightNetwork::from_str("devnet").unwrap(), MidnightNetwork::Devnet);
    assert_eq!(
        MidnightNetwork::from_str("undeployed").unwrap(),
        MidnightNetwork::Undeployed
    );
    assert!(MidnightNetwork::from_str("other").is_err());
}

#[test]
fn test_contract_address_valid() {
    let hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123";
    let addr = MidnightContractAddress::from_str(hex).unwrap();
    assert_eq!(addr.to_string(), hex);
}

#[test]
fn test_contract_address_invalid_length() {
    let hex = "0123456789abcdef";
    let err = MidnightContractAddress::from_str(hex).unwrap_err();
    assert!(matches!(err, Error::InvalidAddressLength { .. }));
}

#[test]
fn test_contract_address_invalid_hex() {
    let hex = "g123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef01";
    let err = MidnightContractAddress::from_str(hex).unwrap_err();
    assert!(matches!(err, Error::InvalidAddressHex { .. }));
}

#[test]
fn test_contract_address_invalid_case() {
    let hex = "0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef01";
    let err = MidnightContractAddress::from_str(hex).unwrap_err();
    assert!(matches!(err, Error::InvalidAddressCase));
}

#[test]
fn test_midnight_did_valid() {
    let did = "did:midnight:mainnet:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123";
    let parsed = MidnightDid::from_str(did).unwrap();
    assert_eq!(parsed.network(), MidnightNetwork::Mainnet);
    assert_eq!(
        parsed.contract_address().to_string(),
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123"
    );
    assert_eq!(parsed.to_string(), did);
}

#[test]
fn test_midnight_did_invalid_method() {
    let did = "did:wrong:mainnet:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef01";
    let err = MidnightDid::from_str(did).unwrap_err();
    assert!(matches!(err, Error::InvalidMethod { .. }));
}

#[test]
fn test_midnight_did_invalid_segment_count() {
    let did = "did:midnight:mainnet";
    let err = MidnightDid::from_str(did).unwrap_err();
    assert!(matches!(err, Error::InvalidSegmentCount { .. }));
}
