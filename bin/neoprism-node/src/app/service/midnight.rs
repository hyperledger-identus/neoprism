use std::path::PathBuf;
use std::str::FromStr;

use identus_did_core::{Did, DidDocument, DidResolver, ResolutionOptions, ResolutionResult};
use identus_did_midnight::did::MidnightDid;
use identus_did_midnight::dlt::ContractStateDecoder;
use identus_did_midnight_sources::indexer_api::{IndexerApiError, get_contract_state};
use identus_did_midnight_sources::serde_cli::CliContractStateDecoder;

use crate::app::service::error::ResolutionError;

#[derive(Clone)]
pub struct MidnightDidService {
    indexer_url: String,
    decoder: CliContractStateDecoder,
}

impl MidnightDidService {
    pub fn new<P: Into<PathBuf>>(indexer_url: &str, cli_path: P) -> Self {
        Self {
            indexer_url: indexer_url.to_string(),
            decoder: CliContractStateDecoder::new(cli_path.into()),
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
        let did_doc = match self.decoder.decode(&did, contract_state) {
            Ok(doc) => doc,
            Err(e) => Err(ResolutionError::InternalError {
                source: anyhow::Error::from_boxed(e),
            })?,
        };
        Ok(did_doc)
    }
}

#[async_trait::async_trait]
impl DidResolver for MidnightDidService {
    async fn resolve(&self, did: &Did, _options: &ResolutionOptions) -> ResolutionResult {
        let did_str = did.to_string();
        match self.resolve_did(&did_str).await {
            Ok(did_doc) => ResolutionResult::success(did_doc),
            Err(e) => e.into(),
        }
    }
}
