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
        tracing::info!("Spawn cursor persist worker with {:?} interval", DELAY);
        tokio::spawn(async move {
            loop {
                let recv_result = self.cursor_rx.changed().await;
                tokio::time::sleep(DELAY).await;

                if let Err(e) = recv_result {
                    tracing::error!("Error getting cursor to persist: {}", e);
                }

                let cursor = self.cursor_rx.borrow_and_update().clone();
                let Some(cursor) = cursor else { continue };
                tracing::info!(
                    "Persisting cursor on slot ({}, {})",
                    cursor.slot,
                    identus_apollo::hex::HexStr::from(cursor.block_hash.as_slice()).to_string(),
                );

                let persist_result = self.store.set_cursor(cursor).await;
                if let Err(e) = persist_result {
                    tracing::error!("Error persisting cursor: {}", e);
                }
            }
        })
    }
}
