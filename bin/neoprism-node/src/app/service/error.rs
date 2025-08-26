use axum::http::StatusCode;
use identus_did_core::{
    DidDocumentMetadata, DidResolutionError, DidResolutionErrorCode, DidResolutionMetadata, ResolutionResult,
};
use identus_did_prism::{did, protocol};

#[derive(Debug, derive_more::From, derive_more::Display, derive_more::Error)]
pub enum ResolutionError {
    #[from]
    #[display("invalid did input")]
    InvalidDid { source: InvalidDid },
    #[display("did is not found")]
    NotFound,
    #[from]
    #[display("unexpected server error")]
    InternalError { source: anyhow::Error },
}

#[derive(Debug, derive_more::From, derive_more::Display, derive_more::Error)]
pub enum InvalidDid {
    #[from]
    #[display("failed to parse did")]
    ParsingFail { source: did::Error },
    #[from]
    #[display("failed to process did state from did")]
    ProcessFail { source: protocol::error::ProcessError },
}

impl From<ResolutionError> for ResolutionResult {
    fn from(err: ResolutionError) -> Self {
        let error = match err {
            ResolutionError::InvalidDid { .. } => DidResolutionError {
                r#type: DidResolutionErrorCode::InvalidDid,
                title: Some("Invalid DID".to_string()),
                detail: Some(err.to_string()),
            },
            ResolutionError::NotFound => DidResolutionError {
                r#type: DidResolutionErrorCode::NotFound,
                title: Some("DID Not Found".to_string()),
                detail: Some(err.to_string()),
            },
            ResolutionError::InternalError { .. } => DidResolutionError {
                r#type: DidResolutionErrorCode::InternalError,
                title: Some("Internal Error".to_string()),
                detail: Some(err.to_string()),
            },
        };

        ResolutionResult {
            did_document: None,
            did_resolution_metadata: DidResolutionMetadata {
                content_type: None,
                error: Some(error),
            },
            did_document_metadata: DidDocumentMetadata::default(),
        }
    }
}

impl ResolutionError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            ResolutionError::InvalidDid { .. } => StatusCode::BAD_REQUEST,
            ResolutionError::NotFound => StatusCode::NOT_FOUND,
            ResolutionError::InternalError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
