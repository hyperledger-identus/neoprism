from pydantic import BaseModel

from ..models import Healthcheck, Service

IMAGE = "postgres:16"


class Options(BaseModel):
    host_port: int | None = None


def mk_service(options: Options) -> Service:
    ports = [f"{options.host_port}:5432"] if options.host_port else None

    return Service(
        image=IMAGE,
        ports=ports,
        environment={
            "POSTGRES_DB": "postgres",
            "POSTGRES_PASSWORD": "postgres",
            "POSTGRES_USER": "postgres",
        },
        healthcheck=Healthcheck(test=["CMD", "pg_isready", "-U", "postgres"]),
    )
