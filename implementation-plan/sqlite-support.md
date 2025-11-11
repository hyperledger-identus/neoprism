# SQLite Backend Support — Implementation Plan

## Scope Overview
- keep PostgreSQL as the default backend but add SQLite as an embedded option for `neoprism-node`
- avoid drift by mirroring every migration in both dialects and running CI parity checks
- expose an engine-agnostic backup/restore flow so data can move between SQLite and PostgreSQL
- require an explicit `--db-backend {postgres,sqlite}` flag/env var and provide a sensible default SQLite path for embedded mode

## Phase 1 — Storage Abstraction
1. Define a `NodeStorage` trait (or type alias) that bundles `RawOperationRepo + IndexedOperationRepo + IndexerStateRepo + DltCursorRepo + Clone + Send + Sync`.
2. Move the existing `PostgresDb` implementation into `lib/node-storage/src/backend/postgres.rs`; re-export via `pub use backend::{PostgresDb, SqliteDb, NodeStorageBackend};`.
3. Introduce `SqliteDb` built on `sqlx::SqlitePool` and `lazybe::db::sqlite::SqliteDbCtx`; ensure every trait impl mirrors the Postgres logic.
4. Add a backend selector enum plus factory (`NodeStorageBackend::connect(DbBackendConfig) -> Result<NodeStorage>`) to hide engine-specific wiring.

## Phase 2 — Migrations & Schema
1. Split migrations into `lib/node-storage/migrations/postgres` (existing SQL) and `lib/node-storage/migrations/sqlite` (new scripts with equivalent schema, foreign keys, and views).
2. Replace `sqlx::migrate!("./migrations")` with backend-specific paths.
3. Update `sqlfluff` and any formatting/tooling hooks to lint only Postgres SQL; add a lightweight formatter (or document manual steps) for SQLite files.
4. Add CI scripts/tests that run `sqlx migrate run` for both backends and compare normalized schemas to detect divergence early.

## Phase 3 — Dependencies & Environment
1. Enable the `sqlite` feature for `sqlx`, `sea-query`, and `lazybe` in the workspace; gate it behind a new Cargo feature (e.g., `sqlite-backend`) if binary size becomes a concern.
2. Extend the dev shell (`nix/devShells/development.nix`) with `libsqlite3`/`sqlite` and ensure `sqlx-cli` can talk to SQLite files.
3. Document/update `cargo sqlx prepare` steps so developers can regenerate offline metadata for both engines if needed.

## Phase 4 — CLI, Config, and Runtime
1. Extend `DbArgs` with `#[arg(long, env = "NPRISM_DB_BACKEND", value_enum)] pub backend: DbBackend`, defaulting to `Postgres`.
2. Derive the SQLite default URL when `backend == Sqlite` (per-platform app-data path that includes the network name) but still allow overriding via `--db-url`.
3. Update `init_database`, workers, and services to accept the abstracted storage type instead of `PostgresDb` directly.
4. Ensure multi-component setups (DLT sync/index workers, resolver service) clone the backend safely; verify Send/Sync bounds.

## Phase 5 — Tooling & Documentation
1. Add `just` targets for SQLite usage (`just db-init-sqlite`, etc.) and clarify that `db-up`/`db-dump` remain Postgres-specific.
2. Extend README + docs with:
   - instructions for choosing a backend
   - default SQLite file locations & permissions (0700 parent dir)
   - limitations (single-writer, WAL requirement, performance expectations)
3. Update Docker/Dhall configs to keep using Postgres but mention the embedded option for local/dev scenarios.

## Phase 6 — Validation, Backup & Restore
1. Implement backend-agnostic backup/export APIs (serialize DID operations + cursor state), plus matching import logic; wire them into new `just db-backup` / `db-restore` recipes.
2. Add integration tests that:
   - run migrations + smoke tests against temporary Postgres (test container) and SQLite (temp file)
   - verify that exported data from one backend can be imported into the other.
3. Ensure `just test` (or a new CI lane) exercises both backends, at least for the critical repository tests.

## Open Questions / Follow-Ups
- How do we want to distribute/pre-seed SQLite databases in release artifacts (empty file vs. migrate on first run)?
- Do we need feature flags in dependent crates (e.g., only link SQLite when CLI flag enabled), or is always-on acceptable?
- Are there operational metrics/telemetry differences we should surface when running in embedded mode?
