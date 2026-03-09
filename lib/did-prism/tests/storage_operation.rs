use std::ops::Deref;

use identus_apollo::crypto::secp256k1::Secp256k1PrivateKey;
use identus_apollo::hash::Sha256Digest;
use identus_apollo::hex::HexStr;
use identus_did_prism::did::operation::StorageData;
use identus_did_prism::did::{CanonicalPrismDid, PrismDidOps};
use identus_did_prism::proto;
use identus_did_prism::protocol::resolver;

const VDR_KEY: [u8; 32] = [2; 32];
const VDR_KEY_NAME: &str = "vdr-0";

mod test_utils;

#[test]
fn create_storage_entry() {
    let (create_did_op, _, did, _, vdr_sk) = create_did_with_vdr_key();
    let (create_storage_op, _) = new_create_storage_op(&did, &vdr_sk, vec![1, 2, 3], vec![0]);

    let operations = test_utils::populate_metadata(vec![create_did_op, create_storage_op]);
    let state = resolver::resolve_published(operations).0.unwrap();

    assert_eq!(state.storage.len(), 1);
    assert_eq!(*state.storage[0].data, StorageData::Bytes(vec![1, 2, 3]));
}

#[test]
fn create_multiple_storage_entries() {
    let (create_did_op, _, did, _, vdr_sk) = create_did_with_vdr_key();
    let (create_storage_op_1, create_storage_op_hash_1) = new_create_storage_op(&did, &vdr_sk, vec![1, 2, 3], vec![0]);
    let (create_storage_op_2, create_storage_op_hash_2) = new_create_storage_op(&did, &vdr_sk, vec![4, 5, 6], vec![1]);

    let operations = test_utils::populate_metadata(vec![create_did_op, create_storage_op_1, create_storage_op_2]);
    let state = resolver::resolve_published(operations).0.unwrap();

    assert_eq!(state.storage.len(), 2);
    assert_eq!(
        *state
            .storage
            .iter()
            .find(|s| s.init_operation_hash.deref() == &create_storage_op_hash_1)
            .unwrap()
            .data,
        StorageData::Bytes(vec![1, 2, 3])
    );
    assert_eq!(
        *state
            .storage
            .iter()
            .find(|s| s.init_operation_hash.deref() == &create_storage_op_hash_2)
            .unwrap()
            .data,
        StorageData::Bytes(vec![4, 5, 6])
    );
}

#[test]
fn update_storage_entry() {
    let (create_did_op, _, did, _, vdr_sk) = create_did_with_vdr_key();
    let (create_storage_op, create_storage_op_hash) = new_create_storage_op(&did, &vdr_sk, vec![1, 2, 3], vec![0]);
    let (update_storage_op, update_storage_op_hash) = test_utils::new_signed_operation(
        VDR_KEY_NAME,
        &vdr_sk,
        proto::prism::prism_operation::Operation::UpdateStorageEntry(proto::prism_storage::ProtoUpdateStorageEntry {
            previous_event_hash: create_storage_op_hash.to_vec(),
            data: Some(proto::prism_storage::proto_update_storage_entry::Data::Bytes(vec![
                4, 5, 6,
            ])),
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, create_storage_op, update_storage_op]);
    let state = resolver::resolve_published(operations).0.unwrap();

    assert_eq!(state.storage.len(), 1);
    assert_eq!(state.storage[0].init_operation_hash.deref(), &create_storage_op_hash);
    assert_eq!(state.storage[0].last_operation_hash.deref(), &update_storage_op_hash);
    assert_eq!(state.storage[0].data.deref(), &StorageData::Bytes(vec![4, 5, 6]));
}

#[test]
fn deactivate_storage_entry() {
    let (create_did_op, _, did, _, vdr_sk) = create_did_with_vdr_key();
    let (create_storage_op, create_storage_op_hash) = new_create_storage_op(&did, &vdr_sk, vec![1, 2, 3], vec![0]);
    let (deactivate_storage_op, _) = test_utils::new_signed_operation(
        VDR_KEY_NAME,
        &vdr_sk,
        proto::prism::prism_operation::Operation::DeactivateStorageEntry(
            proto::prism_storage::ProtoDeactivateStorageEntry {
                previous_event_hash: create_storage_op_hash.to_vec(),
                special_fields: Default::default(),
            },
        ),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, create_storage_op, deactivate_storage_op]);
    let state = resolver::resolve_published(operations).0.unwrap();

    assert!(state.storage.is_empty());
}

