//! Tests for `lib/did-prism/src/lib.rs` (proto extensions)
//!
//! Covers: `SignedPrismOperation::operation_hash`, `SignedPrismOperation::operation_id`

use identus_did_prism::proto;

mod test_utils;

#[test]
fn signed_prism_operation_operation_hash_returns_some_when_operation_present() {
    let (signed_op, _hash, _key) = test_utils::new_create_did_operation(None);

    let result = signed_op.operation_hash();

    assert!(result.is_some());
    let returned_hash = result.unwrap();

    // The hash should equal the PrismOperation's operation_hash
    let inner_op = signed_op.operation.as_ref().unwrap();
    assert_eq!(returned_hash, inner_op.operation_hash());
}

#[test]
fn signed_prism_operation_operation_hash_returns_none_when_operation_absent() {
    let signed_op = proto::prism::SignedPrismOperation {
        operation: protobuf::MessageField::none(),
        signed_with: String::new(),
        signature: vec![],
        special_fields: Default::default(),
    };

    let result = signed_op.operation_hash();

    assert!(result.is_none());
}

#[test]
fn signed_prism_operation_operation_id_returns_consistent_id() {
    let (signed_op, _hash, _key) = test_utils::new_create_did_operation(None);

    let id1 = signed_op.operation_id();
    let id2 = signed_op.operation_id();

    // Calling operation_id twice on the same operation returns the same id
    assert_eq!(id1, id2);
}

#[test]
fn signed_prism_operation_operation_id_differs_from_inner_operation_hash() {
    let (signed_op, _hash, _key) = test_utils::new_create_did_operation(None);

    let operation_id = signed_op.operation_id();
    let inner_op_hash = signed_op.operation.as_ref().unwrap().operation_hash();

    // operation_id hashes the entire signed operation (including signature),
    // so it must differ from the inner operation hash
    assert_ne!(operation_id.as_bytes(), inner_op_hash.as_bytes());
}

#[test]
fn signed_prism_operation_operation_id_differs_for_different_signatures() {
    let (signed_op_a, _hash, _key) = test_utils::new_create_did_operation(None);

    // Build a second signed operation with the same inner operation but different signature
    let inner_op = signed_op_a.operation.as_ref().unwrap().clone();
    let signed_op_b = proto::prism::SignedPrismOperation {
        operation: Some(inner_op).into(),
        signed_with: signed_op_a.signed_with.clone(),
        signature: vec![0u8; 64], // different (fake) signature
        special_fields: Default::default(),
    };

    let id_a = signed_op_a.operation_id();
    let id_b = signed_op_b.operation_id();

    // Different signatures → different operation IDs
    assert_ne!(id_a, id_b);
}
