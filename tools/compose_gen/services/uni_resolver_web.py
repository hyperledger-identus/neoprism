from pydantic import BaseModel

from ..models import Service


class Options(BaseModel):
    """Universal Resolver Web service options."""

    image: str = "universalresolver/uni-resolver-web:0.44.0-4922fcc"
    host_port: int
    prism_driver_url: str = "http://neoprism-indexer:8080/api"


def mk_service(options: Options) -> Service:
    """Build Universal Resolver Web service configuration."""
    return Service(
        image=options.image,
        ports=[f"{options.host_port}:8080"],
        environment={"uniresolver_web_driver_url_did_prism": options.prism_driver_url},
    )
