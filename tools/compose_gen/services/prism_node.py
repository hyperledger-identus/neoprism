from pydantic import BaseModel

from ..models import Service, ServiceDependency

IMAGE = "ghcr.io/input-output-hk/prism-node:2.6.1"


class Options(BaseModel):
    """Prism Node service options."""

    image_override: str | None = None
    node_db_host: str
    db_sync_db_host: str
    bootstrap_testnet_host: str
    wallet_api_host: str
    wallet_api_port: int = 8090
    wallet_passphrase: str
    wallet_id: str
    wallet_payment_address: str
    host_port: int | None = None
    confirmation_blocks: int = 112


def mk_service(options: Options) -> Service:
    """Build Prism Node service configuration."""
    image = options.image_override or IMAGE
    ports = [f"{options.host_port}:50053"] if options.host_port else None

    return Service(
        image=image,
        ports=ports,
        environment={
            "NODE_PSQL_HOST": f"{options.node_db_host}:5432",
            "NODE_PSQL_DATABASE": "postgres",
            "NODE_PSQL_USERNAME": "postgres",
            "NODE_PSQL_PASSWORD": "postgres",
            "NODE_LEDGER": "cardano",
            "NODE_CARDANO_CONFIRMATION_BLOCKS": str(options.confirmation_blocks),
            "NODE_REFRESH_AND_SUBMIT_PERIOD": "1s",
            "NODE_MOVE_SCHEDULED_TO_PENDING_PERIOD": "1s",
            "NODE_SCHEDULE_SYNC_PERIOD": "1s",
            "NODE_CARDANO_NETWORK": "testnet",
            "NODE_CARDANO_WALLET_PASSPHRASE": options.wallet_passphrase,
            "NODE_CARDANO_WALLET_ID": options.wallet_id,
            "NODE_CARDANO_PAYMENT_ADDRESS": options.wallet_payment_address,
            "NODE_CARDANO_WALLET_API_HOST": options.wallet_api_host,
            "NODE_CARDANO_WALLET_API_PORT": str(options.wallet_api_port),
            "NODE_CARDANO_PRISM_GENESIS_BLOCK": "0",
            "NODE_CARDANO_DB_SYNC_HOST": f"{options.db_sync_db_host}:5432",
            "NODE_CARDANO_DB_SYNC_DATABASE": "postgres",
            "NODE_CARDANO_DB_SYNC_USERNAME": "postgres",
            "NODE_CARDANO_DB_SYNC_PASSWORD": "postgres",
        },
        depends_on={
            options.node_db_host: ServiceDependency(condition="service_healthy"),
            options.db_sync_db_host: ServiceDependency(condition="service_healthy"),
            options.wallet_api_host: ServiceDependency(condition="service_healthy"),
            options.bootstrap_testnet_host: ServiceDependency(
                condition="service_completed_successfully"
            ),
        },
    )
