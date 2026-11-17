use axum::Router;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use maud::Markup;
use models::PageQuery;

use crate::IndexerUiState;
use crate::http::urls;

mod list;
pub(in crate::http) mod models;
mod views;

pub fn router() -> Router<IndexerUiState> {
    Router::new()
        .route(urls::Explorer::AXUM_PATH, get(index))
        .route(urls::ExplorerDltCursor::AXUM_PATH, get(dlt_cursor))
        .route(urls::ExplorerDidList::AXUM_PATH, get(did_list))
}

async fn index(
    query: Result<Query<PageQuery>, QueryRejection>,
    State(state): State<IndexerUiState>,
) -> (StatusCode, Markup) {
    let network = state.dlt_source.as_ref().map(|i| i.network);
    let cursor = state.dlt_source.as_ref().and_then(|i| i.cursor_rx.borrow().to_owned());
    let (status, list) = query_list(query, &state).await;
    (status, views::index(network, cursor, list))
}

async fn dlt_cursor(State(state): State<IndexerUiState>) -> Markup {
    let cursor = state.dlt_source.as_ref().and_then(|i| i.cursor_rx.borrow().to_owned());
    views::dlt_cursor_card(cursor)
}

async fn did_list(
    query: Result<Query<PageQuery>, QueryRejection>,
    State(state): State<IndexerUiState>,
) -> (StatusCode, Markup) {
    query_list(query, &state).await
}

async fn query_list(query: Result<Query<PageQuery>, QueryRejection>, state: &IndexerUiState) -> (StatusCode, Markup) {
    match query {
        Ok(Query(page)) => retrieve_list(page, state).await,
        Err(_) => (StatusCode::BAD_REQUEST, list::invalid_page()),
    }
}

async fn retrieve_list(query: PageQuery, state: &IndexerUiState) -> (StatusCode, Markup) {
    let page = query.page.unwrap_or(1).max(1);
    // The storage paginator uses a u32 offset. Larger requested pages cannot
    // contain records (the total count is also u32); obtain only real page metadata.
    let offset_overflows = (page - 1).checked_mul(100).is_none();
    let storage_page = if offset_overflows { 0 } else { page - 1 };
    match state.prism_did_service.get_all_dids(Some(storage_page)).await {
        Ok(mut dids) => {
            if offset_overflows {
                dids.items.clear();
                dids.current_page = page - 1;
            }
            (StatusCode::OK, list::render(dids))
        }
        Err(error) => {
            tracing::error!(page, error = ?error, "failed to retrieve indexed DIDs for explorer");
            (StatusCode::SERVICE_UNAVAILABLE, list::unavailable(page))
        }
    }
}
