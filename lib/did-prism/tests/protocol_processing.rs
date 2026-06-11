//! Tests for protocol/mod.rs — OperationProcessingContext, init contexts, DidStateRc, finalize.
//!
//! These tests target the uncovered code paths in the protocol processing pipeline:
//! - resolve_unpublished (init_unpublished_context)
//! - error paths in init_published_context
//! - error paths in OperationProcessingContext::process
//! - DidStateRc::finalize timestamp computation
//! - DidStateRc conflict error paths for services and storage

use chrono::{DateTime, TimeZone, Utc};
use identus_apollo::crypto::secp256k1::Secp256k1PrivateKey;
use identus_did_prism::did::{CanonicalPrismDid, PrismDidOps};
use identus_did_prism::dlt::{BlockMetadata, OperationMetadata, TxId};
use identus_did_prism::prelude::MessageExt;
use identus_did_prism::proto;
use identus_did_prism::proto::prism::prism_operation::Operation;
use identus_did_prism::protocol::resolver;

mod test_utils;

// ---------------------------------------------------------------------------
// resolve_unpublished — exercises init_unpublished_context
// ---------------------------------------------------------------------------

#[test]
fn resolve_unpublished_from_create_did() {
    let master_sk = Secp256k1PrivateKey::from_slice(&[1u8; 32]).unwrap();
    let create_op = proto::prism::PrismOperation {
        operation: Some(Operation::CreateDid(proto::prism_ssi::ProtoCreateDID {
            did_data: Some(proto::prism_ssi::proto_create_did::DIDCreationData {
                public_keys: vec![test_utils::new_public_key(
                    "master-0",
                    proto::prism_ssi::KeyUsage::MASTER_KEY,
                    &master_sk,
                )],
                services: vec![],
                context: vec![],
                special_fields: Default::default(),
            })
            .into(),
            special_fields: Default::default(),
        })),
        special_fields: Default::default(),
    };

    let state = resolver::resolve_unpublished(create_op).unwrap();

    assert!(
        !state.is_published,
        "unpublished context should set is_published = false"
    );
    assert_eq!(state.public_keys.len(), 1);
    assert_eq!(state.public_keys[0].id.as_str(), "master-0");
    // Unpublished context uses UNIX_EPOCH metadata
    assert_eq!(state.created_at, DateTime::UNIX_EPOCH);
    assert_eq!(state.updated_at, DateTime::UNIX_EPOCH);
}

#[test]
fn resolve_unpublished_from_non_create_operation_fails() {
    let update_op = proto::prism::PrismOperation {
        operation: Some(Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: vec![0u8; 32],
            id: "abcdef".to_string(),
            actions: vec![],
            special_fields: Default::default(),
        })),
        special_fields: Default::default(),
    };

    let result = resolver::resolve_unpublished(update_op);
    assert!(
        result.is_err(),
        "init_unpublished_context should reject non-CreateDid operations"
    );
}

#[test]
fn resolve_unpublished_from_empty_operation_fails() {
    let empty_op = proto::prism::PrismOperation {
        operation: None,
        special_fields: Default::default(),
    };

    let result = resolver::resolve_unpublished(empty_op);
    assert!(
        result.is_err(),
        "init_unpublished_context should reject operations with no inner operation"
    );
}

// ---------------------------------------------------------------------------
// init_published_context error paths via resolve_published
// ---------------------------------------------------------------------------

#[test]
fn resolve_published_rejects_non_create_operation_init() {
    // An UpdateDid as the first operation should be rejected
    let update_op = proto::prism::SignedPrismOperation {
        signed_with: "master-0".to_string(),
        signature: vec![],
        operation: Some(proto::prism::PrismOperation {
            operation: Some(Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
                previous_operation_hash: vec![0u8; 32],
                id: "abcdef".to_string(),
                actions: vec![],
                special_fields: Default::default(),
            })),
            special_fields: Default::default(),
        })
        .into(),
        special_fields: Default::default(),
    };

    let operations = test_utils::populate_metadata(vec![update_op]);
    let (state, debug) = resolver::resolve_published(operations);

    assert!(
        state.is_none(),
        "should not produce a state from non-Create first operation"
    );
    assert_eq!(debug.len(), 1);
    assert!(debug[0].2.is_some(), "the init operation should have an error");
}

#[test]
fn resolve_published_rejects_missing_operation_in_signed_op() {
    // SignedPrismOperation with no inner PrismOperation
    let empty_signed_op = proto::prism::SignedPrismOperation {
        signed_with: "master-0".to_string(),
        signature: vec![],
        operation: None.into(),
        special_fields: Default::default(),
    };

    let operations = test_utils::populate_metadata(vec![empty_signed_op]);
    let (state, debug) = resolver::resolve_published(operations);

    assert!(state.is_none(), "should not produce a state from missing operation");
    assert_eq!(debug.len(), 1);
    assert!(debug[0].2.is_some());
}

#[test]
fn resolve_published_rejects_missing_inner_operation() {
    // SignedPrismOperation where PrismOperation has no inner operation
    let signed_op = proto::prism::SignedPrismOperation {
        signed_with: "master-0".to_string(),
        signature: vec![],
        operation: Some(proto::prism::PrismOperation {
            operation: None,
            special_fields: Default::default(),
        })
        .into(),
        special_fields: Default::default(),
    };

    let operations = test_utils::populate_metadata(vec![signed_op]);
    let (state, debug) = resolver::resolve_published(operations);

    assert!(
        state.is_none(),
        "should not produce a state from missing inner operation"
    );
    assert_eq!(debug.len(), 1);
    assert!(debug[0].2.is_some());
}

#[test]
fn resolve_published_skips_invalid_init_then_succeeds_with_valid_create() {
    // First op: invalid (no inner operation)
    let invalid_op = proto::prism::SignedPrismOperation {
        signed_with: "master-0".to_string(),
        signature: vec![],
        operation: Some(proto::prism::PrismOperation {
            operation: None,
            special_fields: Default::default(),
        })
        .into(),
        special_fields: Default::default(),
    };
    // Second op: valid CreateDid
    let (valid_create_op, _, _) = test_utils::new_create_did_operation(None);

    let operations = test_utils::populate_metadata(vec![invalid_op, valid_create_op]);
    let (state, debug) = resolver::resolve_published(operations);

    assert!(state.is_some(), "should produce a state after skipping invalid init");
    assert_eq!(debug.len(), 2);
    // First operation failed init
    assert!(debug[0].2.is_some());
    // Second operation succeeded as init
    assert!(debug[1].2.is_none());
}

#[test]
fn resolve_published_empty_operations_returns_none() {
    let (state, debug) = resolver::resolve_published(vec![]);
    assert!(state.is_none());
    assert!(debug.is_empty());
}

// ---------------------------------------------------------------------------
// OperationProcessingContext::process error paths
// ---------------------------------------------------------------------------

#[test]
fn process_create_did_on_existing_state_returns_error() {
    let (create_did_op, _, _) = test_utils::new_create_did_operation(None);
    // Second CreateDid with matching signature (so sig check passes, but operation type is rejected)
    let signing_key = Secp256k1PrivateKey::from_slice(&[1u8; 32]).unwrap();
    let second_create = proto::prism::PrismOperation {
        operation: Some(Operation::CreateDid(proto::prism_ssi::ProtoCreateDID {
            did_data: Some(proto::prism_ssi::proto_create_did::DIDCreationData {
                public_keys: vec![test_utils::new_public_key(
                    "master-0",
                    proto::prism_ssi::KeyUsage::MASTER_KEY,
                    &signing_key,
                )],
                services: vec![],
                context: vec![],
                special_fields: Default::default(),
            })
            .into(),
            special_fields: Default::default(),
        })),
        special_fields: Default::default(),
    };
    let second_signed = proto::prism::SignedPrismOperation {
        signed_with: "master-0".to_string(),
        signature: signing_key.sign(&second_create.encode_to_vec()),
        operation: Some(second_create).into(),
        special_fields: Default::default(),
    };

    let operations = test_utils::populate_metadata(vec![create_did_op, second_signed]);
    let (state, debug) = resolver::resolve_published(operations);

    // State exists (from first CreateDid)
    let state = state.unwrap();
    assert_eq!(state.public_keys.len(), 1);
    // Second CreateDid was rejected
    assert_eq!(debug.len(), 2);
    assert!(
        debug[1].2.is_some(),
        "second CreateDid should fail with DidStateUpdateFromCreateOperation"
    );
}

#[test]
fn process_operation_with_missing_signed_operation_returns_error() {
    let (create_did_op, _, _) = test_utils::new_create_did_operation(None);
    // A signed operation where the inner PrismOperation.operation is None
    let bad_op = proto::prism::SignedPrismOperation {
        signed_with: "master-0".to_string(),
        signature: vec![],
        operation: Some(proto::prism::PrismOperation {
            operation: None,
            special_fields: Default::default(),
        })
        .into(),
        special_fields: Default::default(),
    };

    let operations = test_utils::populate_metadata(vec![create_did_op, bad_op]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    assert_eq!(state.public_keys.len(), 1, "original state should be preserved");
    assert_eq!(debug.len(), 2);
    // Second operation failed
    let error = debug[1].2.as_ref().unwrap();
    assert!(
        format!("{error}").contains("missing"),
        "expected missing operation error, got: {error}"
    );
}

#[test]
fn process_operation_with_wrong_signature_returns_error() {
    let (create_did_op, _, _) = test_utils::new_create_did_operation(None);
    // Create an update signed by a different key
    let wrong_sk = Secp256k1PrivateKey::from_slice(&[9u8; 32]).unwrap();
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();
    let (bad_update_op, _) = test_utils::new_signed_operation(
        "master-0",
        &wrong_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: did.suffix.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![test_utils::add_service_action("svc-0")],
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, bad_update_op]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    assert!(
        state.services.is_empty(),
        "update should be rejected due to bad signature"
    );
    assert_eq!(debug.len(), 2);
    let error = debug[1].2.as_ref().unwrap();
    assert!(
        format!("{error}").contains("signature verification failed"),
        "expected signature verification error, got: {error}"
    );
}

#[test]
fn process_operation_with_unknown_key_id_returns_error() {
    let (create_did_op, _, _) = test_utils::new_create_did_operation(None);
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();
    let unknown_sk = Secp256k1PrivateKey::from_slice(&[9u8; 32]).unwrap();
    let (bad_update_op, _) = test_utils::new_signed_operation(
        "unknown-key",
        &unknown_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: did.suffix.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![test_utils::add_service_action("svc-0")],
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, bad_update_op]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    assert!(
        state.services.is_empty(),
        "update should be rejected due to unknown key"
    );
    let error = debug[1].2.as_ref().unwrap();
    assert!(
        format!("{error}").contains("not found"),
        "expected key not found error, got: {error}"
    );
}

#[test]
fn update_removing_last_master_key_fails_validation() {
    let (create_did_op, create_did_op_hash, _) = test_utils::new_create_did_operation(None);
    let master_sk = Secp256k1PrivateKey::from_slice(&[1u8; 32]).unwrap();
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    // Remove the only master key via UpdateDID
    let (revoke_op, _revoke_op_hash) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![proto::prism_ssi::UpdateDIDAction {
                action: Some(proto::prism_ssi::update_didaction::Action::RemoveKey(
                    proto::prism_ssi::RemoveKeyAction {
                        keyId: "master-0".to_string(),
                        special_fields: Default::default(),
                    },
                )),
                special_fields: Default::default(),
            }],
            special_fields: Default::default(),
        }),
    );

    // The update fails candidate-state validation: no master key would remain
    let operations = test_utils::populate_metadata(vec![create_did_op, revoke_op]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    assert_eq!(
        state.public_keys.len(),
        1,
        "state should remain unchanged after failed update"
    );
    let error = debug[1].2.as_ref().unwrap();
    assert!(
        format!("{error:?}").contains("AfterUpdateMissingMasterKey"),
        "expected missing master key error, got: {error:?}"
    );
}

