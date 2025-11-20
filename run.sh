#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "[1/7] cargo clean"
(cd "$ROOT_DIR" && cargo clean)

echo "[2/7] cargo build --all-features"
(cd "$ROOT_DIR" && cargo build --all-features)

echo "[3/7] just build-config"
(cd "$ROOT_DIR" && just build-config)

echo "[4/7] just test"
(cd "$ROOT_DIR" && just test)

echo "[5/7] just e2e::docker-publish-local"
(cd "$ROOT_DIR" && just e2e::docker-publish-local)

echo "[6/7] just e2e::run"
(cd "$ROOT_DIR" && just e2e::run)

echo "[7/7] sqlite dev stack"
(cd "$ROOT_DIR/docker/prism-test" && docker-compose -f compose-dev-sqlite.yml up -d --wait)
(cd "$ROOT_DIR/tests/prism-test" && SKIP_CONFIRMATION_CHECK_MILLIS=2000 sbt test)
(cd "$ROOT_DIR/docker/prism-test" && docker-compose -f compose-dev-sqlite.yml down --volumes --remove-orphans)

echo "All checks completed successfully."
