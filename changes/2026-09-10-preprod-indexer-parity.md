# Preprod Indexer Parity — Change Specification

Status: implemented locally (2026-09-10)

Issue: <https://github.com/hyperledger-identus/neoprism/issues/325>

## Goal

Demonstrate that an immutable published NeoPRISM image and an immutable
sdk-rust-powered snapshot discover and index the same PRISM operations from a
real Cardano network.

Preprod is the default and expected development network. Mainnet execution is
available only through an explicit opt-in because it has a larger operational
cost.

## Decision

Run the two images as isolated indexers with separate PostgreSQL databases and
the same network configuration. Both indexers consume the same relay from the
fixed PRISM genesis chain point already defined by the Oura source.

The test uses a shared slot boundary rather than a moving chain tip:

1. Wait for both persisted DLT cursors to pass an explicit target slot, or
   derive a snapshot boundary behind the slower cursor after a bounded sampling
   period.
2. Allow the indexing queues to drain and require all raw operations at or
   below the boundary to have an indexing outcome.
3. Stop both indexers while leaving their databases available.
4. Export and compare canonical operation manifests at or below the boundary.

An explicit target slot makes a run reproducible. The derived boundary is a
convenience for exploratory runs and is always written to the evidence report.

## Canonical Comparison

The manifest includes fields that define operation identity, chain placement,
payload, and processing outcome:

- operation ID;
- transaction hash;
- signed operation bytes;
- slot and block number;
- absolute transaction sequence and operation sequence;
- indexed flag;
- indexed operation kind (SSI, VDR, or none);
- canonical DID assigned by indexing.

Database-generated UUIDs, migration metadata, ingestion timestamps, and
indexing timestamps are deliberately excluded. Rows are sorted by operation ID
before comparison.

Duplicate operation IDs are treated as an invalid manifest. Missing, extra, or
changed rows make the test fail and are listed in both JSON and Markdown
reports.

## Runtime Contract

- The baseline defaults to the published
  `hyperledgeridentus/identus-neoprism:0.14.2` image and remains configurable.
- The candidate image must be supplied explicitly; a mutable implicit
  `latest` candidate is not accepted.
- Reports capture the requested image references and locally resolved image IDs
  and registry digests.
- The default relay is the public Cardano Preprod relay already used by the
  repository example.
- A timeout prevents an endless synchronization run.
- The harness submits no transactions and requires no wallet keys.
- Ordinary pull-request checks do not run the live-network test. A future
  scheduled or manually dispatched workflow may run it as a slow lane.

## Evidence

Each run writes:

- baseline and candidate normalized CSV manifests;
- a machine-readable JSON comparison report;
- a human-readable Markdown summary;
- container logs;
- raw cursor and container-resource samples as CSV;
- image identities, network, boundary, cursor positions, counts, and timing;
- total and scan duration, time-to-boundary, effective cursor rate, sampled
  CPU/memory/process usage, restart counts, and candidate-to-baseline ratios.

Resource measurements are comparative diagnostics rather than acceptance gates.
Docker CPU percentages can exceed 100% on multi-core hosts, and CPU/memory
results depend on the runner, Docker runtime, relay, and concurrent workload.
Hard QoS thresholds require repeated runs on a controlled runner first.

Generated evidence lives under `artifacts/indexer-parity/` and is not committed.

## Acceptance Criteria

- The harness can run unattended against Preprod with a bounded timeout.
- Both indexers cross the same comparison boundary and finish processing its
  operations.
- Equal manifests return success.
- Missing, extra, duplicate, or changed operations return failure with
  actionable evidence.
- Offline unit tests cover normalization and difference reporting.
- Raw and summarized QoS evidence is retained without making an uncalibrated
  local resource measurement a semantic parity failure.
- Local documentation explains image preparation, execution, evidence, cost,
  and limitations.

## Initial Preprod Evidence

A bounded smoke run compared the published `0.14.2` image with an image built
from sdk-rust adoption commit `757531e2d7e26ab5a0c950b619c5f5cdfb0a0735`.
Both indexers reached persisted slot `10723656`. Their canonical manifests
through slot `10722656` were byte-for-byte equal:

- two operations in each manifest;
- zero pending operations;
- zero duplicate, missing, extra, or changed operations;
- matching operation IDs, payloads, transaction hashes, chain positions,
  operation kinds, and DIDs.

This proves the harness and an early Preprod prefix. It is not the final
historical parity claim: release evidence still requires an explicit target
slot covering a materially larger operation set.

The first instrumented run used Docker Desktop on arm64 with 12 Docker CPUs
and an 8.32 GB Docker memory allocation. It completed in 136.060 seconds, of
which 134.269 seconds was the live scan. Sixteen samples per indexer found:

| Metric | Published `0.14.2` | sdk-rust candidate | Candidate / baseline |
| --- | ---: | ---: | ---: |
| Time to shared required cursor | 93.022 s | 93.022 s | 1.000 |
| Mean CPU | 0.867% | 0.954% | 1.100 |
| Peak CPU | 2.010% | 1.900% | 0.945 |
| Mean memory | 10.447 MiB | 10.071 MiB | 0.964 |
| Peak memory | 10.980 MiB | 10.700 MiB | 0.975 |
| Peak process count | 19 | 19 | — |
| Container restarts | 0 | 0 | — |

There were no measurement warnings. This short local smoke run shows that the
instrumentation works and did not reveal an obvious resource regression. It is
not a statistically meaningful performance conclusion.

## Limitations

- Equality proves parity only through the recorded slot boundary and for the
  selected DLT source.
- Relay availability and network throughput affect runtime.
- A derived snapshot boundary is suitable for exploratory validation but is
  not a replacement for a pinned boundary in release evidence.
- A database snapshot or pre-seeded cursor can be added later if repeated
  PRISM-genesis synchronization becomes too expensive; both versions must then
  receive logically identical starting state.
- A single run can identify a large regression, but it does not establish a
  stable performance baseline. QoS gates need repeated measurements on a fixed
  runner with controlled resource limits and an agreed variance budget.
