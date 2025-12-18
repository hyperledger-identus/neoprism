#![allow(non_snake_case)]
#![feature(error_reporter)]

use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use app::service::PrismDidService;
use axum::Router;
use clap::Parser;
use cli::Cli;
use dirs::data_dir;
use identus_did_prism::dlt::{DltCursor, NetworkIdentifier};
use identus_did_prism_indexer::dlt::dbsync::DbSyncSource;
use identus_did_prism_indexer::dlt::oura::OuraN2NSource;
use identus_did_prism_submitter::DltSink;
use identus_did_prism_submitter::dlt::cardano_wallet::CardanoWalletSink;
use identus_did_resolver_http::DidResolverStateDyn;
use node_storage::{PostgresDb, SqliteDb, StorageBackend};
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::app::worker::{DltIndexWorker, DltSyncWorker};
use crate::cli::{DbArgs, DevArgs, DltSinkArgs, DltSourceArgs, IndexerArgs, ServerArgs, StandaloneArgs, SubmitterArgs};

mod app;
mod cli;
mod http;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Copy)]
enum RunMode {
    Indexer,
    Submitter,
    Standalone,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DbBackend {
    Postgres,
    Sqlite,
}

type SharedStorage = Arc<dyn StorageBackend>;

#[derive(Clone)]
struct AppState {
    run_mode: RunMode,
}

#[derive(Clone)]
struct IndexerState {
    prism_did_service: PrismDidService,
}

impl IndexerState {
    fn to_did_resolver_state_dyn(&self) -> DidResolverStateDyn {
        DidResolverStateDyn {
            resolver: Arc::new(self.prism_did_service.clone()),
        }
    }
}

#[derive(Clone)]
struct IndexerUiState {
    prism_did_service: PrismDidService,
    dlt_source: Option<DltSourceState>,
}

#[derive(Clone)]
struct SubmitterState {
    dlt_sink: Arc<dyn DltSink + Send + Sync>,
}

#[derive(Clone)]
struct DltSourceState {
    cursor_rx: tokio::sync::watch::Receiver<Option<DltCursor>>,
    network: NetworkIdentifier,
}

pub async fn run_command() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        cli::Command::Indexer(args) => run_indexer_command(args).await?,
        cli::Command::Submitter(args) => run_submitter_command(args).await?,
        cli::Command::Standalone(args) => run_standalone_command(args).await?,
        cli::Command::Dev(args) => run_dev_command(args).await?,
        cli::Command::GenerateOpenapi(args) => generate_openapi(args)?,
    };
    Ok(())
}

fn generate_openapi(args: crate::cli::GenerateOpenApiArgs) -> anyhow::Result<()> {
    let oas = crate::http::build_openapi(&RunMode::Standalone, 8080, None);
    let openapi_json = oas.to_pretty_json()?;

    if let Some(path) = args.output {
        fs::write(path, &openapi_json)?;
    } else {
        println!("{openapi_json}");
    }
    Ok(())
}

async fn run_indexer_command(args: IndexerArgs) -> anyhow::Result<()> {
    let network = args.dlt_source.cardano_network.clone().into();
    let db = init_database(&args.db, Some(&network)).await;
    let cursor_rx = init_dlt_source(&args.dlt_source, &network, db.clone()).await;
    let app_state = AppState {
        run_mode: RunMode::Indexer,
    };
    let indexer_state = IndexerState {
        prism_did_service: PrismDidService::new(db.clone()),
    };
    let indexer_ui_state = IndexerUiState {
        prism_did_service: PrismDidService::new(db.clone()),
        dlt_source: cursor_rx.map(|cursor_rx| DltSourceState { cursor_rx, network }),
    };
    run_server(
        app_state,
        Some(indexer_ui_state),
        Some(indexer_state),
        None,
        &args.server,
    )
    .await
}

async fn run_submitter_command(args: SubmitterArgs) -> anyhow::Result<()> {
    let dlt_sink = init_dlt_sink(&args.dlt_sink);
    let app_state = AppState {
        run_mode: RunMode::Submitter,
    };
    let submitter_state = SubmitterState { dlt_sink };
    run_server(app_state, None, None, Some(submitter_state), &args.server).await
}

async fn run_standalone_command(args: StandaloneArgs) -> anyhow::Result<()> {
    let network = args.dlt_source.cardano_network.clone().into();
    let db = init_database(&args.db, Some(&network)).await;
    let cursor_rx = init_dlt_source(&args.dlt_source, &network, db.clone()).await;
    let dlt_sink = init_dlt_sink(&args.dlt_sink);
    let app_state = AppState {
        run_mode: RunMode::Standalone,
    };
    let indexer_state = IndexerState {
        prism_did_service: PrismDidService::new(db.clone()),
    };
    let indexer_ui_state = IndexerUiState {
        prism_did_service: PrismDidService::new(db.clone()),
        dlt_source: cursor_rx.map(|cursor_rx| DltSourceState { cursor_rx, network }),
    };
    let submitter_state = SubmitterState { dlt_sink };
    run_server(
        app_state,
        Some(indexer_ui_state),
        Some(indexer_state),
        Some(submitter_state),
        &args.server,
    )
    .await
}