// ---------------------------------------------------------------------------
// DidStateRc::finalize timestamp computation
// ---------------------------------------------------------------------------

#[test]
fn resolve_published_timestamps_from_block_metadata() {
    let (create_did_op, create_did_op_hash, master_sk) = test_utils::new_create_did_operation(None);
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    let create_time = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
    let update_time = Utc.with_ymd_and_hms(2025, 3, 20, 14, 30, 0).unwrap();

    let (update_op, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![test_utils::add_service_action("svc-0")],
            special_fields: Default::default(),
        }),
    );

    let create_metadata = OperationMetadata {
        block_metadata: BlockMetadata {
            slot_number: 0.into(),
            block_number: 0.into(),
            cbt: create_time,
            absn: 0,
            tx_id: TxId::from(identus_apollo::hash::sha256([0u8; 32])),
        },
        osn: 0,
    };
    let update_metadata = OperationMetadata {
        block_metadata: BlockMetadata {
            slot_number: 1.into(),
            block_number: 1.into(),
            cbt: update_time,
            absn: 0,
            tx_id: TxId::from(identus_apollo::hash::sha256([1u8; 32])),
        },
        osn: 1,
    };

    let operations = vec![(create_metadata, create_did_op), (update_metadata, update_op)];
    let (state, _debug) = resolver::resolve_published(operations);
    let state = state.unwrap();

    // created_at should be the earliest time (create_time)
    assert_eq!(state.created_at, create_time);
    // updated_at should be the latest time (update_time)
    assert_eq!(state.updated_at, update_time);
    // Service should have been added
    assert_eq!(state.services.len(), 1);
}

#[test]
fn resolve_published_timestamps_single_operation() {
    let (create_did_op, _, _) = test_utils::new_create_did_operation(None);
    let create_time = Utc.with_ymd_and_hms(2025, 6, 1, 12, 0, 0).unwrap();
    let metadata = OperationMetadata {
        block_metadata: BlockMetadata {
            slot_number: 0.into(),
            block_number: 0.into(),
            cbt: create_time,
            absn: 0,
            tx_id: TxId::from(identus_apollo::hash::sha256([0u8; 32])),
        },
        osn: 0,
    };

    let (state, _debug) = resolver::resolve_published(vec![(metadata, create_did_op)]);
    let state = state.unwrap();

    // With a single operation, created_at == updated_at
    assert_eq!(state.created_at, create_time);
    assert_eq!(state.updated_at, create_time);
}

// ---------------------------------------------------------------------------
// DidStateRc service conflict errors
// ---------------------------------------------------------------------------

#[test]
fn resolve_published_add_duplicate_service_returns_error() {
    let (create_did_op, create_did_op_hash, master_sk) = test_utils::new_create_did_operation(None);
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    // First update: add service "svc-0"
    let (add_svc_1, add_svc_1_hash) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![test_utils::add_service_action("svc-0")],
            special_fields: Default::default(),
        }),
    );

    // Second update: add the SAME service "svc-0" again — should fail
    let (add_svc_2, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: add_svc_1_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![test_utils::add_service_action("svc-0")],
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, add_svc_1, add_svc_2]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    // Only the first service should be present
    assert_eq!(state.services.len(), 1);
    let error = debug[2].2.as_ref().unwrap();
    assert!(
        format!("{error:?}").contains("AddServiceWithExistingId"),
        "expected duplicate service error, got: {error:?}"
    );
}

#[test]
fn resolve_published_remove_nonexistent_service_returns_error() {
    let (create_did_op, create_did_op_hash, master_sk) = test_utils::new_create_did_operation(None);
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    let (remove_svc_op, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![proto::prism_ssi::UpdateDIDAction {
                action: Some(proto::prism_ssi::update_didaction::Action::RemoveService(
                    proto::prism_ssi::RemoveServiceAction {
                        serviceId: "nonexistent-svc".to_string(),
                        special_fields: Default::default(),
                    },
                )),
                special_fields: Default::default(),
            }],
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, remove_svc_op]);
    let (state, debug) = resolver::resolve_published(operations);

    let _state = state.unwrap();
    let error = debug[1].2.as_ref().unwrap();
    assert!(
        format!("{error:?}").contains("RevokeServiceNotExists"),
        "expected service not exists error, got: {error:?}"
    );
}

#[test]
fn resolve_published_remove_already_revoked_service_returns_error() {
    let (create_did_op, create_did_op_hash, master_sk) = test_utils::new_create_did_operation(None);
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    // Add and then remove a service
    let (add_svc, add_svc_hash) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![test_utils::add_service_action("svc-0")],
            special_fields: Default::default(),
        }),
    );
    let (remove_svc, remove_svc_hash) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: add_svc_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![proto::prism_ssi::UpdateDIDAction {
                action: Some(proto::prism_ssi::update_didaction::Action::RemoveService(
                    proto::prism_ssi::RemoveServiceAction {
                        serviceId: "svc-0".to_string(),
                        special_fields: Default::default(),
                    },
                )),
                special_fields: Default::default(),
            }],
            special_fields: Default::default(),
        }),
    );
    // Try removing again — should fail with already revoked
    let (remove_svc_again, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: remove_svc_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![proto::prism_ssi::UpdateDIDAction {
                action: Some(proto::prism_ssi::update_didaction::Action::RemoveService(
                    proto::prism_ssi::RemoveServiceAction {
                        serviceId: "svc-0".to_string(),
                        special_fields: Default::default(),
                    },
                )),
                special_fields: Default::default(),
            }],
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, add_svc, remove_svc, remove_svc_again]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    assert!(state.services.is_empty(), "service should be removed");
    let error = debug[3].2.as_ref().unwrap();
    assert!(
        format!("{error:?}").contains("RevokeServiceIsAlreadyRevoked"),
        "expected service already revoked error, got: {error:?}"
    );
}

#[test]
fn resolve_published_update_nonexistent_service_returns_error() {
    let (create_did_op, create_did_op_hash, master_sk) = test_utils::new_create_did_operation(None);
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    let (update_svc, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![proto::prism_ssi::UpdateDIDAction {
                action: Some(proto::prism_ssi::update_didaction::Action::UpdateService(
                    proto::prism_ssi::UpdateServiceAction {
                        serviceId: "nonexistent-svc".to_string(),
                        type_: "NewType".to_string(),
                        service_endpoints: String::new(),
                        special_fields: Default::default(),
                    },
                )),
                special_fields: Default::default(),
            }],
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, update_svc]);
    let (state, debug) = resolver::resolve_published(operations);

    let _state = state.unwrap();
    let error = debug[1].2.as_ref().unwrap();
    assert!(
        format!("{error:?}").contains("UpdateServiceNotExists"),
        "expected service not exists error, got: {error:?}"
    );
}

// ---------------------------------------------------------------------------
// DidStateRc public key conflict errors
// ---------------------------------------------------------------------------

#[test]
fn resolve_published_add_duplicate_public_key_returns_error() {
    let (create_did_op, create_did_op_hash, master_sk) = test_utils::new_create_did_operation(None);
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    // The CreateDID already added "master-0", so adding it again should fail
    let auth_sk = Secp256k1PrivateKey::from_slice(&[4u8; 32]).unwrap();
    let (add_dup_key, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![proto::prism_ssi::UpdateDIDAction {
                action: Some(proto::prism_ssi::update_didaction::Action::AddKey(
                    proto::prism_ssi::AddKeyAction {
                        key: ::protobuf::MessageField::some(test_utils::new_public_key(
                            "master-0",
                            proto::prism_ssi::KeyUsage::AUTHENTICATION_KEY,
                            &auth_sk,
                        )),
                        special_fields: Default::default(),
                    },
                )),
                special_fields: Default::default(),
            }],
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, add_dup_key]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    assert_eq!(state.public_keys.len(), 1, "duplicate key should not be added");
    let error = debug[1].2.as_ref().unwrap();
    assert!(
        format!("{error:?}").contains("AddPublicKeyWithExistingId"),
        "expected duplicate key error, got: {error:?}"
    );
}

#[test]
fn resolve_published_revoke_nonexistent_public_key_returns_error() {
    let (create_did_op, create_did_op_hash, master_sk) = test_utils::new_create_did_operation(None);
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    let (revoke_key, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![proto::prism_ssi::UpdateDIDAction {
                action: Some(proto::prism_ssi::update_didaction::Action::RemoveKey(
                    proto::prism_ssi::RemoveKeyAction {
                        keyId: "nonexistent-key".to_string(),
                        special_fields: Default::default(),
                    },
                )),
                special_fields: Default::default(),
            }],
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, revoke_key]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    assert_eq!(state.public_keys.len(), 1, "state should be unchanged");
    let error = debug[1].2.as_ref().unwrap();
    assert!(
        format!("{error:?}").contains("RevokePublicKeyNotExists"),
        "expected key not exists error, got: {error:?}"
    );
}

