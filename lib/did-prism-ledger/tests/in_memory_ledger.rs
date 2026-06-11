//! Tests for the in-memory ledger implementation.
//!
//! Covers: `create_ledger()`, `InMemoryDltSink`, and `InMemoryDltSource`.

use std::time::Duration;

use identus_did_prism::dlt::TxId;
use identus_did_prism::proto::prism::PrismObject;
use identus_did_prism::proto::prism::prism_operation::Operation;
use identus_did_prism_indexer::DltSource;
use identus_did_prism_ledger::in_memory::{InMemoryDltSink, create_ledger};
use identus_did_prism_submitter::DltSink;

/// Helper: create a minimal signed operation for testing.
fn make_signed_operation(signed_with: &str) -> identus_did_prism::proto::prism::SignedPrismOperation {
    use identus_did_prism::proto::prism::PrismOperation;
    use identus_did_prism::proto::prism_ssi::ProtoCreateDID;
    use identus_did_prism::proto::prism_ssi::proto_create_did::DIDCreationData;

    let did_data = DIDCreationData {
        public_keys: vec![],
        services: vec![],
        context: vec![],
        special_fields: Default::default(),
    };
    let create_did = ProtoCreateDID {
        did_data: Some(did_data).into(),
        special_fields: Default::default(),
    };
    let operation = PrismOperation {
        operation: Some(Operation::CreateDid(create_did)),
        special_fields: Default::default(),
    };
    identus_did_prism::proto::prism::SignedPrismOperation {
        signed_with: signed_with.to_string(),
        signature: vec![0u8; 64],
        operation: Some(operation).into(),
        special_fields: Default::default(),
    }
}

// ---------------------------------------------------------------------------
// InMemoryDltSink tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sink_publish_single_operation_returns_tx_id() {
    // Keep the source alive: dropping it lets the relay task close the block
    // channel, which would make publish fail under a multi-threaded runtime.
    let (_source, sink) = create_ledger();

    let signed_op = make_signed_operation("master-0");
    let result = sink.publish_operations(vec![signed_op]).await;
    assert!(result.is_ok(), "publish_operations should succeed");
    let tx_id = result.unwrap();
    assert_eq!(tx_id.to_vec().len(), 32, "TxId should be 32 bytes");
}

#[tokio::test]
async fn sink_publish_multiple_operations_generates_unique_tx_ids() {
    let (_source, sink) = create_ledger();

    let op1 = make_signed_operation("master-0");
    let op2 = make_signed_operation("master-1");
    let op3 = make_signed_operation("master-2");

    let tx1 = sink.publish_operations(vec![op1]).await.unwrap();
    let tx2 = sink.publish_operations(vec![op2]).await.unwrap();
    let tx3 = sink.publish_operations(vec![op3]).await.unwrap();

    assert_ne!(tx1, tx2, "each publish should produce a unique tx id");
    assert_ne!(tx2, tx3, "each publish should produce a unique tx id");
    assert_ne!(tx1, tx3, "each publish should produce a unique tx id");
}

#[tokio::test]
async fn sink_publish_fails_when_channel_closed() {
    let (block_tx, _block_rx) = tokio::sync::mpsc::channel::<(PrismObject, TxId)>(1);
    drop(_block_rx); // close the receiver so send fails
    let sink = InMemoryDltSink::new(block_tx);

    let signed_op = make_signed_operation("master-0");
    let result = sink.publish_operations(vec![signed_op]).await;
    assert!(result.is_err(), "publish should fail when receiver is closed");
}

// ---------------------------------------------------------------------------
// create_ledger integration tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn create_ledger_publish_and_receive_single_object() {
    let (source, sink) = create_ledger();

    let signed_op = make_signed_operation("master-0");
    let tx_id = sink.publish_operations(vec![signed_op]).await.unwrap();

    let mut stream_rx = source.into_stream().unwrap();

    let published = tokio::time::timeout(Duration::from_secs(2), stream_rx.recv())
        .await
        .expect("timed out waiting for published object")
        .expect("stream ended unexpectedly");

    assert_eq!(published.block_metadata.slot_number, 0.into());
    assert_eq!(published.block_metadata.block_number, 0.into());
    assert_eq!(published.block_metadata.absn, 0);
    assert_eq!(published.block_metadata.tx_id, tx_id, "tx_id should match");

    let block_content = published.prism_object.block_content.into_option().unwrap();
    assert_eq!(block_content.operations.len(), 1);
    assert_eq!(block_content.operations[0].signed_with, "master-0");
}

#[tokio::test]
async fn create_ledger_sequential_slot_and_block_numbers() {
    let (source, sink) = create_ledger();

    let op1 = make_signed_operation("master-0");
    let op2 = make_signed_operation("master-1");
    let op3 = make_signed_operation("master-2");

    let _ = sink.publish_operations(vec![op1]).await.unwrap();
    let _ = sink.publish_operations(vec![op2]).await.unwrap();
    let _ = sink.publish_operations(vec![op3]).await.unwrap();

    let mut stream_rx = source.into_stream().unwrap();

    for expected_slot in 0u64..3 {
        let published = tokio::time::timeout(Duration::from_secs(2), stream_rx.recv())
            .await
            .expect("timed out waiting for published object")
            .expect("stream ended unexpectedly");

        assert_eq!(
            published.block_metadata.slot_number,
            expected_slot.into(),
            "slot number should be {expected_slot}"
        );
        assert_eq!(
            published.block_metadata.block_number,
            expected_slot.into(),
            "block number should match slot number"
        );
    }
}

