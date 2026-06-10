use std::str::FromStr;

use identus_apollo::base64::{Base64UrlStr, Base64UrlStrNoPad};

// ---------------------------------------------------------------------------
// Base64UrlStr
// ---------------------------------------------------------------------------

#[test]
fn base64_url_from_bytes_roundtrip() {
    let original = b"hello world";
    let encoded: Base64UrlStr = original.as_slice().into();
    assert_eq!(encoded.to_string(), "aGVsbG8gd29ybGQ=");
    let decoded = encoded.to_bytes();
    assert_eq!(decoded, original.to_vec());
}

#[test]
fn base64_url_from_empty_bytes() {
    let encoded: Base64UrlStr = Vec::<u8>::new().as_slice().into();
    assert_eq!(encoded.to_string(), "");
    assert!(encoded.to_bytes().is_empty());
}

#[test]
fn base64_url_from_binary_data() {
    let data: Vec<u8> = (0u8..=255).collect();
    let encoded: Base64UrlStr = data.as_slice().into();
    let decoded = encoded.to_bytes();
    assert_eq!(decoded, data);
}

#[test]
fn base64_url_from_str_valid() {
    let b64 = Base64UrlStr::from_str("aGVsbG8gd29ybGQ=").unwrap();
    assert_eq!(b64.to_bytes(), b"hello world".to_vec());
}

#[test]
fn base64_url_from_str_empty() {
    let b64 = Base64UrlStr::from_str("").unwrap();
    assert!(b64.to_bytes().is_empty());
}

#[test]
fn base64_url_from_str_invalid() {
    let result = Base64UrlStr::from_str("!!!invalid!!!");
    assert!(result.is_err());
}

#[test]
fn base64_url_from_str_invalid_padding() {
    let result = Base64UrlStr::from_str("aGVsbG8");
    assert!(result.is_err());
}

#[test]
fn base64_url_error_message_contains_value_and_type() {
    let err = Base64UrlStr::from_str("!!!bad!!!").unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("!!!bad!!!"),
        "error message should contain the invalid value"
    );
    assert!(
        msg.contains("Base64UrlStr"),
        "error message should contain the type name"
    );
}

#[test]
fn base64_url_display() {
    let b64: Base64UrlStr = b"data".as_slice().into();
    assert_eq!(format!("{b64}"), "ZGF0YQ==");
}

#[test]
fn base64_url_equality() {
    let a: Base64UrlStr = b"test".as_slice().into();
    let b: Base64UrlStr = b"test".as_slice().into();
    let c: Base64UrlStr = b"other".as_slice().into();
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn base64_url_clone() {
    let a: Base64UrlStr = b"test".as_slice().into();
    let b = a.clone();
    assert_eq!(a, b);
}

#[test]
fn base64_url_hash_equal_values_same_hash() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    let a: Base64UrlStr = b"test".as_slice().into();
    let b: Base64UrlStr = b"test".as_slice().into();
    set.insert(a.clone());
    assert!(set.contains(&b));
}

#[test]
fn base64_url_into_inner() {
    let b64: Base64UrlStr = b"hello".as_slice().into();
    let inner: String = b64.into();
    assert_eq!(inner, "aGVsbG8=");
}

#[test]
fn base64_url_as_ref_str() {
    let b64: Base64UrlStr = b"hello".as_slice().into();
    let reference: &String = b64.as_ref();
    assert_eq!(reference, "aGVsbG8=");
}

#[test]
fn base64_url_url_safe_characters() {
    // URL-safe base64 uses - and _ instead of + and /
    // Encode data that would produce +/ in standard base64
    let data: Vec<u8> = vec![0xfb, 0xff, 0xfe];
    let encoded: Base64UrlStr = data.as_slice().into();
    let s = encoded.to_string();
    assert!(
        !s.contains('+') && !s.contains('/'),
        "URL-safe base64 should not contain + or /, got: {s}"
    );
}

