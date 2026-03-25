use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::time::Duration;

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
    /// Start the node in standalone mode with an in-memory blockchain for development.
    Dev(DevArgs),
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
    pub network: NetworkArgs,
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
pub struct DevArgs {
    #[clap(flatten)]
    pub server: ServerArgs,
    #[clap(flatten)]
    pub db: DbArgs,
}

#[derive(Args)]
pub struct GenerateOpenApiArgs {
    /// Output file for the OpenAPI spec (stdout if not provided)
    #[arg(long)]
    pub output: Option<PathBuf>,
}

#[derive(Args)]
#[command(next_help_heading = "Server")]
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
    /// External URL for Swagger server list (e.g. https://example.com)
    #[arg(long, env = "NPRISM_EXTERNAL_URL")]
    pub external_url: Option<String>,
}

#[derive(Args)]
#[command(next_help_heading = "Database")]
pub struct DbArgs {
    /// Database URL (e.g. postgres://user:pass@host:5432/db or sqlite:///path/to/db). Defaults to an embedded SQLite file when omitted.
    #[arg(long, env = "NPRISM_DB_URL")]
    pub db_url: Option<String>,
    /// Skip database migration on node startup.
    #[arg(long, env = "NPRISM_SKIP_MIGRATION")]
    pub skip_migration: bool,
}

#[derive(Args)]
#[command(next_help_heading = "Network")]
pub struct NetworkArgs {
    /// The Cardano network (mainnet, preprod, preview, or custom).
    #[arg(long, env = "NPRISM_CARDANO_NETWORK", default_value = "mainnet")]
    pub cardano_network: NetworkIdentifierCliOption,
}

/// Type of DLT source to use for event synchronization.
#[derive(Clone, Debug, ValueEnum)]
pub enum DltSourceType {
    #[value(name = "oura")]
    Oura,
    #[value(name = "dbsync")]
    Dbsync,
    #[value(name = "blockfrost")]
    Blockfrost,
}

#[derive(Args)]
#[command(next_help_heading = "Oura")]
pub struct OuraArgs {
    /// Address of the Cardano relay node (e.g. backbone.mainnet.cardanofoundation.org:3001)
    #[arg(long, env = "NPRISM_CARDANO_RELAY_ADDR")]
    pub cardano_relay_addr: Option<String>,
}

#[derive(Args)]
#[command(next_help_heading = "DB-Sync")]
pub struct DbSyncArgs {
    /// DB-Sync URL (e.g. postgres://user:pass@host:5432/db)
    #[arg(long, env = "NPRISM_CARDANO_DBSYNC_URL")]
    pub cardano_dbsync_url: Option<String>,
    /// Duration to wait before polling DB Sync for the next update.
    #[arg(long, env = "NPRISM_CARDANO_DBSYNC_POLL_INTERVAL", default_value = "10s", value_parser = humantime::parse_duration)]
    pub cardano_dbsync_poll_interval: Duration,
}

#[derive(Args)]
#[command(next_help_heading = "Blockfrost")]
pub struct BlockfrostArgs {
    /// Blockfrost API key.
    #[arg(long, env = "NPRISM_BLOCKFROST_API_KEY")]
    pub blockfrost_api_key: Option<String>,
    /// Blockfrost base URL.
    #[arg(
        long,
        env = "NPRISM_BLOCKFROST_BASE_URL",
        default_value = "https://cardano-mainnet.blockfrost.io/api/v0"
    )]
    pub blockfrost_base_url: String,
    /// Duration to wait before polling Blockfrost for the next update.
    #[arg(long, env = "NPRISM_BLOCKFROST_POLL_INTERVAL", default_value = "10s", value_parser = humantime::parse_duration)]
    pub blockfrost_poll_interval: Duration,
    /// Delay between Blockfrost API calls.
    /// Set this to throttle requests and stay within Blockfrost API limits.
    #[arg(long, env = "NPRISM_BLOCKFROST_API_DELAY", default_value = "100ms", value_parser = humantime::parse_duration)]
    pub blockfrost_api_delay: Duration,
    /// Blockfrost API calls concurrency limit
    #[arg(long, env = "NPRISM_BLOCKFROST_CONCURRENCY_LIMIT", default_value_t = 4)]
    pub blockfrost_concurrency_limit: usize,
}

