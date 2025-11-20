# Database Backends

NeoPRISM can persist indexed operations in PostgreSQL or in an embedded SQLite file. Pick the backend at runtime with:

```bash
neoprism-node indexer --db-backend {postgres|sqlite} ...
```

or set the corresponding environment variable:

```
NPRISM_DB_BACKEND=postgres   # default
NPRISM_DB_BACKEND=sqlite
```

When PostgreSQL is selected you must also provide `NPRISM_DB_URL` (or `--db-url`). SQLite derives its file path automatically but allows you to override it with `sqlite://...` URLs.

## Comparison

| Mode | Recommended usage | Pros | Trade-offs |
|------|-------------------|------|------------|
| PostgreSQL | Production deployments or any scenario that needs horizontal scaling and concurrent writers | Battle-tested RDBMS, works with existing replicas/backups, matches historical NeoPRISM behavior | Requires a managed Postgres instance; compose stacks need the extra container |
| SQLite | Local development, demos, CI smoke tests, single-node appliances | No external service, tiny footprint, file is bundled with backups | Single writer, WAL/locking semantics, best kept to one running node |

## PostgreSQL specifics

- Provide `NPRISM_DB_URL` / `--db-url` in standard libpq form (`postgres://user:pass@host:port/db`).
- The helper targets `just db-up`, `db-down`, `db-dump`, and `db-restore` spin up and manage a Dockerized Postgres instance for local work.
- All pre-existing migrations live under `lib/node-storage/migrations/postgres` and continue to be linted via `sqlfluff`.
- The Docker images and compose stacks ship with PostgreSQL enabled so existing deployments do not need any additional flags.

## SQLite specifics

- Select `--db-backend sqlite` (or `NPRISM_DB_BACKEND=sqlite`). If `--db-url` is omitted, the node stores data in your platform app directory (for example `~/Library/Application Support/NeoPRISM/<network>/neoprism.db` on macOS).
- Override the location by passing `sqlite://absolute/path/to/db`. The helper commands `just db-init-sqlite` and `just db-clean-sqlite` manage migrations for the default file under `data/sqlite/`.
- The parent directory is created with `700` permissions on Unix hosts to keep the file private.
- SQLite enforces WAL mode automatically, but remember that only one process should write to the file at a time. Schedule periodic `VACUUM` runs if you prune large chunks of data.

## Testing both backends

The e2e suite and the `full-check.sh` helper exercise both PostgreSQL and SQLite compose stacks (`dev`, `dev-sqlite`, `ci`, `ci-sqlite`). When troubleshooting, you can run any stack in isolation:

```bash
just e2e::up dev-sqlite
(cd tests/prism-test && sbt test)
just e2e::down dev-sqlite
```

See the [PRISM specification tests](../prism-test/README.md) section for more detail.
