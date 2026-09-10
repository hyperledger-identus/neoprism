from pydantic import BaseModel

from ..models import ComposeConfig
from ..services import db, neoprism


class Options(BaseModel):
    baseline_image: str = (
        "${NEOPRISM_BASELINE_IMAGE:-hyperledgeridentus/identus-neoprism:0.14.2}"
    )
    candidate_image: str = "${NEOPRISM_CANDIDATE_IMAGE:?candidate image is required}"
    network: str = "${NEOPRISM_PARITY_NETWORK:-preprod}"
    relay_address: str = (
        "${NEOPRISM_PARITY_RELAY_ADDR:-preprod-node.play.dev.cardano.org:3001}"
    )


def _mk_indexer(*, image: str, db_host: str, options: Options) -> neoprism.Service:
    return neoprism.mk_service(
        neoprism.Options(
            image_override=image,
            network=options.network,
            storage_backend=neoprism.PostgresStorageBackend(host=db_host),
            confirmation_blocks=112,
            index_interval="1s",
            command=neoprism.IndexerCommand(
                dlt_source=neoprism.OuraDltSource(address=options.relay_address)
            ),
        )
    )


def mk_stack(options: Options | None = None) -> ComposeConfig:
    options = options or Options()
    return ComposeConfig(
        services={
            "db-baseline": db.mk_service(db.Options()),
            "db-candidate": db.mk_service(db.Options()),
            "baseline": _mk_indexer(
                image=options.baseline_image,
                db_host="db-baseline",
                options=options,
            ),
            "candidate": _mk_indexer(
                image=options.candidate_image,
                db_host="db-candidate",
                options=options,
            ),
        }
    )
