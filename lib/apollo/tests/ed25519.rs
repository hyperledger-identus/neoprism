#![cfg(feature = "ed25519")]

use ed25519_dalek::Signer;
use identus_apollo::crypto::ed25519::Ed25519PublicKey;
use identus_apollo::crypto::{EncodeArray, EncodeVec, Verifiable};
use identus_apollo::jwk::EncodeJwk;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A well-known Ed25519 signing key (32 bytes).
fn sample_signing_key() -> ed25519_dalek::SigningKey {
    ed25519_dalek::SigningKey::from_bytes(&[
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, //
        0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, //
        0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, //
        0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20, //
    ])
}

fn sample_public_key() -> Ed25519PublicKey {
    let signing = sample_signing_key();
    Ed25519PublicKey::from_slice(signing.verifying_key().as_bytes()).unwrap()
}

// ---------------------------------------------------------------------------
// Ed25519PublicKey::from_slice — invalid key bytes (covers line 17 `?` path)
// ---------------------------------------------------------------------------

#[test]
fn from_slice_invalid_32_bytes_returns_error() {
    // [0x02; 32] is not a valid Ed25519 point (y=2 does not produce a valid x on the curve)
    let result = Ed25519PublicKey::from_slice(&[0x02; 32]);
    assert!(
        result.is_err(),
        "all-0x02 bytes should be rejected as an invalid Ed25519 key"
    );
}

// ---------------------------------------------------------------------------
// EncodeVec for Ed25519PublicKey (covers lines 23-25)
// ---------------------------------------------------------------------------

#[test]
fn encode_vec_returns_32_bytes_matching_encode_array() {
    let pk = sample_public_key();
    let vec = pk.encode_vec();
    let arr: [u8; 32] = pk.encode_array();
    assert_eq!(vec.len(), 32);
    assert_eq!(vec.as_slice(), arr);
}

#[test]
fn encode_vec_roundtrip() {
    let pk = sample_public_key();
    let bytes = pk.encode_vec();
    let recovered = Ed25519PublicKey::from_slice(&bytes).unwrap();
    assert_eq!(pk, recovered);
}

// ---------------------------------------------------------------------------
// Verifiable for Ed25519PublicKey (covers lines 35-40)
// ---------------------------------------------------------------------------

#[test]
fn verify_valid_signature_returns_true() {
    let signing = sample_signing_key();
    let verifying = signing.verifying_key();
    let pk = Ed25519PublicKey::from_slice(verifying.as_bytes()).unwrap();
    let message = b"test message for ed25519";
    let signature = signing.sign(message);
    assert!(
        pk.verify(message, &signature.to_bytes()),
        "valid Ed25519 signature should verify"
    );
}

#[test]
fn verify_wrong_message_returns_false() {
    let signing = sample_signing_key();
    let verifying = signing.verifying_key();
    let pk = Ed25519PublicKey::from_slice(verifying.as_bytes()).unwrap();
    let signature = signing.sign(b"correct message");
    assert!(
        !pk.verify(b"wrong message", &signature.to_bytes()),
        "signature should not verify against a different message"
    );
}

#[test]
fn verify_wrong_key_returns_false() {
    let signing1 = sample_signing_key();
    let signing2 = ed25519_dalek::SigningKey::from_bytes(&[0xAB; 32]);
    let pk2 = Ed25519PublicKey::from_slice(signing2.verifying_key().as_bytes()).unwrap();
    let signature = signing1.sign(b"message");
    assert!(
        !pk2.verify(b"message", &signature.to_bytes()),
        "signature should not verify against a different public key"
    );
}

#[test]
fn verify_invalid_signature_bytes_returns_false() {
    let pk = sample_public_key();
    // Random garbage is not a valid Ed25519 signature
    assert!(!pk.verify(b"message", b"this is not a valid signature at all"));
}

#[test]
fn verify_empty_signature_returns_false() {
    let pk = sample_public_key();
    assert!(!pk.verify(b"message", &[]));
}

#[test]
fn verify_truncated_signature_returns_false() {
    let signing = sample_signing_key();
    let verifying = signing.verifying_key();
    let pk = Ed25519PublicKey::from_slice(verifying.as_bytes()).unwrap();
    let signature = signing.sign(b"message");
    let truncated = &signature.to_bytes()[..32];
    assert!(!pk.verify(b"message", truncated));
}

// ---------------------------------------------------------------------------
// EncodeJwk for Ed25519PublicKey (already covered but add parity checks)
// ---------------------------------------------------------------------------

#[test]
fn encode_jwk_has_correct_kty_and_crv() {
    let pk = sample_public_key();
    let jwk = pk.encode_jwk();
    assert_eq!(jwk.kty, "OKP");
    assert_eq!(jwk.crv, "Ed25519");
}

#[test]
fn encode_jwk_has_x_but_no_y() {
    let pk = sample_public_key();
    let jwk = pk.encode_jwk();
    assert!(jwk.x.is_some(), "Ed25519 JWK should contain x");
    assert!(jwk.y.is_none(), "Ed25519 JWK should not contain y");
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
// Clone / PartialEq / Debug
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
    assert!(debug.contains("Ed25519PublicKey"), "debug should contain type name");
}
