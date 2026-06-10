use std::cmp::Ordering;
use std::str::FromStr;

use chrono::DateTime;
use identus_apollo::hash::sha256;
use identus_did_prism::dlt::{
    BlockMetadata, BlockNo, DltCursor, NetworkIdentifier, OperationMetadata, PublishedPrismObject, SlotNo, TxId,
};
use identus_did_prism::proto::prism::PrismObject;

// ── DltCursor ──────────────────────────────────────────────────────────────

#[test]
fn dlt_cursor_construction_with_all_fields() {
    let block_hash = vec![1u8, 2, 3];
    let cbt = DateTime::from_timestamp(1_700_000_000, 0).unwrap();
    let cursor = DltCursor {
        slot: 100,
        block_hash: block_hash.clone(),
        cbt: Some(cbt),
        blockfrost_page: Some(3),
    };
    assert_eq!(cursor.slot, 100);
    assert_eq!(cursor.block_hash, block_hash);
    assert_eq!(cursor.cbt, Some(cbt));
    assert_eq!(cursor.blockfrost_page, Some(3));
}

#[test]
fn dlt_cursor_equality() {
    let a = DltCursor {
        slot: 42,
        block_hash: vec![0xAA],
        cbt: None,
        blockfrost_page: None,
    };
    let b = DltCursor {
        slot: 42,
        block_hash: vec![0xAA],
        cbt: None,
        blockfrost_page: None,
    };
    assert_eq!(a, b);
}

#[test]
fn dlt_cursor_inequality_different_slot() {
    let a = DltCursor {
        slot: 1,
        block_hash: vec![],
        cbt: None,
        blockfrost_page: None,
    };
    let b = DltCursor {
        slot: 2,
        block_hash: vec![],
        cbt: None,
        blockfrost_page: None,
    };
    assert_ne!(a, b);
}

#[test]
fn dlt_cursor_clone() {
    let cursor = DltCursor {
        slot: 10,
        block_hash: vec![99],
        cbt: None,
        blockfrost_page: Some(1),
    };
    let cloned = cursor.clone();
    assert_eq!(cursor, cloned);
}

// ── BlockMetadata ──────────────────────────────────────────────────────────

fn sample_tx_id() -> TxId {
    TxId::from(sha256([0u8; 32]))
}

fn sample_block_metadata(block_number: u64, absn: u32) -> BlockMetadata {
    BlockMetadata {
        slot_number: SlotNo::from(1000),
        block_number: BlockNo::from(block_number),
        cbt: DateTime::UNIX_EPOCH,
        tx_id: sample_tx_id(),
        absn,
    }
}

#[test]
fn block_metadata_construction_and_access() {
    let bm = sample_block_metadata(50, 2);
    assert_eq!(bm.slot_number.inner(), 1000);
    assert_eq!(bm.block_number.inner(), 50);
    assert_eq!(bm.absn, 2);
}

#[test]
fn block_metadata_equality() {
    let a = sample_block_metadata(10, 0);
    let b = sample_block_metadata(10, 0);
    assert_eq!(a, b);
}

#[test]
fn block_metadata_inequality() {
    let a = sample_block_metadata(10, 0);
    let b = sample_block_metadata(20, 0);
    assert_ne!(a, b);
}

#[test]
fn block_metadata_clone() {
    let bm = sample_block_metadata(99, 5);
    let cloned = bm.clone();
    assert_eq!(bm, cloned);
}

// ── OperationMetadata ──────────────────────────────────────────────────────

fn sample_op_metadata(block_number: u64, absn: u32, osn: u32) -> OperationMetadata {
    OperationMetadata {
        block_metadata: sample_block_metadata(block_number, absn),
        osn,
    }
}

#[test]
fn operation_metadata_compare_time_asc_same_block() {
    let a = sample_op_metadata(10, 0, 1);
    let b = sample_op_metadata(10, 0, 2);
    assert_eq!(OperationMetadata::compare_time_asc(&a, &b), Ordering::Less);
    assert_eq!(OperationMetadata::compare_time_asc(&b, &a), Ordering::Greater);
}