#[derive(Args)]
#[command(next_help_heading = "DLT Source")]
pub struct DltSourceArgs {
    /// Type of DLT source to use for event synchronization.
    #[arg(long, env = "NPRISM_DLT_SOURCE_TYPE", value_enum)]
    pub dlt_source_type: DltSourceType,
    /// Duration to wait before checking for unindexed operations.
    #[arg(long, env = "NPRISM_INDEX_INTERVAL", default_value = "10s", value_parser = humantime::parse_duration)]
    pub index_interval: Duration,
    /// Number of confirmation blocks to wait before considering the block valid.
    #[arg(long, env = "NPRISM_CONFIRMATION_BLOCKS", default_value_t = 112)]
    pub confirmation_blocks: u16,
    #[clap(flatten)]
    pub network: NetworkArgs,
    #[clap(flatten)]
    pub cardano_relay: OuraArgs,
    #[clap(flatten)]
    pub dbsync: DbSyncArgs,
    #[clap(flatten)]
    pub blockfrost: BlockfrostArgs,
}

/// Type of DLT sink to use for transaction submission.
#[derive(Clone, Debug, ValueEnum)]
pub enum DltSinkType {
    #[value(name = "cardano-wallet")]
    CardanoWallet,
    #[value(name = "embedded-wallet")]
    EmbeddedWallet,
}

#[derive(Args)]
#[command(next_help_heading = "Cardano Wallet")]
pub struct CardanoWalletArgs {
    /// Base URL of the Cardano wallet. Required when --dlt-sink-type=cardano-wallet.
    #[arg(long, env = "NPRISM_CARDANO_WALLET_URL")]
    pub cardano_wallet_url: Option<String>,
    /// Wallet ID to use for making transactions. Required when --dlt-sink-type=cardano-wallet.
    #[arg(long, env = "NPRISM_CARDANO_WALLET_WALLET_ID")]
    pub cardano_wallet_wallet_id: Option<String>,
    /// Passphrase for the wallet. Required when --dlt-sink-type=cardano-wallet.
    #[arg(long, env = "NPRISM_CARDANO_WALLET_PASSPHRASE")]
    pub cardano_wallet_passphrase: Option<String>,
    /// Payment address for making transactions. Required when --dlt-sink-type=cardano-wallet.
    #[arg(long, env = "NPRISM_CARDANO_WALLET_PAYMENT_ADDR")]
    pub cardano_wallet_payment_addr: Option<String>,
}

#[derive(Args)]
#[command(next_help_heading = "Embedded Wallet")]
pub struct EmbeddedWalletArgs {
    /// Path to the embedded wallet binary. Required when --dlt-sink-type=embedded-wallet.
    #[arg(long, env = "NPRISM_EMBEDDED_WALLET_BIN")]
    pub embedded_wallet_bin: Option<PathBuf>,
    /// Base URL of the Cardano submit API. Required when --dlt-sink-type=embedded-wallet.
    #[arg(long, env = "NPRISM_EMBEDDED_WALLET_SUBMIT_API_URL")]
    pub embedded_wallet_submit_api_url: Option<String>,
    /// Blockfrost API URL. Defaults to mainnet Blockfrost.
    #[arg(
        long,
        env = "NPRISM_EMBEDDED_WALLET_BLOCKFROST_URL",
        default_value = "https://cardano-mainnet.blockfrost.io/api/v0"
    )]
    pub embedded_wallet_blockfrost_url: String,
    /// Blockfrost API key for public Blockfrost. Mutually exclusive with --embedded-wallet-blockfrost-url.
    #[arg(long, env = "NPRISM_EMBEDDED_WALLET_BLOCKFROST_API_KEY")]
    pub embedded_wallet_blockfrost_api_key: Option<String>,
    /// Mnemonic phrase for the embedded wallet. Required when --dlt-sink-type=embedded-wallet.
    #[arg(long, env = "NPRISM_EMBEDDED_WALLET_MNEMONIC")]
    pub embedded_wallet_mnemonic: Option<String>,
}

#[derive(Args)]
#[command(next_help_heading = "DLT Sink")]
pub struct DltSinkArgs {
    /// Type of DLT sink to use for transaction submission.
    #[arg(long, env = "NPRISM_DLT_SINK_TYPE", value_enum)]
    pub dlt_sink_type: DltSinkType,
    #[clap(flatten)]
    pub cardano_wallet: CardanoWalletArgs,
    #[clap(flatten)]
    pub embedded_wallet: EmbeddedWalletArgs,
}

#[derive(Clone, ValueEnum)]
pub enum NetworkIdentifierCliOption {
    Mainnet,
    Preprod,
    Preview,
    Custom,
}

impl From<NetworkIdentifierCliOption> for NetworkIdentifier {
    fn from(value: NetworkIdentifierCliOption) -> Self {
        match value {
            NetworkIdentifierCliOption::Mainnet => NetworkIdentifier::Mainnet,
            NetworkIdentifierCliOption::Preprod => NetworkIdentifier::Preprod,
            NetworkIdentifierCliOption::Preview => NetworkIdentifier::Preview,
            NetworkIdentifierCliOption::Custom => NetworkIdentifier::Custom,
        }
    }
}
