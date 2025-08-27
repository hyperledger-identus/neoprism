use std::path::Path;

use axum::Router;
use axum::response::Redirect;
use axum::routing::get;
use features::{api, ui_explorer, ui_resolver};
use tower_http::services::ServeDir;

use crate::{AppState, IndexerState, IndexerUiState, RunMode, SubmitterState};

mod components;
mod features;
mod urls;

pub use features::api::open_api;

pub struct Routers {
    pub app_router: Router<AppState>,
    pub indexer_ui_router: Router<IndexerUiState>,
    pub indexer_router: Router<IndexerState>,
    pub submitter_router: Router<SubmitterState>,
}

pub fn router(assets_dir: &Path, mode: RunMode) -> Routers {
    tracing::info!("Serving static asset from {:?}", assets_dir);

    let api_router = api::router(mode);

    let ui_router = Router::new()
        .nest_service(urls::AssetBase::AXUM_PATH, ServeDir::new(assets_dir))
        .merge(ui_explorer::router())
        .merge(ui_resolver::router());

    let home_router = match mode {
        RunMode::Submitter | RunMode::Midnight => Router::new().route(
            urls::Home::AXUM_PATH,
            get(Redirect::temporary(&urls::Swagger::new_uri())),
        ),
        RunMode::Indexer | RunMode::Standalone => Router::new().route(
            urls::Home::AXUM_PATH,
            get(Redirect::temporary(&urls::Resolver::new_uri(None))),
        ),
    };

    Routers {
        app_router: api_router.app_router.merge(home_router),
        indexer_ui_router: api_router.indexer_ui_router.merge(ui_router),
        indexer_router: api_router.indexer_router,
        submitter_router: api_router.submitter_router,
    }
}