#[test]
fn create_storage_entry_with_non_vdr_key() {
    let (create_did_op, _, did, _, vdr_sk) = create_did_with_vdr_key();
    let (create_storage_op, _) = test_utils::new_signed_operation(
        "master-0",
        &vdr_sk,
        proto::prism::prism_operation::Operation::CreateStorageEntry(proto::prism_storage::ProtoCreateStorageEntry {
            did_prism_hash: did.suffix.to_vec(),
            nonce: vec![0],
            data: Some(proto::prism_storage::proto_create_storage_entry::Data::Bytes(vec![
                1, 2, 3,
            ])),
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, create_storage_op]);
    let state = resolver::resolve_published(operations).0.unwrap();

    assert!(state.storage.is_empty());
}

#[test]
fn update_storage_entry_with_invalid_prev_event_hash() {
    let (create_did_op, _, did, _, vdr_sk) = create_did_with_vdr_key();
    let (create_storage_op, _) = new_create_storage_op(&did, &vdr_sk, vec![1, 2, 3], vec![0]);
    let (update_storage_op_1, update_op_hash_1) = test_utils::new_signed_operation(
        VDR_KEY_NAME,
        &vdr_sk,
        proto::prism::prism_operation::Operation::UpdateStorageEntry(proto::prism_storage::ProtoUpdateStorageEntry {
            previous_event_hash: [0; 32].to_vec(),
            data: Some(proto::prism_storage::proto_update_storage_entry::Data::Bytes(vec![
                4, 5, 6,
            ])),
            special_fields: Default::default(),
        }),
    );
    let (update_storage_op_2, _) = test_utils::new_signed_operation(
        VDR_KEY_NAME,
        &vdr_sk,
        proto::prism::prism_operation::Operation::UpdateStorageEntry(proto::prism_storage::ProtoUpdateStorageEntry {
            previous_event_hash: update_op_hash_1.to_vec(),
            data: Some(proto::prism_storage::proto_update_storage_entry::Data::Bytes(vec![
                4, 5, 6,
            ])),
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![
        create_did_op,
        create_storage_op,
        update_storage_op_1,
        update_storage_op_2,
    ]);
    let state = resolver::resolve_published(operations).0.unwrap();

    assert_eq!(state.storage.len(), 1);
    assert_eq!(state.storage[0].data.deref(), &StorageData::Bytes(vec![1, 2, 3]));
}

#[test]
fn update_storage_entry_with_non_vdr_key() {
    let (create_did_op, _, did, master_sk, vdr_sk) = create_did_with_vdr_key();
    let (create_storage_op, create_storage_op_hash) = new_create_storage_op(&did, &vdr_sk, vec![1, 2, 3], vec![0]);
    let (update_storage_op, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        proto::prism::prism_operation::Operation::UpdateStorageEntry(proto::prism_storage::ProtoUpdateStorageEntry {
            previous_event_hash: create_storage_op_hash.to_vec(),
            data: Some(proto::prism_storage::proto_update_storage_entry::Data::Bytes(vec![
                4, 5, 6,
            ])),
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, create_storage_op, update_storage_op]);
    let state = resolver::resolve_published(operations).0.unwrap();

    assert_eq!(state.storage.len(), 1);
    assert_eq!(state.storage[0].data.deref(), &StorageData::Bytes(vec![1, 2, 3]));
}

#[test]
fn update_storage_entry_with_revoked_key() {
    let (create_did_op, create_did_op_hash, did, master_sk, vdr_sk) = create_did_with_vdr_key();
    let (create_storage_op, create_storage_op_hash) = new_create_storage_op(&did, &vdr_sk, vec![1, 2, 3], vec![0]);
    let (revoke_key_op, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        proto::prism::prism_operation::Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![proto::prism_ssi::UpdateDIDAction {
                action: Some(proto::prism_ssi::update_didaction::Action::RemoveKey(
                    proto::prism_ssi::RemoveKeyAction {
                        keyId: VDR_KEY_NAME.to_string(),
                        special_fields: Default::default(),
                    },
                )),
                special_fields: Default::default(),
            }],
            special_fields: Default::default(),
        }),
    );
    let (update_storage_op, _) = test_utils::new_signed_operation(
        VDR_KEY_NAME,
        &vdr_sk,
        proto::prism::prism_operation::Operation::UpdateStorageEntry(proto::prism_storage::ProtoUpdateStorageEntry {
            previous_event_hash: create_storage_op_hash.to_vec(),
            data: Some(proto::prism_storage::proto_update_storage_entry::Data::Bytes(vec![
                4, 5, 6,
            ])),
            special_fields: Default::default(),
        }),
    );

    let operations =
        test_utils::populate_metadata(vec![create_did_op, create_storage_op, revoke_key_op, update_storage_op]);
    let state = resolver::resolve_published(operations).0.unwrap();

    assert_eq!(state.storage.len(), 1);
    assert_eq!(state.storage[0].data.deref(), &StorageData::Bytes(vec![1, 2, 3]));
}

#[test]
fn create_storage_entry_with_revoked_key() {
    let (create_did_op, create_did_op_hash, did, master_sk, vdr_sk) = create_did_with_vdr_key();
    let (revoke_key_op, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        proto::prism::prism_operation::Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![proto::prism_ssi::UpdateDIDAction {
                action: Some(proto::prism_ssi::update_didaction::Action::RemoveKey(
                    proto::prism_ssi::RemoveKeyAction {
                        keyId: VDR_KEY_NAME.to_string(),
                        special_fields: Default::default(),
                    },
                )),
                special_fields: Default::default(),
            }],
            special_fields: Default::default(),
        }),
    );
    let (create_storage_op, _) = new_create_storage_op(&did, &vdr_sk, vec![1, 2, 3], vec![0]);

    let operations = test_utils::populate_metadata(vec![create_did_op, revoke_key_op, create_storage_op]);
    let state = resolver::resolve_published(operations).0.unwrap();

    assert_eq!(state.storage.len(), 0);
}

