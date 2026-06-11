use std::rc::Rc;
use std::str::FromStr;

use chrono::Utc;
use identus_apollo::base64::Base64UrlStrNoPad;
// Secp256k1PrivateKey is not needed in this test file
use identus_apollo::hash::Sha256Digest;
use identus_apollo::hex::HexStr;
use identus_did_prism::did::error::{DidSyntaxError, Error as DidError};
use identus_did_prism::did::operation::StorageData;
use identus_did_prism::did::{CanonicalPrismDid, DidState, LongFormPrismDid, PrismDid, PrismDidOps, StorageState};
use identus_did_prism::proto;
use identus_did_prism::proto::MessageExt;

mod test_utils;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn new_create_operation() -> proto::prism::PrismOperation {
    let (signed_op, _, _) = test_utils::new_create_did_operation(None);
    signed_op.operation.unwrap()
}

fn new_update_operation() -> proto::prism::PrismOperation {
    let inner = proto::prism::prism_operation::Operation::UpdateDid(proto::prism_ssi::ProtoUpdateDID {
        id: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        previous_operation_hash: vec![0; 32],
        actions: vec![],
        special_fields: Default::default(),
    });
    proto::prism::PrismOperation {
        operation: Some(inner),
        special_fields: Default::default(),
    }
}

fn zeros_hash() -> Sha256Digest {
    Sha256Digest::from_bytes(&[0u8; 32]).unwrap()
}

fn zeros_suffix_str() -> String {
    "0000000000000000000000000000000000000000000000000000000000000000".to_string()
}

// ---------------------------------------------------------------------------
// CanonicalPrismDid
// ---------------------------------------------------------------------------

#[test]
fn canonical_from_suffix_str_valid() {
    let did = CanonicalPrismDid::from_suffix_str(&zeros_suffix_str()).unwrap();
    assert_eq!(did.suffix_hex().to_string(), zeros_suffix_str());
}

#[test]
fn canonical_from_suffix_str_invalid_hex() {
    let result = CanonicalPrismDid::from_suffix_str("not_hex");
    let err = result.unwrap_err();
    assert!(
        matches!(err, DidSyntaxError::DidSuffixInvalidStr { .. }),
        "expected DidSuffixInvalidStr, got: {err:?}"
    );
}

#[test]
fn canonical_from_suffix_str_wrong_length() {
    let result = CanonicalPrismDid::from_suffix_str("abcd");
    assert!(result.is_err());
}

#[test]
fn canonical_from_suffix_valid() {
    let hex = HexStr::from_str(&zeros_suffix_str()).unwrap();
    let did = CanonicalPrismDid::from_suffix(hex).unwrap();
    assert_eq!(did.suffix_hex().to_string(), zeros_suffix_str());
}

#[test]
fn canonical_from_suffix_wrong_length() {
    let hex = HexStr::from_str("abcd").unwrap();
    let result = CanonicalPrismDid::from_suffix(hex);
    assert!(result.is_err());
}

#[test]
fn canonical_from_operation_success() {
    let operation = new_create_operation();
    let did = CanonicalPrismDid::from_operation(&operation).unwrap();
    // The suffix should match the operation hash
    assert_eq!(did.suffix(), &operation.operation_hash());
}

#[test]
fn canonical_from_operation_not_create() {
    let operation = new_update_operation();
    let result = CanonicalPrismDid::from_operation(&operation);
    assert!(result.is_err());
}

#[test]
fn canonical_display_format() {
    let did = CanonicalPrismDid::from_suffix_str(&zeros_suffix_str()).unwrap();
    assert_eq!(did.to_string(), format!("did:prism:{}", zeros_suffix_str()));
}

#[test]
fn canonical_suffix_method() {
    let did = CanonicalPrismDid::from_suffix_str(&zeros_suffix_str()).unwrap();
    assert_eq!(did.method(), "prism");
    assert_eq!(did.suffix().as_bytes().len(), 32);
}

#[test]
fn canonical_into_canonical_is_identity() {
    let did = CanonicalPrismDid::from_suffix_str(&zeros_suffix_str()).unwrap();
    let canonical = did.clone().into_canonical();
    assert_eq!(did, canonical);
}