#[test]
fn resolve_published_revoke_already_revoked_key_returns_error() {
    let master_sk = Secp256k1PrivateKey::from_slice(&[1u8; 32]).unwrap();
    let auth_sk = Secp256k1PrivateKey::from_slice(&[3u8; 32]).unwrap();

    // Create DID with master key + auth key
    let (create_did_op, create_did_op_hash, _) =
        test_utils::new_create_did_operation(Some(test_utils::CreateDidOptions {
            public_keys: Some(vec![test_utils::new_public_key(
                "auth-0",
                proto::prism_ssi::KeyUsage::AUTHENTICATION_KEY,
                &auth_sk,
            )]),
            ..Default::default()
        }));
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    // Revoke auth-0
    let (revoke_op, revoke_op_hash) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![proto::prism_ssi::UpdateDIDAction {
                action: Some(proto::prism_ssi::update_didaction::Action::RemoveKey(
                    proto::prism_ssi::RemoveKeyAction {
                        keyId: "auth-0".to_string(),
                        special_fields: Default::default(),
                    },
                )),
                special_fields: Default::default(),
            }],
            special_fields: Default::default(),
        }),
    );

    // Try to revoke auth-0 again
    let (revoke_again, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: revoke_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![proto::prism_ssi::UpdateDIDAction {
                action: Some(proto::prism_ssi::update_didaction::Action::RemoveKey(
                    proto::prism_ssi::RemoveKeyAction {
                        keyId: "auth-0".to_string(),
                        special_fields: Default::default(),
                    },
                )),
                special_fields: Default::default(),
            }],
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, revoke_op, revoke_again]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    // Only master key remains
    assert_eq!(state.public_keys.len(), 1);
    assert_eq!(state.public_keys[0].id.as_str(), "master-0");
    let error = debug[2].2.as_ref().unwrap();
    assert!(
        format!("{error:?}").contains("RevokePublicKeyIsAlreadyRevoked"),
        "expected key already revoked error, got: {error:?}"
    );
}

// ---------------------------------------------------------------------------
// DidStateRc storage conflict errors
// ---------------------------------------------------------------------------

