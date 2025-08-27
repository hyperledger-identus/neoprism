use std::str::FromStr;

use identus_did_core::DidDocument;
use identus_did_midnight::did::MidnightDid;
use identus_did_midnight::dlt::ContractStateDecoder;
use identus_did_midnight_sources::indexer_api::{IndexerApiError, get_contract_state};
use identus_did_midnight_sources::serde_cli::CliContractStateDecoder;

use crate::app::service::error::ResolutionError;

#[derive(Clone)]
pub struct MidnightDidService {
    indexer_url: String,
}

impl MidnightDidService {
    pub fn new(indexer_url: &str) -> Self {
        Self {
            indexer_url: indexer_url.to_string(),
        }
    }

    pub async fn resolve_did(&self, did: &str) -> Result<DidDocument, ResolutionError> {
        let did = match MidnightDid::from_str(did) {
            Ok(did) => did,
            Err(e) => Err(ResolutionError::InvalidDid { source: e.into() })?,
        };
        let contract_state = match get_contract_state(&self.indexer_url, &did).await {
            Ok(state) => state,
            Err(IndexerApiError::MissingDataFields { .. }) => return Err(ResolutionError::NotFound),
            Err(e) => Err(ResolutionError::InternalError { source: e.into() })?,
        };
        let did_doc = match CliContractStateDecoder.decode(&did, contract_state) {
            Ok(doc) => doc,
            Err(e) => Err(ResolutionError::InternalError {
                source: anyhow::Error::from_boxed(e),
            })?,
        };
        Ok(did_doc)
    }
}
