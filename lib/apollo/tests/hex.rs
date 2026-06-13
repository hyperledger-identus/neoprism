#![cfg(feature = "hex")]

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::str::FromStr;

use identus_apollo::hex::HexStr;

// ---------------------------------------------------------------------------
// HexStr::from() — From<&[u8]>, From<Vec<u8>>, From<&str> as bytes
// ---------------------------------------------------------------------------

#[test]
fn hex_str_from_bytes_slice() {
    let data = b"hello";
    let hex_str = HexStr::from(&data[..]);
    assert_eq!(hex_str.to_string(), "68656c6c6f");
}

#[test]
fn hex_str_from_vec() {
    let data: Vec<u8> = vec![0xde, 0xad, 0xbe, 0xef];
    let hex_str = HexStr::from(data);
    assert_eq!(hex_str.to_string(), "deadbeef");
}

#[test]
fn hex_str_from_empty_bytes() {
    let data: &[u8] = &[];
    let hex_str = HexStr::from(data);
    assert_eq!(hex_str.to_string(), "");
}

#[test]
fn hex_str_from_byte_array() {
    let data: [u8; 4] = [0xca, 0xfe, 0xba, 0xbe];
    let hex_str = HexStr::from(data);
    assert_eq!(hex_str.to_string(), "cafebabe");
}

// ---------------------------------------------------------------------------
// HexStr::from_str() — success cases
// ---------------------------------------------------------------------------

#[test]
fn from_str_valid_hex_lowercase() {
    let hex_str = HexStr::from_str("deadbeef").unwrap();
    assert_eq!(hex_str.to_bytes(), vec![0xde, 0xad, 0xbe, 0xef]);
}

#[test]
fn from_str_valid_hex_uppercase() {
    let hex_str = HexStr::from_str("DEADBEEF").unwrap();
    assert_eq!(hex_str.to_bytes(), vec![0xde, 0xad, 0xbe, 0xef]);
}

#[test]
fn from_str_valid_hex_mixed_case() {
    let hex_str = HexStr::from_str("dEaDbEeF").unwrap();
    assert_eq!(hex_str.to_bytes(), vec![0xde, 0xad, 0xbe, 0xef]);
}

#[test]
fn from_str_empty_string() {
    let hex_str = HexStr::from_str("").unwrap();
    assert_eq!(hex_str.to_bytes(), Vec::<u8>::new());
}

#[test]
fn from_str_single_byte() {
    let hex_str = HexStr::from_str("ff").unwrap();
    assert_eq!(hex_str.to_bytes(), vec![0xff]);
}

// ---------------------------------------------------------------------------
// HexStr::from_str() — error cases
// ---------------------------------------------------------------------------

#[test]
fn from_str_odd_length_returns_error() {
    let result = HexStr::from_str("abc");
    let err = result.expect_err("odd-length hex should fail");
    let msg = err.to_string();
    assert!(msg.contains("abc"), "error message should contain the input value");
    assert!(msg.contains("HexStr"), "error message should contain the type name");
    assert!(
        msg.contains("unable to hex decode"),
        "error message should contain prefix"
    );
}

#[test]
fn from_str_invalid_hex_chars_returns_error() {
    let result = HexStr::from_str("zzzz");
    let err = result.expect_err("invalid hex chars should fail");
    let msg = err.to_string();
    assert!(msg.contains("zzzz"), "error message should contain the input value");
}

#[test]
fn from_str_non_ascii_returns_error() {
    let result = HexStr::from_str("ññ");
    assert!(result.is_err(), "non-ASCII should fail hex decode");
}

// ---------------------------------------------------------------------------
// Error display formatting
// ---------------------------------------------------------------------------

#[test]
fn error_display_contains_all_fields() {
    let err = HexStr::from_str("xyz").unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("unable to hex decode"), "prefix");
    assert!(msg.contains("xyz"), "value");
    assert!(msg.contains("HexStr"), "type name");
}

#[test]
fn error_debug_format() {
    let err = HexStr::from_str("xyz").unwrap_err();
    let debug = format!("{err:?}");
    // Debug derive should show the struct name and fields
    assert!(debug.contains("Error"), "debug should contain Error");
}

// ---------------------------------------------------------------------------
// HexStr::to_bytes()
// ---------------------------------------------------------------------------

#[test]
fn to_bytes_roundtrip() {
    let original = b"hello world";
    let hex_str = HexStr::from(&original[..]);
    let decoded = hex_str.to_bytes();
    assert_eq!(decoded, original.to_vec());
}