#[test]
fn operation_metadata_compare_time_asc_different_absn() {
    let a = sample_op_metadata(10, 0, 0);
    let b = sample_op_metadata(10, 1, 0);
    assert_eq!(OperationMetadata::compare_time_asc(&a, &b), Ordering::Less);
}

#[test]
fn operation_metadata_compare_time_asc_different_block() {
    let a = sample_op_metadata(5, 10, 10);
    let b = sample_op_metadata(6, 0, 0);
    assert_eq!(OperationMetadata::compare_time_asc(&a, &b), Ordering::Less);
}

#[test]
fn operation_metadata_compare_time_asc_equal() {
    let a = sample_op_metadata(10, 5, 3);
    let b = sample_op_metadata(10, 5, 3);
    assert_eq!(OperationMetadata::compare_time_asc(&a, &b), Ordering::Equal);
}

#[test]
fn operation_metadata_compare_time_desc_is_reverse_of_asc() {
    let a = sample_op_metadata(10, 0, 1);
    let b = sample_op_metadata(10, 0, 2);
    assert_eq!(OperationMetadata::compare_time_desc(&a, &b), Ordering::Greater);
    assert_eq!(OperationMetadata::compare_time_desc(&b, &a), Ordering::Less);
}

#[test]
fn operation_metadata_compare_time_desc_equal() {
    let a = sample_op_metadata(10, 5, 3);
    let b = sample_op_metadata(10, 5, 3);
    assert_eq!(OperationMetadata::compare_time_desc(&a, &b), Ordering::Equal);
}

#[test]
fn operation_metadata_equality() {
    let a = sample_op_metadata(10, 0, 0);
    let b = sample_op_metadata(10, 0, 0);
    assert_eq!(a, b);
}

#[test]
fn operation_metadata_clone() {
    let om = sample_op_metadata(10, 3, 7);
    let cloned = om.clone();
    assert_eq!(om, cloned);
}

// ── PublishedPrismObject ───────────────────────────────────────────────────

#[test]
fn published_prism_object_construction() {
    let bm = sample_block_metadata(10, 0);
    let prism_object = PrismObject::default();
    let published = PublishedPrismObject {
        block_metadata: bm.clone(),
        prism_object: prism_object.clone(),
    };
    assert_eq!(published.block_metadata, bm);
    assert_eq!(published.prism_object, prism_object);
}

#[test]
fn published_prism_object_clone() {
    let published = PublishedPrismObject {
        block_metadata: sample_block_metadata(1, 0),
        prism_object: PrismObject::default(),
    };
    let cloned = published.clone();
    assert_eq!(published.block_metadata, cloned.block_metadata);
}

// ── SlotNo ─────────────────────────────────────────────────────────────────

#[test]
fn slot_no_from_u64_and_inner() {
    let slot = SlotNo::from(42u64);
    assert_eq!(slot.inner(), 42);
}

#[test]
fn slot_no_into_u64() {
    let slot = SlotNo::from(123u64);
    let val: u64 = slot.into();
    assert_eq!(val, 123);
}

#[test]
fn slot_no_display() {
    let slot = SlotNo::from(8086u64);
    assert_eq!(format!("{}", slot), "8086");
}

#[test]
fn slot_no_debug() {
    let slot = SlotNo::from(8086u64);
    assert_eq!(format!("{:?}", slot), "8086");
}

#[test]
fn slot_no_equality() {
    assert_eq!(SlotNo::from(1u64), SlotNo::from(1u64));
    assert_ne!(SlotNo::from(1u64), SlotNo::from(2u64));
}

#[test]
fn slot_no_ordering() {
    assert!(SlotNo::from(1u64) < SlotNo::from(2u64));
    assert!(SlotNo::from(10u64) > SlotNo::from(5u64));
}

