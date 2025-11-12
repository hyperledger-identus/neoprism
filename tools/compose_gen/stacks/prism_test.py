from pydantic import BaseModel

from ..metadata import VERSION
from ..models import ComposeConfig
from ..services import (
    caddy,
    cardano_dbsync,
    cardano_node,
    cardano_submit_api,
    cardano_wallet,
    db,
    neoprism,
    prism_node,
    ryo,
)


class Options(BaseModel):
    """Prism test stack options."""

    ci: bool = False


def mk_stack(options: Options) -> ComposeConfig:
    """Build prism-test stack configuration."""
    network_magic = 42
    testnet_volume = "node-testnet"
    cardano_node_host = "cardano-node"
    wallet_id = "9263a1248b046fe9e1aabc4134b03dc5c3a7ee3d"
    wallet_passphrase = "super_secret"
    wallet_payment_address = "addr_test1qp83v2wq3z9mkcjj5ejlupgwt6tcly5mtmz36rpm8w4atvqd5jzpz23y8l4dwfd9l46fl2p86nmkkx5keewdevqxhlyslv99j3"  # noqa: E501

    # Blockfrost services
    bf_services = {
        "bf-proxy": caddy.mk_service(
            caddy.Options(
                host_port=18082, target_port=3000, caddyfile="./Caddyfile-blockfrost"
            )
        ),
        "bf-ryo": ryo.mk_service(
            ryo.Options(
                dbsync_db=ryo.DbSyncDbArgs(
                    host="db-dbsync",
                    port="5432",
                    db_name="postgres",
                    username="postgres",
                    password="postgres",
                ),
                network="custom",
                testnet_volume=testnet_volume,
                config_file="./ryo.yaml",
                bootstrap_testnet_host="bootstrap-testnet",
            )
        ),
    }

    # Cardano services
    cardano_services = {
        "cardano-node": cardano_node.mk_node_service(
            cardano_node.NodeOptions(
                network_magic=network_magic, testnet_volume=testnet_volume
            )
        ),
        "bootstrap-testnet": cardano_node.mk_bootstrap_service(
            cardano_node.BootstrapOptions(
                network_magic=network_magic,
                testnet_volume=testnet_volume,
                cardano_node_host=cardano_node_host,
                wallet_base_url="http://cardano-wallet:8090/v2",
                wallet_passphrase=wallet_passphrase,
                wallet_payment_address=wallet_payment_address,
                init_wallet_hurl_file="./init-wallet.hurl",
                init_did_hurl_file="./init-did.hurl",
            )
        ),
        "cardano-dbsync": cardano_dbsync.mk_service(
            cardano_dbsync.Options(
                testnet_volume=testnet_volume,
                cardano_node_host=cardano_node_host,
                db_host="db-dbsync",
                config_file="./dbsync-config.yaml",
            )
        ),
        "cardano-wallet": cardano_wallet.mk_service(
            cardano_wallet.Options(
                testnet_volume=testnet_volume,
                cardano_node_host=cardano_node_host,
                host_port=18081,
            )
        ),
        "cardano-submit-api": cardano_submit_api.mk_service(
            cardano_submit_api.Options(
                testnet_volume=testnet_volume,
                cardano_node_host=cardano_node_host,
                network_magic=network_magic,
            )
        ),
    }

    # PRISM services
    prism_services = {
        "neoprism-standalone": neoprism.mk_service(
            neoprism.Options(
                image_override=f"identus-neoprism:{VERSION}" if options.ci else None,
                host_port=18080,
                db_host="db-neoprism",
                confirmation_blocks=0,
                index_interval=1,
                dlt_source=neoprism.DbSyncDltSource(
                    type="dbsync",
                    args=neoprism.DbSyncDltSourceArgs(
                        url="postgresql://postgres:postgres@db-dbsync:5432/postgres",
                        poll_interval=1,
                    ),
                ),
                dlt_sink=neoprism.DltSink(
                    wallet_host="cardano-wallet",
                    wallet_port=8090,
                    wallet_id=wallet_id,
                    wallet_passphrase=wallet_passphrase,
                    wallet_payment_address=wallet_payment_address,
                ),
            ),
        ),
        "prism-node": prism_node.mk_service(
            prism_node.Options(
                node_db_host="db-prism-node",
                db_sync_db_host="db-dbsync",
                bootstrap_testnet_host="bootstrap-testnet",
                wallet_api_host="cardano-wallet",
                wallet_passphrase=wallet_passphrase,
                wallet_id=wallet_id,
                wallet_payment_address=wallet_payment_address,
                host_port=50053,
                confirmation_blocks=0,
            )
        ),
        "db-neoprism": db.mk_service(db.Options()),
        "db-dbsync": db.mk_service(db.Options()),
        "db-prism-node": db.mk_service(db.Options()),
    }

    # Combine all services
    all_services = {**prism_services, **cardano_services, **bf_services}

    return ComposeConfig(services=all_services, volumes={"node-testnet": {}})