#[test]
fn resolve_published_update_nonexistent_storage_entry_returns_error() {
    let vdr_sk = Secp256k1PrivateKey::from_slice(&[2u8; 32]).unwrap();
    let (create_did_op, _, _) = test_utils::new_create_did_operation(Some(test_utils::CreateDidOptions {
        public_keys: Some(vec![test_utils::new_public_key(
            "vdr-0",
            proto::prism_ssi::KeyUsage::VDR_KEY,
            &vdr_sk,
        )]),
        ..Default::default()
    }));
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    // Test: update_storage with nonexistent prev hash
    let (create_storage_op, _create_storage_hash) = test_utils::new_signed_operation(
        "vdr-0",
        &vdr_sk,
        Operation::CreateStorageEntry(proto::prism_storage::ProtoCreateStorageEntry {
            did_prism_hash: did.suffix.to_vec(),
            nonce: vec![0],
            data: Some(proto::prism_storage::proto_create_storage_entry::Data::Bytes(vec![
                1, 2, 3,
            ])),
            special_fields: Default::default(),
        }),
    );
    let (update_bad, _) = test_utils::new_signed_operation(
        "vdr-0",
        &vdr_sk,
        Operation::UpdateStorageEntry(proto::prism_storage::ProtoUpdateStorageEntry {
            previous_event_hash: vec![0u8; 32], // nonexistent
            data: Some(proto::prism_storage::proto_update_storage_entry::Data::Bytes(vec![
                4, 5, 6,
            ])),
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, create_storage_op, update_bad]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    // Storage should be unchanged (1 entry, original data)
    assert_eq!(state.storage.len(), 1);
    assert_eq!(
        *state.storage[0].data,
        identus_did_prism::did::operation::StorageData::Bytes(vec![1, 2, 3])
    );
    let error = debug[2].2.as_ref().unwrap();
    assert!(
        format!("{error:?}").contains("UpdateStorageEntryNotExists"),
        "expected storage entry not exists error, got: {error:?}"
    );
}

#[test]
fn resolve_published_update_revoked_storage_entry_after_did_deactivation() {
    let vdr_sk = Secp256k1PrivateKey::from_slice(&[2u8; 32]).unwrap();
    let master_sk = Secp256k1PrivateKey::from_slice(&[1u8; 32]).unwrap();
    let (create_did_op, create_did_op_hash, _) =
        test_utils::new_create_did_operation(Some(test_utils::CreateDidOptions {
            public_keys: Some(vec![test_utils::new_public_key(
                "vdr-0",
                proto::prism_ssi::KeyUsage::VDR_KEY,
                &vdr_sk,
            )]),
            ..Default::default()
        }));
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    let (create_storage_op, create_storage_hash) = test_utils::new_signed_operation(
        "vdr-0",
        &vdr_sk,
        Operation::CreateStorageEntry(proto::prism_storage::ProtoCreateStorageEntry {
            did_prism_hash: did.suffix.to_vec(),
            nonce: vec![0],
            data: Some(proto::prism_storage::proto_create_storage_entry::Data::Bytes(vec![
                1, 2, 3,
            ])),
            special_fields: Default::default(),
        }),
    );
    // Deactivate DID — this revokes all storage entries
    let (deactivate_did, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::DeactivateDid(proto::prism_ssi::ProtoDeactivateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            special_fields: Default::default(),
        }),
    );
    // Now try to update the storage — the signing key is revoked, so it fails at check_signature
    let (update_storage, _) = test_utils::new_signed_operation(
        "vdr-0",
        &vdr_sk,
        Operation::UpdateStorageEntry(proto::prism_storage::ProtoUpdateStorageEntry {
            previous_event_hash: create_storage_hash.to_vec(),
            data: Some(proto::prism_storage::proto_update_storage_entry::Data::Bytes(vec![
                4, 5, 6,
            ])),
            special_fields: Default::default(),
        }),
    );

    let operations =
        test_utils::populate_metadata(vec![create_did_op, create_storage_op, deactivate_did, update_storage]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    assert!(state.is_deactivated());
    assert!(state.storage.is_empty());
    let error = debug[3].2.as_ref().unwrap();
    assert!(
        format!("{error:?}").contains("SignedPrismOperationSignedWithRevokedKey"),
        "expected signed-with-revoked-key error, got: {error:?}"
    );
}

// ---------------------------------------------------------------------------
// DidStateRc unmatched previous operation hash
// ---------------------------------------------------------------------------

#[test]
fn resolve_published_update_did_with_wrong_prev_hash_returns_error() {
    let (create_did_op, _, master_sk) = test_utils::new_create_did_operation(None);
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    let (update_op, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: vec![0u8; 32], // wrong hash
            id: did.suffix_hex().to_string(),
            actions: vec![test_utils::add_service_action("svc-0")],
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, update_op]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    assert!(
        state.services.is_empty(),
        "update should be rejected due to wrong prev hash"
    );
    let error = debug[1].2.as_ref().unwrap();
    assert!(
        format!("{error:?}").contains("UnmatchedPreviousOperationHash"),
        "expected unmatched previous operation hash error, got: {error:?}"
    );
}

// ---------------------------------------------------------------------------
// PatchContext via UpdateDid
// ---------------------------------------------------------------------------

#[test]
fn resolve_published_update_with_context_patch() {
    let (create_did_op, create_did_op_hash, master_sk) = test_utils::new_create_did_operation(None);
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    let context = vec!["https://example.com/context/v1".to_string()];
    let (update_op, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![proto::prism_ssi::UpdateDIDAction {
                action: Some(proto::prism_ssi::update_didaction::Action::PatchContext(
                    proto::prism_ssi::PatchContextAction {
                        context: context.clone(),
                        special_fields: Default::default(),
                    },
                )),
                special_fields: Default::default(),
            }],
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, update_op]);
    let (state, _) = resolver::resolve_published(operations);

    let state = state.unwrap();
    assert_eq!(state.context, context);
}

// ---------------------------------------------------------------------------
// DidState.is_published flag
// ---------------------------------------------------------------------------

#[test]
fn resolve_published_sets_is_published_true() {
    let (create_did_op, _, _) = test_utils::new_create_did_operation(None);
    let operations = test_utils::populate_metadata(vec![create_did_op]);
    let state = resolver::resolve_published(operations).0.unwrap();

    assert!(state.is_published, "published context should set is_published = true");
}

// ===========================================================================
// v1.rs coverage: VDR key invalid signature for storage operations
// ===========================================================================

#[test]
fn v1_check_signature_vdr_key_invalid_signature_for_storage_op() {
    let vdr_sk = Secp256k1PrivateKey::from_slice(&[2u8; 32]).unwrap();
    let (create_did_op, _create_did_op_hash, _) =
        test_utils::new_create_did_operation(Some(test_utils::CreateDidOptions {
            public_keys: Some(vec![test_utils::new_public_key(
                "vdr-0",
                proto::prism_ssi::KeyUsage::VDR_KEY,
                &vdr_sk,
            )]),
            ..Default::default()
        }));
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    // Create a valid storage entry first
    let (create_storage_op, create_storage_hash) = test_utils::new_signed_operation(
        "vdr-0",
        &vdr_sk,
        Operation::CreateStorageEntry(proto::prism_storage::ProtoCreateStorageEntry {
            did_prism_hash: did.suffix.to_vec(),
            nonce: vec![0],
            data: Some(proto::prism_storage::proto_create_storage_entry::Data::Bytes(vec![
                1, 2, 3,
            ])),
            special_fields: Default::default(),
        }),
    );

    // Try to update storage with a WRONG key signing the storage operation
    let wrong_sk = Secp256k1PrivateKey::from_slice(&[9u8; 32]).unwrap();
    let (update_storage_bad_sig, _) = test_utils::new_signed_operation(
        "vdr-0", // signed_with vdr-0, but signed with wrong_sk
        &wrong_sk,
        Operation::UpdateStorageEntry(proto::prism_storage::ProtoUpdateStorageEntry {
            previous_event_hash: create_storage_hash.to_vec(),
            data: Some(proto::prism_storage::proto_update_storage_entry::Data::Bytes(vec![
                4, 5, 6,
            ])),
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, create_storage_op, update_storage_bad_sig]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    // Storage should be unchanged (1 entry, original data)
    assert_eq!(state.storage.len(), 1);
    assert_eq!(
        *state.storage[0].data,
        identus_did_prism::did::operation::StorageData::Bytes(vec![1, 2, 3])
    );
    let error = debug[2].2.as_ref().unwrap();
    assert!(
        format!("{error}").contains("signature verification failed"),
        "expected invalid signature error for VDR key on storage op, got: {error}"
    );
}

// ===========================================================================
// v1.rs coverage: CreateDID with embedded services
// ===========================================================================

#[test]
fn v1_create_did_with_embedded_services() {
    let (create_did_op, _, _) = test_utils::new_create_did_operation(Some(test_utils::CreateDidOptions {
        services: Some(vec![proto::prism_ssi::Service {
            id: "svc-0".to_string(),
            type_: "LinkedDomains".to_string(),
            service_endpoint: "https://example.com".to_string(),
            special_fields: Default::default(),
        }]),
        ..Default::default()
    }));

    let operations = test_utils::populate_metadata(vec![create_did_op]);
    let (state, _debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    assert_eq!(state.public_keys.len(), 1, "should have master key");
    assert_eq!(state.services.len(), 1, "should have the embedded service");
    assert_eq!(state.services[0].id.as_str(), "svc-0");
    let identus_did_prism::did::operation::ServiceEndpoint::One(
        identus_did_prism::did::operation::ServiceEndpointValue::Uri(ref uri),
    ) = state.services[0].service_endpoint
    else {
        panic!("expected URI endpoint");
    };
    assert_eq!(uri, "https://example.com");
}

// ===========================================================================
// v1.rs coverage: Deactivate DID with wrong prev_operation_hash
// ===========================================================================

#[test]
fn v1_deactivate_did_with_wrong_prev_hash() {
    let (create_did_op, _create_did_op_hash, master_sk) = test_utils::new_create_did_operation(None);
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    let (deactivate_bad_hash, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::DeactivateDid(proto::prism_ssi::ProtoDeactivateDID {
            previous_operation_hash: vec![0u8; 32], // wrong hash
            id: did.suffix_hex().to_string(),
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, deactivate_bad_hash]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    // DID should NOT be deactivated (still has master key)
    assert!(!state.is_deactivated(), "deactivate should fail with wrong prev hash");
    assert_eq!(state.public_keys.len(), 1);
    let error = debug[1].2.as_ref().unwrap();
    assert!(
        format!("{error:?}").contains("UnmatchedPreviousOperationHash"),
        "expected unmatched previous operation hash error, got: {error:?}"
    );
}

// ===========================================================================
// v1.rs coverage: Deactivate DID with services and storage present
// ===========================================================================

#[test]
fn v1_deactivate_did_with_services_and_storage() {
    let vdr_sk = Secp256k1PrivateKey::from_slice(&[2u8; 32]).unwrap();
    let master_sk = Secp256k1PrivateKey::from_slice(&[1u8; 32]).unwrap();
    let (create_did_op, create_did_op_hash, _) =
        test_utils::new_create_did_operation(Some(test_utils::CreateDidOptions {
            public_keys: Some(vec![test_utils::new_public_key(
                "vdr-0",
                proto::prism_ssi::KeyUsage::VDR_KEY,
                &vdr_sk,
            )]),
            services: Some(vec![proto::prism_ssi::Service {
                id: "svc-0".to_string(),
                type_: "LinkedDomains".to_string(),
                service_endpoint: "https://example.com".to_string(),
                special_fields: Default::default(),
            }]),
            ..Default::default()
        }));
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    // Create a storage entry
    let (create_storage_op, _create_storage_hash) = test_utils::new_signed_operation(
        "vdr-0",
        &vdr_sk,
        Operation::CreateStorageEntry(proto::prism_storage::ProtoCreateStorageEntry {
            did_prism_hash: did.suffix.to_vec(),
            nonce: vec![0],
            data: Some(proto::prism_storage::proto_create_storage_entry::Data::Bytes(vec![
                1, 2, 3,
            ])),
            special_fields: Default::default(),
        }),
    );

    // Deactivate the DID — should revoke all keys, services, and storage
    let (deactivate_op, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::DeactivateDid(proto::prism_ssi::ProtoDeactivateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, create_storage_op, deactivate_op]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    assert!(state.is_deactivated(), "DID should be deactivated");
    assert!(state.public_keys.is_empty(), "all public keys should be removed");
    assert!(state.services.is_empty(), "all services should be removed");
    assert!(state.storage.is_empty(), "all storage entries should be removed");
    // No errors expected
    assert!(debug[0].2.is_none());
    assert!(debug[1].2.is_none());
    assert!(debug[2].2.is_none());
}

// ===========================================================================
// v1.rs coverage: Protocol version update
// ===========================================================================

#[test]
fn v1_protocol_version_update() {
    let (create_did_op, _create_did_op_hash, master_sk) = test_utils::new_create_did_operation(None);

    let (proto_version_update_op, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::ProtocolVersionUpdate(proto::prism_version::ProtoProtocolVersionUpdate {
            proposer_did: "test-did".to_string(),
            version: Some(proto::prism_version::ProtocolVersionInfo {
                version_name: "2.0.0".to_string(),
                effective_since: 1000,
                protocol_version: Some(proto::prism_version::ProtocolVersion {
                    major_version: 2,
                    minor_version: 0,
                    special_fields: Default::default(),
                })
                .into(),
                special_fields: Default::default(),
            })
            .into(),
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, proto_version_update_op]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    // State should still exist (protocol version update doesn't change state)
    assert_eq!(state.public_keys.len(), 1);
    // No error from the protocol version update
    assert!(
        debug[1].2.is_none(),
        "protocol version update should succeed without error"
    );
}

// ===========================================================================
// v1.rs coverage: UpdateService endpoint only
// ===========================================================================

#[test]
fn v1_update_service_endpoint_only() {
    let (create_did_op, create_did_op_hash, master_sk) = test_utils::new_create_did_operation(None);
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    // First: add a service
    let (add_svc, add_svc_hash) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![test_utils::add_service_action("svc-0")],
            special_fields: Default::default(),
        }),
    );

    // Second: update only the endpoint (type is empty string → None)
    let (update_endpoint, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: add_svc_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![proto::prism_ssi::UpdateDIDAction {
                action: Some(proto::prism_ssi::update_didaction::Action::UpdateService(
                    proto::prism_ssi::UpdateServiceAction {
                        serviceId: "svc-0".to_string(),
                        type_: String::new(), // empty → no type change
                        service_endpoints: "https://updated.example.com".to_string(),
                        special_fields: Default::default(),
                    },
                )),
                special_fields: Default::default(),
            }],
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, add_svc, update_endpoint]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    assert_eq!(state.services.len(), 1);
    let identus_did_prism::did::operation::ServiceEndpoint::One(
        identus_did_prism::did::operation::ServiceEndpointValue::Uri(ref uri),
    ) = state.services[0].service_endpoint
    else {
        panic!("expected URI endpoint");
    };
    assert_eq!(uri, "https://updated.example.com");
    // Type should remain unchanged
    assert!(
        matches!(&state.services[0].r#type, identus_did_prism::did::operation::ServiceType::One(v) if v.to_string() == "LinkedDomains")
    );
    assert!(debug.iter().all(|(_, _, e)| e.is_none()), "no errors expected");
}

// ===========================================================================
// v1.rs coverage: UpdateService type and endpoint simultaneously
// ===========================================================================

#[test]
fn v1_update_service_type_and_endpoint() {
    let (create_did_op, create_did_op_hash, master_sk) = test_utils::new_create_did_operation(None);
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    // First: add a service
    let (add_svc, add_svc_hash) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![test_utils::add_service_action("svc-0")],
            special_fields: Default::default(),
        }),
    );

    // Second: update both type and endpoint
    let (update_both, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: add_svc_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![proto::prism_ssi::UpdateDIDAction {
                action: Some(proto::prism_ssi::update_didaction::Action::UpdateService(
                    proto::prism_ssi::UpdateServiceAction {
                        serviceId: "svc-0".to_string(),
                        type_: "DIDCommunication".to_string(),
                        service_endpoints: "https://didcomm.example.com".to_string(),
                        special_fields: Default::default(),
                    },
                )),
                special_fields: Default::default(),
            }],
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, add_svc, update_both]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    assert_eq!(state.services.len(), 1);
    assert!(
        matches!(&state.services[0].r#type, identus_did_prism::did::operation::ServiceType::One(v) if v.to_string() == "DIDCommunication")
    );
    let identus_did_prism::did::operation::ServiceEndpoint::One(
        identus_did_prism::did::operation::ServiceEndpointValue::Uri(ref uri),
    ) = state.services[0].service_endpoint
    else {
        panic!("expected URI endpoint");
    };
    assert_eq!(uri, "https://didcomm.example.com");
    assert!(debug.iter().all(|(_, _, e)| e.is_none()), "no errors expected");
}

// ===========================================================================
// v1.rs coverage: Public key exceed limit (50)
// ===========================================================================

#[test]
fn v1_public_key_exceed_limit() {
    let master_sk = Secp256k1PrivateKey::from_slice(&[1u8; 32]).unwrap();

    // Create DID with master key only (1 key)
    let (create_did_op, create_did_op_hash, _) = test_utils::new_create_did_operation(None);
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    // Generate 50 additional keys (1 master + 50 = 51, exceeds limit of 50)
    let mut add_key_actions: Vec<proto::prism_ssi::UpdateDIDAction> = Vec::with_capacity(50);
    for i in 0..50u8 {
        let key_sk = Secp256k1PrivateKey::from_slice(&[
            i, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29,
            30, 31, 32,
        ])
        .unwrap();
        add_key_actions.push(proto::prism_ssi::UpdateDIDAction {
            action: Some(proto::prism_ssi::update_didaction::Action::AddKey(
                proto::prism_ssi::AddKeyAction {
                    key: ::protobuf::MessageField::some(test_utils::new_public_key(
                        &format!("auth-{}", i),
                        proto::prism_ssi::KeyUsage::AUTHENTICATION_KEY,
                        &key_sk,
                    )),
                    special_fields: Default::default(),
                },
            )),
            special_fields: Default::default(),
        });
    }

    let (update_op, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: add_key_actions,
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, update_op]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    // The update should fail because we'd have 51 public keys (exceeds limit of 50)
    assert_eq!(
        state.public_keys.len(),
        1,
        "state should remain unchanged after failed update"
    );
    let error = debug[1].2.as_ref().unwrap();
    assert!(
        format!("{error:?}").contains("AfterUpdatePublicKeyExceedLimit"),
        "expected public key exceed limit error, got: {error:?}"
    );
}

// ===========================================================================
// v1.rs coverage: Service exceed limit (50)
// ===========================================================================

#[test]
fn v1_service_exceed_limit() {
    let master_sk = Secp256k1PrivateKey::from_slice(&[1u8; 32]).unwrap();

    // Create DID with master key + 1 service
    let (create_did_op, create_did_op_hash, _) =
        test_utils::new_create_did_operation(Some(test_utils::CreateDidOptions {
            services: Some(vec![proto::prism_ssi::Service {
                id: "svc-init".to_string(),
                type_: "LinkedDomains".to_string(),
                service_endpoint: "https://init.example.com".to_string(),
                special_fields: Default::default(),
            }]),
            ..Default::default()
        }));
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    // Generate 50 additional service actions (1 initial + 50 = 51, exceeds limit of 50)
    let mut add_svc_actions: Vec<proto::prism_ssi::UpdateDIDAction> = Vec::with_capacity(50);
    for i in 0..50u8 {
        add_svc_actions.push(proto::prism_ssi::UpdateDIDAction {
            action: Some(proto::prism_ssi::update_didaction::Action::AddService(
                proto::prism_ssi::AddServiceAction {
                    service: Some(proto::prism_ssi::Service {
                        id: format!("svc-{}", i),
                        type_: "LinkedDomains".to_string(),
                        service_endpoint: format!("https://{}.example.com", i),
                        special_fields: Default::default(),
                    })
                    .into(),
                    special_fields: Default::default(),
                },
            )),
            special_fields: Default::default(),
        });
    }

    let (update_op, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: add_svc_actions,
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, update_op]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    // The update should fail because we'd have 51 services (exceeds limit of 50)
    assert_eq!(state.services.len(), 1, "state should remain with initial service only");
    let error = debug[1].2.as_ref().unwrap();
    assert!(
        format!("{error:?}").contains("AfterUpdateServiceExceedLimit"),
        "expected service exceed limit error, got: {error:?}"
    );
}

// ===========================================================================
// protocol/mod.rs coverage: update_service_type on revoked service
// ===========================================================================

#[test]
fn resolve_published_update_service_type_on_revoked_service_returns_error() {
    let (create_did_op, create_did_op_hash, master_sk) = test_utils::new_create_did_operation(None);
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    // Add a service
    let (add_svc, add_svc_hash) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![test_utils::add_service_action("svc-0")],
            special_fields: Default::default(),
        }),
    );

    // Remove the service (revokes it in internal state)
    let (remove_svc, remove_svc_hash) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: add_svc_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![proto::prism_ssi::UpdateDIDAction {
                action: Some(proto::prism_ssi::update_didaction::Action::RemoveService(
                    proto::prism_ssi::RemoveServiceAction {
                        serviceId: "svc-0".to_string(),
                        special_fields: Default::default(),
                    },
                )),
                special_fields: Default::default(),
            }],
            special_fields: Default::default(),
        }),
    );

    // Try to update the type of the revoked service
    let (update_type_op, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: remove_svc_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![proto::prism_ssi::UpdateDIDAction {
                action: Some(proto::prism_ssi::update_didaction::Action::UpdateService(
                    proto::prism_ssi::UpdateServiceAction {
                        serviceId: "svc-0".to_string(),
                        type_: "NewType".to_string(),
                        service_endpoints: String::new(),
                        special_fields: Default::default(),
                    },
                )),
                special_fields: Default::default(),
            }],
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, add_svc, remove_svc, update_type_op]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    assert!(state.services.is_empty(), "service should have been removed");
    let error = debug[3].2.as_ref().unwrap();
    assert!(
        format!("{error:?}").contains("UpdateServiceIsRevoked"),
        "expected service revoked error, got: {error:?}"
    );
}

