from ..models import ComposeConfig
from ..services import db, neoprism, uni_resolver_web


def mk_stack() -> ComposeConfig:
    services = {
        "db": db.mk_service(db.Options(host_port=5432)),
        "neoprism-indexer": neoprism.mk_service(
            neoprism.Options(
                host_port=8081,
                db_host="db",
                network="mainnet",
                dlt_source=neoprism.RelayDltSource(
                    type="relay",
                    address="backbone.mainnet.cardanofoundation.org:3001",
                ),
            ),
        ),
        "uni-resolver-web": uni_resolver_web.mk_service(
            uni_resolver_web.Options(host_port=8080)
        ),
    }

    return ComposeConfig(services=services)
