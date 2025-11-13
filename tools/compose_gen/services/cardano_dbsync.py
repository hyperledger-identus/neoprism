from pydantic import BaseModel

from ..models import Service, ServiceDependency

IMAGE = "ghcr.io/intersectmbo/cardano-db-sync:13.6.0.5"


class Options(BaseModel):
    testnet_volume: str
    cardano_node_host: str
    config_file: str
    db_host: str


def mk_service(options: Options) -> Service:
    return Service(
        image=IMAGE,
        environment={
            "POSTGRES_HOST": options.db_host,
            "POSTGRES_DB": "postgres",
            "POSTGRES_PORT": "5432",
            "POSTGRES_USER": "postgres",
            "POSTGRES_PASSWORD": "postgres",
        },
        command=[
            "--config",
            "/config/dbsync-config.yaml",
            "--socket-path",
            "/node/testnet/socket/node1/sock",
            "--force-indexes",
        ],
        volumes=[
            f"{options.testnet_volume}:/node/testnet",
            f"{options.config_file}:/config/dbsync-config.yaml",
        ],
        depends_on={
            options.cardano_node_host: ServiceDependency(condition="service_healthy"),
            options.db_host: ServiceDependency(condition="service_healthy"),
        },
    )
