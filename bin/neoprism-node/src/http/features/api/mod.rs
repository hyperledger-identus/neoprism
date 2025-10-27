use axum::Router;
use axum::routing::{get, post};
use identus_did_resolver_http::{HttpBindingOptions, did_resolver_http_binding};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::RunMode;
use crate::http::features::api::indexer::IndexerOpenApiDoc;
use crate::http::features::api::submitter::SubmitterOpenApiDoc;
use crate::http::features::api::system::SystemOpenApiDoc;
use crate::http::{Routers, urls};

mod error;
mod indexer;
mod submitter;
mod system;

#[derive(OpenApi)]
#[openapi(servers(
    (url = "https://neoprism.patlo.dev", description = "Public - mainnet"),
    (url = "https://neoprism-preprod.patlo.dev", description = "Public - preprod")
))]
struct BaseOpenApiDoc;

mod tags {
    pub const SYSTEM: &str = "System API";
    pub const OP_INDEX: &str = "Indexer API";
    pub const OP_SUBMIT: &str = "Submitter API";
}

fn build_openapi_servers(
    port: u16,
    external_url: Option<&str>,
    existing_servers: Option<Vec<utoipa::openapi::Server>>,
) -> Vec<utoipa::openapi::Server> {
    let local_server = utoipa::openapi::ServerBuilder::new()
        .url(format!("http://localhost:{port}"))
        .description(Some("Local"))
        .build();

    let mut servers = vec![local_server];

    if let Some(url) = external_url {
        let external_server = utoipa::openapi::ServerBuilder::new()
            .url(url)
            .description(Some("External"))
            .build();
        servers.push(external_server);
    }

    if let Some(existing) = existing_servers {
        servers.extend(existing);
    }

    servers
}

pub fn build_openapi(mode: &RunMode, port: u16, external_url: Option<&str>) -> utoipa::openapi::OpenApi {
    let did_resolver_oas = did_resolver_http_binding(
        urls::ApiDid::AXUM_PATH,
        HttpBindingOptions {
            openapi_tags: Some(vec![tags::OP_INDEX.to_string()]),
        },
    )
    .openapi;
    let base_oas = BaseOpenApiDoc::openapi().merge_from(SystemOpenApiDoc::openapi());
    let indexer_oas = IndexerOpenApiDoc::openapi().merge_from(did_resolver_oas);
    let submitter_oas = SubmitterOpenApiDoc::openapi();

    let mut merged_oas = match mode {
        RunMode::Indexer => base_oas.merge_from(indexer_oas),
        RunMode::Submitter => base_oas.merge_from(submitter_oas),
        RunMode::Standalone => base_oas.merge_from(indexer_oas).merge_from(submitter_oas),
    };

    let servers = build_openapi_servers(port, external_url, merged_oas.servers.take());
    merged_oas.servers = Some(servers);
    merged_oas
}

pub fn router(mode: RunMode, port: u16, external_url: Option<&str>) -> Routers {
    let oas = build_openapi(&mode, port, external_url);

    let app_router = Router::new()
        .merge(SwaggerUi::new(urls::Swagger::AXUM_PATH).url("/api/openapi.json", oas))
        .route(urls::ApiHealth::AXUM_PATH, get(system::health))
        .route(urls::ApiAppMeta::AXUM_PATH, get(system::app_meta));

    let indexer_router = Router::new()
        .route(urls::ApiDidData::AXUM_PATH, get(indexer::did_data))
        .route(urls::ApiIndexerStats::AXUM_PATH, get(indexer::indexer_stats))
        .route(urls::ApiVdrBlob::AXUM_PATH, get(indexer::resolve_vdr_blob));

    let submitter_router = Router::new().route(
        urls::ApiSignedOpSubmissions::AXUM_PATH,
        post(submitter::submit_signed_operations),
    );

    let did_resolver_router = did_resolver_http_binding(urls::ApiDid::AXUM_PATH, Default::default()).router;

    Routers {
        app_router,
        indexer_router,
        submitter_router,
        did_resolver_router,
        ..Default::default()
    }
}
