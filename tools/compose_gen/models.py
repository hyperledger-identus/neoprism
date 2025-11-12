from __future__ import annotations

from typing import Any, Literal

from pydantic import BaseModel


class Healthcheck(BaseModel):
    """Docker healthcheck configuration."""

    test: list[str]
    interval: str = "2s"
    timeout: str = "5s"
    retries: int = 30


ServiceCondition = Literal[
    "service_started", "service_healthy", "service_completed_successfully"
]


class ServiceDependency(BaseModel):
    """Service dependency configuration."""

    condition: ServiceCondition


class Service(BaseModel):
    """Docker Compose service configuration."""

    image: str
    restart: str | None = "always"
    ports: list[str] | None = None
    command: list[str] | None = None
    entrypoint: list[str] | None = None
    environment: dict[str, str] | None = None
    volumes: list[str] | None = None
    depends_on: dict[str, ServiceDependency] | None = None
    healthcheck: Healthcheck | None = None

    model_config = {"extra": "forbid"}


class ComposeConfig(BaseModel):
    """Docker Compose configuration."""

    services: dict[str, Service]
    volumes: dict[str, dict[str, Any]] | None = None

    model_config = {"extra": "forbid"}
