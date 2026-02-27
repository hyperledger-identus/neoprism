use axum::Json;
use axum::extract::State;
use identus_did_prism::did::operation::OperationId;
use identus_did_prism::dlt::TxId;
use identus_did_prism::prelude::SignedPrismOperation;
use utoipa::OpenApi;

use crate::SubmitterState;
use crate::http::features::api::error::{ApiError, ApiErrorResponseBody};
use crate::http::features::api::submitter::models::{
    ObjectSubmissionRequest, SignedOperationSubmissionRequest, SubmissionResponse,
};
use crate::http::features::api::tags;
use crate::http::urls;

#[derive(OpenApi)]
#[openapi(paths(submit_signed_operations, submit_object))]
pub struct SubmitterOpenApiDoc;

mod models {
    use identus_did_prism::did::operation::{OperationId, PrismObjectHexStr, SignedPrismOperationHexStr};
    use identus_did_prism::dlt::TxId;
    use serde::{Deserialize, Serialize};
    use utoipa::ToSchema;

    #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
    pub struct SignedOperationSubmissionRequest {
        pub signed_operations: Vec<SignedPrismOperationHexStr>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
    pub struct ObjectSubmissionRequest {
        pub object: PrismObjectHexStr,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
    pub struct SubmissionResponse {
        pub tx_id: TxId,
        pub operation_ids: Vec<OperationId>,
    }
}

async fn publish_operations(
    state: &SubmitterState,
    signed_operations: Vec<SignedPrismOperation>,
) -> Result<(TxId, Vec<OperationId>), ApiError> {
    if signed_operations.is_empty() {
        return Err(ApiError::BadRequest {
            message: "submission batch is empty".to_string(),
        });
    }
    let operation_ids: Vec<_> = signed_operations.iter().map(|op| op.operation_id()).collect();

    let tx_id = state
        .dlt_sink
        .publish_operations(signed_operations)
        .await
        .map_err(|e| ApiError::Internal {
            source: anyhow::anyhow!(e),
        })?;

    Ok((tx_id, operation_ids))
}

#[utoipa::path(
    post,
    summary = "Submit signed operations",
    description = "Submits one or more signed PRISM operations to the blockchain. Accepts an array of hex-encoded SignedPrismOperation protobuf messages and returns the transaction ID along with the computed operation IDs for tracking.",
    path = urls::ApiSubmissionsSignedOperations::AXUM_PATH,
    tags = [tags::OP_SUBMIT],
    request_body = SignedOperationSubmissionRequest,
    responses(
        (status = OK, description = "Operations submitted successfully", body = SubmissionResponse),
        (status = BAD_REQUEST, description = "Malformed request or invalid operations", body = ApiErrorResponseBody, content_type = "application/json"),
        (status = INTERNAL_SERVER_ERROR, description = "An unexpected error occurred during submission", body = ApiErrorResponseBody, content_type = "application/json"),
    )
)]
pub async fn submit_signed_operations(
    State(state): State<SubmitterState>,
    Json(req): Json<SignedOperationSubmissionRequest>,
) -> Result<Json<SubmissionResponse>, ApiError> {
    let signed_operations: Vec<SignedPrismOperation> = req.signed_operations.into_iter().map(|i| i.into()).collect();
    let (tx_id, operation_ids) = publish_operations(&state, signed_operations).await?;
    Ok(Json(SubmissionResponse { tx_id, operation_ids }))
}

#[utoipa::path(
    post,
    summary = "Submit a PRISM object",
    description = "Submits a PRISM object containing signed operations to the blockchain. Accepts a hex-encoded PrismObject protobuf message containing a PrismBlock with SignedPrismOperation messages and returns the transaction ID along with the computed operation IDs for tracking.",
    path = urls::ApiSubmissionsObjects::AXUM_PATH,
    tags = [tags::OP_SUBMIT],
    request_body = ObjectSubmissionRequest,
    responses(
        (status = OK, description = "Object submitted successfully", body = SubmissionResponse),
        (status = BAD_REQUEST, description = "Malformed request or invalid object", body = ApiErrorResponseBody, content_type = "application/json"),
        (status = INTERNAL_SERVER_ERROR, description = "An unexpected error occurred during submission", body = ApiErrorResponseBody, content_type = "application/json"),
    )
)]
pub async fn submit_object(
    state: State<SubmitterState>,
    Json(req): Json<ObjectSubmissionRequest>,
) -> Result<Json<SubmissionResponse>, ApiError> {
    let signed_operations = req.object.signed_operations();
    let (tx_id, operation_ids) = publish_operations(&state, signed_operations).await?;
    Ok(Json(SubmissionResponse { tx_id, operation_ids }))
}
