from pydantic import BaseModel

from ..models import Healthcheck, Service, ServiceDependency

IMAGE = "cardanofoundation/cardano-wallet:2025.3.31"


class Options(BaseModel):
    """Cardano wallet service options."""

    host_port: int | None = None
    testnet_volume: str
    cardano_node_host: str


def mk_service(options: Options) -> Service:
    """Build Cardano wallet service configuration."""
    ports = [f"{options.host_port}:8090"] if options.host_port else None

    return Service(
        image=IMAGE,
        entrypoint=[],
        command=[
            "bash",
            "-c",
            """cardano-wallet serve \\
  --database /wallet/db \\
  --node-socket /node/testnet/socket/node1/sock \\
  --testnet /node/testnet/byron-genesis.json \\
  --listen-address 0.0.0.0
""",
        ],
        ports=ports,
        volumes=[f"{options.testnet_volume}:/node/testnet"],
        healthcheck=Healthcheck(
            test=["CMD-SHELL", "cardano-wallet network information"]
        ),
        depends_on={
            options.cardano_node_host: ServiceDependency(condition="service_healthy")
        },
    )
