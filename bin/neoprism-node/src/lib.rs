#![allow(non_snake_case)]
#![feature(error_reporter)]

use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use app::service::PrismDidService;
use axum::Router;
use clap::Parser;
use cli::Cli;
use dirs::data_dir;
use identus_did_prism::dlt::{DltCursor, NetworkIdentifier};
use identus_did_prism_indexer::DltSource;
use identus_did_prism_indexer::dlt::blockfrost::{BlockfrostConfig, BlockfrostSource};
use identus_did_prism_indexer::dlt::dbsync::DbSyncSource;
use identus_did_prism_indexer::dlt::oura::OuraN2NSource;
use identus_did_prism_submitter::DltSink;
use identus_did_prism_submitter::dlt::cardano_wallet::CardanoWalletSink;
use identus_did_resolver_http::DidResolverStateDyn;
use node_storage::{PostgresDb, SqliteDb, StorageBackend};
use tokio::task::JoinSet;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::app::worker::{DltIndexWorker, DltSyncWorker};
use crate::cli::{
    DbArgs, DevArgs, DltSinkArgs, DltSinkType, DltSourceArgs, DltSourceType, IndexerArgs, ServerArgs, StandaloneArgs,
    SubmitterArgs,
};

mod app;
mod cli;
mod http;

/// Return type of [`init_memory_ledger`]: cursor receiver, sink, and worker set.
type MemoryLedger = (
    tokio::sync::watch::Receiver<Option<DltCursor>>,
    Arc<dyn DltSink + Send + Sync + 'static>,
    JoinSet<anyhow::Result<()>>,
);

/// Return type of [`init_dlt_source`]: optional cursor receiver and worker set.
type DltSourceOutput = (
    Option<tokio::sync::watch::Receiver<Option<DltCursor>>>,
    JoinSet<anyhow::Result<()>>,
);

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Copy)]
enum RunMode {
    Indexer,
    Submitter,
    Standalone,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    let network = args.dlt_source.network.cardano_network.clone().into();
    let db = init_database(&args.db, Some(&network)).await;
    let (cursor_rx, mut handles) = init_dlt_source(&args.dlt_source, &network, db.clone()).await;
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
    .await?;
    handles.abort_all();
    Ok(())
}

async fn run_submitter_command(args: SubmitterArgs) -> anyhow::Result<()> {
    let network: NetworkIdentifier = args.network.cardano_network.clone().into();
    let dlt_sink = init_dlt_sink(&args.dlt_sink, &network)?;
    let app_state = AppState {
        run_mode: RunMode::Submitter,
    };
    let submitter_state = SubmitterState { dlt_sink };
    run_server(app_state, None, None, Some(submitter_state), &args.server).await
}

async fn run_standalone_command(args: StandaloneArgs) -> anyhow::Result<()> {
    let network = args.dlt_source.network.cardano_network.clone().into();
    let db = init_database(&args.db, Some(&network)).await;
    let (cursor_rx, mut handles) = init_dlt_source(&args.dlt_source, &network, db.clone()).await;
    let dlt_sink = init_dlt_sink(&args.dlt_sink, &network)?;
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
    .await?;
    handles.abort_all();
    Ok(())
}

async fn run_dev_command(args: DevArgs) -> anyhow::Result<()> {
    let db = init_database(&args.db, Some(&NetworkIdentifier::Custom)).await;
    let (cursor_rx, dlt_sink, mut handles) = init_memory_ledger(db.clone());
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
    .await?;
    handles.abort_all();
    Ok(())
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
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

/// Waits for SIGINT (Ctrl-C) or SIGTERM and returns, allowing the server to
/// drain in-flight connections before exiting.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install CTRL+C signal handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received SIGINT (Ctrl-C), shutting down");
        },
        _ = terminate => {
            tracing::info!("Received SIGTERM, shutting down");
        },
    }
}

