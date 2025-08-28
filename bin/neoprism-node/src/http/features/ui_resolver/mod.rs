use axum::Router;
use axum::extract::{Query, State};
use axum::routing::get;
use maud::Markup;
use models::DidQuery;

use crate::IndexerUiState;
use crate::http::urls;

pub(in crate::http) mod models;
mod views;

pub fn router() -> Router<IndexerUiState> {
    Router::new().route(urls::Resolver::AXUM_PATH, get(index))
}

async fn index(Query(query): Query<DidQuery>, State(state): State<IndexerUiState>) -> Markup {
    let network = state.dlt_source.map(|i| i.network);
    match query.did.as_ref() {
        None => views::index(network),
        Some(did_str) => {
            let (state, debug) = state.prism_did_service.resolve_did(did_str).await;
            views::resolve(network, did_str, state, debug)
        }
    }
}
