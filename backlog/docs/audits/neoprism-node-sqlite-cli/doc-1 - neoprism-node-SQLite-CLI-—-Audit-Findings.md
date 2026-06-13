---
id: doc-1
title: neoprism-node SQLite CLI — Audit & Findings
type: specification
created_date: '2026-06-11 15:56'
tags:
  - neoprism
  - neoprism-node
  - sqlite
  - cli
  - audit
---
# neoprism-node SQLite CLI — Audit & Findings

**Date:** 2026-06-11
**Scope:** Behavior of `neoprism-node` when using the embedded SQLite backend:
CLI surface, default database file location, file creation, and error handling.
**Status:** Findings only — no code changes were made.

---

## Summary

The CLI surface for SQLite is **mostly sensible** and the auto-creation of the
database file under the user's data directory is a good zero-config default.
However, the implementation has **three concrete bugs and several UX rough
edges**, all reproduced by reading the source and running the actual binary.

---

## Intended design (what works as advertised)

### CLI surface

```
Database:
      --db-url <DB_URL>  Database URL (e.g. postgres://user:pass@host:5432/db
                         or sqlite:///path/to/db). Defaults to an embedded
                         SQLite file when omitted [env: NPRISM_DB_URL=]
      --skip-migration   Skip database migration on node startup
                         [env: NPRISM_SKIP_MIGRATION=]
```

- Single `--db-url` flag (env: `NPRISM_DB_URL`) on `indexer`, `standalone`,
  and `dev`. (`submitter` has no DB — correct, no storage needed.)
- Backend is **inferred from the URL scheme**:
  - `postgres://` / `postgresql://` → Postgres
  - `sqlite://` or `sqlite:` → SQLite
- Inferred scheme is forwarded straight to `sqlx`
  (`SqliteConnectOptions::from_str` / `PgConnectOptions::from_str`).
- `NPRISM_DB_URL` is **not** read for the submitter.

### Default location (when `NPRISM_DB_URL` is unset)

Resolved in `default_base_dir()` → `dirs::data_dir()` (XDG-aware), then in
`default_sqlite_url()`:

- Linux:   `~/.local/share/NeoPRISM/<network>/neoprism.db`
- macOS:   `~/Library/Application Support/NeoPRISM/<network>/neoprism.db`
- Windows: `%APPDATA%/NeoPRISM/<network>/neoprism.db`

`<network>` is one of `mainnet | preprod | preview | custom`, taken from
`--cardano-network` (default `mainnet`, or `custom` for `dev`).

### File creation behavior (verified)

Running `neoprism-node dev` with no env and no flags on Linux:

1. Logs `NPRISM_DB_URL not set, defaulting to embedded SQLite at
   sqlite:///home/pat/.local/share/NeoPRISM/custom/neoprism.db`
2. Creates the parent directory `…/NeoPRISM/custom/` with mode `0700`
   (owner-only).
3. Creates `neoprism.db` (~70 KB, full SQLite 3 file with all migrations
   applied).
4. Starts the server.

Observed permissions on Linux:

```
700 drwx------  /home/pat/.local/share/NeoPRISM/custom/
644 -rw-r--r--  /home/pat/.local/share/NeoPRISM/custom/neoprism.db
```

The 0o700 mode on the parent directory matches the "kept private" claim in
`docs/src/configuration/database.md`. WAL is enabled by
`SqliteConnectOptions::journal_mode(Wal)`.

---

## Bugs

### Bug 1 — `sqlite://relative/path.db` panics with `NotFound`

**Severity:** medium. Likely the most common operator mistake.

`sqlite_path_from_url()` strips `sqlite://` and treats the remainder as a
path. If the path has **no parent directory** (a bare filename like
`sqlite://neoprism.db` or a relative path whose parent does not exist),
`Path::parent()` returns `Some("")`, `parent.exists()` is `false`, and
`ensure_sqlite_parent()` then calls `fs::create_dir_all("")` which returns
`ENOENT`:

