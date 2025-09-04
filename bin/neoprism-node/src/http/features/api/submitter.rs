use axum::Json;
use axum::extract::State;
use utoipa::OpenApi;

use crate::SubmitterState;
use crate::http::features::api::error::{ApiError, ApiErrorResponseBody};
use crate::http::features::api::submitter::models::{
    SignedOperationSubmissionRequest, SignedOperationSubmissionResponse,
};
use crate::http::features::api::tags;
use crate::http::urls;

#[derive(OpenApi)]
#[openapi(paths(submit_signed_operations))]
pub struct SubmitterOpenApiDoc;

mod models {
    use identus_did_prism::did::operation::SignedPrismOperationHexStr;
    use identus_did_prism::dlt::TxId;
    use serde::{Deserialize, Serialize};
    use utoipa::ToSchema;

    #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
    pub struct SignedOperationSubmissionRequest {
        pub signed_operations: Vec<SignedPrismOperationHexStr>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
    pub struct SignedOperationSubmissionResponse {
        pub tx_id: TxId,
    }
}

#[utoipa::path(
    post,
    path = urls::ApiSignedOpSubmissions::AXUM_PATH,
    tags = [tags::OP_SUBMIT],
    request_body = SignedOperationSubmissionRequest,
    responses(
        (status = OK, description = "Operations submitted successfully", body = SignedOperationSubmissionResponse),
        (status = BAD_REQUEST, description = "Malformed request or invalid operations", body = ApiErrorResponseBody, content_type = "application/json"),
        (status = INTERNAL_SERVER_ERROR, description = "An unexpected error occurred during submission", body = ApiErrorResponseBody, content_type = "application/json"),
    )
)]
pub async fn submit_signed_operations(
    State(state): State<SubmitterState>,
    Json(req): Json<SignedOperationSubmissionRequest>,
) -> Result<Json<SignedOperationSubmissionResponse>, ApiError> {
    let ops = req.signed_operations.into_iter().map(|i| i.into()).collect();
    let result = state.dlt_sink.publish_operations(ops).await;
    match result {
        Ok(tx_id) => Ok(Json(SignedOperationSubmissionResponse { tx_id })),
        Err(e) => Err(ApiError::Internal {
            source: anyhow::anyhow!(e),
        }),
    }
}
