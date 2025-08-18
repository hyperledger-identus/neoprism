use axum::Json;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::IntoResponse;
use identus_apollo::hex::HexStr;
use identus_did_core::{Did, ResolutionResult};
use identus_did_prism::proto::MessageExt;
use identus_did_prism::proto::node_api::DIDData;
use serde_json;
use utoipa::OpenApi;

use crate::AppState;
use crate::app::service::error::ResolutionError;
use crate::http::features::api::indexer::models::IndexerStats;
use crate::http::features::api::tags;
use crate::http::urls::{ApiDid, ApiDidData, ApiIndexerStats, UniversalResolverDid};

#[derive(OpenApi)]
#[openapi(paths(resolve_did, did_data, indexer_stats, universal_resolver_did))]
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
    summary = "Resolves a W3C Decentralized Identifier (DID) according to the DID Resolution specification.",
    path = ApiDid::AXUM_PATH,
    tags = [tags::OP_INDEX],
    responses(
        (status = OK, description = "Successfully resolved the DID. Returns the DID Resolution Result.", body = ResolutionResult, content_type = "application/did-resolution"),
        (status = BAD_REQUEST, description = "The provided DID is invalid.", body = ResolutionResult, content_type = "application/did-resolution"),
        (status = NOT_FOUND, description = "The DID does not exist in the index.", body = ResolutionResult, content_type = "application/did-resolution"),
        (status = GONE, description = "The DID has been deactivated.", body = ResolutionResult, content_type = "application/did-resolution"),
        (status = INTERNAL_SERVER_ERROR, description = "An unexpected error occurred during resolution.", body = ResolutionResult, content_type = "application/did-resolution"),
    ),
    params(
        ("did" = Did, Path, description = "The Decentralized Identifier (DID) to resolve.")
    ),
    description = "This endpoint is fully compliant with the W3C DID Resolution specification. It returns a DID Resolution Result object, including metadata and the resolved DID Document, following the standard resolution process."
)]
pub async fn resolve_did(path: Path<String>, state: State<AppState>) -> impl IntoResponse {
    let (status, result) = resolution_logic(path, state).await;
    let body = serde_json::to_string(&result).expect("ResolutionResult should always be serializable");
    (status, [(header::CONTENT_TYPE, "application/did-resolution")], body)
}

#[utoipa::path(
    get,
    summary = "Universal Resolver driver endpoint for resolving DIDs, designed for use behind a Universal Resolver proxy.",
    path = UniversalResolverDid::AXUM_PATH,
    tags = [tags::UNI_RESOLVER],
    responses(
        (status = OK, description = "Successfully resolved the DID. Returns the DID Resolution Result.", body = ResolutionResult, content_type = "application/did-resolution"),
        (status = BAD_REQUEST, description = "The provided DID is invalid.", body = ResolutionResult, content_type = "application/did-resolution"),
        (status = NOT_FOUND, description = "The DID does not exist in the index.", body = ResolutionResult, content_type = "application/did-resolution"),
        (status = GONE, description = "The DID has been deactivated.", body = ResolutionResult, content_type = "application/did-resolution"),
        (status = INTERNAL_SERVER_ERROR, description = "An unexpected error occurred during resolution.", body = ResolutionResult, content_type = "application/did-resolution"),
    ),
    params(
        ("did" = Did, Path, description = "The Decentralized Identifier (DID) to resolve using the Universal Resolver driver.")
    ),
    description = "This endpoint implements the Universal Resolver driver interface. It is intended to be used as a backend for the Universal Resolver proxy, enabling DID resolution via the Universal Resolver ecosystem. The response format and behavior are compatible with Universal Resolver expectations."
)]
pub async fn universal_resolver_did(path: Path<String>, state: State<AppState>) -> impl IntoResponse {
    let (status, mut result) = resolution_logic(path, state).await;
    result.did_resolution_metadata.content_type = result
        .did_resolution_metadata
        .content_type
        .map(|_| "application/did".to_string());
    let body = if result.did_document.is_some() {
        serde_json::to_string(&result.did_document)
    } else {
        serde_json::to_string(&result)
    };
    (
        status,
        [(header::CONTENT_TYPE, "application/did")],
        body.expect("ResolutionResult should always be serializable"),
    )
}

pub async fn resolution_logic(
    Path(did): Path<String>,
    State(state): State<AppState>,
) -> (StatusCode, ResolutionResult) {
    let (result, _) = state.did_service.resolve_did(&did).await;
    let (status, resolution_result) = match result {
        Err(e) => (e.status_code(), e.into()),
        Ok((did, did_state)) => (StatusCode::OK, did_state.to_resolution_result(&did)),
    };
    (status, resolution_result)
}

#[utoipa::path(
    get,
    summary = "Returns the DIDData protobuf message for a given DID, encoded in hexadecimal.",
    path = ApiDidData::AXUM_PATH,
    tags = [tags::OP_INDEX],
    responses(
        (status = OK, description = "Successfully retrieved the DIDData protobuf message, encoded as a hexadecimal string.", body = String),
        (status = BAD_REQUEST, description = "The provided DID is invalid."),
        (status = NOT_FOUND, description = "The DID does not exist in the index."),
        (status = INTERNAL_SERVER_ERROR, description = "An unexpected error occurred while retrieving DIDData."),
    ),
    params(("did" = Did, Path, description = "The Decentralized Identifier (DID) for which to retrieve the DIDData protobuf message.")),
    description = "The returned data is a protobuf message compatible with the legacy prism-node implementation. The object is encoded in hexadecimal format. This endpoint is useful for testing and verifying compatibility with existing operations already anchored on the blockchain."
)]
pub async fn did_data(Path(did): Path<String>, State(state): State<AppState>) -> Result<String, StatusCode> {
    let (result, _) = state.did_service.resolve_did(&did).await;
    match result {
        Err(ResolutionError::InvalidDid { .. }) => Err(StatusCode::BAD_REQUEST),
        Err(ResolutionError::NotFound) => Err(StatusCode::NOT_FOUND),
        Err(ResolutionError::InternalError { .. }) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        Ok((_, did_state)) => {
            let dd: DIDData = did_state.into();
            let bytes = dd.encode_to_vec();
            let hex_str = HexStr::from(bytes);
            Ok(hex_str.to_string())
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
    )
)]
pub async fn indexer_stats(State(state): State<AppState>) -> Result<Json<IndexerStats>, StatusCode> {
    let result = state.did_service.get_indexer_stats().await;
    let stats = match result {
        Ok(None) => IndexerStats {
            last_prism_slot_number: None,
            last_prism_block_number: None,
        },
        Ok(Some((slot, block))) => IndexerStats {
            last_prism_block_number: Some(block),
            last_prism_slot_number: Some(slot),
        },
        Err(e) => {
            // TODO: improve error handling
            tracing::error!("{}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)?
        }
    };
    Ok(Json(stats))
}
