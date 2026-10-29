# PRISM Specification Tests

The `prism-test` suite exercises NeoPRISM end-to-end against the official PRISM conformance scenarios (create, update, deactivate, and VDR extensions). Keeping this suite green is the primary guardrail for regressions.

## Recommended workflow

All automation assumes you are inside the Nix development shell:

```bash
nix develop
```

Before creating a Compose project, the runner verifies that `docker`,
`docker-compose`, `nix`, and `sbt` are available. A missing tool fails the
preflight without starting containers; a tool that disappears later is recorded
as a terminal failed receipt after scoped cleanup.

From there, run every supported stack in one go:

```bash
just e2e::run
```

`just e2e::run` uses a unique Compose project for each profile and performs the following loop:

1. Acquires a non-blocking, per-user host lock shared by all worktrees because the generated profiles use fixed host ports and a shared Docker image tag.
2. Runs read-only Docker, disk-space, Compose-file, and stale-network preflight checks.
3. Builds and loads the NeoPRISM image.
4. Iterates through the compose stacks below. The embedded-wallet profile waits, with a 120-second bound, until its deterministic test address has a spendable UTXO.
5. Runs `sbt "clean;Test / testOnly *"` for each profile and requires a non-zero conformance test receipt.
6. Executes `docker-compose ... down --volumes --remove-orphans` in a failure-safe cleanup path.
7. Writes a JSON receipt and per-profile logs under `target/e2e-receipts/`.

| Target | Backend | Compose file | Notes |
|--------|---------|--------------|-------|
| `sqlite-dev` | SQLite | `docker/prism-test/compose-sqlite-dev.yml` | Fast laptop smoke topology |
| `ci` | PostgreSQL | `docker/prism-test/compose-ci.yml` | Heavier topology that matches the CI pipeline |
| `ci-sqlite` | SQLite | `docker/prism-test/compose-ci-sqlite.yml` | Ensures the embedded backend stays compatible under CI load |
| `ci-blockfrost` | PostgreSQL | `docker/prism-test/compose-ci-blockfrost.yml` | Exercises Blockfrost indexing and submission paths |
| `ci-embedded-wallet` | PostgreSQL | `docker/prism-test/compose-ci-embedded-wallet.yml` | Exercises the embedded-wallet submission path |

Run one or more selected profiles with the runner directly:

```bash
python tools/e2e_runner.py --profile sqlite-dev
python tools/e2e_runner.py --profile ci-sqlite --profile ci-embedded-wallet
```

Each receipt records the exact Git revision, dirty state, Docker architecture and version, loaded NeoPRISM image ID, profile durations, embedded-wallet readiness time, conformance test count, missing images at profile start, result, and cleanup result. A profile with missing images is marked `cold_cache`; its total duration includes image download time. CPU and RAM are intentionally non-gating until controlled runs establish a useful baseline.

Need to focus on a single stack? Use `just e2e::up <name>` / `just e2e::down <name>` and run the Scala tests manually:

```bash
just e2e::up sqlite-dev
(cd tests/prism-test && sbt "clean;Test / testOnly *")
just e2e::down sqlite-dev
```

The manual `up` and `down` recipes retain the historical shared Compose project name. Prefer the runner for automated sessions. Full matrix sessions are deliberately serialized because the generated stacks bind fixed host ports such as `18080`.

## Preflight and recovery

The runner never removes global Docker resources automatically. It only tears down the unique project it created. If unused NeoPRISM test networks are found, preflight prints their exact names and a narrowly scoped `docker network rm ...` command without executing it.

If Docker reports `all predefined address pools have been fully subnetted`, inspect the listed networks before running the suggested command. If no stale NeoPRISM networks are listed, use `docker network ls` and coordinate with the owners of other projects instead of running a global prune.

The Cardano and Blockfrost test images currently target `linux/amd64`. Docker Desktop on Apple Silicon may print architecture-emulation warnings; these are expected when the services become healthy and the conformance suite passes.

By default, preflight requires 10 GiB of free host disk space for the cold-cache matrix. Override this only when the Docker images are already present:

```bash
NEOPRISM_E2E_MIN_FREE_GIB=5 just e2e::run
```

Run the runner's offline tests without Docker:

```bash
just e2e::test-runner
```

## Full repository check

Before opening a PR, run the umbrella command from the repository root:

```bash
just full-check
```

It chains formatting, `cargo build`, `just test`, Docker image builds, and the full `just e2e::run` suite. This mirrors the checks we expect to pass in CI.

## Manual compose usage

The compose files live in `docker/prism-test/` and are generated via `just build-config`. If you need to inspect or tweak them manually:

1. Start the desired stack (for example the SQLite development topology):

   ```bash
   cd docker/prism-test
   docker-compose -f compose-sqlite-dev.yml up
   ```

2. In another terminal, run the Scala suite:

   ```bash
   cd tests/prism-test
   sbt "clean;Test / testOnly *"
   ```

Manually editing the YAML files is discouraged—change the Python sources under `tools/compose_gen/` and rerun `just build-config` instead.

## Who should run the suite?

- **NeoPRISM contributors:** run either `just e2e::run` or `just full-check` whenever core logic changes.
- **Downstream PRISM node teams:** point the compose stack at your image (override the `image:` field or use `docker load` to drop in local builds) and reuse the same test harness.

## Extending the suite

To include an additional node implementation or scenario:

1. Update `tests/prism-test/src/test/scala/MainSpec.scala` to register your node layer or new scenarios.
2. Implement a matching `NodeClient` adapter if your HTTP surface differs from the existing NeoPRISM resolver/submitter endpoints.

This keeps the shared conformance suite portable across implementations.
