"""Verify a development or Nix asset directory, without starting the node.

Run after `just build-assets`:
    python tools/verify_ui_assets.py bin/neoprism-node/assets

Or check a Nix package's output:
    python tools/verify_ui_assets.py /nix/store/...-neoprism-ui-assets/assets
"""

import argparse
import hashlib
from pathlib import Path

REPOSITORY = Path(__file__).resolve().parents[1]
SOURCE = REPOSITORY / "bin/neoprism-node/ui-assets"
REQUIRED = (
    "fonts/Poppins-Regular.ttf",
    "fonts/Poppins-Medium.ttf",
    "fonts/Poppins-SemiBold.ttf",
    "fonts/JetBrainsMono-Regular.ttf",
    "fonts/JetBrainsMono-Medium.ttf",
    "licenses/Poppins-OFL.txt",
    "licenses/JetBrainsMono-OFL.txt",
    "neoprism.svg",
    "prism.js",
    "resolver.js",
    "copy.js",
    "prism.css",
    "styles.css",
)
LOGO_SHA256 = "a814603b348f35d806583a528449b1dd88f18253263f7f7fc1792109c4641633"


def verify_assets(delivered: Path) -> list[str]:
    """Check the distribution contains the complete, unmodified asset bundle."""
    failures: list[str] = []
    for relative in REQUIRED:
        asset = delivered / relative
        if not asset.is_file() or asset.stat().st_size == 0:
            failures.append(f"missing or empty asset: {relative}")

    if not SOURCE.is_dir():
        failures.append("source ui-assets directory is missing")
    else:
        for source in sorted(SOURCE.rglob("*")):
            if not source.is_file():
                continue
            relative = source.relative_to(SOURCE)
            asset = delivered / relative
            if not asset.is_file() or asset.read_bytes() != source.read_bytes():
                failures.append(f"asset not preserved: {relative}")

    logo = delivered / "neoprism.svg"
    if logo.is_file() and hashlib.sha256(logo.read_bytes()).hexdigest() != LOGO_SHA256:
        failures.append("logo differs from the approved original SVG")

    return failures


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("assets", type=Path, help="built assets directory")
    args = parser.parse_args()
    failures = verify_assets(args.assets)
    if failures:
        for failure in failures:
            print(f"FAIL: {failure}")
        return 1
    print(f"PASS: complete Prism asset delivery at {args.assets}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
