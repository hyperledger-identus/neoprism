#![cfg(all(feature = "secp256k1", feature = "hex"))]

use std::str::FromStr;

use identus_apollo::crypto::secp256k1::{Secp256k1PrivateKey, Secp256k1PublicKey};
use identus_apollo::crypto::{EncodeArray, EncodeVec, Verifiable};
use identus_apollo::hex::HexStr;
use identus_apollo::jwk::EncodeJwk;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A well-known secp256k1 private key (32 bytes).
fn sample_private_key_bytes() -> [u8; 32] {
    [
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, //
        0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, //
        0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, //
        0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20, //
    ]
}

fn sample_private_key() -> Secp256k1PrivateKey {
    Secp256k1PrivateKey::from_slice(&sample_private_key_bytes()).unwrap()
}

fn sample_public_key() -> Secp256k1PublicKey {
    sample_private_key().to_public_key()
}

// ---------------------------------------------------------------------------
// Secp256k1PublicKey::from_slice
// ---------------------------------------------------------------------------

#[test]
fn public_key_from_slice_compressed_valid() {
    let pk = sample_public_key();
    let compressed = pk.encode_compressed();
    let recovered = Secp256k1PublicKey::from_slice(&compressed).unwrap();
    assert_eq!(pk, recovered);
}

#[test]
fn public_key_from_slice_uncompressed_valid() {
    let pk = sample_public_key();
    let uncompressed = pk.encode_uncompressed();
    let recovered = Secp256k1PublicKey::from_slice(&uncompressed).unwrap();
    assert_eq!(pk, recovered);
}

#[test]
fn public_key_from_slice_invalid_bytes_returns_error() {
    let result = Secp256k1PublicKey::from_slice(&[0u8; 33]);
    assert!(result.is_err(), "random bytes should not parse as a valid public key");
}

#[test]
fn public_key_from_slice_empty_returns_error() {
    let result = Secp256k1PublicKey::from_slice(&[]);
    assert!(result.is_err(), "empty slice should not parse as a valid public key");
}

#[test]
fn public_key_from_slice_wrong_length_returns_error() {
    let result = Secp256k1PublicKey::from_slice(&[0x02; 10]);
    assert!(
        result.is_err(),
        "wrong-length slice should not parse as a valid public key"
    );
}

// ---------------------------------------------------------------------------
// Secp256k1PublicKey encoding
// ---------------------------------------------------------------------------

#[test]
fn encode_compressed_returns_33_bytes() {
    let pk = sample_public_key();
    let compressed = pk.encode_compressed();
    assert_eq!(compressed.len(), 33);
    // Compressed public key starts with 0x02 or 0x03
    assert!(compressed[0] == 0x02 || compressed[0] == 0x03);
}

#[test]
fn encode_uncompressed_returns_65_bytes() {
    let pk = sample_public_key();
    let uncompressed = pk.encode_uncompressed();
    assert_eq!(uncompressed.len(), 65);
    // Uncompressed public key starts with 0x04
    assert_eq!(uncompressed[0], 0x04);
}

#[test]
fn encode_compressed_and_uncompressed_represent_same_key() {
    let pk = sample_public_key();
    let compressed = pk.encode_compressed();
    let uncompressed = pk.encode_uncompressed();
    let from_compressed = Secp256k1PublicKey::from_slice(&compressed).unwrap();
    let from_uncompressed = Secp256k1PublicKey::from_slice(&uncompressed).unwrap();
    assert_eq!(from_compressed, from_uncompressed);
}

#[test]
fn encode_vec_returns_compressed_bytes() {
    let pk = sample_public_key();
    let vec = pk.encode_vec();
    let compressed = pk.encode_compressed();
    assert_eq!(vec, compressed.to_vec());
}

#[test]
fn encode_array_33_returns_compressed() {
    let pk = sample_public_key();
    let arr: [u8; 33] = pk.encode_array();
    assert_eq!(arr, pk.encode_compressed());
}

#[test]
fn encode_array_65_returns_uncompressed() {
    let pk = sample_public_key();
    let arr: [u8; 65] = pk.encode_array();
    assert_eq!(arr, pk.encode_uncompressed());
}

// ---------------------------------------------------------------------------
// Secp256k1PublicKey::curve_point
// ---------------------------------------------------------------------------

#[test]
fn curve_point_returns_32_byte_x_and_y() {
    let pk = sample_public_key();
    let point = pk.curve_point();
    assert_eq!(point.x.len(), 32);
    assert_eq!(point.y.len(), 32);
}

#[test]
fn curve_point_matches_uncompressed_encoding() {
    let pk = sample_public_key();
    let uncompressed = pk.encode_uncompressed();
    let point = pk.curve_point();
    // uncompressed[0] = 0x04 (prefix)
    // uncompressed[1..33] = x
    // uncompressed[33..65] = y
    assert_eq!(&uncompressed[1..33], &point.x);
    assert_eq!(&uncompressed[33..65], &point.y);
}

#[test]
fn curve_point_is_not_all_zeros() {
    let pk = sample_public_key();
    let point = pk.curve_point();
    assert!(
        point.x != [0u8; 32],
        "x coordinate should not be all zeros for a valid key"
    );
    assert!(
        point.y != [0u8; 32],
        "y coordinate should not be all zeros for a valid key"
    );
}