#[test]
fn slot_no_clone() {
    let slot = SlotNo::from(7u64);
    let cloned = slot;
    assert_eq!(slot, cloned);
}

#[test]
fn slot_no_hash() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(SlotNo::from(1u64));
    set.insert(SlotNo::from(1u64));
    set.insert(SlotNo::from(2u64));
    assert_eq!(set.len(), 2);
}

#[test]
fn slot_no_serialize_deserialize() {
    let slot = SlotNo::from(42u64);
    let json = serde_json::to_string(&slot).unwrap();
    assert_eq!(json, "42");
    let deserialized: SlotNo = serde_json::from_str(&json).unwrap();
    assert_eq!(slot, deserialized);
}

// ── BlockNo ────────────────────────────────────────────────────────────────

#[test]
fn block_no_from_u64_and_inner() {
    let block = BlockNo::from(99u64);
    assert_eq!(block.inner(), 99);
}

#[test]
fn block_no_into_u64() {
    let block = BlockNo::from(55u64);
    let val: u64 = block.into();
    assert_eq!(val, 55);
}

#[test]
fn block_no_display() {
    let block = BlockNo::from(42u64);
    assert_eq!(format!("{}", block), "42");
}

#[test]
fn block_no_debug() {
    let block = BlockNo::from(42u64);
    assert_eq!(format!("{:?}", block), "42");
}

#[test]
fn block_no_equality() {
    assert_eq!(BlockNo::from(1u64), BlockNo::from(1u64));
    assert_ne!(BlockNo::from(1u64), BlockNo::from(2u64));
}

#[test]
fn block_no_ordering() {
    assert!(BlockNo::from(1u64) < BlockNo::from(2u64));
}

#[test]
fn block_no_clone() {
    let block = BlockNo::from(7u64);
    let cloned = block;
    assert_eq!(block, cloned);
}

#[test]
fn block_no_hash() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(BlockNo::from(1u64));
    set.insert(BlockNo::from(1u64));
    set.insert(BlockNo::from(2u64));
    assert_eq!(set.len(), 2);
}

#[test]
fn block_no_serialize_deserialize() {
    let block = BlockNo::from(42u64);
    let json = serde_json::to_string(&block).unwrap();
    assert_eq!(json, "42");
    let deserialized: BlockNo = serde_json::from_str(&json).unwrap();
    assert_eq!(block, deserialized);
}

// ── TxId ───────────────────────────────────────────────────────────────────

#[test]
fn tx_id_from_bytes_valid() {
    let digest = sha256([0u8; 32]);
    let tx_id = TxId::from(digest.clone());
    assert_eq!(tx_id.to_vec(), digest.to_vec());
}

#[test]
fn tx_id_from_bytes_error_wrong_length() {
    let result = TxId::from_bytes(&[1u8, 2, 3]);
    assert!(result.is_err());
}

#[test]
fn tx_id_to_vec() {
    let digest = sha256([0u8; 32]);
    let tx_id = TxId::from(digest.clone());
    assert_eq!(tx_id.to_vec(), digest.to_vec());
}

#[test]
fn tx_id_display_shows_hex() {
    let digest = sha256([0u8; 32]);
    let tx_id = TxId::from(digest.clone());
    let hex = identus_apollo::hex::HexStr::from(digest.as_bytes());
    assert_eq!(format!("{}", tx_id), format!("{}", hex));
}

#[test]
fn tx_id_debug_shows_hex() {
    let digest = sha256([0u8; 32]);
    let tx_id = TxId::from(digest.clone());
    let hex = identus_apollo::hex::HexStr::from(digest.as_bytes());
    assert_eq!(format!("{:?}", tx_id), format!("{}", hex));
}

#[test]
fn tx_id_from_str_valid() {
    let digest = sha256([0u8; 32]);
    let expected = TxId::from(digest.clone());
    let hex = identus_apollo::hex::HexStr::from(digest.as_bytes());
    let parsed: TxId = TxId::from_str(&hex.to_string()).unwrap();
    assert_eq!(parsed.to_vec(), expected.to_vec());
}