async fn run_dev_command(args: DevArgs) -> anyhow::Result<()> {
    let db = init_database(&args.db, Some(&NetworkIdentifier::Custom)).await;
    let (cursor_rx, dlt_sink) = init_memory_ledger(db.clone());
    let app_state = AppState {
        run_mode: RunMode::Standalone,
    };
    let indexer_state = IndexerState {
        prism_did_service: PrismDidService::new(db.clone()),
    };
    let indexer_ui_state = IndexerUiState {
        prism_did_service: PrismDidService::new(db.clone()),
        dlt_source: Some(DltSourceState {
            cursor_rx,
            network: NetworkIdentifier::Custom,
        }),
    };
    let submitter_state = SubmitterState { dlt_sink };
    run_server(
        app_state,
        Some(indexer_ui_state),
        Some(indexer_state),
        Some(submitter_state),
        &args.server,
    )
    .await
}

async fn run_server(
    app_state: AppState,
    indexer_ui_state: Option<IndexerUiState>,
    indexer_state: Option<IndexerState>,
    submitter_state: Option<SubmitterState>,
    server_args: &ServerArgs,
) -> anyhow::Result<()> {
    let layer = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .option_layer(Some(CorsLayer::permissive()).filter(|_| server_args.cors_enabled));
    let routers = http::router(
        &server_args.assets_path,
        app_state.run_mode,
        server_args.port,
        server_args.external_url.as_deref(),
    );
    let router = Router::new()
        .merge(routers.app_router.with_state(app_state))
        .merge(
            indexer_state
                .as_ref()
                .map(|s| routers.did_resolver_router.with_state(s.to_did_resolver_state_dyn()))
                .unwrap_or_default(),
        )
        .merge(
            indexer_state
                .map(|s| routers.indexer_router.with_state(s))
                .unwrap_or_default(),
        )
        .merge(
            submitter_state
                .map(|s| routers.submitter_router.with_state(s))
                .unwrap_or_default(),
        )
        .merge(
            indexer_ui_state
                .map(|s| routers.indexer_ui_router.with_state(s))
                .unwrap_or_default(),
        )
        .layer(layer);
    let bind_addr = format!("{}:{}", server_args.address, server_args.port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("Server is listening on {}", bind_addr);
    axum::serve(listener, router).await?;
    Ok(())
}

async fn init_database(db_args: &DbArgs, network_hint: Option<&NetworkIdentifier>) -> SharedStorage {
    let db_config = resolve_db_config(db_args, network_hint);

    match db_config.backend {
        DbBackend::Postgres => Arc::new(init_postgres_database(&db_config.url, db_args).await),
        DbBackend::Sqlite => init_sqlite_database(&db_config.url, db_args).await,
    }
}

async fn init_postgres_database(db_url: &str, db_args: &DbArgs) -> PostgresDb {
    let db = PostgresDb::connect(db_url)
        .await
        .expect("Unable to connect to database");

    if db_args.skip_migration {
        tracing::info!("Skipping database migrations");
    } else {
        tracing::info!("Applying database migrations");
        db.migrate().await.expect("Failed to apply migrations");
        tracing::info!("Applied database migrations successfully");
    }

    db
}

fn init_memory_ledger(
    db: SharedStorage,
) -> (
    tokio::sync::watch::Receiver<Option<DltCursor>>,
    Arc<dyn DltSink + Send + Sync + 'static>,
) {
    let (dlt_source, dlt_sink) = identus_did_prism_ledger::in_memory::create_ledger();
    let sync_worker = DltSyncWorker::new(db.clone(), dlt_source);
    let index_worker = DltIndexWorker::new(db.clone(), 1);
    let cursor_rx = sync_worker.sync_cursor();
    tokio::spawn(sync_worker.run());
    tokio::spawn(index_worker.run());
    (cursor_rx, dlt_sink)
}

async fn init_dlt_source(
    dlt_args: &DltSourceArgs,
    network: &NetworkIdentifier,
    db: SharedStorage,
) -> Option<tokio::sync::watch::Receiver<Option<DltCursor>>> {
    if let Some(address) = &dlt_args.cardano_relay_addr {
        tracing::info!(
            "Starting DLT sync worker on {} from cardano address {}",
            network,
            address
        );
        let source = OuraN2NSource::since_persisted_cursor_or_genesis(
            db.clone(),
            address,
            network,
            dlt_args.confirmation_blocks,
        )
        .await
        .expect("Failed to create DLT source");

        let sync_worker = DltSyncWorker::new(db.clone(), source);
        let index_worker = DltIndexWorker::new(db.clone(), dlt_args.index_interval);
        let cursor_rx = sync_worker.sync_cursor();
        tokio::spawn(sync_worker.run());
        tokio::spawn(index_worker.run());
        Some(cursor_rx)
    } else if let Some(dbsync_url) = dlt_args.cardano_dbsync_url.as_ref() {
        tracing::info!("Starting DLT sync worker on {} from cardano dbsync", network);
        let source = DbSyncSource::since_persisted_cursor(
            db.clone(),
            dbsync_url,
            dlt_args.confirmation_blocks,
            dlt_args.cardano_dbsync_poll_interval,
        )
        .await
        .expect("Failed to create DLT source");

        let sync_worker = DltSyncWorker::new(db.clone(), source);
        let index_worker = DltIndexWorker::new(db.clone(), dlt_args.index_interval);
        let cursor_rx = sync_worker.sync_cursor();
        tokio::spawn(sync_worker.run());
        tokio::spawn(index_worker.run());
        Some(cursor_rx)
    } else {
        None
    }
}

fn init_dlt_sink(dlt_args: &DltSinkArgs) -> Arc<dyn DltSink + Send + Sync> {
    Arc::new(CardanoWalletSink::new(
        dlt_args.cardano_wallet_base_url.to_string(),
        dlt_args.cardano_wallet_wallet_id.to_string(),
        dlt_args.cardano_wallet_passphrase.to_string(),
        dlt_args.cardano_wallet_payment_addr.to_string(),
    ))
}

async fn init_sqlite_database(db_url: &str, db_args: &DbArgs) -> SharedStorage {
    let db_url = db_url.to_string();

    prepare_sqlite_destination(&db_url).expect("Failed to prepare sqlite database path");

    let db = SqliteDb::connect(&db_url)
        .await
        .expect("Unable to connect to sqlite database");

    if db_args.skip_migration {
        tracing::info!("Skipping database migrations");
    } else {
        tracing::info!("Applying database migrations");
        db.migrate().await.expect("Failed to apply migrations");
        tracing::info!("Applied database migrations successfully");
    }

    Arc::new(db)
}

fn default_sqlite_url(network_hint: Option<&NetworkIdentifier>) -> String {
    let mut base = data_dir().unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    base.push("NeoPRISM");
    if let Some(network) = network_hint {
        base.push(network_identifier_slug(network));
    }
    base.push("neoprism.db");
    ensure_sqlite_parent(&base).expect("Failed to prepare sqlite data directory");
    format!("sqlite://{}", base.to_string_lossy())
}

fn prepare_sqlite_destination(db_url: &str) -> std::io::Result<()> {
    if let Some(path) = sqlite_path_from_url(db_url) {
        ensure_sqlite_parent(&path)?;
    }
    Ok(())
}

fn ensure_sqlite_parent(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        if parent.exists() {
            return Ok(());
        }

        fs::create_dir_all(parent)?;
        #[cfg(unix)]
        {
            fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
        }
    }
    Ok(())
}

