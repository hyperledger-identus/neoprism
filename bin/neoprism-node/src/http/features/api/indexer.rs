use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::IntoResponse;
use identus_apollo::hex::HexStr;
use identus_did_core::{Did, DidDocument, ResolutionResult};
use identus_did_prism::proto::MessageExt;
use identus_did_prism::proto::node_api::DIDData;
use serde_json;
use utoipa::OpenApi;

use crate::IndexerState;
use crate::app::service::PrismDidService;
use crate::app::service::error::ResolutionError;
use crate::http::features::api::error::ApiError;
use crate::http::features::api::indexer::models::IndexerStats;
use crate::http::features::api::tags;
use crate::http::urls::{ApiDid, ApiDidData, ApiIndexerStats, ApiVdrBlob, UniversalResolverDid};

#[derive(OpenApi)]
#[openapi(paths(resolve_did, did_data, indexer_stats, uni_driver_resolve_did, resolve_vdr_blob))]
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

fn get_prism_service(state: &IndexerState) -> Result<&PrismDidService, ApiError> {
    state.prism_did_service.as_ref().ok_or(ApiError::NotImplemented)
}

#[utoipa::path(
    get,
    summary = "Resolve a VDR entry and return its blob data.",
    description = "Returns the raw blob data for a VDR entry, using PrismDidService::resolve_vdr. The response is application/octet-stream.",
    path = ApiVdrBlob::AXUM_PATH,
    tags = [tags::OP_INDEX],
    responses(
        (status = OK, description = "Successfully resolved the VDR entry. Returns the blob data.", content_type = "application/octet-stream"),
        (status = NOT_FOUND, description = "The VDR entry was not found.", content_type = "application/json"),
        (status = INTERNAL_SERVER_ERROR, description = "An unexpected error occurred during VDR resolution.", content_type = "application/json"),
    ),
    params(
        ("entry_hash" = String, Path, description = "The hex-encoded entry hash to resolve.")
    ),
)]
pub async fn resolve_vdr_blob(
    Path(entry_hash): Path<String>,
    State(state): State<IndexerState>,
) -> Result<Bytes, ApiError> {
    let service = get_prism_service(&state)?;
    match service.resolve_vdr(&entry_hash).await {
        Ok(Some(blob)) => Ok(Bytes::from(blob)),
        Ok(None) => Err(ApiError::NotFound)?,
        Err(e) => Err(ApiError::Internal { source: e.into() })?,
    }
}

#[utoipa::path(
    get,
    summary = "Resolves a W3C Decentralized Identifier (DID) according to the DID Resolution specification.",
    description = "This endpoint is fully compliant with the W3C DID Resolution specification. It returns a DID Resolution Result object, including metadata and the resolved DID Document, following the standard resolution process.",
    path = ApiDid::AXUM_PATH,
    tags = [tags::OP_INDEX],
    responses(
        (status = OK, description = "Successfully resolved the DID. Returns the DID Resolution Result.", body = ResolutionResult, content_type = "application/did-resolution"),
        (status = BAD_REQUEST, description = "The provided DID is invalid.", body = ResolutionResult, content_type = "application/did-resolution"),
        (status = NOT_FOUND, description = "The DID does not exist in the index.", body = ResolutionResult, content_type = "application/did-resolution"),
        (status = GONE, description = "The DID has been deactivated.", body = ResolutionResult, content_type = "application/did-resolution"),
        (status = INTERNAL_SERVER_ERROR, description = "An unexpected error occurred during resolution.", body = ResolutionResult, content_type = "application/did-resolution"),
        (status = NOT_IMPLEMENTED, description = "A functionality is not implemented.", body = ResolutionResult, content_type = "application/did-resolution"),
    ),
    params(
        ("did" = Did, Path, description = "The Decentralized Identifier (DID) to resolve.")
    ),
)]
pub async fn resolve_did(path: Path<String>, state: State<IndexerState>) -> impl IntoResponse {
    let (status, result) = resolution_http_binding(path, state).await;
    let body = serde_json::to_string(&result).expect("ResolutionResult should always be serializable");
    (status, [(header::CONTENT_TYPE, "application/did-resolution")], body)
}