// ---------------------------------------------------------------------------
// Base64UrlStrNoPad
// ---------------------------------------------------------------------------

#[test]
fn base64_url_no_pad_from_bytes_roundtrip() {
    let original = b"hello world";
    let encoded: Base64UrlStrNoPad = original.as_slice().into();
    assert_eq!(encoded.to_string(), "aGVsbG8gd29ybGQ");
    let decoded = encoded.to_bytes();
    assert_eq!(decoded, original.to_vec());
}

#[test]
fn base64_url_no_pad_from_empty_bytes() {
    let encoded: Base64UrlStrNoPad = Vec::<u8>::new().as_slice().into();
    assert_eq!(encoded.to_string(), "");
    assert!(encoded.to_bytes().is_empty());
}

#[test]
fn base64_url_no_pad_from_binary_data() {
    let data: Vec<u8> = (0u8..=255).collect();
    let encoded: Base64UrlStrNoPad = data.as_slice().into();
    let decoded = encoded.to_bytes();
    assert_eq!(decoded, data);
}

#[test]
fn base64_url_no_pad_no_padding_character() {
    let encoded: Base64UrlStrNoPad = b"hello".as_slice().into();
    let s = encoded.to_string();
    assert!(
        !s.contains('='),
        "No-pad variant should not contain '=' padding, got: {s}"
    );
}

#[test]
fn base64_url_no_pad_from_str_valid() {
    let b64 = Base64UrlStrNoPad::from_str("aGVsbG8gd29ybGQ").unwrap();
    assert_eq!(b64.to_bytes(), b"hello world".to_vec());
}

#[test]
fn base64_url_no_pad_from_str_empty() {
    let b64 = Base64UrlStrNoPad::from_str("").unwrap();
    assert!(b64.to_bytes().is_empty());
}

#[test]
fn base64_url_no_pad_from_str_invalid() {
    let result = Base64UrlStrNoPad::from_str("!!!invalid!!!");
    assert!(result.is_err());
}

#[test]
fn base64_url_no_pad_from_str_with_padding_rejected() {
    // No-pad decoder should reject strings with '=' padding
    let result = Base64UrlStrNoPad::from_str("aGVsbG8=");
    assert!(result.is_err());
}

#[test]
fn base64_url_no_pad_error_message_contains_value_and_type() {
    let err = Base64UrlStrNoPad::from_str("!!!bad!!!").unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("!!!bad!!!"),
        "error message should contain the invalid value"
    );
    assert!(
        msg.contains("Base64UrlStrNoPad"),
        "error message should contain the type name"
    );
}

#[test]
fn base64_url_no_pad_display() {
    let b64: Base64UrlStrNoPad = b"data".as_slice().into();
    assert_eq!(format!("{b64}"), "ZGF0YQ");
}

#[test]
fn base64_url_no_pad_equality() {
    let a: Base64UrlStrNoPad = b"test".as_slice().into();
    let b: Base64UrlStrNoPad = b"test".as_slice().into();
    let c: Base64UrlStrNoPad = b"other".as_slice().into();
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn base64_url_no_pad_clone() {
    let a: Base64UrlStrNoPad = b"test".as_slice().into();
    let b = a.clone();
    assert_eq!(a, b);
}

#[test]
fn base64_url_no_pad_hash_equal_values_same_hash() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    let a: Base64UrlStrNoPad = b"test".as_slice().into();
    let b: Base64UrlStrNoPad = b"test".as_slice().into();
    set.insert(a.clone());
    assert!(set.contains(&b));
}

#[test]
fn base64_url_no_pad_into_inner() {
    let b64: Base64UrlStrNoPad = b"hello".as_slice().into();
    let inner: String = b64.into();
    assert_eq!(inner, "aGVsbG8");
}

#[test]
fn base64_url_no_pad_as_ref_str() {
    let b64: Base64UrlStrNoPad = b"hello".as_slice().into();
    let reference: &String = b64.as_ref();
    assert_eq!(reference, "aGVsbG8");
}