// ---------------------------------------------------------------------------
// Secp256k1PrivateKey
// ---------------------------------------------------------------------------

#[test]
fn private_key_from_slice_valid() {
    let sk = sample_private_key();
    // Roundtrip: serialize and recover
    let sk2 = Secp256k1PrivateKey::from_slice(&sample_private_key_bytes()).unwrap();
    assert_eq!(sk, sk2);
}

#[test]
fn private_key_from_slice_invalid_returns_error() {
    let result = Secp256k1PrivateKey::from_slice(&[0u8; 32]);
    // All-zero is technically a valid secp256k1 scalar in k256 (it's the point at infinity),
    // but k256 rejects it.
    assert!(result.is_err(), "all-zero should be rejected as an invalid private key");
}

#[test]
fn private_key_from_slice_empty_returns_error() {
    let result = Secp256k1PrivateKey::from_slice(&[]);
    assert!(result.is_err(), "empty slice should be rejected");
}

#[test]
fn private_key_to_public_key_deterministic() {
    let sk = sample_private_key();
    let pk1 = sk.to_public_key();
    let pk2 = sk.to_public_key();
    assert_eq!(pk1, pk2);
}

#[test]
fn private_key_to_public_key_different_keys() {
    let sk1 = Secp256k1PrivateKey::from_slice(&[0x01u8; 32]).unwrap();
    let sk2 = Secp256k1PrivateKey::from_slice(&[0x02u8; 32]).unwrap();
    assert_ne!(sk1.to_public_key(), sk2.to_public_key());
}

// ---------------------------------------------------------------------------
// Secp256k1PrivateKey::sign + Secp256k1PublicKey::verify roundtrip
// ---------------------------------------------------------------------------

#[test]
fn sign_and_verify_roundtrip() {
    let sk = sample_private_key();
    let pk = sk.to_public_key();
    let message = b"test message for signing";
    let signature = sk.sign(message);
    assert!(
        pk.verify(message, &signature),
        "signature produced by sign() should verify"
    );
}

#[test]
fn sign_produces_non_empty_der_signature() {
    let sk = sample_private_key();
    let signature = sk.sign(b"message");
    // DER-encoded ECDSA signatures are at least 8 bytes (overhead) and typically 70-72 bytes
    assert!(!signature.is_empty(), "signature should not be empty");
    // DER format starts with 0x30 (SEQUENCE tag)
    assert_eq!(signature[0], 0x30, "signature should be DER-encoded starting with 0x30");
}

#[test]
fn sign_different_messages_produce_different_signatures() {
    let sk = sample_private_key();
    let sig1 = sk.sign(b"message one");
    let sig2 = sk.sign(b"message two");
    assert_ne!(sig1, sig2, "different messages should produce different signatures");
}

#[test]
fn sign_same_message_deterministic() {
    let sk = sample_private_key();
    let sig1 = sk.sign(b"deterministic test");
    let sig2 = sk.sign(b"deterministic test");
    // ECDSA with deterministic nonce (RFC 6979) should produce the same signature
    assert_eq!(sig1, sig2, "same message should produce deterministic signature");
}

#[test]
fn verify_rejects_wrong_message() {
    let sk = sample_private_key();
    let pk = sk.to_public_key();
    let signature = sk.sign(b"correct message");
    assert!(
        !pk.verify(b"wrong message", &signature),
        "signature should not verify against a different message"
    );
}

#[test]
fn verify_rejects_wrong_key() {
    let sk1 = Secp256k1PrivateKey::from_slice(&[0x01u8; 32]).unwrap();
    let sk2 = Secp256k1PrivateKey::from_slice(&[0x02u8; 32]).unwrap();
    let signature = sk1.sign(b"message");
    assert!(
        !sk2.to_public_key().verify(b"message", &signature),
        "signature should not verify against a different public key"
    );
}

// ---------------------------------------------------------------------------
// Secp256k1PublicKey::verify — error / edge-case branches
// ---------------------------------------------------------------------------

#[test]
fn verify_rejects_invalid_der_signature() {
    let pk = sample_public_key();
    // Random garbage is not valid DER
    assert!(!pk.verify(b"message", b"not valid DER"));
}

#[test]
fn verify_rejects_empty_signature() {
    let pk = sample_public_key();
    assert!(!pk.verify(b"message", &[]));
}

#[test]
fn verify_rejects_truncated_der_signature() {
    let sk = sample_private_key();
    let pk = sk.to_public_key();
    let signature = sk.sign(b"message");
    // Truncate the signature — should be invalid DER or fail verification
    let truncated = &signature[..signature.len() / 2];
    assert!(!pk.verify(b"message", truncated));
}

// ---------------------------------------------------------------------------
// Secp256k1PublicKey Clone / PartialEq / Debug
// ---------------------------------------------------------------------------

#[test]
fn public_key_clone_is_equal() {
    let pk = sample_public_key();
    let clone = pk.clone();
    assert_eq!(pk, clone);
}

