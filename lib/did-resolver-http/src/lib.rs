use std::marker::PhantomData;
use std::str::FromStr;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use identus_did_core::{Did, DidDocument, DidResolutionErrorCode, DidResolver, ResolutionResult};

#[cfg(feature = "openapi")]
const PLACEHOLDER_RESOLVER_PATH: &str = "/placeholder-did-resolver";

pub struct DidResolverHttpBinding {
    pub router: Router<DidResolverStateDyn>,
    #[cfg(feature = "openapi")]
    pub openapi: utoipa::openapi::OpenApi,
}

#[derive(Clone)]
pub struct DidResolverStateDyn {
    resolver: Arc<dyn DidResolver + Send + Sync>,
}

pub fn did_resolver_http_binding(path: &str) -> DidResolverHttpBinding {
    let router = Router::new().route(path, get(did_resolver));

    #[cfg(feature = "openapi")]
    let openapi = {
        #[derive(utoipa::OpenApi)]
        #[openapi(paths(did_resolver))]
        struct OpenApiDoc;

        let mut openapi = <OpenApiDoc as utoipa::OpenApi>::openapi();
        // Replace the placeholder path with the user-provided path.
        // This approach allows us to use the concise macro API, which cannot dynamically accept user input for paths.
        if let Some(path_item) = openapi.paths.get_path_item(PLACEHOLDER_RESOLVER_PATH).cloned() {
            openapi.paths.paths.insert(path.to_string(), path_item);
            openapi.paths.paths.remove(PLACEHOLDER_RESOLVER_PATH);
        }
        openapi
    };

    DidResolverHttpBinding {
        router,
        #[cfg(feature = "openapi")]
        openapi,
    }
}

#[cfg_attr(
    feature = "openapi",
    utoipa::path(
        get,
        summary = "Resolves a W3C Decentralized Identifier (DID) according to the DID Resolution specification.",
        description = "This endpoint is fully compliant with the W3C DID Resolution specification. It returns a DID Resolution Result object, including metadata and the resolved DID Document, following the standard resolution process.\n\nOptional resolution options may be provided as query parameters, but are not yet supported in this implementation.",
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
            ("did" = Did, Path, description = "The Decentralized Identifier (DID) to resolve."),
        ),
    )
)]
pub async fn did_resolver(Path(did): Path<String>, state: State<DidResolverStateDyn>, headers: HeaderMap) -> Response {
    let resolver = &state.resolver;
    let accept = headers
        .get(header::ACCEPT)
        .and_then(|i| i.to_str().ok())
        .map(|i| i.trim());

    let parsed_did = match Did::from_str(&did) {
        Ok(did) => did,
        Err(e) => {
            let result = ResolutionResult::invalid_did(e);
            return HttpBindingResponse::<ResolutionResult>::from(result).into_response();
        }
    };
    let options = Default::default(); // TODO: support resolution options
    let result = resolver.resolve(&parsed_did, &options).await;

    match accept {
        Some("application/did") => HttpBindingResponse::<DidDocument>::from(result).into_response(),
        Some("application/did-resolution") => HttpBindingResponse::<ResolutionResult>::from(result).into_response(),
        _ => HttpBindingResponse::<ResolutionResult>::from(result).into_response(),
    }
}

struct HttpBindingResponse<Format>(ResolutionResult, PhantomData<Format>);

impl From<ResolutionResult> for HttpBindingResponse<ResolutionResult> {
    fn from(value: ResolutionResult) -> Self {
        Self(value, PhantomData)
    }
}

impl From<ResolutionResult> for HttpBindingResponse<DidDocument> {
    fn from(value: ResolutionResult) -> Self {
        Self(value, PhantomData)
    }
}

impl IntoResponse for HttpBindingResponse<ResolutionResult> {
    fn into_response(self) -> Response {
        (
            status_code_from_resolution_result(&self.0),
            [(header::CONTENT_TYPE, "application/did-resolution")],
            Json(self.0),
        )
            .into_response()
    }
}

impl IntoResponse for HttpBindingResponse<DidDocument> {
    fn into_response(self) -> Response {
        (
            status_code_from_resolution_result(&self.0),
            [(header::CONTENT_TYPE, "application/did")],
            Json(self.0.did_document),
        )
            .into_response()
    }
}

fn status_code_from_resolution_result(result: &ResolutionResult) -> StatusCode {
    let error_code = result.did_resolution_metadata.error.as_ref().map(|i| &i.r#type);
    let mut status_code = match error_code {
        None => StatusCode::OK,
        Some(DidResolutionErrorCode::InvalidDid) => StatusCode::BAD_REQUEST,
        Some(DidResolutionErrorCode::InvalidDidUrl) => StatusCode::BAD_REQUEST,
        Some(DidResolutionErrorCode::InvalidOptions) => StatusCode::BAD_REQUEST,
        Some(DidResolutionErrorCode::NotFound) => StatusCode::NOT_FOUND,
        Some(DidResolutionErrorCode::RepresentationNotSupported) => StatusCode::NOT_ACCEPTABLE,
        Some(DidResolutionErrorCode::MethodNotSupported) => StatusCode::NOT_IMPLEMENTED,
        Some(DidResolutionErrorCode::UnsupportedPublicKeyType) => StatusCode::NOT_IMPLEMENTED,
        Some(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };

    if result.did_document_metadata.deactivated == Some(true) {
        status_code = StatusCode::GONE;
    }

    status_code
}