fn sqlite_path_from_url(db_url: &str) -> Option<PathBuf> {
    const SQLITE_SCHEME: &str = "sqlite://";
    let rest = db_url.strip_prefix(SQLITE_SCHEME)?;
    let path_part = rest.split('?').next().unwrap_or_default();
    if path_part.is_empty() || path_part.starts_with(':') {
        return None;
    }
    Some(Path::new(path_part).to_path_buf())
}

fn resolve_db_config(db_args: &DbArgs, network_hint: Option<&NetworkIdentifier>) -> DatabaseConfig {
    if let Some(db_url) = &db_args.db_url {
        let backend = infer_db_backend(db_url);
        return DatabaseConfig {
            backend,
            url: db_url.clone(),
        };
    }

    let url = default_sqlite_url(network_hint);
    tracing::info!("NPRISM_DB_URL not set, defaulting to embedded SQLite at {}", url);
    DatabaseConfig {
        backend: DbBackend::Sqlite,
        url,
    }
}

fn infer_db_backend(db_url: &str) -> DbBackend {
    if db_url.starts_with("postgres://") || db_url.starts_with("postgresql://") {
        return DbBackend::Postgres;
    }
    if db_url.starts_with("sqlite://") || db_url.starts_with("sqlite:") {
        return DbBackend::Sqlite;
    }

    panic!("NPRISM_DB_URL must start with postgres:// or sqlite://");
}

struct DatabaseConfig {
    backend: DbBackend,
    url: String,
}

fn network_identifier_slug(network: &NetworkIdentifier) -> &'static str {
    match network {
        NetworkIdentifier::Mainnet => "mainnet",
        NetworkIdentifier::Preprod => "preprod",
        NetworkIdentifier::Preview => "preview",
        NetworkIdentifier::Custom => "custom",
    }
}