#[test]
fn deactivate_storage_entry_with_invalid_prev_operation_hash() {
    let (create_did_op, _, did, _, vdr_sk) = create_did_with_vdr_key();
    let (create_storage_op, _) = new_create_storage_op(&did, &vdr_sk, vec![1, 2, 3], vec![0]);
    let (deactivate_storage_op, _) = test_utils::new_signed_operation(
        VDR_KEY_NAME,
        &vdr_sk,
        proto::prism::prism_operation::Operation::DeactivateStorageEntry(
            proto::prism_storage::ProtoDeactivateStorageEntry {
                previous_event_hash: [0; 32].to_vec(),
                special_fields: Default::default(),
            },
        ),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, create_storage_op, deactivate_storage_op]);
    let state = resolver::resolve_published(operations).0.unwrap();

    assert_eq!(state.storage.len(), 1);
    assert_eq!(state.storage[0].data.deref(), &StorageData::Bytes(vec![1, 2, 3]));
}

#[test]
fn storage_revoked_after_deactivate_did() {
    let (create_did_op, create_did_op_hash, did, master_sk, vdr_sk) = create_did_with_vdr_key();
    let (create_storage_op, _) = new_create_storage_op(&did, &vdr_sk, vec![1, 2, 3], vec![0]);
    let (deactivate_did_op, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        proto::prism::prism_operation::Operation::DeactivateDid(proto::prism_ssi::ProtoDeactivateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, create_storage_op, deactivate_did_op]);
    let state = resolver::resolve_published(operations).0.unwrap();

    assert!(state.storage.is_empty());
}

/// Test that `init_operation_hash` equals the hash of the CreateStorageEntry operation,
/// and `last_operation_hash` equals `init_operation_hash` when no updates are applied.
/// This property is used by the VDR entry metadata endpoint to return the correct hashes.
#[test]
fn storage_entry_hashes_after_create() {
    let (create_did_op, _, did, _, vdr_sk) = create_did_with_vdr_key();
    let (create_storage_op, create_storage_op_hash) = new_create_storage_op(&did, &vdr_sk, vec![1, 2, 3], vec![0]);

    let operations = test_utils::populate_metadata(vec![create_did_op, create_storage_op]);
    let state = resolver::resolve_published(operations).0.unwrap();

    assert_eq!(state.storage.len(), 1);
    let entry = &state.storage[0];
    // After create, init_operation_hash and last_operation_hash are the same
    assert_eq!(entry.init_operation_hash.deref(), &create_storage_op_hash);
    assert_eq!(entry.last_operation_hash.deref(), &create_storage_op_hash);
    assert_eq!(entry.init_operation_hash, entry.last_operation_hash);
}