// ===========================================================================
// protocol/mod.rs coverage: update_service_endpoint on nonexistent service
// ===========================================================================

#[test]
fn resolve_published_update_service_endpoint_nonexistent_returns_error() {
    let (create_did_op, create_did_op_hash, master_sk) = test_utils::new_create_did_operation(None);
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    // UpdateService with only endpoint set (type empty), targeting a nonexistent service
    let (update_ep, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![proto::prism_ssi::UpdateDIDAction {
                action: Some(proto::prism_ssi::update_didaction::Action::UpdateService(
                    proto::prism_ssi::UpdateServiceAction {
                        serviceId: "nonexistent-svc".to_string(),
                        type_: String::new(), // empty → skip update_service_type
                        service_endpoints: "https://new.example.com".to_string(),
                        special_fields: Default::default(),
                    },
                )),
                special_fields: Default::default(),
            }],
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, update_ep]);
    let (state, debug) = resolver::resolve_published(operations);

    let _state = state.unwrap();
    let error = debug[1].2.as_ref().unwrap();
    assert!(
        format!("{error:?}").contains("UpdateServiceNotExists"),
        "expected service not exists error from update_service_endpoint, got: {error:?}"
    );
}

// ===========================================================================
// protocol/mod.rs coverage: update_service_endpoint on revoked service
// ===========================================================================

#[test]
fn resolve_published_update_service_endpoint_on_revoked_service_returns_error() {
    let (create_did_op, create_did_op_hash, master_sk) = test_utils::new_create_did_operation(None);
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    // Add a service
    let (add_svc, add_svc_hash) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![test_utils::add_service_action("svc-0")],
            special_fields: Default::default(),
        }),
    );

    // Remove the service (revokes it)
    let (remove_svc, remove_svc_hash) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: add_svc_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![proto::prism_ssi::UpdateDIDAction {
                action: Some(proto::prism_ssi::update_didaction::Action::RemoveService(
                    proto::prism_ssi::RemoveServiceAction {
                        serviceId: "svc-0".to_string(),
                        special_fields: Default::default(),
                    },
                )),
                special_fields: Default::default(),
            }],
            special_fields: Default::default(),
        }),
    );

    // Try to update the endpoint of the revoked service (type empty → skip update_service_type)
    let (update_ep, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: remove_svc_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![proto::prism_ssi::UpdateDIDAction {
                action: Some(proto::prism_ssi::update_didaction::Action::UpdateService(
                    proto::prism_ssi::UpdateServiceAction {
                        serviceId: "svc-0".to_string(),
                        type_: String::new(), // empty → skip update_service_type
                        service_endpoints: "https://new.example.com".to_string(),
                        special_fields: Default::default(),
                    },
                )),
                special_fields: Default::default(),
            }],
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, add_svc, remove_svc, update_ep]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    assert!(state.services.is_empty(), "service should have been removed");
    let error = debug[3].2.as_ref().unwrap();
    assert!(
        format!("{error:?}").contains("UpdateServiceIsRevoked"),
        "expected service revoked error from update_service_endpoint, got: {error:?}"
    );
}

// ===========================================================================
// protocol/mod.rs coverage: add_storage with duplicate operation hash
// ===========================================================================

