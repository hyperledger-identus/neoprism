#![cfg(feature = "x25519")]

use identus_apollo::crypto::x25519::X25519PublicKey;
use identus_apollo::crypto::{EncodeArray, EncodeVec};
use identus_apollo::jwk::EncodeJwk;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A well-known 32-byte X25519 public key (the identity point is valid for x25519-dalek).
fn sample_public_key_bytes() -> [u8; 32] {
    // Any 32 bytes are valid for x25519-dalek::PublicKey (it does not validate the point).
    [
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, //
        0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, //
        0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, //
        0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20, //
    ]
}

fn sample_public_key() -> X25519PublicKey {
    X25519PublicKey::from_slice(&sample_public_key_bytes()).unwrap()
}

// ---------------------------------------------------------------------------
// X25519PublicKey::from_slice
// ---------------------------------------------------------------------------

#[test]
fn from_slice_valid_32_bytes_succeeds() {
    let key = X25519PublicKey::from_slice(&sample_public_key_bytes());
    assert!(key.is_ok());
}

#[test]
fn from_slice_empty_bytes_returns_error() {
    let result = X25519PublicKey::from_slice(&[]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("32"), "error should mention expected size: {msg}");
    assert!(msg.contains("0"), "error should mention actual size: {msg}");
}

#[test]
fn from_slice_too_few_bytes_returns_error() {
    let result = X25519PublicKey::from_slice(&[0u8; 16]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("32"), "error should mention expected size: {msg}");
    assert!(msg.contains("16"), "error should mention actual size: {msg}");
}

#[test]
fn from_slice_33_bytes_returns_error() {
    // Regression test: from_slice must reject oversized input (it previously
    // used split_first_chunk::<32>() which silently dropped trailing bytes).
    // See .work/bugs.md for details.
    let mut input = [0xABu8; 33];
    input[32] = 0xFF; // extra byte that must NOT be silently ignored
    let result = X25519PublicKey::from_slice(&input);
    assert!(result.is_err(), "from_slice must reject >32 bytes");
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("32"), "error should mention expected size: {msg}");
    assert!(msg.contains("33"), "error should mention actual size: {msg}");
}

#[test]
fn from_slice_exactly_32_bytes_succeeds() {
    let key = X25519PublicKey::from_slice(&[0xAB; 32]).unwrap();
    // Verify round-trip via encode_array
    assert_eq!(key.encode_array(), [0xABu8; 32]);
}

// ---------------------------------------------------------------------------
// EncodeVec for X25519PublicKey (covers lines 23-25, previously uncovered)
// ---------------------------------------------------------------------------

#[test]
fn encode_vec_returns_32_bytes() {
    let pk = sample_public_key();
    let vec = pk.encode_vec();
    assert_eq!(vec.len(), 32);
}

#[test]
fn encode_vec_matches_encode_array() {
    let pk = sample_public_key();
    let vec = pk.encode_vec();
    let arr: [u8; 32] = pk.encode_array();
    assert_eq!(vec.as_slice(), arr);
}

#[test]
fn encode_vec_roundtrip() {
    let pk = sample_public_key();
    let bytes = pk.encode_vec();
    let recovered = X25519PublicKey::from_slice(&bytes).unwrap();
    assert_eq!(pk, recovered);
}

// ---------------------------------------------------------------------------
// EncodeArray<32> for X25519PublicKey
// ---------------------------------------------------------------------------

#[test]
fn encode_array_returns_same_bytes_as_input() {
    let input = sample_public_key_bytes();
    let pk = X25519PublicKey::from_slice(&input).unwrap();
    assert_eq!(pk.encode_array(), input);
}

// ---------------------------------------------------------------------------
// EncodeJwk for X25519PublicKey
// ---------------------------------------------------------------------------

#[test]
fn encode_jwk_has_correct_kty_and_crv() {
    let pk = sample_public_key();
    let jwk = pk.encode_jwk();
    assert_eq!(jwk.kty, "OKP");
    assert_eq!(jwk.crv, "X25519");
}

#[test]
fn encode_jwk_has_x_but_no_y() {
    let pk = sample_public_key();
    let jwk = pk.encode_jwk();
    assert!(jwk.x.is_some(), "X25519 JWK should contain x");
    assert!(jwk.y.is_none(), "X25519 JWK should not contain y");
}

#[test]
fn encode_jwk_x_matches_encode_array() {
    use identus_apollo::base64::Base64UrlStrNoPad;
    let pk = sample_public_key();
    let jwk = pk.encode_jwk();
    let expected_x = Base64UrlStrNoPad::from(pk.encode_array());
    assert_eq!(jwk.x.as_ref().unwrap(), &expected_x);
}

// ---------------------------------------------------------------------------
// Clone / PartialEq / Debug / Hash
// ---------------------------------------------------------------------------

#[test]
fn public_key_clone_is_equal() {
    let pk = sample_public_key();
    let clone = pk.clone();
    assert_eq!(pk, clone);
}

#[test]
fn public_key_debug_contains_type_name() {
    let pk = sample_public_key();
    let debug = format!("{pk:?}");
    assert!(
        debug.contains("X25519PublicKey"),
        "debug should contain type name: {debug}"
    );
}

#[test]
fn public_keys_from_same_bytes_are_equal() {
    let pk1 = X25519PublicKey::from_slice(&sample_public_key_bytes()).unwrap();
    let pk2 = X25519PublicKey::from_slice(&sample_public_key_bytes()).unwrap();
    assert_eq!(pk1, pk2);
}

#[test]
fn public_keys_from_different_bytes_are_not_equal() {
    let pk1 = X25519PublicKey::from_slice(&[0x01; 32]).unwrap();
    let pk2 = X25519PublicKey::from_slice(&[0x02; 32]).unwrap();
    assert_ne!(pk1, pk2);
}
