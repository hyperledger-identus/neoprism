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
    summary = "W3C DID resolution endpoint",
    path = ApiDid::AXUM_PATH,
    tags = [tags::OP_INDEX],
    responses(
        (status = OK, description = "DID Resolution Result", body = ResolutionResult, content_type = "application/did-resolution"),
        (status = BAD_REQUEST, description = "Invalid DID", body = ResolutionResult, content_type = "application/did-resolution"),
        (status = NOT_FOUND, description = "DID not found", body = ResolutionResult, content_type = "application/did-resolution"),
        (status = GONE, description = "DID deactivated", body = ResolutionResult, content_type = "application/did-resolution"),
        (status = INTERNAL_SERVER_ERROR, description = "Internal server error", body = ResolutionResult, content_type = "application/did-resolution"),
    ),
    params(
        ("did" = Did, Path, description = "The DID to resolve")
    )
)]
pub async fn resolve_did(path: Path<String>, state: State<AppState>) -> impl IntoResponse {
    let (status, result) = resolution_logic(path, state).await;
    let body = serde_json::to_string(&result).expect("ResolutionResult should always be serializable");
    (status, [(header::CONTENT_TYPE, "application/did-resolution")], body)
}

#[utoipa::path(
    get,
    summary = "Universal Resolver driver endpoint for DID resolution",
    path = UniversalResolverDid::AXUM_PATH,
    tags = [tags::UNI_RESOLVER],
    responses(
        (status = OK, description = "DID Resolution Result", body = ResolutionResult, content_type = "application/did-resolution"),
        (status = BAD_REQUEST, description = "Invalid DID", body = ResolutionResult, content_type = "application/did-resolution"),
        (status = NOT_FOUND, description = "DID not found", body = ResolutionResult, content_type = "application/did-resolution"),
        (status = GONE, description = "DID deactivated", body = ResolutionResult, content_type = "application/did-resolution"),
        (status = INTERNAL_SERVER_ERROR, description = "Internal server error", body = ResolutionResult, content_type = "application/did-resolution"),
    ),
    params(
        ("did" = Did, Path, description = "The DID to resolve")
    )
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
    summary = "Adapter for returning DIDData protobuf message",
    path = ApiDidData::AXUM_PATH,
    tags = [tags::OP_INDEX],
    responses(
        (status = OK, description = "DIDData proto message in hexacedimal format", body = String),
        (status = BAD_REQUEST, description = "Invalid DID"),
        (status = NOT_FOUND, description = "DID not found"),
        (status = INTERNAL_SERVER_ERROR, description = "Internal server error"),
    ),
    params(("did" = Did, Path, description = "The DID to resolve"))
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
    path = ApiIndexerStats::AXUM_PATH,
    tags = [tags::OP_INDEX],
    responses(
        (status = OK, description = "DIDData proto message in hexacedimal format", body = IndexerStats),
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
