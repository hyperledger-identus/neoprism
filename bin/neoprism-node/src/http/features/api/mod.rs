use axum::Router;
use axum::routing::{get, post};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::http::features::api::indexer::IndexerOpenApiDoc;
use crate::http::features::api::submitter::SubmitterOpenApiDoc;
use crate::http::features::api::system::SystemOpenApiDoc;
use crate::http::urls;
use crate::{AppState, RunMode};

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
    pub const SYSTEM: &str = "System";
    pub const OP_INDEX: &str = "PRISM indexer";
    pub const OP_SUBMIT: &str = "PRISM submitter";
}

pub fn router(mode: RunMode) -> Router<AppState> {
    let base_oas = BaseOpenApiDoc::openapi().merge_from(SystemOpenApiDoc::openapi());
    let indexer_oas = IndexerOpenApiDoc::openapi();
    let submitter_oas = SubmitterOpenApiDoc::openapi();

    let oas = match mode {
        RunMode::Indexer => base_oas.merge_from(indexer_oas),
        RunMode::Submitter => base_oas.merge_from(submitter_oas),
        RunMode::Standalone => base_oas.merge_from(indexer_oas).merge_from(submitter_oas),
    };

    let base_router = Router::new()
        .merge(SwaggerUi::new(urls::Swagger::AXUM_PATH).url("/api/openapi.json", oas))
        .route(urls::ApiHealth::AXUM_PATH, get(system::health))
        .route(urls::ApiAppMeta::AXUM_PATH, get(system::app_meta));

    let indexer_router = Router::new()
        .route(urls::ApiDid::AXUM_PATH, get(indexer::resolve_did))
        .route(urls::ApiDidData::AXUM_PATH, get(indexer::did_data))
        .route(urls::ApiIndexerStats::AXUM_PATH, get(indexer::indexer_stats));

    let submitter_router = Router::new().route(
        urls::ApiSignedOpSubmissions::AXUM_PATH,
        post(submitter::submit_signed_operations),
    );

    match mode {
        RunMode::Indexer => base_router.merge(indexer_router),
        RunMode::Submitter => base_router.merge(submitter_router),
        RunMode::Standalone => base_router.merge(indexer_router).merge(submitter_router),
    }
}
