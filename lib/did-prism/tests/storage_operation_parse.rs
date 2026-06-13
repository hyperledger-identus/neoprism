//! Unit tests for `did::operation::storage` parse functions and StorageData conversions.
//!
//! These tests directly exercise the parse methods and all StorageData variants,
//! complementing the integration-level tests in `storage_operation.rs`.

use identus_did_prism::did::error::{CreateStorageOperationError, UpdateStorageOperationError};
use identus_did_prism::did::operation::{
    CreateStorageOperation, DeactivateStorageOperation, StatusListData, StorageData, UpdateStorageOperation,
};
use identus_did_prism::proto::prism_storage::{
    ProtoCreateStorageEntry, ProtoDeactivateStorageEntry, ProtoUpdateStorageEntry, StatusListEntry,
    proto_create_storage_entry, proto_update_storage_entry,
};

// ---------------------------------------------------------------------------
// StorageData: From<ProtoCreateStorageData>
// ---------------------------------------------------------------------------

#[test]
fn storage_data_from_create_bytes() {
    let data = proto_create_storage_entry::Data::Bytes(vec![1, 2, 3]);
    let storage_data: StorageData = data.into();
    assert_eq!(storage_data, StorageData::Bytes(vec![1, 2, 3]));
}

#[test]
fn storage_data_from_create_ipfs() {
    let data = proto_create_storage_entry::Data::Ipfs("QmExample".to_string());
    let storage_data: StorageData = data.into();
    assert_eq!(storage_data, StorageData::Ipfs("QmExample".to_string()));
}

#[test]
fn storage_data_from_create_status_list() {
    let sle = StatusListEntry {
        state: 1,
        name: "revocationList".to_string(),
        details: "example details".to_string(),
        special_fields: Default::default(),
    };
    let data = proto_create_storage_entry::Data::StatusListEntry(sle);
    let storage_data: StorageData = data.into();
    assert_eq!(
        storage_data,
        StorageData::StatusList(StatusListData {
            state: 1,
            name: "revocationList".to_string(),
            detail: "example details".to_string(),
        })
    );
}

// ---------------------------------------------------------------------------
// StorageData: From<ProtoUpdateStorageData>
// ---------------------------------------------------------------------------

#[test]
fn storage_data_from_update_bytes() {
    let data = proto_update_storage_entry::Data::Bytes(vec![4, 5, 6]);
    let storage_data: StorageData = data.into();
    assert_eq!(storage_data, StorageData::Bytes(vec![4, 5, 6]));
}

#[test]
fn storage_data_from_update_ipfs() {
    let data = proto_update_storage_entry::Data::Ipfs("QmUpdateCid".to_string());
    let storage_data: StorageData = data.into();
    assert_eq!(storage_data, StorageData::Ipfs("QmUpdateCid".to_string()));
}

#[test]
fn storage_data_from_update_status_list() {
    let sle = StatusListEntry {
        state: 0,
        name: "statusList".to_string(),
        details: "update details".to_string(),
        special_fields: Default::default(),
    };
    let data = proto_update_storage_entry::Data::StatusListEntry(sle);
    let storage_data: StorageData = data.into();
    assert_eq!(
        storage_data,
        StorageData::StatusList(StatusListData {
            state: 0,
            name: "statusList".to_string(),
            detail: "update details".to_string(),
        })
    );
}

// ---------------------------------------------------------------------------
// StorageData derived traits
// ---------------------------------------------------------------------------

#[test]
fn storage_data_debug_clone_equality() {
    let a = StorageData::Bytes(vec![1, 2, 3]);
    let b = a.clone();
    assert_eq!(a, b);
    // Verify Debug produces output
    let debug_str = format!("{a:?}");
    assert!(debug_str.contains("Bytes"));

    let ipfs = StorageData::Ipfs("cid".to_string());
    let ipfs2 = ipfs.clone();
    assert_eq!(ipfs, ipfs2);

    let sl = StorageData::StatusList(StatusListData {
        state: 42,
        name: "n".to_string(),
        detail: "d".to_string(),
    });
    let sl2 = sl.clone();
    assert_eq!(sl, sl2);
}