```
$ neoprism-node dev --db-url "sqlite://neoprism.db"
thread 'main' panicked at bin/neoprism-node/src/lib.rs:536:41:
Failed to prepare sqlite database path: Os { code: 2, kind: NotFound, ... }
```

Suggested fix: a single guard in `ensure_sqlite_parent` —
`if parent.as_os_str().is_empty() { return Ok(()); }` — plus a unit test
`ensure_sqlite_parent_accepts_bare_filename`. `sqlite_path_from_url` is only
invoked for user-supplied `--db-url` values (the default goes through
`default_sqlite_url` which always emits an absolute path), so this only
affects user-supplied URLs.

The existing test `ensure_sqlite_parent_creates_missing_directory` covers
the multi-level case but **not the bare-filename case** — that is why CI
passes today.

### Bug 2 — Invalid URL schemes panic instead of erroring cleanly

**Severity:** low-to-medium. A polished CLI should not panic for invalid input.

`infer_db_backend()` panics with `NPRISM_DB_URL must start with postgres://
or sqlite://` for any unknown scheme (including the empty string and the
single character `:`):

```
$ neoprism-node dev --db-url "mysql://localhost/test"
thread 'main' panicked at bin/neoprism-node/src/lib.rs:625:5:
NPRISM_DB_URL must start with postgres:// or sqlite://
```

The clap-derived error path uses `anyhow::Error` everywhere else, so this
stands out. Replacing the `panic!` with a `Result` / `anyhow::bail!` and
returning a clean error would match the rest of the CLI.

Minor inconsistency: `postgres://` and `postgresql://` are both accepted,
but for SQLite only `sqlite://` and the single-slash `sqlite:` form are
accepted. The docs only mention `sqlite://`, which is fine, but the
asymmetry with Postgres is a minor surprise.

---

## Rough edges

### 1. Default directory is global to the OS user; no per-instance override

The default uses `dirs::data_dir()`, which honors `XDG_DATA_HOME` on Linux
good, but has no way to pick a project-local default. Combined with the
per-network subdirectory, two `neoprism-node dev` runs from different
working directories silently write to the same
`~/.local/share/NeoPRISM/custom/neoprism.db`. The `justfile` sidesteps this
by exporting `NPRISM_DB_URL="sqlite::memory:"` for `just run`, but the
moment a user drops `just run` and invokes the binary directly, the data
lands in their home directory.

For a tool that is also bundled in Docker (`docker/prism-test/...`), this
default behavior is exactly what you want for a server install, but
**confusing for local development** — the README and the `dev` subcommand
description do not warn about this.

### 2. `sqlite::memory:` is supported but undocumented in `--db-url` help

`sqlite::memory:` (in-memory, no file) is supported end-to-end — see the
`init_sqlite_database_in_memory_with_migration` test and
`app/service/prism.rs:315`. The plumbing works, but the flag help text only
shows the `sqlite:///path/to/db` example. For dev/CI users it is the right
knob to know about, and the `just run` target already relies on it:
`export NPRISM_DB_URL="sqlite::memory:"`.

### 3. No way to print the resolved default path without starting the node

The first time the user runs the node, they see the resolved path in INFO
logs. That is fine, but there is no `neoprism-node dev --print-default-db-url`
or `just` recipe to query it, so a user who only reads the docs (or has
`RUST_LOG=warn`) has no easy way to find out where the default file lives
until they actually start the node.

### 4. Inconsistent file-permission hardening

`ensure_sqlite_parent` sets `0700` on the parent directory (good). The
`.db` file itself is created by SQLite with the process umask — typically
`0644` (world-readable). For a database that may contain DID-related
metadata, leaving the file world-readable contradicts the "kept private"
claim in `docs/src/configuration/database.md`.

Suggested fix: call `fs::set_permissions(&db_path, 0o600)` after
`SqliteDb::connect`, or set the umask / use a `create_if_missing` option
that takes permissions.

### 5. Network hint is silently ignored when `--db-url` is set