#[tokio::test]
async fn create_ledger_timestamps_are_recent() {
    let (source, sink) = create_ledger();

    let before = chrono::Utc::now();
    let signed_op = make_signed_operation("master-0");
    let _ = sink.publish_operations(vec![signed_op]).await.unwrap();

    let mut stream_rx = source.into_stream().unwrap();
    let published = tokio::time::timeout(Duration::from_secs(2), stream_rx.recv())
        .await
        .expect("timed out")
        .expect("stream ended");
    let after = chrono::Utc::now();

    assert!(
        published.block_metadata.cbt >= before && published.block_metadata.cbt <= after,
        "timestamp should be between before and after"
    );
}

// ---------------------------------------------------------------------------
// InMemoryDltSource tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn source_into_stream_relay_multiple_objects() {
    let (source, sink) = create_ledger();

    for i in 0..5 {
        let op = make_signed_operation(&format!("key-{i}"));
        let _ = sink.publish_operations(vec![op]).await.unwrap();
    }

    let mut stream_rx = source.into_stream().unwrap();

    let mut count = 0;
    loop {
        match tokio::time::timeout(Duration::from_secs(2), stream_rx.recv()).await {
            Ok(Some(_)) => {
                count += 1;
                if count == 5 {
                    break;
                }
            }
            Ok(None) => panic!("stream ended early, received {count}/5 objects"),
            Err(_) => panic!("timed out waiting for object {count}"),
        }
    }
    assert_eq!(count, 5);
}

#[tokio::test]
async fn source_sync_cursor_updates_on_each_object() {
    let (source, sink) = create_ledger();

    let mut cursor_rx = source.sync_cursor();
    assert!(cursor_rx.borrow().is_none(), "initial cursor should be None");

    let mut stream_rx = source.into_stream().unwrap();

    let op = make_signed_operation("master-0");
    let _ = sink.publish_operations(vec![op]).await.unwrap();

    let _ = tokio::time::timeout(Duration::from_secs(2), stream_rx.recv())
        .await
        .expect("timed out")
        .expect("stream ended");

    cursor_rx.changed().await.unwrap();
    let cursor = cursor_rx.borrow().clone().expect("cursor should be set");
    assert_eq!(cursor.slot, 0, "first cursor slot should be 0");
    assert_eq!(
        cursor.block_hash,
        0u64.to_le_bytes().to_vec(),
        "block_hash should be derived from block number"
    );
    assert!(cursor.cbt.is_some(), "cbt should be set");
    assert!(cursor.blockfrost_page.is_none(), "blockfrost_page should be None");
}

#[tokio::test]
async fn source_sync_cursor_slot_increments() {
    let (source, sink) = create_ledger();

    let mut cursor_rx = source.sync_cursor();
    let mut stream_rx = source.into_stream().unwrap();

    for i in 0u64..3 {
        let op = make_signed_operation(&format!("key-{i}"));
        let _ = sink.publish_operations(vec![op]).await.unwrap();

        let _ = tokio::time::timeout(Duration::from_secs(2), stream_rx.recv())
            .await
            .expect("timed out")
            .expect("stream ended");

        cursor_rx.changed().await.unwrap();
        let cursor = cursor_rx.borrow().clone().unwrap();
        assert_eq!(cursor.slot, i, "cursor slot should be {i}");
    }
}

#[tokio::test]
async fn source_into_stream_produces_correct_prism_objects() {
    let (source, sink) = create_ledger();

    let signed_op = make_signed_operation("test-key-42");
    let _ = sink.publish_operations(vec![signed_op]).await.unwrap();

    let mut stream_rx = source.into_stream().unwrap();

    let published = tokio::time::timeout(Duration::from_secs(2), stream_rx.recv())
        .await
        .expect("timed out")
        .expect("stream ended");

    let block = published
        .prism_object
        .block_content
        .into_option()
        .expect("block_content should be present");
    assert_eq!(block.operations.len(), 1);
    assert_eq!(block.operations[0].signed_with, "test-key-42");
}

#[tokio::test]
async fn source_stream_ends_when_sink_dropped() {
    let (source, sink) = create_ledger();
    let mut stream_rx = source.into_stream().unwrap();

    let op = make_signed_operation("master-0");
    let _ = sink.publish_operations(vec![op]).await.unwrap();

    let first = tokio::time::timeout(Duration::from_secs(2), stream_rx.recv())
        .await
        .expect("timed out on first")
        .expect("should receive first object");
    assert_eq!(first.block_metadata.slot_number, 0.into());

    // Explicitly drop the sink so the internal relay task sees the channel close
    drop(sink);

    // After sink is dropped, the relay task exits, closing the downstream channel
    let second = tokio::time::timeout(Duration::from_secs(3), stream_rx.recv()).await;
    assert!(
        second.is_ok() && second.unwrap().is_none(),
        "stream should end after sink is dropped"
    );
}
