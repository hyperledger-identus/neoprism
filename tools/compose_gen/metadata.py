"""Project version information."""

from pathlib import Path


def _read_version() -> str:
    """Read version from root version file."""
    version_file = Path(__file__).parent.parent.parent / "version"
    return version_file.read_text().strip()


VERSION: str = _read_version()
"""Current project version."""

__all__ = ["VERSION"]