async fn init_database(db_args: &DbArgs, network_hint: Option<&NetworkIdentifier>) -> SharedStorage {
    let db_config = resolve_db_config(db_args, network_hint);

    match db_config.backend {
        DbBackend::Postgres => Arc::new(init_postgres_database(&db_config.url, db_args).await),
        DbBackend::Sqlite => init_sqlite_database(&db_config.url, db_args).await,
    }
}

async fn init_postgres_database(db_url: &str, db_args: &DbArgs) -> SharedStorage {
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

    Arc::new(db)
}

fn init_memory_ledger(db: SharedStorage) -> MemoryLedger {
    let (dlt_source, dlt_sink) = identus_did_prism_ledger::in_memory::create_ledger();
    let sync_worker = DltSyncWorker::new(db.clone(), dlt_source);
    let index_worker = DltIndexWorker::new(db.clone(), Duration::from_secs(1));
    let cursor_rx = sync_worker.sync_cursor();
    let mut handles = JoinSet::new();
    handles.spawn(sync_worker.run());
    handles.spawn(index_worker.run());
    (cursor_rx, dlt_sink, handles)
}

/// Helper to spawn sync and index workers from a DLT source.
fn spawn_dlt_workers<Src: DltSource + Send + 'static>(
    db: SharedStorage,
    source: Src,
    index_interval: Duration,
) -> DltSourceOutput {
    let sync_worker = DltSyncWorker::new(db.clone(), source);
    let index_worker = DltIndexWorker::new(db, index_interval);
    let cursor_rx = sync_worker.sync_cursor();
    let mut handles = JoinSet::new();
    handles.spawn(sync_worker.run());
    handles.spawn(index_worker.run());
    (Some(cursor_rx), handles)
}

async fn init_dlt_source(dlt_args: &DltSourceArgs, network: &NetworkIdentifier, db: SharedStorage) -> DltSourceOutput {
    match dlt_args.dlt_source_type {
        DltSourceType::Oura => {
            let address = dlt_args
                .cardano_relay
                .cardano_relay_addr
                .clone()
                .expect("--cardano-relay-addr is required when --dlt-source-type=oura");

            tracing::info!(
                "Starting DLT sync worker on {} from cardano address {}",
                network,
                address
            );
            let source = OuraN2NSource::since_persisted_cursor_or_genesis(
                db.clone(),
                &address,
                network,
                dlt_args.confirmation_blocks,
            )
            .await
            .expect("Failed to create DLT source");

            spawn_dlt_workers(db, source, dlt_args.index_interval)
        }
        DltSourceType::Dbsync => {
            let dbsync_url = dlt_args
                .dbsync
                .cardano_dbsync_url
                .clone()
                .expect("--cardano-dbsync-url is required when --dlt-source-type=dbsync");

            tracing::info!("Starting DLT sync worker on {} from cardano dbsync", network);
            let source = DbSyncSource::since_persisted_cursor(
                db.clone(),
                &dbsync_url,
                dlt_args.confirmation_blocks,
                dlt_args.dbsync.cardano_dbsync_poll_interval,
            )
            .await
            .expect("Failed to create DLT source");

            spawn_dlt_workers(db, source, dlt_args.index_interval)
        }
        DltSourceType::Blockfrost => {
            let api_key = dlt_args
                .blockfrost
                .blockfrost_api_key
                .clone()
                .expect("--blockfrost-api-key is required when --dlt-source-type=blockfrost");

            tracing::info!("Starting DLT sync worker on {} from Blockfrost", network);
            let source = BlockfrostSource::since_persisted_cursor(
                db.clone(),
                &api_key,
                &dlt_args.blockfrost.blockfrost_base_url,
                BlockfrostConfig {
                    confirmation_blocks: dlt_args.confirmation_blocks,
                    poll_interval: dlt_args.blockfrost.blockfrost_poll_interval,
                    concurrency_limit: dlt_args.blockfrost.blockfrost_concurrency_limit,
                    api_delay: dlt_args.blockfrost.blockfrost_api_delay,
                },
            )
            .await
            .expect("Failed to create Blockfrost source");

            spawn_dlt_workers(db, source, dlt_args.index_interval)
        }
    }
}