#[test]
fn canonical_to_did() {
    let did = CanonicalPrismDid::from_suffix_str(&zeros_suffix_str()).unwrap();
    let core_did = did.to_did();
    assert_eq!(core_did.to_string(), format!("did:prism:{}", zeros_suffix_str()));
}

// ---------------------------------------------------------------------------
// LongFormPrismDid
// ---------------------------------------------------------------------------

#[test]
fn long_form_from_operation_success() {
    let operation = new_create_operation();
    let did = LongFormPrismDid::from_operation(&operation).unwrap();
    assert_eq!(did.suffix(), &operation.operation_hash());
    // The encoded state should be the base64url encoding of the operation bytes
    let expected_encoded = Base64UrlStrNoPad::from(operation.encode_to_vec());
    assert_eq!(did.encoded_state, expected_encoded);
}

#[test]
fn long_form_from_operation_none_operation() {
    let operation = proto::prism::PrismOperation {
        operation: None,
        special_fields: Default::default(),
    };
    let result = LongFormPrismDid::from_operation(&operation);
    assert!(result.is_err());
    match result.unwrap_err() {
        DidError::OperationMissingFromPrismOperation => {}
        other => panic!("expected OperationMissingFromPrismOperation, got: {other}"),
    }
}

#[test]
fn long_form_from_operation_update_rejected() {
    let operation = new_update_operation();
    let result = LongFormPrismDid::from_operation(&operation);
    assert!(result.is_err());
    match result.unwrap_err() {
        DidError::LongFormDidNotFromCreateOperation => {}
        other => panic!("expected LongFormDidNotFromCreateOperation, got: {other}"),
    }
}

#[test]
fn long_form_operation_roundtrip() {
    let operation = new_create_operation();
    let did = LongFormPrismDid::from_operation(&operation).unwrap();
    let decoded = did.operation().unwrap();
    assert_eq!(decoded.encode_to_vec(), operation.encode_to_vec());
}

#[test]
fn long_form_into_canonical() {
    let operation = new_create_operation();
    let did = LongFormPrismDid::from_operation(&operation).unwrap();
    let canonical = did.clone().into_canonical();
    assert_eq!(canonical.suffix(), did.suffix());
}

#[test]
fn long_form_display_format() {
    let operation = new_create_operation();
    let did = LongFormPrismDid::from_operation(&operation).unwrap();
    let display = did.to_string();
    assert!(display.starts_with("did:prism:"));
    assert!(display.contains(":")); // has the extra colon for encoded state
}

#[test]
fn long_form_suffix_method() {
    let operation = new_create_operation();
    let did = LongFormPrismDid::from_operation(&operation).unwrap();
    assert_eq!(did.method(), "prism");
}

// ---------------------------------------------------------------------------
// PrismDid::from_str — Canonical
// ---------------------------------------------------------------------------

#[test]
fn prism_did_from_str_canonical_valid() {
    let did = PrismDid::from_str(&format!("did:prism:{}", zeros_suffix_str())).unwrap();
    assert!(matches!(did, PrismDid::Canonical(_)));
    assert_eq!(did.to_string(), format!("did:prism:{}", zeros_suffix_str()));
}

#[test]
fn prism_did_from_str_wrong_prefix() {
    let result = PrismDid::from_str("did:web:example.com");
    assert!(result.is_err());
    match result.unwrap_err() {
        DidError::InvalidDidSyntax { source } => {
            let msg = source.to_string();
            assert!(msg.contains("unrecognized did pattern"), "got: {msg}");
        }
        other => panic!("expected InvalidDidSyntax, got: {other}"),
    }
}

#[test]
fn prism_did_from_str_no_prefix() {
    let result = PrismDid::from_str("notadid");
    assert!(result.is_err());
}

#[test]
fn prism_did_from_str_canonical_invalid_hex_suffix() {
    let result = PrismDid::from_str("did:prism:ZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ");
    assert!(result.is_err());
}

#[test]
fn prism_did_from_str_canonical_short_suffix() {
    let result = PrismDid::from_str("did:prism:abcd1234");
    assert!(result.is_err());
}

