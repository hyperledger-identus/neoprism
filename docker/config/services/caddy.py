from pydantic import BaseModel

from ..models import Service

IMAGE = "caddy:2.10.2"


class Options(BaseModel):
    """Caddy service options."""

    image_override: str | None = None
    host_port: int | None = None
    target_port: int = 3000
    caddyfile: str = "./Caddyfile"


def mk_service(options: Options) -> Service:
    """Build Caddy service configuration."""
    image = options.image_override or IMAGE
    ports = (
        [f"{options.host_port}:{options.target_port}"] if options.host_port else None
    )

    return Service(
        image=image,
        ports=ports,
        volumes=[f"{options.caddyfile}:/etc/caddy/Caddyfile"],
    )
