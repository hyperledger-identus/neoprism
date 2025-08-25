use std::path::Path;

use axum::Router;
use axum::response::Redirect;
use axum::routing::get;
use features::{api, ui_explorer, ui_resolver};
use tower_http::services::ServeDir;

use crate::{AppState, IndexerState, RunMode, SubmitterState};

mod components;
mod features;
mod urls;

pub use features::api::open_api;

pub struct AggregateRouter {
    app_router: Router<AppState>,
    indexer_router: Router<IndexerState>,
    submitter_router: Router<SubmitterState>,
}

pub fn router(assets_dir: &Path, mode: RunMode) -> AggregateRouter {
    tracing::info!("Serving static asset from {:?}", assets_dir);

    let api_router = api::router(mode);

    let ui_router = Router::new()
        .nest_service(urls::AssetBase::AXUM_PATH, ServeDir::new(assets_dir))
        .merge(ui_explorer::router())
        .merge(ui_resolver::router());

    match mode {
        RunMode::Submitter => Router::new()
            .route(
                urls::Home::AXUM_PATH,
                get(Redirect::temporary(&urls::Swagger::new_uri())),
            )
            .merge(api_router),
        RunMode::Indexer | RunMode::Standalone => Router::new()
            .route(
                urls::Home::AXUM_PATH,
                get(Redirect::temporary(&urls::Resolver::new_uri(None))),
            )
            .merge(api_router)
            .merge(ui_router),
    }
}
