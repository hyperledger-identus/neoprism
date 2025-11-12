from typing import Literal

from pydantic import BaseModel

from ..metadata import VERSION
from ..models import Healthcheck, Service, ServiceDependency


class DbSyncDltSourceArgs(BaseModel):
    """DbSync DLT source configuration."""

    url: str
    poll_interval: int = 10


class RelayDltSource(BaseModel):
    """Relay DLT source configuration."""

    type: Literal["relay"]
    address: str


class DbSyncDltSource(BaseModel):
    """DbSync DLT source configuration."""

    type: Literal["dbsync"]
    args: DbSyncDltSourceArgs


DltSource = RelayDltSource | DbSyncDltSource


class DltSink(BaseModel):
    """DLT sink configuration for publishing operations."""

    wallet_host: str
    wallet_port: int
    wallet_id: str
    wallet_passphrase: str
    wallet_payment_address: str


class Options(BaseModel):
    """NeoPRISM service options."""

    image_override: str | None = None
    host_port: int | None = None
    db_host: str = "db"
    network: str = "mainnet"
    dlt_source: DltSource
    dlt_sink: DltSink | None = None
    confirmation_blocks: int | None = None
    index_interval: int | None = None
    extra_depends_on: list[str] = []


def mk_service(options: Options) -> Service:
    """Build NeoPRISM service configuration."""
    image = options.image_override or f"hyperledgeridentus/identus-neoprism:{VERSION}"

    # Build environment variables
    environment = {
        "RUST_LOG": "oura=warn,tracing::span=warn,info",
        "NPRISM_DB_URL": f"postgres://postgres:postgres@{options.db_host}:5432/postgres",
        "NPRISM_CARDANO_NETWORK": options.network,
    }

    # Add optional configuration
    if options.confirmation_blocks is not None:
        environment["NPRISM_CONFIRMATION_BLOCKS"] = str(options.confirmation_blocks)

    if options.index_interval is not None:
        environment["NPRISM_INDEX_INTERVAL"] = str(options.index_interval)

    # Add DLT source configuration
    if options.dlt_source.type == "relay":
        environment["NPRISM_CARDANO_RELAY_ADDR"] = options.dlt_source.address
    else:  # dbsync
        environment["NPRISM_CARDANO_DBSYNC_URL"] = options.dlt_source.args.url
        environment["NPRISM_CARDANO_DBSYNC_POLL_INTERVAL"] = str(
            options.dlt_source.args.poll_interval
        )

    # Add DLT sink configuration if provided
    if options.dlt_sink:
        sink = options.dlt_sink
        environment.update(
            {
                "NPRISM_CARDANO_WALLET_BASE_URL": f"http://{sink.wallet_host}:{sink.wallet_port}/v2",
                "NPRISM_CARDANO_WALLET_WALLET_ID": sink.wallet_id,
                "NPRISM_CARDANO_WALLET_PASSPHRASE": sink.wallet_passphrase,
                "NPRISM_CARDANO_WALLET_PAYMENT_ADDR": sink.wallet_payment_address,
            }
        )

    # Build depends_on
    depends_on = {options.db_host: ServiceDependency(condition="service_healthy")}

    if options.dlt_sink:
        depends_on[options.dlt_sink.wallet_host] = ServiceDependency(
            condition="service_healthy"
        )

    # Add extra dependencies
    for dep in options.extra_depends_on:
        depends_on[dep] = ServiceDependency(condition="service_started")

    # Determine command based on mode
    command = ["standalone" if options.dlt_sink else "indexer"]

    # Build ports
    ports = [f"{options.host_port}:8080"] if options.host_port else None

    return Service(
        image=image,
        ports=ports,
        environment=environment,
        command=command,
        depends_on=depends_on,
        healthcheck=Healthcheck(
            test=["CMD", "curl", "-f", "http://localhost:8080/api/_system/health"]
        ),
    )