#[test]
fn to_bytes_empty() {
    let hex_str = HexStr::from(&[][..]);
    assert!(hex_str.to_bytes().is_empty());
}

#[test]
fn to_bytes_all_zeros() {
    let hex_str = HexStr::from_str("00000000").unwrap();
    assert_eq!(hex_str.to_bytes(), vec![0u8; 4]);
}

#[test]
fn to_bytes_all_ff() {
    let hex_str = HexStr::from_str("ffffffff").unwrap();
    assert_eq!(hex_str.to_bytes(), vec![0xff; 4]);
}

// ---------------------------------------------------------------------------
// Clone, PartialEq, Eq
// ---------------------------------------------------------------------------

#[test]
fn hex_str_clone() {
    let hex_str = HexStr::from(b"test");
    let clone = hex_str.clone();
    assert_eq!(hex_str, clone);
}

#[test]
fn hex_str_equality_same() {
    let a = HexStr::from(b"data");
    let b = HexStr::from(b"data");
    assert_eq!(a, b);
}

#[test]
fn hex_str_equality_different() {
    let a = HexStr::from(b"foo");
    let b = HexStr::from(b"bar");
    assert_ne!(a, b);
}

// ---------------------------------------------------------------------------
// Hash
// ---------------------------------------------------------------------------

#[test]
fn hex_str_hash_equal_values() {
    let a = HexStr::from(b"hashme");
    let b = HexStr::from(b"hashme");

    let mut hasher_a = DefaultHasher::new();
    let mut hasher_b = DefaultHasher::new();
    a.hash(&mut hasher_a);
    b.hash(&mut hasher_b);

    assert_eq!(hasher_a.finish(), hasher_b.finish());
}

#[test]
fn hex_str_hash_different_values() {
    let a = HexStr::from(b"aaaa");
    let b = HexStr::from(b"bbbb");

    let mut hasher_a = DefaultHasher::new();
    let mut hasher_b = DefaultHasher::new();
    a.hash(&mut hasher_a);
    b.hash(&mut hasher_b);

    assert_ne!(hasher_a.finish(), hasher_b.finish());
}

// ---------------------------------------------------------------------------
// Display (derive_more::Display on HexStr delegates to inner String)
// ---------------------------------------------------------------------------

#[test]
fn hex_str_display() {
    let hex_str = HexStr::from(b"AB");
    // b"AB" = [0x41, 0x42] → "4142"
    assert_eq!(format!("{hex_str}"), "4142");
}

// ---------------------------------------------------------------------------
// AsRef<String> / Into<String>
// ---------------------------------------------------------------------------

#[test]
fn hex_str_as_ref() {
    let hex_str = HexStr::from(b"test");
    let ref_str: &String = hex_str.as_ref();
    assert_eq!(ref_str, "74657374");
}

#[test]
fn hex_str_into_string() {
    let hex_str = HexStr::from(b"test");
    let s: String = hex_str.into();
    assert_eq!(s, "74657374");
}

// ---------------------------------------------------------------------------
// Serde roundtrip (requires "serde" feature)
// ---------------------------------------------------------------------------

#[cfg(feature = "serde")]
mod serde_tests {
    use identus_apollo::hex::HexStr;

    #[test]
    fn serde_roundtrip() {
        let original = HexStr::from(b"serde test");
        let json = serde_json::to_string(&original).unwrap();
        // Should be a JSON string containing the hex encoding
        assert!(json.contains("73657264652074657374"));
        let deserialized: HexStr = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn serde_deserialize_valid_hex() {
        let json = r#""68656c6c6f""#;
        let hex_str: HexStr = serde_json::from_str(json).unwrap();
        assert_eq!(hex_str.to_bytes(), b"hello".to_vec());
    }

    #[test]
    fn serde_deserialize_invalid_hex_returns_error() {
        let json = r#""zzzz""#;
        let result: Result<HexStr, _> = serde_json::from_str(json);
        assert!(result.is_err(), "deserializing invalid hex should fail");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("unable to hex decode") || err_msg.contains("zzzz"),
            "error should mention hex decode or the value, got: {err_msg}"
        );
    }

    #[test]
    fn serde_deserialize_odd_length_returns_error() {
        let json = r#""abc""#;
        let result: Result<HexStr, _> = serde_json::from_str(json);
        assert!(result.is_err(), "deserializing odd-length hex should fail");
    }

    #[test]
    fn serde_serialize_produces_hex_string() {
        let hex_str = HexStr::from(b"\x00\x01\x02");
        let json = serde_json::to_string(&hex_str).unwrap();
        assert_eq!(json, r#""000102""#);
    }
}
