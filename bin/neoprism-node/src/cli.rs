use std::net::Ipv4Addr;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use identus_did_prism::dlt::NetworkIdentifier;

#[derive(Parser)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Start the node in indexer mode.
    Indexer(IndexerArgs),
    /// Start the node in submitter mode.
    Submitter(SubmitterArgs),
    /// Start the node in standalone mode.
    Standalone(StandaloneArgs),
    /// Generate OpenAPI specification for the API.
    GenerateOpenapi(GenerateOpenApiArgs),
}

#[derive(Args)]
pub struct IndexerArgs {
    #[clap(flatten)]
    pub server: ServerArgs,
    #[clap(flatten)]
    pub db: DbArgs,
    #[clap(flatten)]
    pub dlt_source: DltSourceArgs,
}

#[derive(Args)]
pub struct SubmitterArgs {
    #[clap(flatten)]
    pub server: ServerArgs,
    #[clap(flatten)]
    pub db: DbArgs,
    #[clap(flatten)]
    pub dlt_sink: DltSinkArgs,
}

#[derive(Args)]
pub struct StandaloneArgs {
    #[clap(flatten)]
    pub server: ServerArgs,
    #[clap(flatten)]
    pub db: DbArgs,
    #[clap(flatten)]
    pub dlt_source: DltSourceArgs,
    #[clap(flatten)]
    pub dlt_sink: DltSinkArgs,
}

#[derive(Args)]
pub struct ServerArgs {
    /// Node HTTP server binding address
    #[arg(long, env = "NPRISM_ADDRESS", default_value = "0.0.0.0")]
    pub address: Ipv4Addr,
    /// Node HTTP server listening port
    #[arg(long, short, env = "NPRISM_PORT", default_value_t = 8080)]
    pub port: u16,
    /// The directory containing the web UI assets (CSS, JavaScript files).
    #[arg(long, env = "NPRISM_ASSETS_PATH", default_value = "./bin/neoprism-node/assets")]
    pub assets_path: PathBuf,
    /// Enable permissive CORS (https://docs.rs/tower-http/latest/tower_http/cors/struct.CorsLayer.html#method.permissive)
    #[arg(long, env = "NPRISM_CORS_ENABLED")]
    pub cors_enabled: bool,
}

#[derive(Args)]
pub struct GenerateOpenApiArgs {
    /// Output file for the OpenAPI spec (stdout if not provided)
    #[arg(long)]
    pub output: Option<PathBuf>,
}

#[derive(Args)]
pub struct DbArgs {
    /// Database URL (e.g. postgres://user:pass@host:5432/db)
    #[arg(long, env = "NPRISM_DB_URL")]
    pub db_url: String,
    /// Skip database migration on node startup.
    #[arg(long, env = "NPRISM_SKIP_MIGRATION")]
    pub skip_migration: bool,
}

#[derive(Args)]
pub struct DltSourceArgs {
    /// The Cardano network the node is syncing from.
    #[arg(long, env = "NPRISM_CARDANO_NETWORK", default_value = "mainnet")]
    pub cardano_network: NetworkIdentifierCliOption,
    /// Address of the Cardano relay node to sync from.
    /// If provided, the node will sync events from the Cardano relay node.
    /// (e.g. backbone.mainnet.cardanofoundation.org:3001)
    #[arg(long, env = "NPRISM_CARDANO_RELAY_ADDR", group = "dlt-source")]
    pub cardano_relay_addr: Option<String>,
    /// DB-Sync url.
    /// If provided, the node will sync events from DB Sync.
    /// (e.g. postgres://user:pass@host:5432/db)
    #[arg(long, env = "NPRISM_CARDANO_DBSYNC_URL", group = "dlt-source")]
    pub cardano_dbsync_url: Option<String>,
    /// Number of seconds to wait before polling DB Sync for the next update.
    #[arg(long, env = "NPRISM_CARDANO_DBSYNC_POLL_INTERVAL", default_value_t = 10)]
    pub cardano_dbsync_poll_interval: u64,
    /// Number of seconds to wait before checking for unindexed operations.
    #[arg(long, env = "NPRISM_INDEX_INTERVAL", default_value_t = 10)]
    pub index_interval: u64,
    /// Number of confirmation blocks to wait before considering the block valid.
    #[arg(long, env = "NPRISM_CONFIRMATION_BLOCKS", default_value_t = 112)]
    pub confirmation_blocks: u16,
}

#[derive(Args)]
pub struct DltSinkArgs {
    /// Base url of cardano wallet
    #[arg(long, env = "NPRISM_CARDANO_WALLET_BASE_URL")]
    pub cardano_wallet_base_url: String,
    /// Wallet ID to use for making transactions.
    #[arg(long, env = "NPRISM_CARDANO_WALLET_WALLET_ID")]
    pub cardano_wallet_wallet_id: String,
    /// Passphrase for the wallet
    #[arg(long, env = "NPRISM_CARDANO_WALLET_PASSPHRASE")]
    pub cardano_wallet_passphrase: String,
    /// Payment address for making transactions.
    #[arg(long, env = "NPRISM_CARDANO_WALLET_PAYMENT_ADDR")]
    pub cardano_wallet_payment_addr: String,
}

#[derive(Clone, ValueEnum)]
pub enum NetworkIdentifierCliOption {
    Mainnet,
    Preprod,
    Preview,
}

impl From<NetworkIdentifierCliOption> for NetworkIdentifier {
    fn from(value: NetworkIdentifierCliOption) -> Self {
        match value {
            NetworkIdentifierCliOption::Mainnet => NetworkIdentifier::Mainnet,
            NetworkIdentifierCliOption::Preprod => NetworkIdentifier::Preprod,
            NetworkIdentifierCliOption::Preview => NetworkIdentifier::Preview,
        }
    }
}
