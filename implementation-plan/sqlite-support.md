# SQLite Backend Support — Architecture Decision Record

Status: implemented (2025-11-26)
Issue: https://github.com/hyperledger-identus/neoprism/issues/108

## Context
- We need a lightweight, embedded option alongside PostgreSQL for local/dev/CI usage while keeping parity in schema and behavior.
- Earlier snapshot import/export was removed; we rely on engine-native backup/restore flows.

## Decisions (implemented)
- Backend selection: inferred from `NPRISM_DB_URL` / `--db-url` scheme (`postgres://` vs `sqlite://`); no separate `--db-backend` flag. Missing URL defaults to an embedded SQLite path under the platform app-data directory, keyed by network.
- Storage layer: `PostgresDb` and `SqliteDb` live under `lib/node-storage/src/backend/{postgres,sqlite}.rs` and both implement the shared `StorageBackend` traits; runtime picks the backend via `init_database`.
- Migrations: split per backend under `lib/node-storage/migrations/{postgres,sqlite}` with `sqlx` pointed at the backend-specific folder. Tests assert migration parity across dialects.
- Features/tooling: SQLite is behind the `sqlite-backend` Cargo feature; dev shell bundles `sqlite`/`libsqlite3`/`sqlx-cli` and `just db-init-sqlite` / `db-clean-sqlite` manage the default file.
- CLI/docs/compose: docs describe scheme-based selection and default SQLite location; compose stacks set only `NPRISM_DB_URL` (no backend var). README and configuration guide reflect the single-flag flow.
- E2E coverage: `just e2e::run` exercises both Postgres and SQLite stacks; Docker image builds include the SQLite feature.

## Notes on scope changes
- Engine-agnostic backup/restore is delegated to native tooling (pg dump/restore, SQLite file/`.backup`), not handled inside the node.

## Follow-ups / open questions
- Do we need to ship pre-seeded SQLite files in release artifacts, or always migrate on first run?
- Should we expose a toggle to avoid linking SQLite when unused (to trim binary size)?
- Do we need additional telemetry/metrics specific to embedded mode (e.g., WAL checkpoints, file size alerts)?

TODOs
- Decide on release packaging policy for SQLite (pre-seeded file vs. migrate-on-first-run).
- Add a build-time toggle to omit SQLite when unused, if binary size justifies it.
- Add embedded-mode telemetry/alerts (WAL checkpointing/file growth) if operational feedback is needed.
