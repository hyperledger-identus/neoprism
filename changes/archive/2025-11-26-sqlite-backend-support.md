# SQLite Backend Support — Architecture Decision Record

Status: implemented (2025-11-26)  
Issue: https://github.com/hyperledger-identus/neoprism/issues/108

## Why
- Provide a lightweight embedded backend for local/dev/CI while keeping schema parity and behavior with PostgreSQL.
- Avoid bespoke snapshot import/export; rely on engine-native backup/restore flows.

## What Changes

**Backend Selection**  
- From: Dedicated `--db-backend` flag plus URL.  
- To: Backend inferred from `NPRISM_DB_URL` / `--db-url` scheme (`postgres://` vs `sqlite://`); missing URL defaults to per-network embedded SQLite path.  
- Reason: Simpler UX, fewer mismatches.  
- Impact: Non-breaking; file-path default is new for omitted URLs.

**Storage Layer**  
- From: Single Postgres implementation.  
- To: `PostgresDb` and `SqliteDb` under `lib/node-storage/src/backend/{postgres,sqlite}.rs`, both implementing the shared `StorageBackend`; runtime selects via `init_database`.  
- Reason: Pluggable backends with identical trait surface.  
- Impact: Non-breaking; adds SQLite feature path.

**Migrations**  
- From: Unified migration folder.  
- To: Split migrations under `lib/node-storage/migrations/{postgres,sqlite}` with backend-specific `sqlx` wiring; tests assert parity.  
- Reason: Dialect-specific schema while preventing drift.  
- Impact: Requires maintaining both folders.

**Features & Tooling**  
- From: No SQLite toolchain.  
- To: `sqlite-backend` Cargo feature; dev shell bundles `sqlite`/`libsqlite3`/`sqlx-cli`; `just db-init-sqlite` / `db-clean-sqlite` manage the default file.  
- Reason: Make SQLite first-class for dev/CI.  
- Impact: Feature-guarded; optional dependency weight.

**CLI / Docs / Compose**  
- From: Backend flag and Postgres-first docs.  
- To: Docs show scheme-based selection and default SQLite location; compose stacks set only `NPRISM_DB_URL`; README/config guide aligned.  
- Reason: Single-source configuration and clearer defaults.  
- Impact: Users omit `NPRISM_DB_BACKEND`; otherwise backward compatible.

**E2E Coverage**  
- From: Postgres-only e2e.  
- To: `just e2e::run` and Docker builds cover both Postgres and SQLite stacks (with SQLite feature enabled).  
- Reason: Parity validation across backends.  
- Impact: Longer e2e run time; broader confidence.

## Impact
- Affected components: node CLI/runtime, node-storage backends, migrations, docs, compose stacks, e2e harness.  
- Migration: Transparent; Postgres users keep URLs. Omitting `NPRISM_DB_URL` now implies SQLite default file.  
- Tooling/ops: Need to keep both migration trees in sync; ensure feature flags set for builds that require SQLite.

## Notes on scope changes
- Backup/restore stays engine-native (pg dump/restore, SQLite file/`.backup`); no internal snapshot pipeline.

## Follow-ups / open questions
- Do we need to ship pre-seeded SQLite files in release artifacts, or always migrate on first run?
- Should we expose a toggle to avoid linking SQLite when unused (to trim binary size)?
- Do we need additional telemetry/metrics specific to embedded mode (e.g., WAL checkpoints, file size alerts)?

TODOs
- Decide on release packaging policy for SQLite (pre-seeded file vs. migrate-on-first-run).
- Add a build-time toggle to omit SQLite when unused, if binary size justifies it.
- Add embedded-mode telemetry/alerts (WAL checkpointing/file growth) if operational feedback is needed.
