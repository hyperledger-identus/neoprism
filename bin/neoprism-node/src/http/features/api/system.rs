use axum::Json;
use axum::extract::State;
use utoipa::OpenApi;

use crate::http::features::api::system::models::AppMeta;
use crate::http::features::api::error::ApiErrorResponseBody;
use crate::http::features::api::tags;
use crate::http::urls;
use crate::{AppState, VERSION};

#[derive(OpenApi)]
#[openapi(paths(health, app_meta))]
pub struct SystemOpenApiDoc;

mod models {
    use serde::{Deserialize, Serialize};
    use utoipa::ToSchema;

    use crate::RunMode;

    #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
    pub struct AppMeta {
        pub version: String,
        pub mode: AppMetaRunMode,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
    pub enum AppMetaRunMode {
        Indexer,
        Submitter,
        Standalone,
        #[cfg(feature = "midnight")]
        Midnight,
    }

    impl From<RunMode> for AppMetaRunMode {
        fn from(value: RunMode) -> Self {
            match value {
                RunMode::Indexer => Self::Indexer,
                RunMode::Submitter => Self::Submitter,
                RunMode::Standalone => Self::Standalone,
                #[cfg(feature = "midnight")]
                RunMode::Midnight => Self::Midnight,
            }
        }
    }
}

#[utoipa::path(
    get,
    path = urls::ApiHealth::AXUM_PATH,
    tags = [tags::SYSTEM],
    responses(
        (status = OK, description = "Healthy", body = String, example = "Ok"),
    )
)]
pub async fn health() -> &'static str {
    "Ok"
}

#[utoipa::path(
    get,
    path = urls::ApiAppMeta::AXUM_PATH,
    tags = [tags::SYSTEM],
    responses(
        (status = OK, description = "Healthy", body = AppMeta),
        (status = INTERNAL_SERVER_ERROR, description = "An unexpected error occurred while retrieving app metadata", body = ApiErrorResponseBody),
    )
)]
pub async fn app_meta(State(state): State<AppState>) -> Json<AppMeta> {
    Json(AppMeta {
        version: VERSION.to_string(),
        mode: state.run_mode.into(),
    })
}