#[utoipa::path(
    get,
    summary = "Universal Resolver driver endpoint for resolving DIDs, designed for use behind a Universal Resolver proxy.",
    description = "This endpoint implements the Universal Resolver driver interface. If the DID document is present, only the document is returned; otherwise, the full ResolutionResult is returned. The response format and behavior are compatible with Universal Resolver expectations.",
    path = UniversalResolverDid::AXUM_PATH,
    tags = [tags::OP_INDEX],
    responses(
        (status = OK, description = "Successfully resolved the DID. Returns either the DID Document or the DID Resolution Result.", body = DidDocument, content_type = "application/did"),
        (status = BAD_REQUEST, description = "The provided DID is invalid.", body = ResolutionResult, content_type = "application/did"),
        (status = NOT_FOUND, description = "The DID does not exist in the index.", body = ResolutionResult, content_type = "application/did"),
        (status = GONE, description = "The DID has been deactivated.", body = ResolutionResult, content_type = "application/did"),
        (status = INTERNAL_SERVER_ERROR, description = "An unexpected error occurred during resolution.", body = ResolutionResult, content_type = "application/did"),
        (status = NOT_IMPLEMENTED, description = "A functionality is not implemented.", body = ResolutionResult, content_type = "application/did"),
    ),
    params(
        ("did" = Did, Path, description = "The Decentralized Identifier (DID) to resolve using the Universal Resolver driver.")
    )
)]
pub async fn uni_driver_resolve_did(path: Path<String>, state: State<IndexerState>) -> impl IntoResponse {
    let (status, mut result) = resolution_http_binding(path, state).await;
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

#[utoipa::path(
    get,
    summary = "Returns the DIDData protobuf message for a given DID, encoded in hexadecimal.",
    description = "The returned data is a protobuf message compatible with the legacy prism-node implementation. The object is encoded in hexadecimal format. This endpoint is useful for testing and verifying compatibility with existing operations already anchored on the blockchain.",
    path = ApiDidData::AXUM_PATH,
    tags = [tags::OP_INDEX],
    responses(
        (status = OK, description = "Successfully retrieved the DIDData protobuf message, encoded as a hexadecimal string.", body = String),
        (status = BAD_REQUEST, description = "The provided DID is invalid.", content_type = "application/json"),
        (status = NOT_FOUND, description = "The DID does not exist in the index.", content_type = "application/json"),
        (status = INTERNAL_SERVER_ERROR, description = "An unexpected error occurred while retrieving DIDData.", content_type = "application/json"),
        (status = NOT_IMPLEMENTED, description = "A functionality is not implemented.", content_type = "application/json"),
    ),
    params(("did" = Did, Path, description = "The Decentralized Identifier (DID) for which to retrieve the DIDData protobuf message."))
)]
pub async fn did_data(Path(did): Path<String>, State(state): State<IndexerState>) -> Result<String, ApiError> {
    let service = get_prism_service(&state)?;
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
    let service = get_prism_service(&state)?;
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

async fn resolution_http_binding(
    Path(did): Path<String>,
    State(state): State<IndexerState>,
) -> (StatusCode, ResolutionResult) {
    if did.starts_with("did:prism") {
        if let Some(service) = state.prism_did_service {
            let (result, _) = service.resolve_did(&did).await;
            let (status, resolution_result) = match result {
                Err(e) => {
                    e.log_internal_error();
                    (e.status_code(), e.into())
                }
                Ok((did, did_state)) => (StatusCode::OK, did_state.to_resolution_result(&did)),
            };
            return (status, resolution_result);
        }
    } else if did.starts_with("did:midnight") {
        #[cfg(feature = "midnight")]
        if let Some(service) = state.midnight_did_service {
            let result = service.resolve_did(&did).await;
            let (status, resolution_result) = match result {
                Err(e) => {
                    e.log_internal_error();
                    (e.status_code(), e.into())
                }
                Ok(did_doc) => (StatusCode::OK, ResolutionResult::success(did_doc)),
            };
            return (status, resolution_result);
        }
    }

    let e = ResolutionError::MethodNotSupported;
    (e.status_code(), e.into())
}