#[test]
fn resolve_published_create_storage_with_duplicate_hash_returns_error() {
    let vdr_sk = identus_apollo::crypto::secp256k1::Secp256k1PrivateKey::from_slice(&[2u8; 32]).unwrap();
    let (create_did_op, _, _) = test_utils::new_create_did_operation(Some(test_utils::CreateDidOptions {
        public_keys: Some(vec![test_utils::new_public_key(
            "vdr-0",
            proto::prism_ssi::KeyUsage::VDR_KEY,
            &vdr_sk,
        )]),
        ..Default::default()
    }));
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    // Create a valid storage entry
    let (create_storage_op, _) = test_utils::new_signed_operation(
        "vdr-0",
        &vdr_sk,
        Operation::CreateStorageEntry(proto::prism_storage::ProtoCreateStorageEntry {
            did_prism_hash: did.suffix.to_vec(),
            nonce: vec![0],
            data: Some(proto::prism_storage::proto_create_storage_entry::Data::Bytes(vec![
                1, 2, 3,
            ])),
            special_fields: Default::default(),
        }),
    );

    // Submit the SAME signed operation twice — the second will fail with duplicate hash
    let operations = test_utils::populate_metadata(vec![create_did_op, create_storage_op.clone(), create_storage_op]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    // Only one storage entry should exist
    assert_eq!(state.storage.len(), 1);
    assert_eq!(
        *state.storage[0].data,
        identus_did_prism::did::operation::StorageData::Bytes(vec![1, 2, 3])
    );
    let error = debug[2].2.as_ref().unwrap();
    assert!(
        format!("{error:?}").contains("AddStorageEntryWithExistingHash"),
        "expected duplicate storage hash error, got: {error:?}"
    );
}

// ===========================================================================
// protocol/mod.rs coverage: revoke_storage on already-revoked storage entry
// ===========================================================================

#[test]
fn resolve_published_revoke_storage_already_revoked_returns_error() {
    let vdr_sk = identus_apollo::crypto::secp256k1::Secp256k1PrivateKey::from_slice(&[2u8; 32]).unwrap();
    let (create_did_op, _, _) = test_utils::new_create_did_operation(Some(test_utils::CreateDidOptions {
        public_keys: Some(vec![test_utils::new_public_key(
            "vdr-0",
            proto::prism_ssi::KeyUsage::VDR_KEY,
            &vdr_sk,
        )]),
        ..Default::default()
    }));
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    // Create a storage entry
    let (create_storage_op, create_storage_hash) = test_utils::new_signed_operation(
        "vdr-0",
        &vdr_sk,
        Operation::CreateStorageEntry(proto::prism_storage::ProtoCreateStorageEntry {
            did_prism_hash: did.suffix.to_vec(),
            nonce: vec![0],
            data: Some(proto::prism_storage::proto_create_storage_entry::Data::Bytes(vec![
                1, 2, 3,
            ])),
            special_fields: Default::default(),
        }),
    );

    // First deactivation: prev_event_hash = create_storage_hash
    let (deactivate_op_1, deactivate_hash_1) = test_utils::new_signed_operation(
        "vdr-0",
        &vdr_sk,
        Operation::DeactivateStorageEntry(proto::prism_storage::ProtoDeactivateStorageEntry {
            previous_event_hash: create_storage_hash.to_vec(),
            special_fields: Default::default(),
        }),
    );

    // Second deactivation: prev_event_hash = deactivate_hash_1
    // After the first deactivation, the internal storage entry's prev_operation_hash
    // is updated to deactivate_hash_1. This second deactivation finds the entry but
    // sees it is already revoked.
    let (deactivate_op_2, _) = test_utils::new_signed_operation(
        "vdr-0",
        &vdr_sk,
        Operation::DeactivateStorageEntry(proto::prism_storage::ProtoDeactivateStorageEntry {
            previous_event_hash: deactivate_hash_1.to_vec(),
            special_fields: Default::default(),
        }),
    );

    let operations =
        test_utils::populate_metadata(vec![create_did_op, create_storage_op, deactivate_op_1, deactivate_op_2]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    assert!(state.storage.is_empty(), "storage should be deactivated");
    let error = debug[3].2.as_ref().unwrap();
    assert!(
        format!("{error:?}").contains("RevokeStorageEntryAlreadyRevoked"),
        "expected storage already revoked error, got: {error:?}"
    );
}

// ===========================================================================
// protocol/mod.rs coverage: update_storage on already-revoked storage entry
// ===========================================================================

#[test]
fn resolve_published_update_storage_already_revoked_returns_error() {
    let vdr_sk = identus_apollo::crypto::secp256k1::Secp256k1PrivateKey::from_slice(&[2u8; 32]).unwrap();
    let (create_did_op, _, _) = test_utils::new_create_did_operation(Some(test_utils::CreateDidOptions {
        public_keys: Some(vec![test_utils::new_public_key(
            "vdr-0",
            proto::prism_ssi::KeyUsage::VDR_KEY,
            &vdr_sk,
        )]),
        ..Default::default()
    }));
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    // Create a storage entry
    let (create_storage_op, create_storage_hash) = test_utils::new_signed_operation(
        "vdr-0",
        &vdr_sk,
        Operation::CreateStorageEntry(proto::prism_storage::ProtoCreateStorageEntry {
            did_prism_hash: did.suffix.to_vec(),
            nonce: vec![0],
            data: Some(proto::prism_storage::proto_create_storage_entry::Data::Bytes(vec![
                1, 2, 3,
            ])),
            special_fields: Default::default(),
        }),
    );

    // Deactivate the storage entry
    let (deactivate_op, deactivate_hash) = test_utils::new_signed_operation(
        "vdr-0",
        &vdr_sk,
        Operation::DeactivateStorageEntry(proto::prism_storage::ProtoDeactivateStorageEntry {
            previous_event_hash: create_storage_hash.to_vec(),
            special_fields: Default::default(),
        }),
    );

    // Try to update the deactivated entry using the new prev_hash
    // After deactivation, the internal prev_operation_hash is updated to deactivate_hash.
    // The update finds the entry but sees it is revoked.
    let (update_op, _) = test_utils::new_signed_operation(
        "vdr-0",
        &vdr_sk,
        Operation::UpdateStorageEntry(proto::prism_storage::ProtoUpdateStorageEntry {
            previous_event_hash: deactivate_hash.to_vec(),
            data: Some(proto::prism_storage::proto_update_storage_entry::Data::Bytes(vec![
                4, 5, 6,
            ])),
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, create_storage_op, deactivate_op, update_op]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    assert!(state.storage.is_empty(), "storage should be deactivated");
    let error = debug[3].2.as_ref().unwrap();
    assert!(
        format!("{error:?}").contains("UpdateStorageEntryAlreadyRevoked"),
        "expected storage already revoked error, got: {error:?}"
    );
}

// ===========================================================================
// protocol/mod.rs coverage: init_unpublished_context with invalid CreateDid data
// ===========================================================================

#[test]
fn resolve_unpublished_from_create_did_with_invalid_key_data_fails() {
    // CreateDid with a public key that has no key_data — this passes
    // CanonicalPrismDid::from_operation (only checks operation type) but fails
    // inside processor.create_did when parsing the key.
    let create_op = proto::prism::PrismOperation {
        operation: Some(Operation::CreateDid(proto::prism_ssi::ProtoCreateDID {
            did_data: ::protobuf::MessageField::some(proto::prism_ssi::proto_create_did::DIDCreationData {
                public_keys: vec![proto::prism_ssi::PublicKey {
                    id: "master-0".to_string(),
                    usage: proto::prism_ssi::KeyUsage::MASTER_KEY.into(),
                    key_data: None, // Missing key data — will fail parsing
                    special_fields: Default::default(),
                }],
                services: vec![],
                context: vec![],
                special_fields: Default::default(),
            }),
            special_fields: Default::default(),
        })),
        special_fields: Default::default(),
    };

    let result = resolver::resolve_unpublished(create_op);
    assert!(result.is_err(), "should fail when key data is missing");
}

// ===========================================================================
// v1.rs coverage: Deactivate DID with pre-revoked keys, services, and storage
// Covers the implicit else branches of `if !is_revoked()` in deactivate_did.
// ===========================================================================

#[test]
fn v1_deactivate_did_with_pre_revoked_items() {
    let master_sk = Secp256k1PrivateKey::from_slice(&[1u8; 32]).unwrap();
    let auth_sk = Secp256k1PrivateKey::from_slice(&[3u8; 32]).unwrap();
    let vdr_sk = Secp256k1PrivateKey::from_slice(&[2u8; 32]).unwrap();

    // Create DID with master-0, auth-0, svc-0, vdr-0
    let (create_did_op, create_did_op_hash, _) =
        test_utils::new_create_did_operation(Some(test_utils::CreateDidOptions {
            public_keys: Some(vec![
                test_utils::new_public_key("auth-0", proto::prism_ssi::KeyUsage::AUTHENTICATION_KEY, &auth_sk),
                test_utils::new_public_key("vdr-0", proto::prism_ssi::KeyUsage::VDR_KEY, &vdr_sk),
            ]),
            services: Some(vec![proto::prism_ssi::Service {
                id: "svc-0".to_string(),
                type_: "LinkedDomains".to_string(),
                service_endpoint: "https://example.com".to_string(),
                special_fields: Default::default(),
            }]),
            ..Default::default()
        }));
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    // Step 1: Revoke auth-0 via UpdateDID (signed by master-0)
    let (revoke_auth_op, revoke_auth_hash) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![proto::prism_ssi::UpdateDIDAction {
                action: Some(proto::prism_ssi::update_didaction::Action::RemoveKey(
                    proto::prism_ssi::RemoveKeyAction {
                        keyId: "auth-0".to_string(),
                        special_fields: Default::default(),
                    },
                )),
                special_fields: Default::default(),
            }],
            special_fields: Default::default(),
        }),
    );

    // Step 2: Revoke svc-0 via UpdateDID (signed by master-0)
    let (revoke_svc_op, revoke_svc_hash) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: revoke_auth_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![proto::prism_ssi::UpdateDIDAction {
                action: Some(proto::prism_ssi::update_didaction::Action::RemoveService(
                    proto::prism_ssi::RemoveServiceAction {
                        serviceId: "svc-0".to_string(),
                        special_fields: Default::default(),
                    },
                )),
                special_fields: Default::default(),
            }],
            special_fields: Default::default(),
        }),
    );

    // Step 3: Create storage entry (signed by vdr-0)
    let (create_storage_op, create_storage_hash) = test_utils::new_signed_operation(
        "vdr-0",
        &vdr_sk,
        Operation::CreateStorageEntry(proto::prism_storage::ProtoCreateStorageEntry {
            did_prism_hash: did.suffix.to_vec(),
            nonce: vec![0],
            data: Some(proto::prism_storage::proto_create_storage_entry::Data::Bytes(vec![
                1, 2, 3,
            ])),
            special_fields: Default::default(),
        }),
    );

    // Step 4: Deactivate storage entry (signed by vdr-0)
    let (deactivate_storage_op, _deactivate_storage_hash) = test_utils::new_signed_operation(
        "vdr-0",
        &vdr_sk,
        Operation::DeactivateStorageEntry(proto::prism_storage::ProtoDeactivateStorageEntry {
            previous_event_hash: create_storage_hash.to_vec(),
            special_fields: Default::default(),
        }),
    );

    // Step 5: Deactivate the DID (signed by master-0)
    // prev_operation_hash should match revoke_svc_hash (storage ops don't update SSI chain)
    let (deactivate_did_op, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::DeactivateDid(proto::prism_ssi::ProtoDeactivateDID {
            previous_operation_hash: revoke_svc_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![
        create_did_op,
        revoke_auth_op,
        revoke_svc_op,
        create_storage_op,
        deactivate_storage_op,
        deactivate_did_op,
    ]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();

    // DID should be fully deactivated
    assert!(state.is_deactivated(), "DID should be deactivated");
    assert!(state.public_keys.is_empty(), "all public keys should be revoked");
    assert!(state.services.is_empty(), "all services should be revoked");
    assert!(state.storage.is_empty(), "all storage entries should be revoked");

    // No errors expected in any operation
    for (i, (_, _, err)) in debug.iter().enumerate() {
        assert!(err.is_none(), "operation {} should not have error: {:?}", i, err);
    }
}

// ===========================================================================
// v1.rs coverage: UpdateService with type only (no endpoint change)
// Covers the implicit else branch of `if let Some(ep) = service_endpoints`.
// ===========================================================================

#[test]
fn v1_update_service_type_only() {
    let (create_did_op, create_did_op_hash, master_sk) = test_utils::new_create_did_operation(None);
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    // First: add a service
    let (add_svc, add_svc_hash) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![test_utils::add_service_action("svc-0")],
            special_fields: Default::default(),
        }),
    );

    // Second: update ONLY the type (empty service_endpoints → None in parsed action)
    let (update_type_only, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: add_svc_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![proto::prism_ssi::UpdateDIDAction {
                action: Some(proto::prism_ssi::update_didaction::Action::UpdateService(
                    proto::prism_ssi::UpdateServiceAction {
                        serviceId: "svc-0".to_string(),
                        type_: "DIDCommunication".to_string(),
                        service_endpoints: String::new(), // empty → parsed as None
                        special_fields: Default::default(),
                    },
                )),
                special_fields: Default::default(),
            }],
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, add_svc, update_type_only]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    assert_eq!(state.services.len(), 1);
    // Type should be updated to DIDCommunication
    assert!(
        matches!(&state.services[0].r#type, identus_did_prism::did::operation::ServiceType::One(v) if v.to_string() == "DIDCommunication")
    );
    // Endpoint should remain unchanged from the original add
    let identus_did_prism::did::operation::ServiceEndpoint::One(
        identus_did_prism::did::operation::ServiceEndpointValue::Uri(ref uri),
    ) = state.services[0].service_endpoint
    else {
        panic!("expected URI endpoint");
    };
    assert_eq!(uri, "https://example.com");
    assert!(debug.iter().all(|(_, _, e)| e.is_none()), "no errors expected");
}