#[test]
fn status_list_data_debug_clone_equality() {
    let a = StatusListData {
        state: 1,
        name: "test".to_string(),
        detail: "detail".to_string(),
    };
    let b = a.clone();
    assert_eq!(a, b);
    assert_eq!(a.state, 1);
    assert_eq!(a.name, "test");
    assert_eq!(a.detail, "detail");
}

// ---------------------------------------------------------------------------
// CreateStorageOperation::parse
// ---------------------------------------------------------------------------

#[test]
fn create_storage_operation_parse_success() {
    let proto = ProtoCreateStorageEntry {
        did_prism_hash: vec![0u8; 32], // valid 32-byte hash
        nonce: vec![42],
        data: Some(proto_create_storage_entry::Data::Bytes(vec![1, 2, 3])),
        special_fields: Default::default(),
    };

    let op = CreateStorageOperation::parse(&proto).unwrap();
    assert_eq!(op.nonce, vec![42]);
    assert_eq!(op.data, StorageData::Bytes(vec![1, 2, 3]));
}

#[test]
fn create_storage_operation_parse_with_ipfs_data() {
    let proto = ProtoCreateStorageEntry {
        did_prism_hash: vec![0u8; 32],
        nonce: vec![0],
        data: Some(proto_create_storage_entry::Data::Ipfs("QmTest".to_string())),
        special_fields: Default::default(),
    };

    let op = CreateStorageOperation::parse(&proto).unwrap();
    assert_eq!(op.data, StorageData::Ipfs("QmTest".to_string()));
}

#[test]
fn create_storage_operation_parse_with_status_list_data() {
    let sle = StatusListEntry {
        state: 5,
        name: "slName".to_string(),
        details: "slDetail".to_string(),
        special_fields: Default::default(),
    };
    let proto = ProtoCreateStorageEntry {
        did_prism_hash: vec![0u8; 32],
        nonce: vec![0],
        data: Some(proto_create_storage_entry::Data::StatusListEntry(sle)),
        special_fields: Default::default(),
    };

    let op = CreateStorageOperation::parse(&proto).unwrap();
    assert_eq!(
        op.data,
        StorageData::StatusList(StatusListData {
            state: 5,
            name: "slName".to_string(),
            detail: "slDetail".to_string(),
        })
    );
}

#[test]
fn create_storage_operation_parse_empty_data_returns_error() {
    let proto = ProtoCreateStorageEntry {
        did_prism_hash: vec![0u8; 32],
        nonce: vec![0],
        data: None,
        special_fields: Default::default(),
    };

    let result = CreateStorageOperation::parse(&proto);
    assert!(result.is_err());
    let err = result.unwrap_err();
    // Verify the error display message
    let msg = format!("{err}");
    assert!(msg.contains("missing storage data"), "unexpected error: {msg}");
}

#[test]
fn create_storage_operation_parse_invalid_did_hash_returns_error() {
    // Empty did_prism_hash is not a valid 32-byte hash → DidSyntaxError
    let proto = ProtoCreateStorageEntry {
        did_prism_hash: vec![],
        nonce: vec![0],
        data: Some(proto_create_storage_entry::Data::Bytes(vec![1])),
        special_fields: Default::default(),
    };

    let result = CreateStorageOperation::parse(&proto);
    let err = result.unwrap_err();
    assert!(
        matches!(err, CreateStorageOperationError::InvalidDidSyntax { .. }),
        "expected InvalidDidSyntax, got: {err:?}"
    );
}

