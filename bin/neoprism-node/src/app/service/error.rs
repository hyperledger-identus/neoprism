use axum::http::StatusCode;
use identus_did_core::{DidResolutionError, DidResolutionErrorCode, DidResolutionMetadata, ResolutionResult};
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
    #[display("did resolution is not supported for this did method")]
    MethodNotSupported,
}

impl ResolutionError {
    pub fn log_internal_error(&self) {
        if let ResolutionError::InternalError { source } = self {
            let msg = source.chain().map(|e| e.to_string()).collect::<Vec<_>>().join("\n");
            tracing::error!("{msg}");
        }
    }
}

#[derive(Debug, derive_more::From, derive_more::Display, derive_more::Error)]
pub enum InvalidDid {
    #[from]
    #[display("failed to process did state from did")]
    ProcessStateFailed { source: protocol::error::ProcessError },
    #[from]
    #[display("failed to parse prism did")]
    InvalidPrismDid { source: did::Error },
    #[cfg(feature = "midnight")]
    #[from]
    #[display("failed to parse midnight did")]
    InvalidMidnightDid { source: identus_did_midnight::error::Error },
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
            ResolutionError::MethodNotSupported => DidResolutionError {
                r#type: DidResolutionErrorCode::MethodNotSupported,
                title: None,
                detail: None,
            },
        };

        ResolutionResult {
            did_resolution_metadata: DidResolutionMetadata {
                content_type: None,
                error: Some(error),
            },
            did_document_metadata: Default::default(),
            did_document: Default::default(),
        }
    }
}

// TODO: remove
impl ResolutionError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            ResolutionError::InvalidDid { .. } => StatusCode::BAD_REQUEST,
            ResolutionError::NotFound => StatusCode::NOT_FOUND,
            ResolutionError::InternalError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ResolutionError::MethodNotSupported => StatusCode::NOT_IMPLEMENTED,
        }
    }
}
