//! Tests for `lib/did-prism/src/did/operation/mod.rs`
//!
//! Covers: OperationParameters, SignedPrismOperationHexStr, PrismObjectHexStr, OperationId.

use std::str::FromStr;

use identus_apollo::hash::Sha256Digest;
use identus_did_prism::did::operation::OperationId;
use identus_did_prism::proto;
use identus_did_prism::proto::MessageExt;

mod test_utils;

// ---------------------------------------------------------------------------
// OperationParameters
// ---------------------------------------------------------------------------

#[test]
fn operation_parameters_v1_has_expected_defaults() {
    use identus_did_prism::did::operation::OperationParameters;
    let params = OperationParameters::v1();
    assert_eq!(params.max_services, 50);
    assert_eq!(params.max_public_keys, 50);
    assert_eq!(params.max_id_size, 50);
    assert_eq!(params.max_type_size, 100);
    assert_eq!(params.max_service_endpoint_size, 300);
}

// ---------------------------------------------------------------------------
// SignedPrismOperationHexStr  (serialize / deserialize round-trip)
// ---------------------------------------------------------------------------

#[test]
fn signed_prism_operation_hex_str_roundtrip() {
    use identus_did_prism::did::operation::SignedPrismOperationHexStr;

    let (signed_op, _, _) = test_utils::new_create_did_operation(None);
    let wrapper = SignedPrismOperationHexStr::from(signed_op.clone());

    // Serialize to JSON string
    let json = serde_json::to_string(&wrapper).unwrap();

    // The JSON value should be a quoted hex string
    assert!(json.starts_with('"') && json.ends_with('"'));

    // Deserialize back
    let deserialized: SignedPrismOperationHexStr = serde_json::from_str(&json).unwrap();
    let inner: proto::prism::SignedPrismOperation = deserialized.into();

    // The round-tripped operation bytes should match
    assert_eq!(inner.encode_to_vec(), signed_op.encode_to_vec());
}

#[test]
fn signed_prism_operation_hex_str_deserialize_invalid_hex() {
    use identus_did_prism::did::operation::SignedPrismOperationHexStr;

    let result: Result<SignedPrismOperationHexStr, _> = serde_json::from_str("\"not-valid-hex!zzz\"");
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("value is not a valid hex"),
        "unexpected error: {err_msg}"
    );
}

#[test]
fn signed_prism_operation_hex_str_deserialize_invalid_protobuf() {
    use identus_did_prism::did::operation::SignedPrismOperationHexStr;

    // Valid hex but not a valid SignedPrismOperation protobuf
    let bad_hex = "\"aabbccdd\"";
    let result: Result<SignedPrismOperationHexStr, _> = serde_json::from_str(bad_hex);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("value cannot be decoded to signed prism operation"),
        "unexpected error: {err_msg}"
    );
}

// ---------------------------------------------------------------------------
// PrismObjectHexStr  (serialize / deserialize round-trip + signed_operations)
// ---------------------------------------------------------------------------

#[test]
fn prism_object_hex_str_roundtrip() {
    use identus_did_prism::did::operation::PrismObjectHexStr;

    let (signed_op, _, _) = test_utils::new_create_did_operation(None);

    let prism_object = proto::prism::PrismObject {
        block_content: Some(proto::prism::PrismBlock {
            operations: vec![signed_op],
            special_fields: Default::default(),
        })
        .into(),
        special_fields: Default::default(),
    };

    let wrapper = PrismObjectHexStr::from(prism_object.clone());

    // Serialize
    let json = serde_json::to_string(&wrapper).unwrap();
    assert!(json.starts_with('"') && json.ends_with('"'));

    // Deserialize
    let deserialized: PrismObjectHexStr = serde_json::from_str(&json).unwrap();
    let inner: proto::prism::PrismObject = deserialized.into();
    assert_eq!(inner.encode_to_vec(), prism_object.encode_to_vec());
}

#[test]
fn prism_object_hex_str_signed_operations_returns_operations() {
    use identus_did_prism::did::operation::PrismObjectHexStr;

    let (signed_op, _, _) = test_utils::new_create_did_operation(None);

    let prism_object = proto::prism::PrismObject {
        block_content: Some(proto::prism::PrismBlock {
            operations: vec![signed_op.clone()],
            special_fields: Default::default(),
        })
        .into(),
        special_fields: Default::default(),
    };

    let wrapper = PrismObjectHexStr::from(prism_object);
    let ops = wrapper.signed_operations();
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].encode_to_vec(), signed_op.encode_to_vec());
}

#[test]
fn prism_object_hex_str_signed_operations_empty_when_no_block_content() {
    use identus_did_prism::did::operation::PrismObjectHexStr;

    let prism_object = proto::prism::PrismObject {
        block_content: None.into(),
        special_fields: Default::default(),
    };

    let wrapper = PrismObjectHexStr::from(prism_object);
    let ops = wrapper.signed_operations();
    assert!(ops.is_empty());
}

#[test]
fn prism_object_hex_str_deserialize_invalid_hex() {
    use identus_did_prism::did::operation::PrismObjectHexStr;

    let result: Result<PrismObjectHexStr, _> = serde_json::from_str("\"zzzzz!!\"");
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("value is not a valid hex"),
        "unexpected error: {err_msg}"
    );
}

#[test]
fn prism_object_hex_str_deserialize_invalid_protobuf() {
    use identus_did_prism::did::operation::PrismObjectHexStr;

    let result: Result<PrismObjectHexStr, _> = serde_json::from_str("\"aabbccdd\"");
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("value cannot be decoded to prism object"),
        "unexpected error: {err_msg}"
    );
}