// ===========================================================================
// v1.rs coverage: check_signature with oversized signed_with key ID
// Covers line 33 — SignedPrismOperationInvalidSignedWith
// ===========================================================================

#[test]
fn v1_check_signature_oversized_signed_with_returns_error() {
    let (create_did_op, _, _) = test_utils::new_create_did_operation(None);
    // Create a signed_with key ID that exceeds max_id_size (50)
    let long_key_id = "a".repeat(51);
    let bad_op = proto::prism::SignedPrismOperation {
        signed_with: long_key_id,
        signature: vec![],
        operation: Some(proto::prism::PrismOperation {
            operation: Some(Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
                previous_operation_hash: vec![0u8; 32],
                id: "abcdef".to_string(),
                actions: vec![],
                special_fields: Default::default(),
            })),
            special_fields: Default::default(),
        })
        .into(),
        special_fields: Default::default(),
    };

    let operations = test_utils::populate_metadata(vec![create_did_op, bad_op]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    assert_eq!(state.public_keys.len(), 1, "state should be unchanged");
    let error = debug[1].2.as_ref().unwrap();
    assert!(
        format!("{error}").contains("signed_with"),
        "expected invalid signed_with error, got: {error}"
    );
}

// ===========================================================================
// v1.rs coverage: create_did with duplicate key IDs in creation data
// Covers line 94 — add_public_key error path
// ===========================================================================

#[test]
fn v1_create_did_with_duplicate_key_id_in_creation_fails() {
    let master_sk = Secp256k1PrivateKey::from_slice(&[1u8; 32]).unwrap();
    let auth_sk = Secp256k1PrivateKey::from_slice(&[3u8; 32]).unwrap();

    // Create a DID with TWO keys having the same ID "master-0"
    let create_op = proto::prism::SignedPrismOperation {
        signed_with: "master-0".to_string(),
        signature: vec![],
        operation: Some(proto::prism::PrismOperation {
            operation: Some(Operation::CreateDid(proto::prism_ssi::ProtoCreateDID {
                did_data: Some(proto::prism_ssi::proto_create_did::DIDCreationData {
                    public_keys: vec![
                        test_utils::new_public_key("master-0", proto::prism_ssi::KeyUsage::MASTER_KEY, &master_sk),
                        // Same ID "master-0" — will cause AddPublicKeyWithExistingId
                        test_utils::new_public_key(
                            "master-0",
                            proto::prism_ssi::KeyUsage::AUTHENTICATION_KEY,
                            &auth_sk,
                        ),
                    ],
                    services: vec![],
                    context: vec![],
                    special_fields: Default::default(),
                })
                .into(),
                special_fields: Default::default(),
            })),
            special_fields: Default::default(),
        })
        .into(),
        special_fields: Default::default(),
    };

    let operations = test_utils::populate_metadata(vec![create_op]);
    let (state, debug) = resolver::resolve_published(operations);

    // The init should fail because of the duplicate key ID
    assert!(state.is_none(), "DID creation should fail with duplicate key IDs");
    let error = debug[0].2.as_ref().unwrap();
    assert!(
        format!("{error:?}").contains("AddPublicKeyWithExistingId"),
        "expected duplicate key error, got: {error:?}"
    );
}

// ===========================================================================
// v1.rs coverage: create_did with duplicate service IDs in creation data
// Covers line 97 — add_service error path
// ===========================================================================

#[test]
fn v1_create_did_with_duplicate_service_id_in_creation_fails() {
    let master_sk = Secp256k1PrivateKey::from_slice(&[1u8; 32]).unwrap();

    // Create a DID with TWO services having the same ID "svc-0"
    let create_op = proto::prism::SignedPrismOperation {
        signed_with: "master-0".to_string(),
        signature: vec![],
        operation: Some(proto::prism::PrismOperation {
            operation: Some(Operation::CreateDid(proto::prism_ssi::ProtoCreateDID {
                did_data: Some(proto::prism_ssi::proto_create_did::DIDCreationData {
                    public_keys: vec![test_utils::new_public_key(
                        "master-0",
                        proto::prism_ssi::KeyUsage::MASTER_KEY,
                        &master_sk,
                    )],
                    services: vec![
                        proto::prism_ssi::Service {
                            id: "svc-0".to_string(),
                            type_: "LinkedDomains".to_string(),
                            service_endpoint: "https://a.example.com".to_string(),
                            special_fields: Default::default(),
                        },
                        // Same ID "svc-0" — will cause AddServiceWithExistingId
                        proto::prism_ssi::Service {
                            id: "svc-0".to_string(),
                            type_: "DIDCommunication".to_string(),
                            service_endpoint: "https://b.example.com".to_string(),
                            special_fields: Default::default(),
                        },
                    ],
                    context: vec![],
                    special_fields: Default::default(),
                })
                .into(),
                special_fields: Default::default(),
            })),
            special_fields: Default::default(),
        })
        .into(),
        special_fields: Default::default(),
    };

    let operations = test_utils::populate_metadata(vec![create_op]);
    let (state, debug) = resolver::resolve_published(operations);

    // The init should fail because of the duplicate service ID
    assert!(state.is_none(), "DID creation should fail with duplicate service IDs");
    let error = debug[0].2.as_ref().unwrap();
    assert!(
        format!("{error:?}").contains("AddServiceWithExistingId"),
        "expected duplicate service error, got: {error:?}"
    );
}

// ===========================================================================
// v1.rs coverage: update_did with invalid key ID in action (parse error)
// Covers line 111 — UpdateDidOperation::parse error
// ===========================================================================

#[test]
fn v1_update_did_with_oversized_key_id_in_action_returns_parse_error() {
    let (create_did_op, create_did_op_hash, master_sk) = test_utils::new_create_did_operation(None);
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    // AddKey action with a key ID exceeding max_id_size (50)
    let long_key_id = "x".repeat(51);
    let auth_sk = Secp256k1PrivateKey::from_slice(&[4u8; 32]).unwrap();
    let update_op = proto::prism::PrismOperation {
        operation: Some(Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![proto::prism_ssi::UpdateDIDAction {
                action: Some(proto::prism_ssi::update_didaction::Action::AddKey(
                    proto::prism_ssi::AddKeyAction {
                        key: ::protobuf::MessageField::some(test_utils::new_public_key(
                            &long_key_id,
                            proto::prism_ssi::KeyUsage::AUTHENTICATION_KEY,
                            &auth_sk,
                        )),
                        special_fields: Default::default(),
                    },
                )),
                special_fields: Default::default(),
            }],
            special_fields: Default::default(),
        })),
        special_fields: Default::default(),
    };
    let signed_update = proto::prism::SignedPrismOperation {
        signed_with: "master-0".to_string(),
        signature: master_sk.sign(&update_op.encode_to_vec()),
        operation: Some(update_op).into(),
        special_fields: Default::default(),
    };

    let operations = test_utils::populate_metadata(vec![create_did_op, signed_update]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    assert_eq!(state.public_keys.len(), 1, "state should be unchanged");
    let error = debug[1].2.as_ref().unwrap();
    // The error should be about invalid public key (key ID too long)
    assert!(
        format!("{error:?}").contains("InvalidKeyId") || format!("{error:?}").contains("InvalidPublicKey"),
        "expected invalid key ID error, got: {error:?}"
    );
}

// ===========================================================================
// v1.rs coverage: deactivate_did with invalid previous_operation_hash
// Covers line 138 — DeactivateDidOperation::parse error
// ===========================================================================

