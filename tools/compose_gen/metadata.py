from pathlib import Path


def _read_version() -> str:
    version_file = Path(__file__).parent.parent.parent / "version"
    return version_file.read_text().strip()


VERSION: str = _read_version()

__all__ = ["VERSION"]
