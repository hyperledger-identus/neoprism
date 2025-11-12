from pydantic import BaseModel

from ..models import Healthcheck, Service, ServiceDependency

IMAGE = "patextreme/cardano-testnet:20251111-145358"


class NodeOptions(BaseModel):
    """Cardano node service options."""

    network_magic: int
    testnet_volume: str


class BootstrapOptions(BaseModel):
    """Cardano bootstrap service options."""

    network_magic: int
    testnet_volume: str
    cardano_node_host: str
    wallet_base_url: str
    wallet_passphrase: str
    wallet_payment_address: str
    init_wallet_hurl_file: str
    init_did_hurl_file: str


def mk_node_service(options: NodeOptions) -> Service:
    """Build Cardano node service configuration."""
    return Service(
        image=IMAGE,
        restart=None,
        command=["initTestnet"],
        volumes=[f"{options.testnet_volume}:/node/testnet"],
        environment={
            "CARDANO_NODE_SOCKET_PATH": "/node/testnet/socket/node1/sock",
            "CARDANO_NODE_NETWORK_ID": str(options.network_magic),
        },
        healthcheck=Healthcheck(test=["CMD-SHELL", "cardano-cli query tip"]),
    )


def mk_bootstrap_service(options: BootstrapOptions) -> Service:
    """Build Cardano bootstrap service configuration."""
    return Service(
        image=IMAGE,
        restart=None,
        volumes=[
            f"{options.testnet_volume}:/node/testnet",
            f"{options.init_wallet_hurl_file}:/node/init-wallet.hurl",
            f"{options.init_did_hurl_file}:/node/init-did.hurl",
        ],
        command=[
            "bash",
            "-c",
            """transactGenesis
hurl ./init-wallet.hurl
hurl ./init-did.hurl

# blockfrost-ryo expects a different location
cp testnet/conway-genesis.json testnet/genesis.json
cp testnet/byron-genesis.json testnet/byron_genesis.json
""",
        ],
        environment={
            "HURL_WALLET_BASE_URL": options.wallet_base_url,
            "HURL_WALLET_PASSPHRASE": options.wallet_passphrase,
            "GENESIS_PAYMENT_ADDR": options.wallet_payment_address,
            "CARDANO_NODE_SOCKET_PATH": "/node/testnet/socket/node1/sock",
            "CARDANO_NODE_NETWORK_ID": str(options.network_magic),
        },
        depends_on={
            options.cardano_node_host: ServiceDependency(condition="service_healthy")
        },
    )
