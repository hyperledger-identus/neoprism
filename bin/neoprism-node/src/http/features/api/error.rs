use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde_json;

use crate::app::service::error::ResolutionError;

#[derive(Debug, derive_more::Display, derive_more::Error)]
pub enum ApiError {
    #[display("service not available")]
    NotImplemented,
    #[display("not found")]
    NotFound,
    #[display("bad request: {message}")]
    BadRequest { message: String },
    #[display("internal server error")]
    Internal { source: anyhow::Error },
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self {
            ApiError::NotImplemented => StatusCode::NOT_IMPLEMENTED,
            ApiError::NotFound => StatusCode::NOT_FOUND,
            ApiError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            ApiError::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let body = serde_json::json!({ "error": self.to_string() });
        if let ApiError::Internal { source } = self {
            let msg = source.chain().map(|e| e.to_string()).collect::<Vec<_>>().join("\n");
            tracing::error!("{msg}");
        }
        (status, [(header::CONTENT_TYPE, "application/json")], body.to_string()).into_response()
    }
}

impl From<ResolutionError> for ApiError {
    fn from(value: ResolutionError) -> Self {
        match value {
            ResolutionError::NotFound => ApiError::NotFound,
            ResolutionError::MethodNotSupported => ApiError::NotImplemented,
            ResolutionError::InternalError { source } => ApiError::Internal { source },
            ResolutionError::InvalidDid { source } => ApiError::BadRequest {
                message: source.to_string(),
            },
        }
    }
}
