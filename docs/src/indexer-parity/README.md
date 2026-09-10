# Live Indexer Parity Test

The live parity harness runs a published NeoPRISM release and a candidate image
against the same Cardano chain prefix. It is intended for release and migration
evidence, especially when replacing internal components such as Apollo with
sdk-rust.

The harness is read-only. It runs two Oura indexers and two isolated PostgreSQL
databases, submits no transactions, and needs no wallet credentials.

## Prepare the Candidate

Build and load the candidate snapshot before starting the test. For example:

```shell
nix build .#neoprism-docker-latest
docker load < result
docker tag identus-neoprism:latest neoprism:sdk-rust-snapshot
```

The exact loaded tag can vary, but it must be supplied explicitly. The report
captures the resolved local image ID and any registry digest.

## Run on Preprod

```shell
just indexer-parity::run neoprism:sdk-rust-snapshot
```

The default baseline is
`hyperledgeridentus/identus-neoprism:0.14.2`. Override it or pin a repeatable
slot when needed:

```shell
just indexer-parity::run neoprism:sdk-rust-snapshot \
  --baseline-image hyperledgeridentus/identus-neoprism:0.14.2 \
  --target-slot 12345678 \
  --timeout-seconds 7200
```

Without `--target-slot`, the harness samples both cursors for five minutes and
chooses a boundary 1,000 slots behind the slower indexer. This is useful for an
exploratory comparison. Release evidence should use an explicit historical
slot.

Mainnet is deliberately guarded:

```shell
just indexer-parity::run neoprism:sdk-rust-snapshot \
  --network mainnet \
  --allow-mainnet
```

## Evidence and Exit Codes

Evidence is written beneath `artifacts/indexer-parity/`:

- `baseline.csv` and `candidate.csv` contain normalized operation manifests;
- `report.json` contains machine-readable differences and run metadata;
- `report.md` is the review summary;
- `cursor-samples.csv` contains elapsed time and both persisted cursors;
- `resource-samples.csv` contains sampled CPU, memory, and process use for each
  indexer;
- `containers.log` captures both indexers and databases.

The command exits with zero for equal manifests, one for a semantic difference,
and two for configuration or harness failure. Containers and ephemeral database
state are removed after the report is captured. Pass `--keep` to retain them for
diagnosis.

The report summarizes total duration, live scan duration, time-to-boundary,
effective cursor rate, average/p95/peak CPU, average/p95/peak memory, peak
process count, and container restarts. It also calculates candidate-to-baseline
ratios for the most useful comparative values. Change the five-second sampling
cadence with `--metrics-sample-seconds` when needed.

## Cost and Interpretation

Runtime depends on relay throughput and the selected boundary. Both images start
from NeoPRISM's fixed PRISM genesis point, not Cardano absolute genesis. The
default timeout is two hours.

Resource telemetry is diagnostic. Docker CPU percentages may exceed 100% on
multi-core hosts, and results vary with the runner, Docker runtime, relay, and
other workloads. Establish a distribution from repeated runs on a controlled
runner before defining a performance budget or making QoS metrics gating.

A passing report proves equality only through its recorded slot, with the Oura
source and configuration shown in the report. It does not establish parity for
DB-Sync, Blockfrost, or future operations beyond that boundary.