#[test]
fn prism_did_from_str_unrecognized_pattern() {
    // Has colon but not matching canonical or long-form patterns
    let result = PrismDid::from_str("did:prism:1234:extra:stuff");
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// PrismDid::from_str — Long-form
// ---------------------------------------------------------------------------

#[test]
fn prism_did_from_str_long_form_valid() {
    let operation = new_create_operation();
    let long_form = LongFormPrismDid::from_operation(&operation).unwrap();
    let long_form_str = long_form.to_string();

    let did = PrismDid::from_str(&long_form_str).unwrap();
    assert!(matches!(did, PrismDid::LongForm(_)));
}

#[test]
fn prism_did_from_str_long_form_suffix_mismatch() {
    // Create a valid long-form DID, but alter the suffix
    let operation = new_create_operation();
    let long_form = LongFormPrismDid::from_operation(&operation).unwrap();
    let long_form_str = long_form.to_string();

    // Change the first character of the suffix (hex) to create a mismatch
    let prefix = "did:prism:";
    let after_prefix = &long_form_str[prefix.len()..];
    let colon_pos = after_prefix.find(':').unwrap();
    let suffix_part = &after_prefix[..colon_pos];
    let state_part = &after_prefix[colon_pos + 1..];

    // Flip one hex digit
    let mut flipped_suffix = suffix_part.to_string();
    let first_char = flipped_suffix.chars().next().unwrap();
    let new_char = if first_char == '0' { '1' } else { '0' };
    flipped_suffix.replace_range(..1, &new_char.to_string());

    let tampered = format!("{prefix}{flipped_suffix}:{state_part}");
    let result = PrismDid::from_str(&tampered);
    match result.unwrap_err() {
        DidError::InvalidDidSyntax {
            source: DidSyntaxError::DidSuffixEncodedStateUnmatched { did, expected_did },
        } => {
            assert_eq!(did, tampered);
            assert_eq!(expected_did, long_form.into_canonical());
        }
        other => panic!("expected DidSuffixEncodedStateUnmatched, got: {other}"),
    }
}

#[test]
fn prism_did_from_str_long_form_invalid_base64() {
    // Valid hex suffix but invalid base64url encoded state
    let result = PrismDid::from_str(&format!("did:prism:{}:!not_valid_base64!!!", zeros_suffix_str()));
    assert!(result.is_err());
}

#[test]
fn prism_did_from_str_long_form_invalid_protobuf() {
    // Valid hex suffix + valid base64url but not a valid protobuf
    let garbage = Base64UrlStrNoPad::from(vec![0xDE, 0xAD, 0xBE, 0xEF]);
    let result = PrismDid::from_str(&format!("did:prism:{}:{}", zeros_suffix_str(), garbage));
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// PrismDid enum dispatch
// ---------------------------------------------------------------------------

#[test]
fn prism_did_display_matches_inner() {
    let canonical = CanonicalPrismDid::from_suffix_str(&zeros_suffix_str()).unwrap();
    let did: PrismDid = canonical.into();
    assert_eq!(did.to_string(), format!("did:prism:{}", zeros_suffix_str()));
}

#[test]
fn prism_did_clone_equality() {
    let did1 = PrismDid::from_str(&format!("did:prism:{}", zeros_suffix_str())).unwrap();
    let did2 = did1.clone();
    assert_eq!(did1, did2);
}

#[test]
fn prism_did_suffix_hex() {
    let did = PrismDid::from_str(&format!("did:prism:{}", zeros_suffix_str())).unwrap();
    assert_eq!(did.suffix_hex().to_string(), zeros_suffix_str());
}

// ---------------------------------------------------------------------------
// DidState
// ---------------------------------------------------------------------------

#[test]
fn did_state_is_deactivated_true_when_no_public_keys() {
    let did = CanonicalPrismDid::from_suffix_str(&zeros_suffix_str()).unwrap();
    let state = DidState {
        did,
        context: vec![],
        last_operation_hash: Rc::new(zeros_hash()),
        public_keys: vec![],
        services: vec![],
        storage: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        is_published: true,
    };
    assert!(state.is_deactivated());
}

#[test]
fn did_state_is_deactivated_false_when_has_public_keys() {
    // Use the protocol resolver to get a real DidState with public keys
    let (signed_op, _, _) = test_utils::new_create_did_operation(None);
    let operations = test_utils::populate_metadata(vec![signed_op]);
    let state = identus_did_prism::protocol::resolver::resolve_published(operations)
        .0
        .unwrap();
    assert!(!state.is_deactivated());
}

#[test]
fn did_state_into_did_data_conversion() {
    let did = CanonicalPrismDid::from_suffix_str(&zeros_suffix_str()).unwrap();
    let state = DidState {
        did: did.clone(),
        context: vec!["https://www.w3.org/ns/did/v1".to_string()],
        last_operation_hash: Rc::new(zeros_hash()),
        public_keys: vec![],
        services: vec![],
        storage: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        is_published: true,
    };
    let data: proto::node_api::DIDData = state.into();
    assert_eq!(data.id, zeros_suffix_str());
    assert_eq!(data.context, vec!["https://www.w3.org/ns/did/v1"]);
}

#[test]
fn did_state_into_did_data_strips_did_prefix() {
    let suffix = zeros_suffix_str();
    let did = CanonicalPrismDid::from_suffix_str(&suffix).unwrap();
    let state = DidState {
        did,
        context: vec![],
        last_operation_hash: Rc::new(zeros_hash()),
        public_keys: vec![],
        services: vec![],
        storage: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        is_published: false,
    };
    let data: proto::node_api::DIDData = state.into();
    // The id field should be just the suffix, not the full DID string
    assert_eq!(data.id, suffix);
    assert!(!data.id.contains("did:prism:"));
}

// ---------------------------------------------------------------------------
// StorageState → node_api::StorageData
// ---------------------------------------------------------------------------

#[test]
fn storage_state_bytes_conversion_success() {
    let state = StorageState {
        init_operation_hash: Rc::new(zeros_hash()),
        last_operation_hash: Rc::new(zeros_hash()),
        data: Rc::new(StorageData::Bytes(vec![1, 2, 3, 4])),
    };
    let result: Result<proto::node_api::StorageData, _> = state.try_into();
    let storage_data = result.unwrap();
    assert_eq!(storage_data.init_operation_hash, vec![0u8; 32]);
    assert_eq!(storage_data.prev_operation_hash, vec![0u8; 32]);
    assert!(matches!(
        storage_data.data,
        Some(proto::node_api::storage_data::Data::Bytes(ref b)) if b == &vec![1, 2, 3, 4]
    ));
}

#[test]
fn storage_state_ipfs_conversion_fails() {
    let state = StorageState {
        init_operation_hash: Rc::new(zeros_hash()),
        last_operation_hash: Rc::new(zeros_hash()),
        data: Rc::new(StorageData::Ipfs("QmExample".to_string())),
    };
    let result: Result<proto::node_api::StorageData, _> = state.try_into();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "only bytes is supported in DIDData");
}

#[test]
fn storage_state_status_list_conversion_fails() {
    let state = StorageState {
        init_operation_hash: Rc::new(zeros_hash()),
        last_operation_hash: Rc::new(zeros_hash()),
        data: Rc::new(StorageData::StatusList(
            identus_did_prism::did::operation::StatusListData {
                state: 1,
                name: "test".to_string(),
                detail: "detail".to_string(),
            },
        )),
    };
    let result: Result<proto::node_api::StorageData, _> = state.try_into();
    assert!(result.is_err());
}

#[test]
fn storage_state_clone_equality() {
    let state = StorageState {
        init_operation_hash: Rc::new(zeros_hash()),
        last_operation_hash: Rc::new(zeros_hash()),
        data: Rc::new(StorageData::Bytes(vec![42])),
    };
    let state2 = state.clone();
    assert_eq!(state, state2);
}

// ---------------------------------------------------------------------------
// LongFormPrismDid::operation() error paths
// ---------------------------------------------------------------------------

#[test]
fn long_form_operation_decode_failure_with_invalid_encoded_state() {
    // Construct a LongFormPrismDid with garbage encoded state that is not valid protobuf
    let suffix = Sha256Digest::from_bytes(&[0u8; 32]).unwrap();
    let garbage_state = Base64UrlStrNoPad::from(vec![0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE]);
    let did = LongFormPrismDid {
        suffix,
        encoded_state: garbage_state,
    };
    let result = did.operation();
    assert!(result.is_err());
    match result.unwrap_err() {
        DidError::InvalidDidSyntax { source } => {
            let msg = source.to_string();
            assert!(
                msg.contains("failed to decode did suffix"),
                "expected decode error message, got: {msg}"
            );
        }
        other => panic!("expected InvalidDidSyntax, got: {other}"),
    }
}

#[test]
fn prism_did_from_str_long_form_with_invalid_base64_no_pad_length() {
    // Use an encoded state that matches regex [A-Za-z0-9_-]+ but is invalid for
    // base64url-no-pad decoding (length 1 is invalid: ≡ 1 mod 4)
    let result = PrismDid::from_str(&format!("did:prism:{}:A", zeros_suffix_str()));
    assert!(result.is_err());
    match result.unwrap_err() {
        DidError::InvalidDidSyntax { source } => {
            let msg = source.to_string();
            assert!(
                msg.contains("encoded state"),
                "expected encoded state error message, got: {msg}"
            );
        }
        other => panic!("expected InvalidDidSyntax, got: {other}"),
    }
}

#[test]
fn prism_did_from_str_long_form_with_non_create_operation() {
    // Encode an UpdateDid operation as the long-form state, with a valid base64url
    // encoding. This should pass protobuf decode but fail at LongFormPrismDid::from_operation
    // because it's not a CreateDid operation.
    let update_operation = new_update_operation();
    let encoded_state = Base64UrlStrNoPad::from(update_operation.encode_to_vec());
    let suffix_hex = zeros_suffix_str();
    let did_str = format!("did:prism:{}:{}", suffix_hex, encoded_state);
    let result = PrismDid::from_str(&did_str);
    assert!(result.is_err());
}

#[test]
fn did_state_into_did_data_filters_non_bytes_storage() {
    // DidState with both Bytes and Ipfs storage should only include Bytes in DIDData
    let did = CanonicalPrismDid::from_suffix_str(&zeros_suffix_str()).unwrap();
    let bytes_storage = StorageState {
        init_operation_hash: Rc::new(zeros_hash()),
        last_operation_hash: Rc::new(zeros_hash()),
        data: Rc::new(StorageData::Bytes(vec![1, 2, 3])),
    };
    let ipfs_storage = StorageState {
        init_operation_hash: Rc::new(zeros_hash()),
        last_operation_hash: Rc::new(zeros_hash()),
        data: Rc::new(StorageData::Ipfs("QmExample".to_string())),
    };
    let state = DidState {
        did: did.clone(),
        context: vec![],
        last_operation_hash: Rc::new(zeros_hash()),
        public_keys: vec![],
        services: vec![],
        storage: vec![bytes_storage, ipfs_storage],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        is_published: true,
    };
    let data: proto::node_api::DIDData = state.into();
    // Only the Bytes storage entry should survive the conversion
    assert_eq!(data.storage_data.len(), 1);
    assert!(matches!(
        data.storage_data[0].data,
        Some(proto::node_api::storage_data::Data::Bytes(ref b)) if b == &vec![1, 2, 3]
    ));
}

// ---------------------------------------------------------------------------
// PrismDidOps trait via enum dispatch
// ---------------------------------------------------------------------------

#[test]
fn prism_did_ops_suffix_on_canonical() {
    let did = CanonicalPrismDid::from_suffix_str(&zeros_suffix_str()).unwrap();
    assert_eq!(did.suffix().as_bytes().len(), 32);
}

#[test]
fn prism_did_ops_suffix_on_long_form() {
    let operation = new_create_operation();
    let did = LongFormPrismDid::from_operation(&operation).unwrap();
    assert_eq!(did.suffix().as_bytes().len(), 32);
}

#[test]
fn prism_did_ops_method_on_both_variants() {
    let canonical = CanonicalPrismDid::from_suffix_str(&zeros_suffix_str()).unwrap();
    assert_eq!(canonical.method(), "prism");

    let operation = new_create_operation();
    let long_form = LongFormPrismDid::from_operation(&operation).unwrap();
    assert_eq!(long_form.method(), "prism");
}