/// Resolves the mnemonic from either a direct value or a file path.
///
/// Returns an error if both sources are provided (mutually exclusive)
/// or if neither is provided.
fn resolve_mnemonic(mnemonic_value: Option<&str>, mnemonic_file: Option<&Path>) -> anyhow::Result<String> {
    match (mnemonic_value, mnemonic_file) {
        (Some(_), Some(_)) => {
            anyhow::bail!(
                "--embedded-wallet-mnemonic and --embedded-wallet-mnemonic-file are mutually exclusive; provide only one"
            );
        }
        (Some(value), None) => Ok(value.to_string()),
        (None, Some(path)) => {
            let content =
                fs::read_to_string(path).with_context(|| format!("failed to read mnemonic file {}", path.display()))?;
            Ok(content.trim().to_string())
        }
        (None, None) => {
            anyhow::bail!(
                "either --embedded-wallet-mnemonic or --embedded-wallet-mnemonic-file is required when --dlt-sink-type=embedded-wallet"
            );
        }
    }
}

fn init_dlt_sink(
    dlt_args: &DltSinkArgs,
    network: &NetworkIdentifier,
) -> anyhow::Result<Arc<dyn DltSink + Send + Sync>> {
    match dlt_args.dlt_sink_type {
        DltSinkType::CardanoWallet => {
            let cardano_wallet_base_url = dlt_args
                .cardano_wallet
                .cardano_wallet_base_url
                .clone()
                .expect("--cardano-wallet-base-url is required when --dlt-sink-type=cardano-wallet");

            let cardano_wallet_wallet_id = dlt_args
                .cardano_wallet
                .cardano_wallet_wallet_id
                .clone()
                .expect("--cardano-wallet-wallet-id is required when --dlt-sink-type=cardano-wallet");

            let cardano_wallet_passphrase = dlt_args
                .cardano_wallet
                .cardano_wallet_passphrase
                .clone()
                .expect("--cardano-wallet-passphrase is required when --dlt-sink-type=cardano-wallet");

            let cardano_wallet_payment_addr = dlt_args
                .cardano_wallet
                .cardano_wallet_payment_addr
                .clone()
                .expect("--cardano-wallet-payment-addr is required when --dlt-sink-type=cardano-wallet");

            Ok(Arc::new(CardanoWalletSink::new(
                cardano_wallet_base_url,
                cardano_wallet_wallet_id,
                cardano_wallet_passphrase,
                cardano_wallet_payment_addr,
            )))
        }
        DltSinkType::EmbeddedWallet => {
            use identus_did_prism_submitter::dlt::embedded_wallet::{
                EmbeddedWalletSink, EmbeddedWalletSinkConfig, Network,
            };

            let embedded_wallet_bin = dlt_args
                .embedded_wallet
                .embedded_wallet_bin
                .clone()
                .expect("--embedded-wallet-bin is required when --dlt-sink-type=embedded-wallet");

            let submit_api_url = dlt_args.embedded_wallet.embedded_wallet_submit_api_url.clone();

            let blockfrost_url = dlt_args.embedded_wallet.embedded_wallet_blockfrost_url.clone();

            let blockfrost_api_key = dlt_args
                .embedded_wallet
                .embedded_wallet_blockfrost_api_key
                .clone()
                .filter(|s| !s.is_empty());

            let mnemonic = resolve_mnemonic(
                dlt_args.embedded_wallet.embedded_wallet_mnemonic.as_deref(),
                dlt_args.embedded_wallet.embedded_wallet_mnemonic_file.as_deref(),
            )?;

            let network = match network {
                NetworkIdentifier::Mainnet => Network::Mainnet,
                NetworkIdentifier::Preprod => Network::Preprod,
                NetworkIdentifier::Preview => Network::Preview,
                NetworkIdentifier::Custom => Network::Custom,
            };

            let config = EmbeddedWalletSinkConfig {
                embedded_wallet_bin,
                submit_api_url,
                blockfrost_url,
                blockfrost_api_key,
                network,
                mnemonic: Arc::from(mnemonic),
            };

            Ok(Arc::new(EmbeddedWalletSink::new(config)))
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    // --- sqlite_path_from_url ---

    #[test]
    fn sqlite_path_from_url_returns_path_for_absolute_url() {
        let path = sqlite_path_from_url("sqlite:///tmp/neoprism.db");
        assert_eq!(path, Some(PathBuf::from("/tmp/neoprism.db")));
    }

    #[test]
    fn sqlite_path_from_url_strips_query_string() {
        let path = sqlite_path_from_url("sqlite:///tmp/neoprism.db?mode=rwc");
        assert_eq!(path, Some(PathBuf::from("/tmp/neoprism.db")));
    }

    #[test]
    fn sqlite_path_from_url_returns_none_for_in_memory() {
        assert!(sqlite_path_from_url("sqlite://:memory:").is_none());
    }

    #[test]
    fn sqlite_path_from_url_returns_none_for_empty_path() {
        assert!(sqlite_path_from_url("sqlite://").is_none());
    }

    #[test]
    fn sqlite_path_from_url_returns_none_for_non_sqlite_scheme() {
        assert!(sqlite_path_from_url("postgres://localhost/db").is_none());
    }

    // --- infer_db_backend ---

    #[test]
    fn infer_db_backend_detects_postgres_scheme() {
        assert_eq!(infer_db_backend("postgres://localhost/test"), DbBackend::Postgres);
    }

    #[test]
    fn infer_db_backend_detects_postgresql_scheme() {
        assert_eq!(infer_db_backend("postgresql://localhost/test"), DbBackend::Postgres);
    }

    #[test]
    fn infer_db_backend_detects_sqlite_scheme() {
        assert_eq!(infer_db_backend("sqlite:///tmp/test.db"), DbBackend::Sqlite);
    }

    #[test]
    fn infer_db_backend_detects_sqlite_short_scheme() {
        assert_eq!(infer_db_backend("sqlite:/tmp/test.db"), DbBackend::Sqlite);
    }

    #[test]
    #[should_panic(expected = "NPRISM_DB_URL must start with postgres:// or sqlite://")]
    fn infer_db_backend_panics_on_unknown_scheme() {
        infer_db_backend("mysql://localhost/test");
    }

    // --- network_identifier_slug ---

    #[test]
    fn network_slug_mainnet() {
        assert_eq!(network_identifier_slug(&NetworkIdentifier::Mainnet), "mainnet");
    }

    #[test]
    fn network_slug_preprod() {
        assert_eq!(network_identifier_slug(&NetworkIdentifier::Preprod), "preprod");
    }

    #[test]
    fn network_slug_preview() {
        assert_eq!(network_identifier_slug(&NetworkIdentifier::Preview), "preview");
    }

    #[test]
    fn network_slug_custom() {
        assert_eq!(network_identifier_slug(&NetworkIdentifier::Custom), "custom");
    }

    // --- ensure_sqlite_parent / prepare_sqlite_destination ---

    #[test]
    fn ensure_sqlite_parent_creates_missing_directory() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("sub").join("neoprism.db");
        let parent = db_path.parent().unwrap();
        assert!(!parent.exists());
        ensure_sqlite_parent(&db_path).unwrap();
        assert!(parent.exists());
        // On Unix the directory must be private (mode 700).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(parent).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o700, "DB parent directory should be owner-only (0700)");
        }
    }

    #[test]
    fn ensure_sqlite_parent_is_ok_when_directory_already_exists() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("neoprism.db");
        let entry_count_before = fs::read_dir(dir.path()).unwrap().count();
        // Parent already exists — should be a no-op (no new sub-dirs created).
        ensure_sqlite_parent(&db_path).unwrap();
        let entry_count_after = fs::read_dir(dir.path()).unwrap().count();
        assert_eq!(entry_count_before, entry_count_after);
    }

    #[test]
    fn prepare_sqlite_destination_creates_parent_for_sqlite_url() {
        let dir = tempfile::tempdir().unwrap();
        let db_url = format!("sqlite://{}/sub/neoprism.db", dir.path().display());
        prepare_sqlite_destination(&db_url).unwrap();
        assert!(dir.path().join("sub").exists());
    }

    #[test]
    fn prepare_sqlite_destination_is_noop_for_non_sqlite_url() {
        // Should succeed without touching the filesystem.
        prepare_sqlite_destination("postgres://localhost/test").unwrap();
    }

    // --- resolve_mnemonic ---

    #[test]
    fn resolve_mnemonic_from_direct_value() {
        let result = resolve_mnemonic(Some("word1 word2 word3"), None);
        assert_eq!(result.unwrap(), "word1 word2 word3");
    }

    #[test]
    fn resolve_mnemonic_from_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("mnemonic.txt");
        fs::write(&file_path, "word1 word2 word3\n").unwrap();
        let result = resolve_mnemonic(None, Some(&file_path));
        assert_eq!(result.unwrap(), "word1 word2 word3");
    }

    #[test]
    fn resolve_mnemonic_from_file_trims_whitespace() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("mnemonic.txt");
        fs::write(&file_path, "  word1 word2 word3  \n\n").unwrap();
        let result = resolve_mnemonic(None, Some(&file_path));
        assert_eq!(result.unwrap(), "word1 word2 word3");
    }

    #[test]
    fn resolve_mnemonic_conflict_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("mnemonic.txt");
        fs::write(&file_path, "word1 word2\n").unwrap();
        let result = resolve_mnemonic(Some("direct mnemonic"), Some(&file_path));
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("mutually exclusive"),
            "expected mutual-exclusion error, got: {err}"
        );
    }

    #[test]
    fn resolve_mnemonic_neither_provided_returns_error() {
        let result = resolve_mnemonic(None::<&str>, None::<&Path>);
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("required"),
            "expected required-error, got: {err}"
        );
    }

    #[test]
    fn resolve_mnemonic_missing_file_returns_error() {
        let result = resolve_mnemonic(None, Some(Path::new("/nonexistent/mnemonic.txt")));
        assert!(result.is_err(), "expected error for missing file");
    }

    // --- default_sqlite_url ---

    #[test]
    fn default_sqlite_url_produces_sqlite_scheme() {
        let url = default_sqlite_url(Some(&NetworkIdentifier::Mainnet));
        assert!(url.starts_with("sqlite://"), "expected sqlite:// scheme, got: {url}");
    }

    #[test]
    fn default_sqlite_url_includes_network_slug() {
        let url = default_sqlite_url(Some(&NetworkIdentifier::Preprod));
        assert!(url.contains("/preprod/"), "expected /preprod/ in URL, got: {url}");
    }

    #[test]
    fn default_sqlite_url_includes_db_filename() {
        let url = default_sqlite_url(Some(&NetworkIdentifier::Preview));
        assert!(url.contains("neoprism.db"), "expected neoprism.db in URL, got: {url}");
    }

    #[test]
    fn default_sqlite_url_without_network_hint() {
        let url = default_sqlite_url(None);
        assert!(
            url.starts_with("sqlite://") && url.contains("NeoPRISM") && url.contains("neoprism.db"),
            "expected NeoPRISM/neoprism.db in URL, got: {url}"
        );
        // Without a network hint, there should be no network subdirectory
        let path_part = url.strip_prefix("sqlite://").unwrap();
        // The path should contain NeoPRISM directly followed by neoprism.db
        // (no mainnet/preprod/preview/custom segment)
        assert!(
            !path_part.contains("/mainnet/"),
            "should not contain mainnet slug, got: {url}"
        );
    }

    #[test]
    fn default_sqlite_url_creates_parent_directory() {
        // default_sqlite_url ensures the parent directory exists
        let url = default_sqlite_url(Some(&NetworkIdentifier::Custom));
        if let Some(path) = sqlite_path_from_url(&url)
            && let Some(parent) = path.parent()
        {
            assert!(parent.exists(), "parent directory should exist");
        }
    }

    // --- resolve_db_config ---

    #[test]
    fn resolve_db_config_with_postgres_url() {
        let db_args = DbArgs {
            db_url: Some("postgres://user:pass@localhost:5432/testdb".to_string()),
            skip_migration: false,
        };
        let config = resolve_db_config(&db_args, Some(&NetworkIdentifier::Mainnet));
        assert_eq!(config.backend, DbBackend::Postgres);
        assert_eq!(config.url, "postgres://user:pass@localhost:5432/testdb");
    }

    #[test]
    fn resolve_db_config_with_sqlite_url() {
        let db_args = DbArgs {
            db_url: Some("sqlite:///tmp/test.db".to_string()),
            skip_migration: false,
        };
        let config = resolve_db_config(&db_args, Some(&NetworkIdentifier::Mainnet));
        assert_eq!(config.backend, DbBackend::Sqlite);
        assert_eq!(config.url, "sqlite:///tmp/test.db");
    }

    #[test]
    fn resolve_db_config_defaults_to_sqlite_when_no_url() {
        let db_args = DbArgs {
            db_url: None,
            skip_migration: false,
        };
        let config = resolve_db_config(&db_args, Some(&NetworkIdentifier::Preprod));
        assert_eq!(config.backend, DbBackend::Sqlite);
        assert!(config.url.starts_with("sqlite://"));
        assert!(config.url.contains("preprod"));
    }

    // --- generate_openapi ---

    #[test]
    fn generate_openapi_to_stdout() {
        // Test the generate_openapi with no output file (stdout)
        // We verify the inner logic directly (the actual stdout capture is not easy in tests).
        let oas = http::build_openapi(&RunMode::Standalone, 8080, None);
        let json = oas.to_pretty_json().unwrap();
        // Verify it's valid JSON with expected OpenAPI structure
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(
            parsed.get("openapi").is_some(),
            "expected 'openapi' field in OpenAPI JSON"
        );
        assert!(parsed.get("paths").is_some(), "expected 'paths' field in OpenAPI JSON");
    }

    #[test]
    fn generate_openapi_to_file() {
        let dir = tempfile::tempdir().unwrap();
        let output_path = dir.path().join("openapi.json");
        let args = crate::cli::GenerateOpenApiArgs {
            output: Some(output_path.clone()),
        };
        generate_openapi(args).unwrap();
        assert!(output_path.exists(), "output file should be created");
        let content = fs::read_to_string(&output_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed.get("openapi").is_some());
    }

    #[test]
    fn generate_openapi_indexer_mode() {
        let oas = http::build_openapi(&RunMode::Indexer, 8080, None);
        let json = oas.to_pretty_json().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.get("paths").is_some());
    }

    #[test]
    fn generate_openapi_submitter_mode() {
        let oas = http::build_openapi(&RunMode::Submitter, 8080, None);
        let json = oas.to_pretty_json().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.get("paths").is_some());
    }

    #[test]
    fn generate_openapi_with_external_url() {
        let oas = http::build_openapi(&RunMode::Standalone, 9090, Some("https://example.com"));
        let json = oas.to_pretty_json().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let servers = parsed.get("servers").unwrap().as_array().unwrap();
        assert!(
            servers
                .iter()
                .any(|s| s.get("url").unwrap().as_str().unwrap().contains("example.com")),
            "expected external URL in servers"
        );
    }

    // --- init_sqlite_database (in-memory) ---

    #[tokio::test]
    async fn init_sqlite_database_in_memory_skip_migration() {
        let db_args = DbArgs {
            db_url: None,
            skip_migration: true,
        };
        let _db = init_sqlite_database("sqlite::memory:", &db_args).await;
        // Database connected successfully (no panic). Tables may not exist yet
        // since migration was skipped — that is the expected behaviour.
    }

    #[tokio::test]
    async fn init_sqlite_database_in_memory_with_migration() {
        let db_args = DbArgs {
            db_url: None,
            skip_migration: false,
        };
        let db = init_sqlite_database("sqlite::memory:", &db_args).await;
        // Migration should succeed on in-memory SQLite
        assert!(
            db.get_last_indexed_block().await.is_ok(),
            "storage should be usable after migration"
        );
    }

    #[tokio::test]
    async fn init_sqlite_database_file_based() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db_url = format!("sqlite://{}", db_path.display());
        let db_args = DbArgs {
            db_url: None,
            skip_migration: false,
        };
        let db = init_sqlite_database(&db_url, &db_args).await;
        assert!(
            db.get_last_indexed_block().await.is_ok(),
            "file-based SQLite should be usable after migration"
        );
    }

    // --- init_database ---

    #[tokio::test]
    async fn init_database_with_sqlite_url() {
        let db_args = DbArgs {
            db_url: Some("sqlite::memory:".to_string()),
            skip_migration: false,
        };
        let db = init_database(&db_args, Some(&NetworkIdentifier::Custom)).await;
        assert!(
            db.get_last_indexed_block().await.is_ok(),
            "init_database should produce a working SQLite storage"
        );
    }

    #[tokio::test]
    async fn init_database_without_url_defaults_to_sqlite() {
        let db_args = DbArgs {
            db_url: None,
            skip_migration: false,
        };
        let db = init_database(&db_args, Some(&NetworkIdentifier::Custom)).await;
        assert!(
            db.get_last_indexed_block().await.is_ok(),
            "init_database should default to SQLite and be usable"
        );
    }

    // --- init_memory_ledger ---

    #[tokio::test]
    async fn init_memory_ledger_creates_workers() {
        let db_args = DbArgs {
            db_url: Some("sqlite::memory:".to_string()),
            skip_migration: true,
        };
        let db = init_database(&db_args, Some(&NetworkIdentifier::Custom)).await;
        let (cursor_rx, _dlt_sink, mut handles) = init_memory_ledger(db);

        // Cursor should start as None
        let cursor = cursor_rx.borrow().clone();
        assert!(cursor.is_none(), "cursor should start as None");

        // Workers should be spawned (JoinSet should have tasks)
        assert!(!handles.is_empty(), "should have spawned worker tasks");

        // Clean up
        handles.abort_all();
    }

    #[tokio::test]
    async fn init_memory_ledger_dlt_sink_is_usable() {
        let db_args = DbArgs {
            db_url: Some("sqlite::memory:".to_string()),
            skip_migration: true,
        };
        let db = init_database(&db_args, Some(&NetworkIdentifier::Custom)).await;
        let (_cursor_rx, dlt_sink, mut handles) = init_memory_ledger(db);

        // The DLT sink should not be None (it's Arc<dyn DltSink>)
        // Verify it can be cloned
        let sink2 = dlt_sink.clone();
        drop(sink2);

        handles.abort_all();
    }

    // --- RunMode / DbBackend / state structs ---

    #[test]
    fn db_backend_equality() {
        assert_eq!(DbBackend::Postgres, DbBackend::Postgres);
        assert_eq!(DbBackend::Sqlite, DbBackend::Sqlite);
        assert_ne!(DbBackend::Postgres, DbBackend::Sqlite);
    }

    #[test]
    fn run_mode_variants() {
        let _indexer = RunMode::Indexer;
        let _submitter = RunMode::Submitter;
        let _standalone = RunMode::Standalone;
    }

    #[test]
    fn database_config_fields() {
        let config = DatabaseConfig {
            backend: DbBackend::Sqlite,
            url: "sqlite::memory:".to_string(),
        };
        assert_eq!(config.backend, DbBackend::Sqlite);
        assert_eq!(config.url, "sqlite::memory:");
    }
}
