from pydantic import BaseModel

from ..models import ComposeConfig
from ..services import caddy, db, neoprism, ryo


class Options(BaseModel):
    """Blockfrost NeoPRISM demo stack options."""

    dbsync_url: str = "${DBSYNC_URL}"
    dbsync_host: str = "${DBSYNC_HOST}"
    dbsync_port: str = "${DBSYNC_PORT:-5432}"
    dbsync_db: str = "${DBSYNC_DB}"
    dbsync_user: str = "${DBSYNC_USER}"
    dbsync_password: str = "${DBSYNC_PASSWORD}"
    network: str = "${NETWORK:-mainnet}"


def mk_stack(options: Options) -> ComposeConfig:
    """Build blockfrost-neoprism-demo stack configuration."""
    services = {
        "neoprism": neoprism.mk_service(
            neoprism.Options(
                db_host="db-neoprism",
                network=options.network,
                host_port=8080,
                dlt_source=neoprism.DbSyncDltSource(
                    type="dbsync",
                    args=neoprism.DbSyncDltSourceArgs(
                        url=options.dbsync_url, poll_interval=10
                    ),
                ),
            ),
        ),
        "db-neoprism": db.mk_service(db.Options()),
        "bf-ryo": ryo.mk_service(
            ryo.Options(
                dbsync_db=ryo.DbSyncDbArgs(
                    host=options.dbsync_host,
                    port=options.dbsync_port,
                    db_name=options.dbsync_db,
                    username=options.dbsync_user,
                    password=options.dbsync_password,
                ),
                network=options.network,
                testnet_volume=None,
                config_file="./ryo.yaml",
                bootstrap_testnet_host=None,
                wait_for_db_sync=False,
                genesis_data_folder=None,
            )
        ),
        "caddy": caddy.mk_service(
            caddy.Options(
                host_port=3000, target_port=3000, caddyfile="./Caddyfile-blockfrost"
            )
        ),
    }

    return ComposeConfig(services=services, volumes={"node-testnet": {}})
