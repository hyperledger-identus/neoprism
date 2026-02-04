from typing import Literal

from pydantic import BaseModel

from ..metadata import VERSION
from ..models import Healthcheck, Service, ServiceDependency


class OuraDltSource(BaseModel):
    source_type: Literal["relay"] = "relay"
    address: str


class DbSyncDltSource(BaseModel):
    source_type: Literal["dbsync"] = "dbsync"
    url: str
    poll_interval: str = "10s"


class BlockfrostDltSource(BaseModel):
    source_type: Literal["blockfrost"] = "blockfrost"
    api_key: str
    base_url: str
    poll_interval: str = "10s"
    api_delay: str | None = None
    concurrency_limit: int | None = None


class DltSink(BaseModel):
    wallet_host: str
    wallet_port: int
    wallet_id: str
    wallet_passphrase: str
    wallet_payment_address: str


class IndexerCommand(BaseModel):
    command: Literal["indexer"] = "indexer"
    dlt_source: OuraDltSource | DbSyncDltSource | BlockfrostDltSource


class StandaloneCommand(BaseModel):
    command: Literal["standalone"] = "standalone"
    dlt_source: OuraDltSource | DbSyncDltSource | BlockfrostDltSource
    dlt_sink: DltSink


class DevCommand(BaseModel):
    command: Literal["dev"] = "dev"


class PostgresStorageBackend(BaseModel):
    backend: Literal["postgres"] = "postgres"
    host: str = "db"

    @property
    def db_url(self) -> str:
        return f"postgres://postgres:postgres@{self.host}:5432/postgres"


class SqliteStorageBackend(BaseModel):
    backend: Literal["sqlite"] = "sqlite"
    db_url: str = "sqlite:///var/lib/neoprism/sqlite/neoprism.db"


class Options(BaseModel):
    command: IndexerCommand | StandaloneCommand | DevCommand
    storage_backend: PostgresStorageBackend | SqliteStorageBackend = (
        PostgresStorageBackend()
    )
    network: str = "mainnet"
    host_port: int | None = None
    confirmation_blocks: int | None = None
    index_interval: str | None = None
    volumes: list[str] | None = None
    external_url: str | None = None
    image_override: str | None = None


def mk_service(options: Options) -> Service:
    image = options.image_override or f"hyperledgeridentus/identus-neoprism:{VERSION}"

    # Build environment variables
    environment = {
        "RUST_LOG": "oura=warn,tracing::span=warn,info",
        "NPRISM_CARDANO_NETWORK": options.network,
    }
    depends_on: dict[str, ServiceDependency] = {}

    if isinstance(options.storage_backend, SqliteStorageBackend):
        environment["NPRISM_DB_URL"] = options.storage_backend.db_url
    else:
        environment["NPRISM_DB_URL"] = options.storage_backend.db_url
        depends_on[options.storage_backend.host] = ServiceDependency(
            condition="service_healthy"
        )

    # Add optional configuration
    if options.confirmation_blocks is not None:
        environment["NPRISM_CONFIRMATION_BLOCKS"] = str(options.confirmation_blocks)

    if options.index_interval is not None:
        environment["NPRISM_INDEX_INTERVAL"] = str(options.index_interval)

    if options.external_url is not None:
        environment["NPRISM_EXTERNAL_URL"] = str(options.external_url)

    # DLT source - both indexer and standalone need this
    if isinstance(options.command, (IndexerCommand, StandaloneCommand)):
        _add_dlt_source_env(environment, options.command.dlt_source)

    # DLT sink - only standalone needs this
    if isinstance(options.command, StandaloneCommand):
        _add_dlt_sink_env(environment, options.command.dlt_sink)
        depends_on[options.command.dlt_sink.wallet_host] = ServiceDependency(
            condition="service_healthy"
        )

    # Determine command based on mode
    command = [options.command.command]

    # Build ports
    ports = [f"{options.host_port}:8080"] if options.host_port else None

    return Service(
        image=image,
        ports=ports,
        environment=environment,
        command=command,
        depends_on=depends_on or None,
        healthcheck=Healthcheck(
            test=["CMD", "curl", "-f", "http://localhost:8080/api/_system/health"]
        ),
        volumes=options.volumes,
    )


def _add_dlt_source_env(
    env: dict[str, str], source: OuraDltSource | DbSyncDltSource | BlockfrostDltSource
) -> None:
    """Add DLT source-specific environment variables."""
    if isinstance(source, OuraDltSource):
        env["NPRISM_CARDANO_RELAY_ADDR"] = source.address
    elif isinstance(source, DbSyncDltSource):
        env["NPRISM_CARDANO_DBSYNC_URL"] = source.url
        env["NPRISM_CARDANO_DBSYNC_POLL_INTERVAL"] = str(source.poll_interval)
    else:
        # BlockfrostDltSource is the only remaining option in the union
        env["NPRISM_BLOCKFROST_API_KEY"] = source.api_key
        env["NPRISM_BLOCKFROST_BASE_URL"] = source.base_url
        env["NPRISM_BLOCKFROST_POLL_INTERVAL"] = source.poll_interval
        if source.api_delay is not None:
            env["NPRISM_BLOCKFROST_API_DELAY"] = source.api_delay
        if source.concurrency_limit is not None:
            env["NPRISM_BLOCKFROST_CONCURRENCY_LIMIT"] = str(source.concurrency_limit)


def _add_dlt_sink_env(env: dict[str, str], sink: DltSink) -> None:
    """Add DLT sink-specific environment variables."""
    env.update(
        {
            "NPRISM_CARDANO_WALLET_BASE_URL": f"http://{sink.wallet_host}:{sink.wallet_port}/v2",
            "NPRISM_CARDANO_WALLET_WALLET_ID": sink.wallet_id,
            "NPRISM_CARDANO_WALLET_PASSPHRASE": sink.wallet_passphrase,
            "NPRISM_CARDANO_WALLET_PAYMENT_ADDR": sink.wallet_payment_address,
        }
    )