/// Test that `last_operation_hash` tracks the latest update in a chain of updates,
/// while `init_operation_hash` remains the hash of the original CreateStorageEntry.
/// This is the hash chain that the cloud-agent uses as `previous_event_hash` for subsequent operations.
#[test]
fn storage_entry_hash_chain_after_sequential_updates() {
    let (create_did_op, _, did, _, vdr_sk) = create_did_with_vdr_key();
    let (create_storage_op, create_storage_op_hash) = new_create_storage_op(&did, &vdr_sk, vec![1, 2, 3], vec![0]);
    let (update_storage_op_1, update_op_hash_1) = test_utils::new_signed_operation(
        VDR_KEY_NAME,
        &vdr_sk,
        proto::prism::prism_operation::Operation::UpdateStorageEntry(proto::prism_storage::ProtoUpdateStorageEntry {
            previous_event_hash: create_storage_op_hash.to_vec(),
            data: Some(proto::prism_storage::proto_update_storage_entry::Data::Bytes(vec![
                4, 5, 6,
            ])),
            special_fields: Default::default(),
        }),
    );
    let (update_storage_op_2, update_op_hash_2) = test_utils::new_signed_operation(
        VDR_KEY_NAME,
        &vdr_sk,
        proto::prism::prism_operation::Operation::UpdateStorageEntry(proto::prism_storage::ProtoUpdateStorageEntry {
            previous_event_hash: update_op_hash_1.to_vec(),
            data: Some(proto::prism_storage::proto_update_storage_entry::Data::Bytes(vec![
                7, 8, 9,
            ])),
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![
        create_did_op,
        create_storage_op,
        update_storage_op_1,
        update_storage_op_2,
    ]);
    let state = resolver::resolve_published(operations).0.unwrap();

    assert_eq!(state.storage.len(), 1);
    let entry = &state.storage[0];
    // init_operation_hash stays constant (= create hash)
    assert_eq!(entry.init_operation_hash.deref(), &create_storage_op_hash);
    // last_operation_hash tracks the latest update
    assert_eq!(entry.last_operation_hash.deref(), &update_op_hash_2);
    // They diverge after update
    assert_ne!(entry.init_operation_hash, entry.last_operation_hash);
    // Data is from the latest update
    assert_eq!(entry.data.deref(), &StorageData::Bytes(vec![7, 8, 9]));
}

/// Test that `init_operation_hash` can be used to look up entries and is a stable hex identifier.
/// This verifies the round-trip that the VDR entry metadata endpoint relies on:
/// hash bytes -> hex string -> lookup by init_operation_hash.
#[test]
fn storage_entry_hash_hex_round_trip() {
    let (create_did_op, _, did, _, vdr_sk) = create_did_with_vdr_key();
    let (create_storage_op, create_storage_op_hash) = new_create_storage_op(&did, &vdr_sk, vec![1, 2, 3], vec![0]);

    let operations = test_utils::populate_metadata(vec![create_did_op, create_storage_op]);
    let state = resolver::resolve_published(operations).0.unwrap();

    let entry = &state.storage[0];
    // Verify hex round-trip for init_operation_hash (used as entry_hash in the API)
    let hex_str = HexStr::from(entry.init_operation_hash.to_vec());
    let round_tripped = Sha256Digest::from_bytes(&hex_str.to_bytes()).unwrap();
    assert_eq!(&round_tripped, entry.init_operation_hash.deref());
    assert_eq!(&round_tripped, &create_storage_op_hash);

    // Verify hex round-trip for last_operation_hash (used as latest_event_hash in the API)
    let hex_str = HexStr::from(entry.last_operation_hash.to_vec());
    let round_tripped = Sha256Digest::from_bytes(&hex_str.to_bytes()).unwrap();
    assert_eq!(&round_tripped, entry.last_operation_hash.deref());
}

/// Test that DID deactivation clears all storage entries from the resolved state.
/// This means the VDR entry metadata endpoint will return 404 for deactivated DIDs,
/// which is the correct behavior since the storage is no longer accessible.
#[test]
fn deactivated_did_has_empty_storage_and_no_public_keys() {
    let (create_did_op, create_did_op_hash, did, master_sk, vdr_sk) = create_did_with_vdr_key();
    let (create_storage_op, _) = new_create_storage_op(&did, &vdr_sk, vec![1, 2, 3], vec![0]);
    let (deactivate_did_op, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        proto::prism::prism_operation::Operation::DeactivateDid(proto::prism_ssi::ProtoDeactivateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, create_storage_op, deactivate_did_op]);
    let state = resolver::resolve_published(operations).0.unwrap();

    // DID is deactivated when all public keys are revoked
    assert!(state.is_deactivated());
    assert!(state.public_keys.is_empty());
    // Storage entries are also revoked when DID is deactivated
    assert!(state.storage.is_empty());
}

/// Test that an active (non-deactivated) DID with storage has public keys and is_deactivated() returns false.
#[test]
fn active_did_with_storage_is_not_deactivated() {
    let (create_did_op, _, did, _, vdr_sk) = create_did_with_vdr_key();
    let (create_storage_op, _) = new_create_storage_op(&did, &vdr_sk, vec![1, 2, 3], vec![0]);

    let operations = test_utils::populate_metadata(vec![create_did_op, create_storage_op]);
    let state = resolver::resolve_published(operations).0.unwrap();

    assert!(!state.is_deactivated());
    assert!(!state.public_keys.is_empty());
    assert_eq!(state.storage.len(), 1);
}

/// Test that deactivating a storage entry (not the DID) removes only the storage entry
/// while the DID remains active with its public keys intact.
#[test]
fn deactivate_storage_entry_keeps_did_active() {
    let (create_did_op, _, did, _, vdr_sk) = create_did_with_vdr_key();
    let (create_storage_op, create_storage_op_hash) = new_create_storage_op(&did, &vdr_sk, vec![1, 2, 3], vec![0]);
    let (deactivate_storage_op, _) = test_utils::new_signed_operation(
        VDR_KEY_NAME,
        &vdr_sk,
        proto::prism::prism_operation::Operation::DeactivateStorageEntry(
            proto::prism_storage::ProtoDeactivateStorageEntry {
                previous_event_hash: create_storage_op_hash.to_vec(),
                special_fields: Default::default(),
            },
        ),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, create_storage_op, deactivate_storage_op]);
    let state = resolver::resolve_published(operations).0.unwrap();

    // Storage entry is removed but DID stays active
    assert!(state.storage.is_empty());
    assert!(!state.is_deactivated());
    assert!(!state.public_keys.is_empty());
}

/// Test that after updating a storage entry, the `last_operation_hash` can be used as
/// `previous_event_hash` for a subsequent update, forming a valid hash chain.
#[test]
fn storage_entry_update_chain_with_deactivation() {
    let (create_did_op, _, did, _, vdr_sk) = create_did_with_vdr_key();
    let (create_storage_op, create_storage_op_hash) = new_create_storage_op(&did, &vdr_sk, vec![1, 2, 3], vec![0]);
    let (update_storage_op, update_op_hash) = test_utils::new_signed_operation(
        VDR_KEY_NAME,
        &vdr_sk,
        proto::prism::prism_operation::Operation::UpdateStorageEntry(proto::prism_storage::ProtoUpdateStorageEntry {
            previous_event_hash: create_storage_op_hash.to_vec(),
            data: Some(proto::prism_storage::proto_update_storage_entry::Data::Bytes(vec![
                4, 5, 6,
            ])),
            special_fields: Default::default(),
        }),
    );
    // Use the update_op_hash as the previous_event_hash for deactivation
    let (deactivate_storage_op, _) = test_utils::new_signed_operation(
        VDR_KEY_NAME,
        &vdr_sk,
        proto::prism::prism_operation::Operation::DeactivateStorageEntry(
            proto::prism_storage::ProtoDeactivateStorageEntry {
                previous_event_hash: update_op_hash.to_vec(),
                special_fields: Default::default(),
            },
        ),
    );

    let operations = test_utils::populate_metadata(vec![
        create_did_op,
        create_storage_op,
        update_storage_op,
        deactivate_storage_op,
    ]);
    let state = resolver::resolve_published(operations).0.unwrap();

    // Storage was successfully deactivated using the update hash chain
    assert!(state.storage.is_empty());
    assert!(!state.is_deactivated()); // DID itself is still active
}

/// Test that multiple storage entries each maintain independent hash chains.
#[test]
fn multiple_storage_entries_independent_hash_chains() {
    let (create_did_op, _, did, _, vdr_sk) = create_did_with_vdr_key();
    let (create_storage_op_1, create_hash_1) = new_create_storage_op(&did, &vdr_sk, vec![1, 2, 3], vec![0]);
    let (create_storage_op_2, create_hash_2) = new_create_storage_op(&did, &vdr_sk, vec![4, 5, 6], vec![1]);
    // Update only the first entry
    let (update_storage_op_1, update_hash_1) = test_utils::new_signed_operation(
        VDR_KEY_NAME,
        &vdr_sk,
        proto::prism::prism_operation::Operation::UpdateStorageEntry(proto::prism_storage::ProtoUpdateStorageEntry {
            previous_event_hash: create_hash_1.to_vec(),
            data: Some(proto::prism_storage::proto_update_storage_entry::Data::Bytes(vec![
                10, 20, 30,
            ])),
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![
        create_did_op,
        create_storage_op_1,
        create_storage_op_2,
        update_storage_op_1,
    ]);
    let state = resolver::resolve_published(operations).0.unwrap();

    assert_eq!(state.storage.len(), 2);

    // Entry 1: updated, so last_operation_hash diverged from init_operation_hash
    let entry_1 = state
        .storage
        .iter()
        .find(|s| s.init_operation_hash.deref() == &create_hash_1)
        .unwrap();
    assert_eq!(entry_1.last_operation_hash.deref(), &update_hash_1);
    assert_ne!(entry_1.init_operation_hash, entry_1.last_operation_hash);
    assert_eq!(entry_1.data.deref(), &StorageData::Bytes(vec![10, 20, 30]));

    // Entry 2: not updated, so last_operation_hash == init_operation_hash
    let entry_2 = state
        .storage
        .iter()
        .find(|s| s.init_operation_hash.deref() == &create_hash_2)
        .unwrap();
    assert_eq!(entry_2.init_operation_hash, entry_2.last_operation_hash);
    assert_eq!(entry_2.data.deref(), &StorageData::Bytes(vec![4, 5, 6]));
}

/// Test that creating a new storage entry fails after DID deactivation.
/// DID deactivation revokes all keys (including VDR keys), so check_signature()
/// rejects the operation with SignedPrismOperationSignedWithRevokedKey.
#[test]
fn create_storage_entry_after_did_deactivation() {
    let (create_did_op, create_did_op_hash, did, master_sk, vdr_sk) = create_did_with_vdr_key();
    let (deactivate_did_op, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        proto::prism::prism_operation::Operation::DeactivateDid(proto::prism_ssi::ProtoDeactivateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            special_fields: Default::default(),
        }),
    );
    // Attempt to create a storage entry after DID is deactivated
    let (create_storage_op, _) = new_create_storage_op(&did, &vdr_sk, vec![1, 2, 3], vec![0]);

    let operations = test_utils::populate_metadata(vec![create_did_op, deactivate_did_op, create_storage_op]);
    let state = resolver::resolve_published(operations).0.unwrap();

    // DID is deactivated and no storage entry was created
    assert!(state.is_deactivated());
    assert!(state.storage.is_empty());
}

/// Test that updating a storage entry fails after DID deactivation.
/// The VDR key used to sign the update was revoked during DID deactivation.
#[test]
fn update_storage_entry_after_did_deactivation() {
    let (create_did_op, create_did_op_hash, did, master_sk, vdr_sk) = create_did_with_vdr_key();
    let (create_storage_op, create_storage_op_hash) = new_create_storage_op(&did, &vdr_sk, vec![1, 2, 3], vec![0]);
    let (deactivate_did_op, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        proto::prism::prism_operation::Operation::DeactivateDid(proto::prism_ssi::ProtoDeactivateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            special_fields: Default::default(),
        }),
    );
    // Attempt to update the storage entry after DID is deactivated
    let (update_storage_op, _) = test_utils::new_signed_operation(
        VDR_KEY_NAME,
        &vdr_sk,
        proto::prism::prism_operation::Operation::UpdateStorageEntry(proto::prism_storage::ProtoUpdateStorageEntry {
            previous_event_hash: create_storage_op_hash.to_vec(),
            data: Some(proto::prism_storage::proto_update_storage_entry::Data::Bytes(vec![
                4, 5, 6,
            ])),
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![
        create_did_op,
        create_storage_op,
        deactivate_did_op,
        update_storage_op,
    ]);
    let state = resolver::resolve_published(operations).0.unwrap();

    // DID is deactivated and storage was revoked (not updated)
    assert!(state.is_deactivated());
    assert!(state.storage.is_empty());
}

/// Test that deactivating a storage entry fails after DID deactivation.
/// Even though the storage was already revoked by DID deactivation, the explicit
/// deactivate operation is also rejected because the signing key is revoked.
#[test]
fn deactivate_storage_entry_after_did_deactivation() {
    let (create_did_op, create_did_op_hash, did, master_sk, vdr_sk) = create_did_with_vdr_key();
    let (create_storage_op, create_storage_op_hash) = new_create_storage_op(&did, &vdr_sk, vec![1, 2, 3], vec![0]);
    let (deactivate_did_op, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        proto::prism::prism_operation::Operation::DeactivateDid(proto::prism_ssi::ProtoDeactivateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            special_fields: Default::default(),
        }),
    );
    // Attempt to deactivate the storage entry after DID is already deactivated
    let (deactivate_storage_op, _) = test_utils::new_signed_operation(
        VDR_KEY_NAME,
        &vdr_sk,
        proto::prism::prism_operation::Operation::DeactivateStorageEntry(
            proto::prism_storage::ProtoDeactivateStorageEntry {
                previous_event_hash: create_storage_op_hash.to_vec(),
                special_fields: Default::default(),
            },
        ),
    );

    let operations = test_utils::populate_metadata(vec![
        create_did_op,
        create_storage_op,
        deactivate_did_op,
        deactivate_storage_op,
    ]);
    let state = resolver::resolve_published(operations).0.unwrap();

    // DID is deactivated and storage was already revoked by DID deactivation
    assert!(state.is_deactivated());
    assert!(state.storage.is_empty());
}

/// Issue #227: Storage operations should NOT update the DID's SSI chain `prev_operation_hash`.
/// DeactivateDID must reference the last SSI event (CreateDID or UpdateDID), not storage ops.
/// This test verifies that CreateDID → CreateStorage → DeactivateDID(prev=CreateDID_hash) succeeds.
#[test]
fn deactivate_did_after_storage_op_references_ssi_chain() {
    let (create_did_op, create_did_op_hash, did, master_sk, vdr_sk) = create_did_with_vdr_key();
    let (create_storage_op, _create_storage_op_hash) = new_create_storage_op(&did, &vdr_sk, vec![1, 2, 3], vec![0]);
    // DeactivateDID references the CreateDID hash (last SSI op), NOT the CreateStorage hash
    let (deactivate_did_op, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        proto::prism::prism_operation::Operation::DeactivateDid(proto::prism_ssi::ProtoDeactivateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, create_storage_op, deactivate_did_op]);
    let state = resolver::resolve_published(operations).0.unwrap();

    assert!(state.is_deactivated());
    assert!(state.storage.is_empty());
}

/// Issue #227: UpdateDID after a storage operation must reference the last SSI event.
/// CreateDID → CreateStorage → UpdateDID(prev=CreateDID_hash) should succeed.
#[test]
fn update_did_after_storage_op_references_ssi_chain() {
    let (create_did_op, create_did_op_hash, did, master_sk, vdr_sk) = create_did_with_vdr_key();
    let (create_storage_op, _) = new_create_storage_op(&did, &vdr_sk, vec![1, 2, 3], vec![0]);
    // UpdateDID references CreateDID hash (last SSI op), not any storage hash.
    // AddService is used as the action because UpdateDID requires at least one action to be valid.
    let (update_did_op, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        proto::prism::prism_operation::Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![test_utils::add_service_action("service-0")],
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![create_did_op, create_storage_op, update_did_op]);
    let state = resolver::resolve_published(operations).0.unwrap();

    assert!(!state.is_deactivated());
    assert_eq!(state.storage.len(), 1);
    assert!(!state.services.is_empty());
}

/// Issue #227: Full interleaved SSI and storage chain.
/// CreateDID → CreateStorage → UpdateStorage → UpdateDID(prev=CreateDID_hash) → DeactivateDID(prev=UpdateDID_hash)
/// Verifies SSI chain and storage chains are completely independent.
#[test]
fn interleaved_ssi_and_storage_chains_are_independent() {
    let (create_did_op, create_did_op_hash, did, master_sk, vdr_sk) = create_did_with_vdr_key();
    let (create_storage_op, create_storage_op_hash) = new_create_storage_op(&did, &vdr_sk, vec![1, 2, 3], vec![0]);
    let (update_storage_op, _) = test_utils::new_signed_operation(
        VDR_KEY_NAME,
        &vdr_sk,
        proto::prism::prism_operation::Operation::UpdateStorageEntry(proto::prism_storage::ProtoUpdateStorageEntry {
            previous_event_hash: create_storage_op_hash.to_vec(),
            data: Some(proto::prism_storage::proto_update_storage_entry::Data::Bytes(vec![
                4, 5, 6,
            ])),
            special_fields: Default::default(),
        }),
    );
    // UpdateDID references CreateDID hash (last SSI op), not any storage hash
    let (update_did_op, update_did_op_hash) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        proto::prism::prism_operation::Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
            previous_operation_hash: create_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            actions: vec![test_utils::add_service_action("service-0")],
            special_fields: Default::default(),
        }),
    );
    // DeactivateDID references UpdateDID hash (last SSI op), not any storage hash
    let (deactivate_did_op, _) = test_utils::new_signed_operation(
        "master-0",
        &master_sk,
        proto::prism::prism_operation::Operation::DeactivateDid(proto::prism_ssi::ProtoDeactivateDID {
            previous_operation_hash: update_did_op_hash.to_vec(),
            id: did.suffix_hex().to_string(),
            special_fields: Default::default(),
        }),
    );

    let operations = test_utils::populate_metadata(vec![
        create_did_op,
        create_storage_op,
        update_storage_op,
        update_did_op,
        deactivate_did_op,
    ]);
    let state = resolver::resolve_published(operations).0.unwrap();

    assert!(state.is_deactivated());
    assert!(state.storage.is_empty());
    assert!(state.public_keys.is_empty());
}