#[test]
fn base64_url_no_pad_url_safe_characters() {
    let data: Vec<u8> = vec![0xfb, 0xff, 0xfe];
    let encoded: Base64UrlStrNoPad = data.as_slice().into();
    let s = encoded.to_string();
    assert!(
        !s.contains('+') && !s.contains('/'),
        "URL-safe base64 should not contain + or /, got: {s}"
    );
}

// ---------------------------------------------------------------------------
// Serde serialization / deserialization (requires --all-features)
// ---------------------------------------------------------------------------

mod serde_tests {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct Padded {
        value: identus_apollo::base64::Base64UrlStr,
    }

    #[derive(Debug, Deserialize)]
    struct NoPad {
        value: identus_apollo::base64::Base64UrlStrNoPad,
    }

    // -- Base64UrlStr serde ----------------------------------------------------

    #[test]
    fn serde_base64_url_deserialize_valid() {
        let json = r#"{"value":"aGVsbG8gd29ybGQ="}"#;
        let parsed: Padded = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.value.to_bytes(), b"hello world".to_vec());
    }

    #[test]
    fn serde_base64_url_deserialize_empty() {
        let json = r#"{"value":""}"#;
        let parsed: Padded = serde_json::from_str(json).unwrap();
        assert!(parsed.value.to_bytes().is_empty());
    }

    #[test]
    fn serde_base64_url_deserialize_invalid_returns_error() {
        let json = r#"{"value":"!!!invalid!!!"}"#;
        let result = serde_json::from_str::<Padded>(json);
        assert!(result.is_err(), "deserializing invalid base64 should fail");
    }

    #[test]
    fn serde_base64_url_serialize_roundtrip() {
        let original = identus_apollo::base64::Base64UrlStr::from(b"roundtrip");
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: identus_apollo::base64::Base64UrlStr = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }

    // -- Base64UrlStrNoPad serde ------------------------------------------------

    #[test]
    fn serde_base64_url_no_pad_deserialize_valid() {
        let json = r#"{"value":"aGVsbG8gd29ybGQ"}"#;
        let parsed: NoPad = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.value.to_bytes(), b"hello world".to_vec());
    }

    #[test]
    fn serde_base64_url_no_pad_deserialize_empty() {
        let json = r#"{"value":""}"#;
        let parsed: NoPad = serde_json::from_str(json).unwrap();
        assert!(parsed.value.to_bytes().is_empty());
    }

    #[test]
    fn serde_base64_url_no_pad_deserialize_invalid_returns_error() {
        let json = r#"{"value":"!!!invalid!!!"}"#;
        let result = serde_json::from_str::<NoPad>(json);
        assert!(result.is_err(), "deserializing invalid base64 should fail");
    }

    #[test]
    fn serde_base64_url_no_pad_deserialize_with_padding_returns_error() {
        // No-pad deserializer should reject padded strings
        let json = r#"{"value":"aGVsbG8="}"#;
        let result = serde_json::from_str::<NoPad>(json);
        assert!(result.is_err(), "no-pad deserializer should reject padded base64");
    }

    #[test]
    fn serde_base64_url_no_pad_serialize_roundtrip() {
        let original = identus_apollo::base64::Base64UrlStrNoPad::from(b"roundtrip");
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: identus_apollo::base64::Base64UrlStrNoPad = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }
}

// ---------------------------------------------------------------------------
// Cross-type: Base64UrlStr vs Base64UrlStrNoPad
// ---------------------------------------------------------------------------

#[test]
fn padded_and_no_pad_encode_same_data_differ_only_in_padding() {
    let data = b"hello";
    let padded: Base64UrlStr = data.as_slice().into();
    let no_pad: Base64UrlStrNoPad = data.as_slice().into();
    let padded_str = padded.to_string();
    let no_pad_str = no_pad.to_string();
    assert_eq!(
        padded_str.trim_end_matches('='),
        no_pad_str,
        "padded and no-pad should be the same once padding is stripped"
    );
}