#[test]
fn public_key_debug_contains_data() {
    let pk = sample_public_key();
    let debug = format!("{pk:?}");
    assert!(debug.contains("Secp256k1PublicKey"), "debug should contain type name");
}

// ---------------------------------------------------------------------------
// Secp256k1PrivateKey Clone / PartialEq / Debug
// ---------------------------------------------------------------------------

#[test]
fn private_key_clone_is_equal() {
    let sk = sample_private_key();
    let clone = sk.clone();
    assert_eq!(sk, clone);
}

#[test]
fn private_key_debug_contains_data() {
    let sk = sample_private_key();
    let debug = format!("{sk:?}");
    assert!(debug.contains("Secp256k1PrivateKey"), "debug should contain type name");
}

// ---------------------------------------------------------------------------
// CurvePoint
// ---------------------------------------------------------------------------

#[test]
fn curve_point_debug() {
    let pk = sample_public_key();
    let point = pk.curve_point();
    let debug = format!("{point:?}");
    assert!(debug.contains("CurvePoint"));
}

#[test]
fn curve_point_clone_and_equality() {
    let pk = sample_public_key();
    let p1 = pk.curve_point();
    let p2 = p1.clone();
    assert_eq!(p1, p2);
}

#[test]
fn curve_point_hash_equal_values() {
    use std::collections::HashSet;
    let pk = sample_public_key();
    let p1 = pk.curve_point();
    let p2 = p1.clone();
    let mut set = HashSet::new();
    set.insert(p1);
    assert!(set.contains(&p2));
}

// ---------------------------------------------------------------------------
// EncodeJwk for Secp256k1PublicKey
// ---------------------------------------------------------------------------

#[test]
fn encode_jwk_has_correct_kty_and_crv() {
    let pk = sample_public_key();
    let jwk = pk.encode_jwk();
    assert_eq!(jwk.kty, "EC");
    assert_eq!(jwk.crv, "secp256k1");
}

#[test]
fn encode_jwk_has_x_and_y_coordinates() {
    let pk = sample_public_key();
    let jwk = pk.encode_jwk();
    assert!(jwk.x.is_some(), "JWK should contain x coordinate");
    assert!(jwk.y.is_some(), "JWK should contain y coordinate");
}

#[test]
fn encode_jwk_coordinates_match_curve_point() {
    use identus_apollo::base64::Base64UrlStrNoPad;
    let pk = sample_public_key();
    let point = pk.curve_point();
    let jwk = pk.encode_jwk();
    let expected_x = Base64UrlStrNoPad::from(point.x);
    let expected_y = Base64UrlStrNoPad::from(point.y);
    assert_eq!(jwk.x.as_ref().unwrap(), &expected_x);
    assert_eq!(jwk.y.as_ref().unwrap(), &expected_y);
}

#[test]
fn encode_jwk_different_keys_produce_different_jwks() {
    let sk1 = Secp256k1PrivateKey::from_slice(&[0x01u8; 32]).unwrap();
    let sk2 = Secp256k1PrivateKey::from_slice(&[0x02u8; 32]).unwrap();
    let jwk1 = sk1.to_public_key().encode_jwk();
    let jwk2 = sk2.to_public_key().encode_jwk();
    assert_ne!(
        jwk1.x, jwk2.x,
        "different keys should have different x coordinates in JWK"
    );
    assert_ne!(
        jwk1.y, jwk2.y,
        "different keys should have different y coordinates in JWK"
    );
}

// ---------------------------------------------------------------------------
// verify — transcoded signature path (high-S with wrong message)
// ---------------------------------------------------------------------------

/// Test vector 0 from the JVM test vectors has a high-S signature.
/// Verifying it against a DIFFERENT message exercises the transcoded signature
/// fallback path in `verify()` (lines 64-72 and `transcode_signature_to_bitcoin`).
const HIGH_S_TEST_VECTOR: HighSTestVector = HighSTestVector {
    public_key: "025ec3069f260463ab79c6ada107de5ef43da1663eb4092d1718f5d26f57f2884b",
    signature: "3046022100ce972f4df5ab2d6aa20151bd56d92f9db42a6b6e9bdbd78971ea80828e183683022100a30cef9d2d28bc1710cec2c1966eb1e5ac965d0be4774a60ba38a462d56c7e7c",
};

struct HighSTestVector {
    public_key: &'static str,
    signature: &'static str,
}

#[test]
fn verify_high_s_signature_wrong_message_returns_false() {
    let pk_bytes = HexStr::from_str(HIGH_S_TEST_VECTOR.public_key).unwrap().to_bytes();
    let pk = Secp256k1PublicKey::from_slice(&pk_bytes).unwrap();
    let sig = HexStr::from_str(HIGH_S_TEST_VECTOR.signature)
        .unwrap()
        .to_bytes()
        .to_vec();
    // Verify against a deliberately wrong message
    let wrong_message = b"wrong message data";
    let result = pk.verify(wrong_message, &sig);
    // The signature is valid DER with high-S, but wrong message,
    // so it should traverse vanilla → normalized → transcoded paths and return false.
    assert!(!result, "high-S signature against wrong message should fail");
}
