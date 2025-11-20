# PRISM Specification Tests

The `prism-test` suite exercises NeoPRISM end-to-end against the official PRISM conformance scenarios (create, update, deactivate, and VDR extensions). Keeping this suite green is the primary guardrail for regressions.

## Recommended workflow

All automation assumes you are inside the Nix development shell:

```bash
nix develop
```

From there, run every stack (PostgreSQL + SQLite, developer + CI topologies) in one go:

```bash
just e2e::run
```

`just e2e::run` performs the following loop:

1. Builds and `docker load`s the NeoPRISM image (`just e2e::docker-publish-local`).
2. Iterates through the compose stacks below, running `sbt test` for each.
3. Executes `docker-compose ... down --volumes --remove-orphans` so no containers linger between runs.

| Target | Backend | Compose file | Notes |
|--------|---------|--------------|-------|
| `dev` | PostgreSQL | `docker/prism-test/compose-dev.yml` | Mirrors “developer friendly” defaults with Postgres |
| `dev-sqlite` | SQLite | `docker/prism-test/compose-dev-sqlite.yml` | Fastest loop, great for laptop smoke tests |
| `ci` | PostgreSQL | `docker/prism-test/compose-ci.yml` | Heavier topology that matches the CI pipeline |
| `ci-sqlite` | SQLite | `docker/prism-test/compose-ci-sqlite.yml` | Ensures the embedded backend stays compatible under CI load |

Need to focus on a single stack? Use `just e2e::up <name>` / `just e2e::down <name>` and run the Scala tests manually:

```bash
just e2e::up dev-sqlite
(cd tests/prism-test && sbt test)
just e2e::down dev-sqlite
```

## Full repository check

Before opening a PR, run the umbrella script from the repository root:

```bash
./full-check.sh
```

It chains formatting, `cargo build`, `just test`, Docker image builds, the full `just e2e::run` suite, and finally an additional SQLite smoke test. This mirrors the checks we expect to pass in CI.

## Manual compose usage

The compose files live in `docker/prism-test/` and are generated via `just build-config`. If you need to inspect or tweak them manually:

1. Start the desired stack (for example the developer Postgres topology):
   ```bash
   cd docker/prism-test
   docker-compose -f compose-dev.yml up
   ```
2. In another terminal, run the Scala suite:
   ```bash
   cd tests/prism-test
   sbt test
   ```

Manually editing the YAML files is discouraged—change the Python sources under `tools/compose_gen/` and rerun `just build-config` instead.

## Who should run the suite?

- **NeoPRISM contributors:** run either `just e2e::run` or `./full-check.sh` whenever core logic changes.
- **Downstream PRISM node teams:** point the compose stack at your image (override the `image:` field or use `docker load` to drop in local builds) and reuse the same test harness.

## Extending the suite

To include an additional node implementation or scenario:

1. Update `tests/prism-test/src/test/scala/MainSpec.scala` to register your node layer or new scenarios.
2. Implement a matching `NodeClient` adapter if your HTTP surface differs from the existing NeoPRISM resolver/submitter endpoints.

This keeps the shared conformance suite portable across implementations.
