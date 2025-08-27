use axum::Router;
use axum::extract::{Query, State};
use axum::routing::get;
use maud::Markup;
use models::PageQuery;

use crate::IndexerUiState;
use crate::http::urls;

pub(in crate::http) mod models;
mod views;

pub fn router() -> Router<IndexerUiState> {
    Router::new()
        .route(urls::Explorer::AXUM_PATH, get(index))
        .route(urls::ExplorerDltCursor::AXUM_PATH, get(dlt_cursor))
        .route(urls::ExplorerDidList::AXUM_PATH, get(did_list))
}

async fn index(Query(page): Query<PageQuery>, State(state): State<IndexerUiState>) -> Markup {
    let page = page.page.map(|i| i.max(1) - 1);
    let network = state.dlt_source.as_ref().map(|i| i.network);
    let cursor = state.dlt_source.as_ref().and_then(|i| i.cursor_rx.borrow().to_owned());
    let dids = state.prism_did_service.get_all_dids(page).await.unwrap(); // FIXME: unwrap
    views::index(network, cursor, dids)
}

async fn dlt_cursor(State(state): State<IndexerUiState>) -> Markup {
    let cursor = state.dlt_source.as_ref().and_then(|i| i.cursor_rx.borrow().to_owned());
    views::dlt_cursor_card(cursor)
}

async fn did_list(Query(page): Query<PageQuery>, State(state): State<IndexerUiState>) -> Markup {
    let page = page.page.map(|i| i.max(1) - 1);
    let dids = state.prism_did_service.get_all_dids(page).await.unwrap(); // FIXME: unwrap
    views::did_list(dids)
}