#[test]
fn tx_id_from_str_invalid_hex() {
    let result = TxId::from_str("not-valid-hex");
    assert!(result.is_err());
}

#[test]
fn tx_id_from_str_wrong_length() {
    let result = TxId::from_str("aabbcc");
    assert!(result.is_err());
}

#[test]
fn tx_id_equality() {
    let a = TxId::from(sha256([1u8; 32]));
    let b = TxId::from(sha256([1u8; 32]));
    assert_eq!(a, b);
}

#[test]
fn tx_id_inequality() {
    let a = TxId::from(sha256([1u8; 32]));
    let b = TxId::from(sha256([2u8; 32]));
    assert_ne!(a, b);
}

#[test]
fn tx_id_serialize_deserialize_roundtrip() {
    let digest = sha256([0u8; 32]);
    let tx_id = TxId::from(digest);
    let json = serde_json::to_string(&tx_id).unwrap();
    // Should serialize to a hex string
    assert!(json.starts_with('"'));
    assert!(json.ends_with('"'));
    let deserialized: TxId = serde_json::from_str(&json).unwrap();
    assert_eq!(tx_id, deserialized);
}

#[test]
fn tx_id_deserialize_invalid_hex() {
    let result: Result<TxId, _> = serde_json::from_str("\"not-hex\"");
    assert!(result.is_err());
}

#[test]
fn tx_id_deserialize_invalid_length() {
    let result: Result<TxId, _> = serde_json::from_str("\"aabbcc\"");
    assert!(result.is_err());
}

// ── NetworkIdentifier ──────────────────────────────────────────────────────

#[test]
fn network_identifier_display_mainnet() {
    assert_eq!(NetworkIdentifier::Mainnet.to_string(), "mainnet");
}

#[test]
fn network_identifier_display_preprod() {
    assert_eq!(NetworkIdentifier::Preprod.to_string(), "preprod");
}

#[test]
fn network_identifier_display_preview() {
    assert_eq!(NetworkIdentifier::Preview.to_string(), "preview");
}

#[test]
fn network_identifier_display_custom() {
    assert_eq!(NetworkIdentifier::Custom.to_string(), "custom");
}

#[test]
fn network_identifier_from_str() {
    assert_eq!(
        NetworkIdentifier::from_str("mainnet").unwrap(),
        NetworkIdentifier::Mainnet
    );
    assert_eq!(
        NetworkIdentifier::from_str("preprod").unwrap(),
        NetworkIdentifier::Preprod
    );
    assert_eq!(
        NetworkIdentifier::from_str("preview").unwrap(),
        NetworkIdentifier::Preview
    );
    assert_eq!(
        NetworkIdentifier::from_str("custom").unwrap(),
        NetworkIdentifier::Custom
    );
}

#[test]
fn network_identifier_from_str_invalid() {
    let result = NetworkIdentifier::from_str("invalid");
    assert!(result.is_err());
}

#[test]
fn network_identifier_variants_returns_all() {
    let variants = NetworkIdentifier::variants();
    assert_eq!(variants.len(), 4);
    assert!(variants.contains(&NetworkIdentifier::Mainnet));
    assert!(variants.contains(&NetworkIdentifier::Preprod));
    assert!(variants.contains(&NetworkIdentifier::Preview));
    assert!(variants.contains(&NetworkIdentifier::Custom));
}

#[test]
fn network_identifier_equality() {
    assert_eq!(NetworkIdentifier::Mainnet, NetworkIdentifier::Mainnet);
    assert_ne!(NetworkIdentifier::Mainnet, NetworkIdentifier::Preprod);
}

#[test]
fn network_identifier_clone() {
    let variant = NetworkIdentifier::Preview;
    let cloned = variant;
    assert_eq!(variant, cloned);
}
