from pydantic import BaseModel

from ..models import Service, ServiceDependency

IMAGE = "ghcr.io/intersectmbo/cardano-submit-api:10.5.1"


class Options(BaseModel):
    """Cardano Submit API service options."""

    host_port: int | None = None
    testnet_volume: str
    cardano_node_host: str
    network_magic: int


def mk_service(options: Options) -> Service:
    """Build Cardano Submit API service configuration."""
    ports = [f"{options.host_port}:8090"] if options.host_port else None

    return Service(
        image=IMAGE,
        ports=ports,
        command=[
            "--config",
            "/config.yaml",
            "--socket-path",
            "/node/testnet/socket/node1/sock",
            "--testnet-magic",
            str(options.network_magic),
            "--listen-address",
            "0.0.0.0",
        ],
        volumes=[
            f"{options.testnet_volume}:/node/testnet",
            "./cardano-submit-api.yaml:/config.yaml",
        ],
        depends_on={
            options.cardano_node_host: ServiceDependency(condition="service_healthy")
        },
    )