#[test]
fn create_storage_operation_parse_wrong_hash_length_returns_error() {
    // 16 bytes is not a valid suffix length
    let proto = ProtoCreateStorageEntry {
        did_prism_hash: vec![0u8; 16],
        nonce: vec![0],
        data: Some(proto_create_storage_entry::Data::Bytes(vec![1])),
        special_fields: Default::default(),
    };

    let result = CreateStorageOperation::parse(&proto);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// UpdateStorageOperation::parse
// ---------------------------------------------------------------------------

#[test]
fn update_storage_operation_parse_success() {
    let proto = ProtoUpdateStorageEntry {
        previous_event_hash: vec![0u8; 32],
        data: Some(proto_update_storage_entry::Data::Bytes(vec![10, 20])),
        special_fields: Default::default(),
    };

    let op = UpdateStorageOperation::parse(&proto).unwrap();
    assert_eq!(op.data, StorageData::Bytes(vec![10, 20]));
}

#[test]
fn update_storage_operation_parse_with_ipfs() {
    let proto = ProtoUpdateStorageEntry {
        previous_event_hash: vec![0u8; 32],
        data: Some(proto_update_storage_entry::Data::Ipfs("QmUpdated".to_string())),
        special_fields: Default::default(),
    };

    let op = UpdateStorageOperation::parse(&proto).unwrap();
    assert_eq!(op.data, StorageData::Ipfs("QmUpdated".to_string()));
}

#[test]
fn update_storage_operation_parse_with_status_list() {
    let sle = StatusListEntry {
        state: 99,
        name: "upName".to_string(),
        details: "upDetail".to_string(),
        special_fields: Default::default(),
    };
    let proto = ProtoUpdateStorageEntry {
        previous_event_hash: vec![0u8; 32],
        data: Some(proto_update_storage_entry::Data::StatusListEntry(sle)),
        special_fields: Default::default(),
    };

    let op = UpdateStorageOperation::parse(&proto).unwrap();
    assert_eq!(
        op.data,
        StorageData::StatusList(StatusListData {
            state: 99,
            name: "upName".to_string(),
            detail: "upDetail".to_string(),
        })
    );
}

#[test]
fn update_storage_operation_parse_empty_data_returns_error() {
    let proto = ProtoUpdateStorageEntry {
        previous_event_hash: vec![0u8; 32],
        data: None,
        special_fields: Default::default(),
    };

    let result = UpdateStorageOperation::parse(&proto);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("missing storage data"), "unexpected error: {msg}");
}

#[test]
fn update_storage_operation_parse_invalid_hash_returns_error() {
    // previous_event_hash must be 32 bytes for Sha256Digest
    let proto = ProtoUpdateStorageEntry {
        previous_event_hash: vec![0u8; 16], // wrong length
        data: Some(proto_update_storage_entry::Data::Bytes(vec![1])),
        special_fields: Default::default(),
    };

    let result = UpdateStorageOperation::parse(&proto);
    let err = result.unwrap_err();
    assert!(
        matches!(err, UpdateStorageOperationError::InvalidPreviousOperationHash { .. }),
        "expected InvalidPreviousOperationHash, got: {err:?}"
    );
}

#[test]
fn update_storage_operation_parse_empty_hash_returns_error() {
    let proto = ProtoUpdateStorageEntry {
        previous_event_hash: vec![],
        data: Some(proto_update_storage_entry::Data::Bytes(vec![1])),
        special_fields: Default::default(),
    };

    let result = UpdateStorageOperation::parse(&proto);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// DeactivateStorageOperation::parse
// ---------------------------------------------------------------------------

#[test]
fn deactivate_storage_operation_parse_success() {
    let proto = ProtoDeactivateStorageEntry {
        previous_event_hash: vec![0u8; 32],
        special_fields: Default::default(),
    };

    let op = DeactivateStorageOperation::parse(&proto).unwrap();
    // Verify the parsed hash is correct via round-trip
    let hash_bytes = op.prev_operation_hash.to_vec();
    assert_eq!(hash_bytes, vec![0u8; 32]);
}

#[test]
fn deactivate_storage_operation_parse_invalid_hash_returns_error() {
    let proto = ProtoDeactivateStorageEntry {
        previous_event_hash: vec![0u8; 16], // wrong length
        special_fields: Default::default(),
    };

    let result = DeactivateStorageOperation::parse(&proto);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(
        msg.contains("previous operation hash") || msg.contains("invalid"),
        "unexpected error: {msg}"
    );
}

#[test]
fn deactivate_storage_operation_parse_empty_hash_returns_error() {
    let proto = ProtoDeactivateStorageEntry {
        previous_event_hash: vec![],
        special_fields: Default::default(),
    };

    let result = DeactivateStorageOperation::parse(&proto);
    assert!(result.is_err());
}
