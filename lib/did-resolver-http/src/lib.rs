use std::str::FromStr;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use identus_did_core::{Did, DidResolutionErrorCode, DidResolver, ResolutionResult};
use utoipa::OpenApi;

const PLACEHOLDER_RESOLVER_PATH: &str = "/placeholder-did-resolver";

pub struct DidResolverHttpBinding {
    pub router: Router<DidResolverStateDyn>,
    pub openapi: utoipa::openapi::OpenApi,
}

#[derive(Clone)]
pub struct DidResolverStateDyn {
    resolver: Arc<dyn DidResolver + Send + Sync>,
}

pub fn make_resolver_http_binding(path: &str) -> DidResolverHttpBinding {
    let router = Router::new().route(path, get(did_resolver));

    #[derive(utoipa::OpenApi)]
    #[openapi(paths(did_resolver))]
    struct OpenApiDoc;

    let mut openapi = OpenApiDoc::openapi();

    // Replace the placeholder path with the user-provided path.
    // This approach allows us to use the concise macro API, which cannot dynamically accept user input for paths.
    if let Some(path_item) = openapi.paths.get_path_item(PLACEHOLDER_RESOLVER_PATH).cloned() {
        openapi.paths.paths.insert(path.to_string(), path_item.clone());
        openapi.paths.paths.remove(PLACEHOLDER_RESOLVER_PATH);
    }

    DidResolverHttpBinding { router, openapi }
}

#[utoipa::path(
    get,
    summary = "Resolves a W3C Decentralized Identifier (DID) according to the DID Resolution specification.",
    description = "This endpoint is fully compliant with the W3C DID Resolution specification. It returns a DID Resolution Result object, including metadata and the resolved DID Document, following the standard resolution process.",
    path = PLACEHOLDER_RESOLVER_PATH,
    responses(
        (status = OK, description = "Successfully resolved the DID. Returns the DID Resolution Result.", body = ResolutionResult, content_type = "application/did-resolution"),
        (status = BAD_REQUEST, description = "The provided DID is invalid.", body = ResolutionResult, content_type = "application/did-resolution"),
        (status = NOT_FOUND, description = "The DID does not exist or not found.", body = ResolutionResult, content_type = "application/did-resolution"),
        (status = GONE, description = "The DID has been deactivated.", body = ResolutionResult, content_type = "application/did-resolution"),
        (status = INTERNAL_SERVER_ERROR, description = "An unexpected error occurred during resolution.", body = ResolutionResult, content_type = "application/did-resolution"),
        (status = NOT_IMPLEMENTED, description = "A functionality is not implemented.", body = ResolutionResult, content_type = "application/did-resolution"),
    ),
    params(
        ("did" = Did, Path, description = "The Decentralized Identifier (DID) to resolve.")
    ),
)]
pub async fn did_resolver(Path(did): Path<String>, state: State<DidResolverStateDyn>) -> HttpBinding<ResolutionResult> {
    let resolver = &state.resolver;
    let parsed_did = match Did::from_str(&did) {
        Ok(did) => did,
        Err(e) => return ResolutionResult::invalid_did(e).into(),
    };
    let options = Default::default(); // TODO: support resolution options
    let result = resolver.resolve(&parsed_did, &options).await;
    result.into()
}

#[derive(derive_more::From)]
pub struct HttpBinding<T>(#[from] T);

impl IntoResponse for HttpBinding<ResolutionResult> {
    fn into_response(self) -> Response {
        let error_code = self.0.did_resolution_metadata.error.as_ref().map(|i| &i.r#type);

        let mut status_code = match error_code {
            Some(DidResolutionErrorCode::InvalidDid) => StatusCode::BAD_REQUEST,
            Some(DidResolutionErrorCode::InvalidDidUrl) => StatusCode::BAD_REQUEST,
            Some(DidResolutionErrorCode::InvalidOptions) => StatusCode::BAD_REQUEST,
            Some(DidResolutionErrorCode::NotFound) => StatusCode::NOT_FOUND,
            Some(DidResolutionErrorCode::RepresentationNotSupported) => StatusCode::NOT_ACCEPTABLE,
            Some(DidResolutionErrorCode::MethodNotSupported) => StatusCode::NOT_IMPLEMENTED,
            Some(DidResolutionErrorCode::UnsupportedPublicKeyType) => StatusCode::NOT_IMPLEMENTED,
            Some(_) => StatusCode::INTERNAL_SERVER_ERROR,
            None => StatusCode::OK,
        };

        if self.0.did_document_metadata.deactivated == Some(true) {
            status_code = StatusCode::GONE;
        }

        (
            status_code,
            [(header::CONTENT_TYPE, "application/did-resolution")],
            Json(self.0),
        )
            .into_response()
    }
}
