#![allow(non_snake_case)]
#![feature(error_reporter)]

use std::fs;
use std::sync::Arc;

use app::service::DidService;
use clap::Parser;
use cli::Cli;
use identus_did_prism::dlt::{DltCursor, NetworkIdentifier};
use identus_did_prism_indexer::dlt::dbsync::DbSyncSource;
use identus_did_prism_indexer::dlt::oura::OuraN2NSource;
use identus_did_prism_submitter::DltSink;
use identus_did_prism_submitter::dlt::cardano_wallet::CardanoWalletSink;
use lazybe::db::postgres::PostgresDbCtx;
use lazybe::router::RouteConfig;
use node_storage::PostgresDb;
use sqlx::{PgPool, Postgres};
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::app::worker::{DltIndexWorker, DltSyncWorker};
use crate::cli::{DbArgs, DltSinkArgs, DltSourceArgs, IndexerArgs, ServerArgs, StandaloneArgs, SubmitterArgs};

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

#[derive(Clone)]
struct AppState {
    pg_pool: PgPool,
    did_service: DidService,
    dlt_source: Option<DltSourceState>,
    dlt_sink: Option<Arc<dyn DltSink>>,
    run_mode: RunMode,
}

#[derive(Clone)]
struct DltSourceState {
    cursor_rx: tokio::sync::watch::Receiver<Option<DltCursor>>,
    network: NetworkIdentifier,
}

impl RouteConfig for AppState {
    type Ctx = PostgresDbCtx;
    type Db = Postgres;

    fn db_ctx(&self) -> (Self::Ctx, PgPool) {
        (PostgresDbCtx, self.pg_pool.clone())
    }
}

pub async fn run_command() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        cli::Command::Indexer(args) => run_indexer_command(args).await?,
        cli::Command::Submitter(args) => run_submitter_command(args).await?,
        cli::Command::Standalone(args) => run_standalone_command(args).await?,
        cli::Command::GenerateOpenapi(args) => {
            generate_openapi(args)?;
        }
    };
    Ok(())
}

fn generate_openapi(args: crate::cli::GenerateOpenApiArgs) -> anyhow::Result<()> {
    let oas = crate::http::open_api(&RunMode::Standalone);
    let openapi_json = oas.to_pretty_json()?;

    if let Some(path) = args.output {
        fs::write(path, &openapi_json)?;
    } else {
        println!("{openapi_json}");
    }
    Ok(())
}

async fn run_indexer_command(args: IndexerArgs) -> anyhow::Result<()> {
    let db = init_database(&args.db).await;
    let network = args.dlt_source.cardano_network.clone().into();
    let cursor_rx = init_dlt_source(&args.dlt_source, &network, &db).await;
    let app_state = AppState {
        pg_pool: db.pool.clone(),
        run_mode: RunMode::Indexer,
        did_service: DidService::new(&db),
        dlt_source: cursor_rx.map(|cursor_rx| DltSourceState { cursor_rx, network }),
        dlt_sink: None,
    };
    run_server(app_state, &args.server).await
}

async fn run_submitter_command(args: SubmitterArgs) -> anyhow::Result<()> {
    let db = init_database(&args.db).await;
    let dlt_sink = init_dlt_sink(&args.dlt_sink);
    let app_state = AppState {
        pg_pool: db.pool.clone(),
        run_mode: RunMode::Submitter,
        did_service: DidService::new(&db),
        dlt_source: None,
        dlt_sink: Some(dlt_sink),
    };
    run_server(app_state, &args.server).await
}

async fn run_standalone_command(args: StandaloneArgs) -> anyhow::Result<()> {
    let db = init_database(&args.db).await;
    let network = args.dlt_source.cardano_network.clone().into();
    let cursor_rx = init_dlt_source(&args.dlt_source, &network, &db).await;
    let dlt_sink = init_dlt_sink(&args.dlt_sink);
    let app_state = AppState {
        pg_pool: db.pool.clone(),
        run_mode: RunMode::Standalone,
        did_service: DidService::new(&db),
        dlt_source: cursor_rx.map(|cursor_rx| DltSourceState { cursor_rx, network }),
        dlt_sink: Some(dlt_sink),
    };
    run_server(app_state, &args.server).await
}

async fn run_server(app_state: AppState, server_args: &ServerArgs) -> anyhow::Result<()> {
    let layer = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .option_layer(Some(CorsLayer::permissive()).filter(|_| server_args.cors_enabled));
    let router = http::router(&server_args.assets_path, app_state.run_mode)
        .with_state(app_state)
        .layer(layer);
    let bind_addr = format!("{}:{}", server_args.address, server_args.port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("Server is listening on {}", bind_addr);
    axum::serve(listener, router).await?;
    Ok(())
}

async fn init_database(db_args: &DbArgs) -> PostgresDb {
    let db = PostgresDb::connect(&db_args.db_url)
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

async fn init_dlt_source(
    dlt_args: &DltSourceArgs,
    network: &NetworkIdentifier,
    db: &PostgresDb,
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

fn init_dlt_sink(dlt_args: &DltSinkArgs) -> Arc<dyn DltSink> {
    Arc::new(CardanoWalletSink::new(
        dlt_args.cardano_wallet_base_url.to_string(),
        dlt_args.cardano_wallet_wallet_id.to_string(),
        dlt_args.cardano_wallet_passphrase.to_string(),
        dlt_args.cardano_wallet_payment_addr.to_string(),
    ))
}
