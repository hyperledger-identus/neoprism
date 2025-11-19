#!/usr/bin/env bash
set -euo pipefail

# Full project quality gate

echo "[1/7] cargo clean"
cargo clean

echo "[2/7] cargo build --all-features"
cargo build --all-features

echo "[3/7] just build-config"
just build-config

echo "[4/7] just test"
just test

echo "[5/7] just e2e::docker-publish-local"
just e2e::docker-publish-local

echo "[6/7] just e2e::run"
just e2e::run

echo "[7/7] sqlite dev stack"
(cd docker/prism-test && docker-compose -f compose-dev-sqlite.yml up -d --wait)
(cd tests/prism-test && SKIP_CONFIRMATION_CHECK_MILLIS=2000 sbt test)
(cd docker/prism-test && docker-compose -f compose-dev-sqlite.yml down --volumes)

echo "All checks completed successfully."
