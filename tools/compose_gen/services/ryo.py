from pydantic import BaseModel

from ..models import Service, ServiceDependency

IMAGE = "blockfrost/backend-ryo:v4.3.0"


class DbSyncDbArgs(BaseModel):
    host: str
    port: str
    db_name: str
    username: str
    password: str


class Options(BaseModel):
    host_port: int | None = None
    dbsync_db: DbSyncDbArgs
    network: str = "mainnet"
    testnet_volume: str | None = None
    config_file: str
    bootstrap_testnet_host: str | None = None
    wait_for_db_sync: bool = True
    genesis_data_folder: str | None = "/node/testnet"


def mk_service(options: Options) -> Service:
    ports = [f"{options.host_port}:3000"] if options.host_port else None

    # Build volumes
    volumes = [f"{options.config_file}:/app/config/development.yaml"]
    if options.testnet_volume:
        volumes.append(f"{options.testnet_volume}:/node/testnet")

    # Build environment
    environment = {
        "BLOCKFROST_CONFIG_DBSYNC_HOST": options.dbsync_db.host,
        "BLOCKFROST_CONFIG_DBSYNC_PORT": options.dbsync_db.port,
        "BLOCKFROST_CONFIG_DBSYNC_DATABASE": options.dbsync_db.db_name,
        "BLOCKFROST_CONFIG_DBSYNC_USER": options.dbsync_db.username,
        "BLOCKFROST_CONFIG_DBSYNC_PASSWORD": options.dbsync_db.password,
        "BLOCKFROST_CONFIG_NETWORK": options.network,
        "BLOCKFROST_MITHRIL_ENABLED": "false",
        "NODE_ENV": "development",
    }

    if options.genesis_data_folder:
        environment["BLOCKFROST_CONFIG_GENESIS_DATA_FOLDER"] = (
            options.genesis_data_folder
        )

    # Build depends_on
    depends_on: dict[str, ServiceDependency] = {}

    if options.wait_for_db_sync:
        depends_on[options.dbsync_db.host] = ServiceDependency(
            condition="service_healthy"
        )

    if options.bootstrap_testnet_host:
        depends_on[options.bootstrap_testnet_host] = ServiceDependency(
            condition="service_completed_successfully"
        )

    return Service(
        image=IMAGE,
        ports=ports,
        environment=environment,
        volumes=volumes,
        depends_on=depends_on if depends_on else None,
    )
