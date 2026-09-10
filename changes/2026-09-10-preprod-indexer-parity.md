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
- image identities, network, boundary, cursor positions, counts, and timing.

Generated evidence lives under `artifacts/indexer-parity/` and is not committed.

## Acceptance Criteria

- The harness can run unattended against Preprod with a bounded timeout.
- Both indexers cross the same comparison boundary and finish processing its
  operations.
- Equal manifests return success.
- Missing, extra, duplicate, or changed operations return failure with
  actionable evidence.
- Offline unit tests cover normalization and difference reporting.
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

## Limitations

- Equality proves parity only through the recorded slot boundary and for the
  selected DLT source.
- Relay availability and network throughput affect runtime.
- A derived snapshot boundary is suitable for exploratory validation but is
  not a replacement for a pinned boundary in release evidence.
- A database snapshot or pre-seeded cursor can be added later if repeated
  PRISM-genesis synchronization becomes too expensive; both versions must then
  receive logically identical starting state.
