use identus_did_prism::dlt::DltCursor;
use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::dlt::error::DltError;
use crate::repo::DltCursorRepo;

#[cfg(any(feature = "blockfrost", feature = "dbsync"))]
pub mod metadata_map {
    use std::str::FromStr;

    use identus_apollo::hex::HexStr;
    use identus_did_prism::proto::MessageExt;
    use identus_did_prism::proto::prism::PrismObject;
    use serde::{Deserialize, Serialize};

    use crate::dlt::error::MetadataReadError;

    /// PRISM metadata structure from Cardano blockchain.
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct MetadataMapJson {
        /// Byte groups containing hex-encoded data (typically with "0x" prefix)
        pub c: Vec<String>,
        /// Version number of the PRISM protocol
        pub v: u64,
    }

    impl MetadataMapJson {
        /// Parse the byte groups and decode the PRISM object.
        pub fn parse_prism_object(
            self,
            block_hash: &str,
            tx_idx: Option<usize>,
        ) -> Result<PrismObject, MetadataReadError> {
            let byte_group = self
                .c
                .into_iter()
                .map(|s| {
                    if let Some((prefix, hex_suffix)) = s.split_at_checked(2)
                        && let Ok(hex_str) = HexStr::from_str(hex_suffix)
                        && prefix == "0x"
                    {
                        Ok(hex_str.to_bytes())
                    } else {
                        Err(MetadataReadError::InvalidMetadataType {
                            source: "expect metadata byte group to be in hex format".into(),
                            block_hash: Some(block_hash.to_string()),
                            tx_idx,
                        })
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;

            let mut bytes = Vec::with_capacity(64 * byte_group.len());
            for mut b in byte_group.into_iter() {
                bytes.append(&mut b);
            }

            PrismObject::decode(bytes.as_slice()).map_err(|e| MetadataReadError::PrismBlockProtoDecode {
                source: e,
                block_hash: Some(block_hash.to_string()),
                tx_idx,
            })
        }
    }
}

pub struct CursorPersistWorker<Store: DltCursorRepo> {
    cursor_rx: watch::Receiver<Option<DltCursor>>,
    store: Store,
}

impl<Store: DltCursorRepo + Send + 'static> CursorPersistWorker<Store> {
    pub fn new(store: Store, cursor_rx: tokio::sync::watch::Receiver<Option<DltCursor>>) -> Self {
        Self { cursor_rx, store }
    }

    pub fn spawn(mut self) -> JoinHandle<Result<(), DltError>> {
        const DELAY: tokio::time::Duration = tokio::time::Duration::from_secs(60);
        tracing::info!("spawned cursor persist worker with {:?} interval", DELAY);
        tokio::spawn(async move {
            loop {
                let recv_result = self.cursor_rx.changed().await;
                tokio::time::sleep(DELAY).await;

                if let Err(e) = recv_result {
                    tracing::error!("error getting cursor to persist: {:?}", e);
                }

                let cursor = self.cursor_rx.borrow_and_update().clone();
                let Some(cursor) = cursor else { continue };
                tracing::info!(
                    "persisting cursor on slot ({}, {})",
                    cursor.slot,
                    identus_apollo::hex::HexStr::from(cursor.block_hash.as_slice()).to_string()
                );

                let persist_result = self.store.set_cursor(cursor).await;
                if let Err(e) = persist_result {
                    tracing::error!("error persisting cursor: {:?}", e);
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use identus_apollo::hash::sha256;
    use identus_did_prism::dlt::DltCursor;
    use identus_did_prism::proto::MessageExt;
    use identus_did_prism::proto::prism::{PrismBlock, PrismObject};

    use super::*;

    // ------------------------------------------------------------------
    // MetadataMapJson tests
    // ------------------------------------------------------------------

    /// Helper: encode a PrismObject into metadata byte groups ("0x" + hex).
    fn encode_object_as_byte_groups(obj: &PrismObject) -> Vec<String> {
        let bytes = obj.encode_to_vec();
        bytes
            .chunks(64)
            .map(|chunk| {
                let hex = identus_apollo::hex::HexStr::from(chunk).to_string();
                format!("0x{hex}")
            })
            .collect()
    }

    /// Helper: build a minimal PrismObject with one empty operation.
    fn minimal_prism_object() -> PrismObject {
        PrismObject {
            block_content: Some(PrismBlock {
                operations: vec![],
                special_fields: Default::default(),
            })
            .into(),
            special_fields: Default::default(),
        }
    }

    #[test]
    fn metadata_map_parse_valid_single_byte_group() {
        let obj = minimal_prism_object();
        let byte_groups = encode_object_as_byte_groups(&obj);

        let meta = metadata_map::MetadataMapJson { c: byte_groups, v: 1 };

        let result = meta.parse_prism_object("abc123", None).unwrap();
        assert_eq!(result, obj);
    }

    #[test]
    fn metadata_map_parse_valid_multiple_byte_groups() {
        // Create an object large enough to span multiple 64-byte chunks.
        // Use operations with large signature data to exceed 64 bytes.
        let large_sig: Vec<u8> = (0..200u8).collect();
        let obj = PrismObject {
            block_content: Some(PrismBlock {
                operations: (0..10)
                    .map(|_| identus_did_prism::proto::prism::SignedPrismOperation {
                        signed_with: "master-0".to_string(),
                        signature: large_sig.clone(),
                        operation: protobuf::MessageField(None),
                        special_fields: Default::default(),
                    })
                    .collect(),
                special_fields: Default::default(),
            })
            .into(),
            special_fields: Default::default(),
        };

        let byte_groups = encode_object_as_byte_groups(&obj);
        assert!(
            byte_groups.len() > 1,
            "expected multiple byte groups for a large object"
        );

        let meta = metadata_map::MetadataMapJson { c: byte_groups, v: 1 };

        let result = meta.parse_prism_object("deadbeef", Some(5)).unwrap();
        assert_eq!(result, obj);
    }

    #[test]
    fn metadata_map_parse_missing_0x_prefix_returns_error() {
        let meta = metadata_map::MetadataMapJson {
            c: vec!["aabbccdd".to_string()], // no 0x prefix
            v: 1,
        };

        let err = meta.parse_prism_object("blockhash", None).unwrap_err();

        let err_msg = err.to_string();
        assert!(
            err_msg.contains("blockhash"),
            "error should reference block_hash: {err_msg}"
        );
    }

    #[test]
    fn metadata_map_parse_invalid_hex_after_prefix_returns_error() {
        let meta = metadata_map::MetadataMapJson {
            c: vec!["0xZZZZ".to_string()], // invalid hex
            v: 1,
        };

        let err = meta.parse_prism_object("blockhash", Some(3)).unwrap_err();

        let err_msg = err.to_string();
        assert!(
            err_msg.contains("blockhash"),
            "error should reference block_hash: {err_msg}"
        );
    }

    #[test]
    fn metadata_map_parse_invalid_protobuf_returns_error() {
        let meta = metadata_map::MetadataMapJson {
            c: vec!["0xdeadbeef".to_string()], // valid hex, but not valid protobuf
            v: 1,
        };

        let err = meta.parse_prism_object("blockhash", Some(7)).unwrap_err();

        let err_msg = err.to_string();
        assert!(
            err_msg.contains("blockhash"),
            "error should reference block_hash: {err_msg}"
        );
        assert!(
            err_msg.contains("protobuf") || err_msg.contains("decode"),
            "error should mention proto decode: {err_msg}"
        );
    }

    #[test]
    fn metadata_map_parse_empty_byte_groups_returns_default_object() {
        // Zero byte groups means zero bytes -> PrismObject::decode(&[])
        // produces a default PrismObject in protobuf.
        let meta = metadata_map::MetadataMapJson { c: vec![], v: 1 };

        let result = meta.parse_prism_object("empty", None);
        assert!(result.is_ok(), "empty byte groups should decode as default PrismObject");
        let obj = result.unwrap();
        assert_eq!(obj, PrismObject::default());
    }

    #[test]
    fn metadata_map_parse_short_prefix_returns_error() {
        // String shorter than 2 chars should fail split_at_checked(2)
        let meta = metadata_map::MetadataMapJson {
            c: vec!["0".to_string()], // only 1 char
            v: 1,
        };

        let err = meta.parse_prism_object("blockhash", None).unwrap_err();

        let err_msg = err.to_string();
        assert!(
            err_msg.contains("blockhash"),
            "error should reference block_hash: {err_msg}"
        );
    }

    #[test]
    fn metadata_map_parse_wrong_prefix_returns_error() {
        // "ab" prefix instead of "0x"
        let meta = metadata_map::MetadataMapJson {
            c: vec!["ab1234".to_string()],
            v: 1,
        };

        let err = meta.parse_prism_object("blockhash", None).unwrap_err();

        let err_msg = err.to_string();
        assert!(
            err_msg.contains("blockhash"),
            "error should reference block_hash: {err_msg}"
        );
    }

    #[test]
    fn metadata_map_roundtrip_with_prism_object() {
        let obj = minimal_prism_object();
        let encoded = obj.encode_to_vec();
        let decoded = PrismObject::decode(encoded.as_slice()).unwrap();
        assert_eq!(decoded, obj);
    }

    // ------------------------------------------------------------------
    // CursorPersistWorker tests
    // ------------------------------------------------------------------

    /// A simple error type that implements std::error::Error.
    #[derive(Debug, derive_more::Display, derive_more::Error)]
    #[display("test error")]
    struct TestError;

    /// A mock DltCursorRepo that records set_cursor calls.
    #[derive(Debug)]
    struct CapturingRepo {
        calls: Arc<Mutex<Vec<DltCursor>>>,
    }

    #[async_trait::async_trait]
    impl DltCursorRepo for CapturingRepo {
        type Error = TestError;

        async fn set_cursor(&self, cursor: DltCursor) -> Result<(), Self::Error> {
            self.calls.lock().unwrap().push(cursor);
            Ok(())
        }

        async fn get_cursor(&self) -> Result<Option<DltCursor>, Self::Error> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn cursor_worker_new_creates_instance() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let repo = CapturingRepo { calls };
        let (_tx, rx) = watch::channel(None);
        let _worker = CursorPersistWorker::new(repo, rx);
    }

    #[tokio::test]
    async fn cursor_worker_spawn_and_send_cursor() {
        tokio::time::pause();

        let calls = Arc::new(Mutex::new(Vec::new()));
        let (tx, rx) = watch::channel(None);

        let worker = CursorPersistWorker::new(CapturingRepo { calls: calls.clone() }, rx);
        let handle = worker.spawn();

        // Yield to let the worker start and reach changed().await
        tokio::task::yield_now().await;

        // Send a cursor update
        let cursor = DltCursor {
            slot: 42,
            block_hash: sha256([1u8; 32]).to_vec(),
            cbt: None,
            blockfrost_page: None,
        };
        tx.send(Some(cursor.clone())).unwrap();

        // Yield to let the worker receive the change notification and start sleeping
        tokio::task::yield_now().await;

        // Advance time past the 60s DELAY in the worker
        tokio::time::advance(tokio::time::Duration::from_secs(61)).await;

        // Yield to let the worker finish processing
        tokio::task::yield_now().await;

        handle.abort();

        // Verify the cursor was persisted
        let persisted = calls.lock().unwrap().clone();
        assert_eq!(persisted.len(), 1, "expected exactly one set_cursor call");
        assert_eq!(persisted[0].slot, 42);
        assert_eq!(persisted[0].block_hash, cursor.block_hash);
    }

    #[tokio::test]
    async fn cursor_worker_skips_none_cursor() {
        tokio::time::pause();

        let calls = Arc::new(Mutex::new(Vec::new()));
        let (_tx, rx) = watch::channel(None);
        // Channel starts with None — the worker should skip it

        let worker = CursorPersistWorker::new(CapturingRepo { calls: calls.clone() }, rx);
        let handle = worker.spawn();

        // Advance time past the 60s DELAY
        tokio::time::advance(tokio::time::Duration::from_secs(61)).await;

        handle.abort();

        // Should not have persisted anything since cursor was None
        let persisted = calls.lock().unwrap().clone();
        assert!(persisted.is_empty(), "expected no set_cursor calls for None cursor");
    }

    #[tokio::test]
    async fn cursor_worker_handles_sender_dropped() {
        tokio::time::pause();

        let calls = Arc::new(Mutex::new(Vec::new()));
        let (tx, rx) = watch::channel(None);

        let worker = CursorPersistWorker::new(CapturingRepo { calls: calls.clone() }, rx);
        let handle = worker.spawn();

        // Drop the sender — the worker should detect the error on changed()
        drop(tx);

        // Advance time to let the worker process
        tokio::time::advance(tokio::time::Duration::from_secs(61)).await;

        // Worker should still be running (it just logs the error)
        assert!(
            !handle.is_finished(),
            "worker should still be running after sender drop"
        );

        handle.abort();
    }
}
