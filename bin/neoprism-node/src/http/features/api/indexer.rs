use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, State};
use identus_apollo::hex::HexStr;
use identus_did_core::Did;
use identus_did_prism::proto::MessageExt;
use identus_did_prism::proto::node_api::DIDData;
use utoipa::OpenApi;

use crate::IndexerState;
use crate::http::features::api::error::{ApiError, ApiErrorResponseBody};
use crate::http::features::api::indexer::models::IndexerStats;
use crate::http::features::api::tags;
use crate::http::urls::{ApiDidData, ApiIndexerStats, ApiVdrBlob};

#[derive(OpenApi)]
#[openapi(paths(did_data, indexer_stats, resolve_vdr_blob))]
pub struct IndexerOpenApiDoc;

mod models {
    use identus_did_prism::dlt::{BlockNo, SlotNo};
    use serde::{Deserialize, Serialize};
    use utoipa::ToSchema;

    #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
    pub struct IndexerStats {
        pub last_prism_slot_number: Option<SlotNo>,
        pub last_prism_block_number: Option<BlockNo>,
    }
}

#[utoipa::path(
    get,
    summary = "Resolve a VDR entry and return its blob data.",
    description = "Returns the raw blob data for a VDR entry, using PrismDidService::resolve_vdr. The response is application/octet-stream.",
    path = ApiVdrBlob::AXUM_PATH,
    tags = [tags::OP_INDEX],
    responses(
        (status = OK, description = "Successfully resolved the VDR entry. Returns the blob data.", content_type = "application/octet-stream"),
        (status = NOT_FOUND, description = "The VDR entry was not found.", body = ApiErrorResponseBody, content_type = "application/json"),
        (status = INTERNAL_SERVER_ERROR, description = "An unexpected error occurred during VDR resolution.", body = ApiErrorResponseBody, content_type = "application/json"),
    ),
    params(
        ("entry_hash" = String, Path, description = "The hex-encoded entry hash to resolve.")
    ),
)]
pub async fn resolve_vdr_blob(
    Path(entry_hash): Path<String>,
    State(state): State<IndexerState>,
) -> Result<Bytes, ApiError> {
    let service = &state.prism_did_service;
    match service.resolve_vdr(&entry_hash).await {
        Ok(Some(blob)) => Ok(Bytes::from(blob)),
        Ok(None) => Err(ApiError::NotFound)?,
        Err(e) => Err(ApiError::Internal { source: e })?,
    }
}

#[utoipa::path(
    get,
    summary = "Returns the DIDData protobuf message for a given DID, encoded in hexadecimal.",
    description = "The returned data is a protobuf message compatible with the legacy prism-node implementation. The object is encoded in hexadecimal format. This endpoint is useful for testing and verifying compatibility with existing operations already anchored on the blockchain.",
    path = ApiDidData::AXUM_PATH,
    tags = [tags::OP_INDEX],
    responses(
        (status = OK, description = "Successfully retrieved the DIDData protobuf message, encoded as a hexadecimal string.", body = String),
        (status = BAD_REQUEST, description = "The provided DID is invalid.", body = ApiErrorResponseBody, content_type = "application/json"),
        (status = NOT_FOUND, description = "The DID does not exist in the index.", body = ApiErrorResponseBody, content_type = "application/json"),
        (status = INTERNAL_SERVER_ERROR, description = "An unexpected error occurred while retrieving DIDData.", body = ApiErrorResponseBody, content_type = "application/json"),
        (status = NOT_IMPLEMENTED, description = "A functionality is not implemented.", body = ApiErrorResponseBody, content_type = "application/json"),
    ),
    params(("did" = Did, Path, description = "The Decentralized Identifier (DID) for which to retrieve the DIDData protobuf message."))
)]
pub async fn did_data(Path(did): Path<String>, State(state): State<IndexerState>) -> Result<String, ApiError> {
    let service = &state.prism_did_service;
    let (result, _) = service.resolve_did(&did).await;
    match result {
        Err(e) => Err(e)?,
        Ok((_, did_state)) => {
            let dd: DIDData = did_state.into();
            let bytes = dd.encode_to_vec();
            Ok(HexStr::from(bytes).to_string())
        }
    }
}

#[utoipa::path(
    get,
    summary = "Retrieves statistics about the indexer's latest processed slot and block.",
    path = ApiIndexerStats::AXUM_PATH,
    tags = [tags::OP_INDEX],
    responses(
        (status = OK, description = "Successfully retrieved indexer statistics, including the latest processed slot and block numbers.", body = IndexerStats),
        (status = INTERNAL_SERVER_ERROR, description = "An unexpected error occurred while retrieving indexer statistics.", content_type = "application/json"),
        (status = NOT_IMPLEMENTED, description = "Indexer service is not available.", content_type = "application/json"),
    )
)]
pub async fn indexer_stats(State(state): State<IndexerState>) -> Result<Json<IndexerStats>, ApiError> {
    let service = state.prism_did_service;
    let result = service.get_indexer_stats().await;
    let stats = match result {
        Ok(None) => IndexerStats {
            last_prism_slot_number: None,
            last_prism_block_number: None,
        },
        Ok(Some((slot, block))) => IndexerStats {
            last_prism_block_number: Some(block),
            last_prism_slot_number: Some(slot),
        },
        Err(e) => Err(ApiError::Internal { source: e })?,
    };
    Ok(Json(stats))
}
