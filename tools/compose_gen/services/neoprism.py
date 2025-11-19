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
    poll_interval: int = 10


class DltSink(BaseModel):
    wallet_host: str
    wallet_port: int
    wallet_id: str
    wallet_passphrase: str
    wallet_payment_address: str


class IndexerCommand(BaseModel):
    command: Literal["indexer"] = "indexer"
    dlt_source: OuraDltSource | DbSyncDltSource


class StandaloneCommand(BaseModel):
    command: Literal["standalone"] = "standalone"
    dlt_source: OuraDltSource | DbSyncDltSource
    dlt_sink: DltSink


class DevCommand(BaseModel):
    command: Literal["dev"] = "dev"


class Options(BaseModel):
    image_override: str | None = None
    host_port: int | None = None
    db_host: str = "db"
    network: str = "mainnet"
    confirmation_blocks: int | None = None
    index_interval: int | None = None
    command: IndexerCommand | StandaloneCommand | DevCommand
    db_backend: Literal["postgres", "sqlite"] = "postgres"
    sqlite_db_url: str | None = None
    volumes: list[str] | None = None


def mk_service(options: Options) -> Service:
    image = options.image_override or f"hyperledgeridentus/identus-neoprism:{VERSION}"

    # Build environment variables
    environment = {
        "RUST_LOG": "oura=warn,tracing::span=warn,info",
        "NPRISM_CARDANO_NETWORK": options.network,
    }
    depends_on: dict[str, ServiceDependency] = {}

    if options.db_backend == "sqlite":
        environment["NPRISM_DB_BACKEND"] = "sqlite"
        environment["NPRISM_DB_URL"] = (
            options.sqlite_db_url or "sqlite:///var/lib/neoprism/sqlite/neoprism.db"
        )
    else:
        environment["NPRISM_DB_URL"] = (
            f"postgres://postgres:postgres@{options.db_host}:5432/postgres"
        )
        depends_on[options.db_host] = ServiceDependency(condition="service_healthy")

    # Add optional configuration
    if options.confirmation_blocks is not None:
        environment["NPRISM_CONFIRMATION_BLOCKS"] = str(options.confirmation_blocks)

    if options.index_interval is not None:
        environment["NPRISM_INDEX_INTERVAL"] = str(options.index_interval)

    # Add Indexer configuration
    if isinstance(options.command, IndexerCommand):
        # Add source configuration
        if isinstance(options.command.dlt_source, OuraDltSource):
            environment["NPRISM_CARDANO_RELAY_ADDR"] = (
                options.command.dlt_source.address
            )
        elif isinstance(options.command.dlt_source, DbSyncDltSource):
            environment["NPRISM_CARDANO_DBSYNC_URL"] = options.command.dlt_source.url
            environment["NPRISM_CARDANO_DBSYNC_POLL_INTERVAL"] = str(
                options.command.dlt_source.poll_interval
            )

    # Add standalone configuration
    elif isinstance(options.command, StandaloneCommand):
        # Add source configuration
        if isinstance(options.command.dlt_source, OuraDltSource):
            environment["NPRISM_CARDANO_RELAY_ADDR"] = (
                options.command.dlt_source.address
            )
        elif isinstance(options.command.dlt_source, DbSyncDltSource):
            environment["NPRISM_CARDANO_DBSYNC_URL"] = options.command.dlt_source.url
            environment["NPRISM_CARDANO_DBSYNC_POLL_INTERVAL"] = str(
                options.command.dlt_source.poll_interval
            )

        # Add sink configuration
        sink = options.command.dlt_sink
        environment.update(
            {
                "NPRISM_CARDANO_WALLET_BASE_URL": f"http://{sink.wallet_host}:{sink.wallet_port}/v2",
                "NPRISM_CARDANO_WALLET_WALLET_ID": sink.wallet_id,
                "NPRISM_CARDANO_WALLET_PASSPHRASE": sink.wallet_passphrase,
                "NPRISM_CARDANO_WALLET_PAYMENT_ADDR": sink.wallet_payment_address,
            }
        )

    # Build depends_on
    if isinstance(options.command, StandaloneCommand):
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