fn create_did_with_vdr_key() -> (
    proto::prism::SignedPrismOperation,
    Sha256Digest,
    CanonicalPrismDid,
    Secp256k1PrivateKey,
    Secp256k1PrivateKey,
) {
    let vdr_sk = Secp256k1PrivateKey::from_slice(&VDR_KEY).unwrap();
    let options = test_utils::CreateDidOptions {
        public_keys: Some(vec![test_utils::new_public_key(
            VDR_KEY_NAME,
            proto::prism_ssi::KeyUsage::VDR_KEY,
            &vdr_sk,
        )]),
        ..Default::default()
    };
    let (signed_operation, operation_hash, master_sk) = test_utils::new_create_did_operation(Some(options));
    let did = CanonicalPrismDid::from_operation(signed_operation.operation.as_ref().unwrap()).unwrap();
    (signed_operation, operation_hash, did, master_sk, vdr_sk)
}

/// Helper to create a signed CreateStorageEntry operation with the given data and nonce.
fn new_create_storage_op(
    did: &CanonicalPrismDid,
    vdr_sk: &Secp256k1PrivateKey,
    data: Vec<u8>,
    nonce: Vec<u8>,
) -> (proto::prism::SignedPrismOperation, Sha256Digest) {
    test_utils::new_signed_operation(
        VDR_KEY_NAME,
        vdr_sk,
        proto::prism::prism_operation::Operation::CreateStorageEntry(proto::prism_storage::ProtoCreateStorageEntry {
            did_prism_hash: did.suffix.to_vec(),
            nonce,
            data: Some(proto::prism_storage::proto_create_storage_entry::Data::Bytes(data)),
            special_fields: Default::default(),
        }),
    )
}
