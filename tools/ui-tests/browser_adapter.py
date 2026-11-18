"""Session-scoped agent-browser CLI boundary, with validated external envelopes."""

import subprocess
from typing import Any, Self

from pydantic import BaseModel, ConfigDict, model_validator


class BrowserEnvelope(BaseModel):
    model_config = ConfigDict(strict=True)

    success: bool
    data: dict[str, Any] | None = None
    error: str | None = None

    @model_validator(mode="after")
    def require_success_data(self) -> Self:
        if self.success and self.data is None:
            raise ValueError("successful browser response requires object data")
        return self


class Browser:
    def __init__(self, session: str) -> None:
        self.session = session

    def __call__(self, *args: str) -> dict[str, Any]:
        result = subprocess.run(
            ["agent-browser", "--session", self.session, "--json", *args],
            check=True,
            capture_output=True,
            text=True,
        )
        payload = BrowserEnvelope.model_validate_json(result.stdout)
        if not payload.success:
            raise AssertionError(payload)
        assert payload.data is not None
        return payload.data

    def evaluate(self, script: str) -> Any:
        return self("eval", script)["result"]
