from pydantic import BaseModel

from ..models import Service


class Options(BaseModel):
    """Scala DID service options."""

    image: str
    host_port: int | None = None


def mk_service(options: Options) -> Service:
    """Build Scala DID service configuration."""
    ports = [f"{options.host_port}:8980"] if options.host_port else None

    return Service(image=options.image, ports=ports, entrypoint=["/bin/scala-did-node"])
