#!/usr/bin/env bash
set -euo pipefail

# Full project quality gate

echo "[1/6] cargo clean"
cargo clean

echo "[2/6] cargo build --all-features"
cargo build --all-features

echo "[3/6] just build-config"
just build-config

echo "[4/6] just test"
just test

echo "[5/6] just e2e::docker-publish-local"
just e2e::docker-publish-local

echo "[6/6] just e2e::run"
just e2e::run

echo "All checks completed successfully."
