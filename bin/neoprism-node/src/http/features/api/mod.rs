use axum::Router;
use axum::routing::{get, post};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::RunMode;
use crate::http::features::api::indexer::IndexerOpenApiDoc;
use crate::http::features::api::submitter::SubmitterOpenApiDoc;
use crate::http::features::api::system::SystemOpenApiDoc;
use crate::http::{Routers, urls};

mod indexer;
mod submitter;
mod system;

#[derive(OpenApi)]
#[openapi(servers(
    (url = "http://localhost:8080", description = "Local"),
    (url = "https://neoprism.patlo.dev", description = "Public - mainnet"),
    (url = "https://neoprism-preprod.patlo.dev", description = "Public - preprod")
))]
struct BaseOpenApiDoc;

mod tags {
    pub const SYSTEM: &str = "System API";
    pub const OP_INDEX: &str = "Indexer API";
    pub const OP_SUBMIT: &str = "Submitter API";
}

pub fn open_api(mode: &RunMode) -> utoipa::openapi::OpenApi {
    let base_oas = BaseOpenApiDoc::openapi().merge_from(SystemOpenApiDoc::openapi());
    let indexer_oas = IndexerOpenApiDoc::openapi();
    let submitter_oas = SubmitterOpenApiDoc::openapi();

    match mode {
        RunMode::Indexer => base_oas.merge_from(indexer_oas),
        RunMode::Submitter => base_oas.merge_from(submitter_oas),
        RunMode::Standalone => base_oas.merge_from(indexer_oas).merge_from(submitter_oas),
        #[cfg(feature = "midnight")]
        RunMode::Midnight => base_oas.merge_from(indexer_oas),
    }
}

pub fn router(mode: RunMode) -> Routers {
    let oas = open_api(&mode);

    let app_router = Router::new()
        .merge(SwaggerUi::new(urls::Swagger::AXUM_PATH).url("/api/openapi.json", oas))
        .route(urls::ApiHealth::AXUM_PATH, get(system::health))
        .route(urls::ApiAppMeta::AXUM_PATH, get(system::app_meta));

    let indexer_router = Router::new()
        .route(urls::ApiDid::AXUM_PATH, get(indexer::resolve_did))
        .route(urls::ApiDidData::AXUM_PATH, get(indexer::did_data))
        .route(urls::ApiIndexerStats::AXUM_PATH, get(indexer::indexer_stats))
        .route(
            urls::UniversalResolverDid::AXUM_PATH,
            get(indexer::universal_resolver_did),
        );

    let submitter_router = Router::new().route(
        urls::ApiSignedOpSubmissions::AXUM_PATH,
        post(submitter::submit_signed_operations),
    );

    Routers {
        app_router,
        indexer_ui_router: Router::new(),
        indexer_router,
        submitter_router,
    }
}