`default_sqlite_url` uses `--cardano-network` to choose the subdirectory.
But once the user supplies `--db-url sqlite://...`, the network hint is
**not** applied. That is the correct behavior (user override wins), but
`--db-url` is a single arg — there is no way to say "use the default
layout, but for `preprod`" except by re-typing the full path. A
`--db-path` flag that gets combined with `default_base_dir()` would be
friendlier for one-off experiments.

### 6. `SqliteDb::connect` and file creation are conflated

`SqliteDb::connect` calls `SqliteConnectOptions::create_if_missing(true)`,
which is what actually creates the file. `init_sqlite_database` only
creates the parent directory. That layering is fine, but it means **the
first `connect` call is what fails** if the directory is read-only
(instead of the explicit `prepare_sqlite_destination`). And if the user
passes `--skip-migration`, the file is still created and migrations are
still effectively "applied" in the sense that the table-creation SQL is
now embedded — but the schema tables may not exist. A small footgun for
any "freeze the schema first" workflow.

---

## What is good

- **Single flag, scheme-based backend selection** — the PostgreSQL/SQLite
  choice is intuitive.
- **Per-network default subdirectory** — mainnet/preprod/preview/custom
  never collide.
- **XDG-aware default location** — `$XDG_DATA_HOME` is honored (verified
  in `dirs-5.0.1/src/lin.rs`).
- **Parent-directory 0o700 permission** — sensible default for the
  directory.
- **WAL mode enforced unconditionally** — consistent regardless of how the
  user invokes the binary.
- **Migrations applied automatically** unless `--skip-migration` is set;
  the in-memory and file-based paths are both unit-tested.
- **Both `sqlite://` and `sqlite:` (single-slash) schemes accepted** —
  matches `sqlx` conventions.
- **Test coverage**: 11 unit tests cover `sqlite_path_from_url`,
  `infer_db_backend`, `ensure_sqlite_parent`, `default_sqlite_url`,
  `init_sqlite_database` (in-memory + file), and `init_database`. They pass
  today, but the bare-filename case (Bug 1) is not exercised.

---

## Recommendations (in priority order)

1. **Fix `ensure_sqlite_parent` to tolerate empty parents** (Bug 1) — a
   2-line guard, plus a test like `ensure_sqlite_parent_accepts_bare_filename`.
2. **Replace the `panic!` in `infer_db_backend` with a `Result` /
   `anyhow::bail!`** and surface a clean error (Bug 2).
3. **Tighten the `.db` file permissions to 0o600** to match the directory
   hardening claim in the docs.
4. **Mention `sqlite::memory:` in the `--db-url` help text** — it is a
   legitimate, supported value and the `just run` recipe uses it.
5. **Add a `just db-path` (or similar) helper** that prints the resolved
   default path, so users do not have to start the node to discover it.
6. **Document the default directory explicitly in the README and the `dev`
   subcommand help** so first-time users are not surprised by a file
   appearing in `~/.local/share`.
7. **Optional: add a `--db-path` flag** that resolves relative to the
   default base dir, to make "default location but for network X"
   ergonomic.

---

## Files inspected

- `bin/neoprism-node/src/cli.rs` — `DbArgs`, `DevArgs`, `StandaloneArgs`,
  `IndexerArgs`
- `bin/neoprism-node/src/lib.rs` — `init_database`, `init_sqlite_database`,
  `default_base_dir`, `default_sqlite_url`, `prepare_sqlite_destination`,
  `ensure_sqlite_parent`, `sqlite_path_from_url`, `resolve_db_config`,
  `infer_db_backend`
- `lib/node-storage/src/backend/sqlite.rs` — `SqliteDb::connect`,
  `SqliteDb::migrate`, WAL configuration
- `docs/src/configuration/database.md` — user-facing docs
- `justfile` — `sqlite-init`, `sqlite-clean`, `run` (uses
  `sqlite::memory:`)
- `~/.cargo/registry/src/.../dirs-5.0.1/src/lin.rs` — confirmed `data_dir`
  honors `XDG_DATA_HOME`

No files were modified.