// ---------------------------------------------------------------------------
// OperationId
// ---------------------------------------------------------------------------

#[test]
fn operation_id_from_bytes_valid() {
    let hash = Sha256Digest::from_bytes(&[0u8; 32]).unwrap();
    let op_id = OperationId::from_bytes(&[0u8; 32]).unwrap();
    assert_eq!(op_id.as_bytes(), hash.as_bytes());
}

#[test]
fn operation_id_from_bytes_wrong_length() {
    let result = OperationId::from_bytes(&[0u8; 16]);
    assert!(result.is_err());
}

#[test]
fn operation_id_to_vec() {
    let op_id = OperationId::from_bytes(&[1u8; 32]).unwrap();
    let vec = op_id.to_vec();
    assert_eq!(vec.len(), 32);
    assert_eq!(vec, vec![1u8; 32]);
}

#[test]
fn operation_id_as_bytes() {
    let op_id = OperationId::from_bytes(&[2u8; 32]).unwrap();
    assert_eq!(op_id.as_bytes().len(), 32);
    assert!(op_id.as_bytes().iter().all(|&b| b == 2));
}

#[test]
fn operation_id_display_and_debug_shows_hex() {
    let op_id = OperationId::from_bytes(&[0u8; 32]).unwrap();
    let display = format!("{op_id}");
    let debug = format!("{op_id:?}");
    // Both should be 64-char hex strings
    assert_eq!(display.len(), 64);
    assert_eq!(debug.len(), 64);
    assert_eq!(display, "0".repeat(64));
    assert_eq!(debug, "0".repeat(64));
}

#[test]
fn operation_id_serialize_deserialize_roundtrip() {
    let op_id = OperationId::from_bytes(&[0xab; 32]).unwrap();
    let json = serde_json::to_string(&op_id).unwrap();
    // Should be a quoted hex string
    assert!(json.starts_with('"'));
    let deserialized: OperationId = serde_json::from_str(&json).unwrap();
    assert_eq!(op_id, deserialized);
}

#[test]
fn operation_id_deserialize_invalid_hex() {
    let result: Result<OperationId, _> = serde_json::from_str("\"not-hex!\"");
    assert!(result.is_err());
}

#[test]
fn operation_id_deserialize_invalid_digest_length() {
    // Valid hex but wrong length (not 32 bytes / 64 hex chars)
    let result: Result<OperationId, _> = serde_json::from_str("\"aabbcc\"");
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("value is not a valid digest"),
        "unexpected error: {err_msg}"
    );
}

#[test]
fn operation_id_from_str_valid() {
    let hex = "ab".repeat(32); // 64 hex chars = 32 bytes
    let op_id = OperationId::from_str(&hex).unwrap();
    assert_eq!(op_id.as_bytes().len(), 32);
}

#[test]
fn operation_id_from_str_invalid_hex() {
    let result = OperationId::from_str("not-hex!!");
    assert!(result.is_err());
    let err_msg = result.unwrap_err();
    assert!(err_msg.contains("invalid hex string"), "unexpected error: {err_msg}");
}

#[test]
fn operation_id_from_str_wrong_length() {
    let result = OperationId::from_str("aabbcc");
    assert!(result.is_err());
    let err_msg = result.unwrap_err();
    assert!(err_msg.contains("invalid operation id"), "unexpected error: {err_msg}");
}

#[test]
fn operation_id_equality_and_hash() {
    use std::collections::HashSet;

    let id1 = OperationId::from_bytes(&[1u8; 32]).unwrap();
    let id2 = OperationId::from_bytes(&[1u8; 32]).unwrap();
    let id3 = OperationId::from_bytes(&[2u8; 32]).unwrap();

    assert_eq!(id1, id2);
    assert_ne!(id1, id3);

    let mut set = HashSet::new();
    set.insert(id1.clone());
    set.insert(id2.clone());
    set.insert(id3.clone());
    assert_eq!(set.len(), 2);
}

#[test]
fn operation_id_clone_roundtrip() {
    let id = OperationId::from_bytes(&[42u8; 32]).unwrap();
    let cloned = id.clone();
    assert_eq!(id, cloned);
}

// ---------------------------------------------------------------------------
// From / Into conversions
// ---------------------------------------------------------------------------

#[test]
fn signed_prism_operation_hex_str_from_into() {
    use identus_did_prism::did::operation::SignedPrismOperationHexStr;

    let (signed_op, _, _) = test_utils::new_create_did_operation(None);
    let wrapper: SignedPrismOperationHexStr = signed_op.clone().into();
    let recovered: proto::prism::SignedPrismOperation = wrapper.into();
    assert_eq!(recovered.encode_to_vec(), signed_op.encode_to_vec());
}

#[test]
fn prism_object_hex_str_from_into() {
    use identus_did_prism::did::operation::PrismObjectHexStr;

    let obj = proto::prism::PrismObject {
        block_content: None.into(),
        special_fields: Default::default(),
    };
    let wrapper: PrismObjectHexStr = obj.clone().into();
    let recovered: proto::prism::PrismObject = wrapper.into();
    assert_eq!(recovered.encode_to_vec(), obj.encode_to_vec());
}

#[test]
fn operation_id_from_sha256_digest() {
    let digest = Sha256Digest::from_bytes(&[0xff; 32]).unwrap();
    let op_id: OperationId = digest.clone().into();
    assert_eq!(op_id.as_bytes(), digest.as_bytes());
}
