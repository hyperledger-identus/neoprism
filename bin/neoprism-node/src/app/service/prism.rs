use std::sync::Arc;

use identus_apollo::hash::Sha256Digest;
use identus_apollo::hex::HexStr;
use identus_did_core::{Did, DidResolver, ResolutionOptions, ResolutionResult};
use identus_did_prism::did::operation::StorageData;
use identus_did_prism::did::{CanonicalPrismDid, DidState, PrismDid, PrismDidOps};
use identus_did_prism::dlt::{BlockNo, OperationMetadata, SlotNo, TxId};
use identus_did_prism::prelude::SignedPrismOperation;
use identus_did_prism::protocol::resolver::{ResolutionDebug, resolve_published, resolve_unpublished};
use identus_did_prism::utils::paging::Paginated;
use identus_did_prism_indexer::repo::{IndexerStateRepo, RawOperationRepo};
use node_storage::StorageBackend;

use super::error::{InvalidDid, ResolutionError};

#[derive(Clone)]
pub struct PrismDidService {
    db: Arc<dyn StorageBackend>,
}

impl PrismDidService {
    pub fn new(db: Arc<dyn StorageBackend>) -> Self {
        Self { db }
    }

    pub async fn get_indexer_stats(&self) -> anyhow::Result<Option<(SlotNo, BlockNo)>> {
        let result = self.db.get_last_indexed_block().await?;
        Ok(result)
    }

    pub async fn resolve_vdr(&self, entry_hash_hex: &str) -> anyhow::Result<Option<Vec<u8>>> {
        let entry_hash_hex: HexStr = entry_hash_hex.parse()?;
        let entry_hash = Sha256Digest::from_bytes(&entry_hash_hex.to_bytes())?;
        let Some(owner) = self.db.get_did_by_vdr_entry(&entry_hash).await? else {
            return Ok(None);
        };

        let mut debug_acc = vec![];
        let (_, did_state) = self.resolve_did_logic(&owner.to_string(), &mut debug_acc).await?;

        let storage_data = did_state
            .storage
            .into_iter()
            .find(|i| *i.init_operation_hash == entry_hash);

        let Some(data) = storage_data.map(|i| i.data) else {
            return Ok(None);
        };

        match &*data {
            StorageData::Bytes(items) => Ok(Some(items.clone())),
            _ => anyhow::bail!("vdr storage data types other than bytes are not yet supported"),
        }
    }

    pub async fn resolve_did(&self, did: &str) -> (Result<(PrismDid, DidState), ResolutionError>, ResolutionDebug) {
        let mut debug_acc = vec![];
        let result = self.resolve_did_logic(did, &mut debug_acc).await;
        (result, debug_acc)
    }

    async fn resolve_did_logic(
        &self,
        did: &str,
        debug_acc: &mut ResolutionDebug,
    ) -> Result<(PrismDid, DidState), ResolutionError> {
        let did: PrismDid = did.parse().map_err(|e| InvalidDid::InvalidPrismDid { source: e })?;
        let canonical_did = did.clone().into_canonical();

        let operations = self
            .db
            .get_raw_operations_by_did(&canonical_did)
            .await
            .map_err(|e| ResolutionError::InternalError { source: e.into() })?
            .into_iter()
            .map(|record| (record.metadata, record.signed_operation))
            .collect::<Vec<_>>();

        if operations.is_empty() {
            match &did {
                PrismDid::Canonical(_) => Err(ResolutionError::NotFound)?,
                PrismDid::LongForm(long_form_did) => {
                    let operation = long_form_did
                        .operation()
                        .map_err(|e| InvalidDid::InvalidPrismDid { source: e })?;
                    let did_state =
                        resolve_unpublished(operation).map_err(|e| InvalidDid::ProcessStateFailed { source: e })?;
                    Ok((did, did_state))
                }
            }
        } else {
            let (did_state, debug) = resolve_published(operations);
            debug_acc.extend(debug);
            match did_state {
                Some(did_state) => Ok((did, did_state)),
                None => Err(ResolutionError::NotFound),
            }
        }
    }

    pub async fn get_all_dids(&self, page: Option<u32>) -> anyhow::Result<Paginated<CanonicalPrismDid>> {
        let page = page.unwrap_or(0);
        let dids = self.db.get_all_dids(page, 100).await?;
        Ok(dids)
    }

    pub async fn get_raw_operations_by_tx_id(
        &self,
        tx_id: &TxId,
    ) -> anyhow::Result<Vec<(OperationMetadata, SignedPrismOperation)>> {
        Ok(self
            .db
            .get_raw_operations_by_tx_id(tx_id)
            .await?
            .into_iter()
            .map(|record| (record.metadata, record.signed_operation))
            .collect())
    }
}

#[async_trait::async_trait]
impl DidResolver for PrismDidService {
    async fn resolve(&self, did: &Did, _options: &ResolutionOptions) -> ResolutionResult {
        let did_str = did.to_string();
        let mut debug_acc = vec![];
        match self.resolve_did_logic(&did_str, &mut debug_acc).await {
            Ok((prism_did, state)) => state.to_resolution_result(&prism_did),
            Err(e) => e.into(),
        }
    }
}