#[test]
fn v1_deactivate_did_with_invalid_prev_hash_length_returns_parse_error() {
    let (create_did_op, _, master_sk) = test_utils::new_create_did_operation(None);
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    // previous_operation_hash is only 16 bytes instead of 32 — parse error
    let deactivate_op = proto::prism::PrismOperation {
        operation: Some(Operation::DeactivateDid(proto::prism_ssi::ProtoDeactivateDID {
            previous_operation_hash: vec![0u8; 16], // wrong length
            id: did.suffix_hex().to_string(),
            special_fields: Default::default(),
        })),
        special_fields: Default::default(),
    };
    let signed_deactivate = proto::prism::SignedPrismOperation {
        signed_with: "master-0".to_string(),
        signature: master_sk.sign(&deactivate_op.encode_to_vec()),
        operation: Some(deactivate_op).into(),
        special_fields: Default::default(),
    };

    let operations = test_utils::populate_metadata(vec![create_did_op, signed_deactivate]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    assert!(!state.is_deactivated(), "deactivate should fail with invalid hash");
    assert_eq!(state.public_keys.len(), 1);
    let error = debug[1].2.as_ref().unwrap();
    assert!(
        format!("{error:?}").contains("InvalidPreviousOperationHash"),
        "expected invalid previous operation hash error, got: {error:?}"
    );
}

// ===========================================================================
// v1.rs coverage: deactivate_did with invalid DID suffix in id field
// Covers line 138 — DeactivateDidOperation::parse error (invalid suffix)
// ===========================================================================

#[test]
fn v1_deactivate_did_with_invalid_did_suffix_returns_parse_error() {
    let (create_did_op, _, master_sk) = test_utils::new_create_did_operation(None);

    // id is not a valid hex string for a DID suffix
    let deactivate_op = proto::prism::PrismOperation {
        operation: Some(Operation::DeactivateDid(proto::prism_ssi::ProtoDeactivateDID {
            previous_operation_hash: vec![0u8; 32],
            id: "not-valid-hex".to_string(),
            special_fields: Default::default(),
        })),
        special_fields: Default::default(),
    };
    let signed_deactivate = proto::prism::SignedPrismOperation {
        signed_with: "master-0".to_string(),
        signature: master_sk.sign(&deactivate_op.encode_to_vec()),
        operation: Some(deactivate_op).into(),
        special_fields: Default::default(),
    };

    let operations = test_utils::populate_metadata(vec![create_did_op, signed_deactivate]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    assert!(!state.is_deactivated(), "deactivate should fail with invalid DID");
    let error = debug[1].2.as_ref().unwrap();
    assert!(
        format!("{error:?}").contains("DidSuffixInvalid"),
        "expected invalid DID suffix error, got: {error:?}"
    );
}

// ===========================================================================
// v1.rs coverage: create_storage with invalid did_prism_hash
// Covers line 190 — CreateStorageOperation::parse error
// ===========================================================================

#[test]
fn v1_create_storage_with_invalid_did_hash_returns_parse_error() {
    let vdr_sk = Secp256k1PrivateKey::from_slice(&[2u8; 32]).unwrap();
    let (create_did_op, _, _) = test_utils::new_create_did_operation(Some(test_utils::CreateDidOptions {
        public_keys: Some(vec![test_utils::new_public_key(
            "vdr-0",
            proto::prism_ssi::KeyUsage::VDR_KEY,
            &vdr_sk,
        )]),
        ..Default::default()
    }));

    // did_prism_hash is only 16 bytes instead of 32
    let create_storage_op = proto::prism::PrismOperation {
        operation: Some(Operation::CreateStorageEntry(
            proto::prism_storage::ProtoCreateStorageEntry {
                did_prism_hash: vec![0u8; 16], // wrong length
                nonce: vec![0],
                data: Some(proto::prism_storage::proto_create_storage_entry::Data::Bytes(vec![
                    1, 2, 3,
                ])),
                special_fields: Default::default(),
            },
        )),
        special_fields: Default::default(),
    };
    let signed_storage = proto::prism::SignedPrismOperation {
        signed_with: "vdr-0".to_string(),
        signature: vdr_sk.sign(&create_storage_op.encode_to_vec()),
        operation: Some(create_storage_op).into(),
        special_fields: Default::default(),
    };

    let operations = test_utils::populate_metadata(vec![create_did_op, signed_storage]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    assert!(state.storage.is_empty(), "storage should not be created");
    let error = debug[1].2.as_ref().unwrap();
    assert!(
        format!("{error:?}").contains("InvalidDidSyntax") || format!("{error:?}").contains("CreateStorageOperation"),
        "expected create storage parse error, got: {error:?}"
    );
}

// ===========================================================================
// v1.rs coverage: update_storage with invalid previous_event_hash
// Covers line 214 — UpdateStorageOperation::parse error
// ===========================================================================

#[test]
fn v1_update_storage_with_invalid_prev_hash_returns_parse_error() {
    let vdr_sk = Secp256k1PrivateKey::from_slice(&[2u8; 32]).unwrap();
    let (create_did_op, _, _) = test_utils::new_create_did_operation(Some(test_utils::CreateDidOptions {
        public_keys: Some(vec![test_utils::new_public_key(
            "vdr-0",
            proto::prism_ssi::KeyUsage::VDR_KEY,
            &vdr_sk,
        )]),
        ..Default::default()
    }));

    // previous_event_hash is only 16 bytes instead of 32
    let update_storage_op = proto::prism::PrismOperation {
        operation: Some(Operation::UpdateStorageEntry(
            proto::prism_storage::ProtoUpdateStorageEntry {
                previous_event_hash: vec![0u8; 16], // wrong length
                data: Some(proto::prism_storage::proto_update_storage_entry::Data::Bytes(vec![
                    4, 5, 6,
                ])),
                special_fields: Default::default(),
            },
        )),
        special_fields: Default::default(),
    };
    let signed_storage = proto::prism::SignedPrismOperation {
        signed_with: "vdr-0".to_string(),
        signature: vdr_sk.sign(&update_storage_op.encode_to_vec()),
        operation: Some(update_storage_op).into(),
        special_fields: Default::default(),
    };

    let operations = test_utils::populate_metadata(vec![create_did_op, signed_storage]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    assert!(state.storage.is_empty(), "storage should not be updated");
    let error = debug[1].2.as_ref().unwrap();
    assert!(
        format!("{error:?}").contains("UpdateStorageOperation"),
        "expected update storage parse error, got: {error:?}"
    );
}

// ===========================================================================
// v1.rs coverage: deactivate_storage with invalid previous_event_hash
// Covers line 241 — DeactivateStorageOperation::parse error
// ===========================================================================

#[test]
fn v1_deactivate_storage_with_invalid_prev_hash_returns_parse_error() {
    let vdr_sk = Secp256k1PrivateKey::from_slice(&[2u8; 32]).unwrap();
    let (create_did_op, _, _) = test_utils::new_create_did_operation(Some(test_utils::CreateDidOptions {
        public_keys: Some(vec![test_utils::new_public_key(
            "vdr-0",
            proto::prism_ssi::KeyUsage::VDR_KEY,
            &vdr_sk,
        )]),
        ..Default::default()
    }));

    // previous_event_hash is only 16 bytes instead of 32
    let deactivate_storage_op = proto::prism::PrismOperation {
        operation: Some(Operation::DeactivateStorageEntry(
            proto::prism_storage::ProtoDeactivateStorageEntry {
                previous_event_hash: vec![0u8; 16], // wrong length
                special_fields: Default::default(),
            },
        )),
        special_fields: Default::default(),
    };
    let signed_storage = proto::prism::SignedPrismOperation {
        signed_with: "vdr-0".to_string(),
        signature: vdr_sk.sign(&deactivate_storage_op.encode_to_vec()),
        operation: Some(deactivate_storage_op).into(),
        special_fields: Default::default(),
    };

    let operations = test_utils::populate_metadata(vec![create_did_op, signed_storage]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    assert!(state.storage.is_empty(), "storage should not be deactivated");
    let error = debug[1].2.as_ref().unwrap();
    assert!(
        format!("{error:?}").contains("DeactivateStorageOperation"),
        "expected deactivate storage parse error, got: {error:?}"
    );
}

// ===========================================================================
// v1.rs coverage: check_signature with master key on storage operation
// Covers the invalid key usage error path in check_signature
// ===========================================================================

#[test]
fn v1_check_signature_master_key_on_storage_op_returns_invalid_key_error() {
    let (create_did_op, _, master_sk) = test_utils::new_create_did_operation(None);
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    // Try to create a storage entry signed by master-0 (MASTER key), not VDR key
    let create_storage_op = proto::prism::PrismOperation {
        operation: Some(Operation::CreateStorageEntry(
            proto::prism_storage::ProtoCreateStorageEntry {
                did_prism_hash: did.suffix.to_vec(),
                nonce: vec![0],
                data: Some(proto::prism_storage::proto_create_storage_entry::Data::Bytes(vec![
                    1, 2, 3,
                ])),
                special_fields: Default::default(),
            },
        )),
        special_fields: Default::default(),
    };
    let signed_storage = proto::prism::SignedPrismOperation {
        signed_with: "master-0".to_string(),
        signature: master_sk.sign(&create_storage_op.encode_to_vec()),
        operation: Some(create_storage_op).into(),
        special_fields: Default::default(),
    };

    let operations = test_utils::populate_metadata(vec![create_did_op, signed_storage]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    assert!(
        state.storage.is_empty(),
        "storage should not be created with master key"
    );
    let error = debug[1].2.as_ref().unwrap();
    assert!(
        format!("{error}").contains("not expected key"),
        "expected not expected key error, got: {error}"
    );
}

// ===========================================================================
// v1.rs coverage: check_signature with VDR key on non-storage operation
// Covers the invalid key usage error path (using VDR key for UpdateDid)
// ===========================================================================

#[test]
fn v1_check_signature_vdr_key_on_update_did_returns_invalid_key_error() {
    let vdr_sk = Secp256k1PrivateKey::from_slice(&[2u8; 32]).unwrap();
    let (create_did_op, create_did_op_hash, _) =
        test_utils::new_create_did_operation(Some(test_utils::CreateDidOptions {
            public_keys: Some(vec![test_utils::new_public_key(
                "vdr-0",
                proto::prism_ssi::KeyUsage::VDR_KEY,
                &vdr_sk,
            )]),
            ..Default::default()
        }));
    let did = CanonicalPrismDid::from_operation(create_did_op.operation.as_ref().unwrap()).unwrap();

    // Try to update DID signed by vdr-0 (VDR key), not master key
    let update_op = proto::prism::PrismOperation {
        operation: Some(Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![test_utils::add_service_action("svc-0")],
            special_fields: Default::default(),
        })),
        special_fields: Default::default(),
    };
    let signed_update = proto::prism::SignedPrismOperation {
        signed_with: "vdr-0".to_string(),
        signature: vdr_sk.sign(&update_op.encode_to_vec()),
        operation: Some(update_op).into(),
        special_fields: Default::default(),
    };

    let operations = test_utils::populate_metadata(vec![create_did_op, signed_update]);
    let (state, debug) = resolver::resolve_published(operations);

    let state = state.unwrap();
    assert!(state.services.is_empty(), "update should be rejected");
    let error = debug[1].2.as_ref().unwrap();
    assert!(
        format!("{error}").contains("not expected key"),
        "expected not expected key error, got: {error}"
    );
}
