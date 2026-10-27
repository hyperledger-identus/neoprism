# ADR 0001: adopt the SDK-Rust PRISM method kernel behind NeoPRISM facades

- **Status:** Proposed for issue
  [#345](https://github.com/hyperledger-identus/neoprism/issues/345)
- **Date:** 2026-10-27
- **Depends on:** NeoPRISM #324, SDK-Rust #521, SDK-Rust ADRs 0174–0176
- **Parity gate:** NeoPRISM #325

## Context

NeoPRISM currently owns both reusable PRISM method behavior and product/runtime
behavior. Its `identus-did-prism` crate combines:

- canonical and long-form DID syntax;
- protobuf operation values and exact-wire hashing;
- create, update, and deactivate validation;
- deterministic DID state reduction and document projection;
- VDR storage operations;
- Cardano metadata, indexing, persistence-facing values, and resolver
  orchestration.

SDK-Rust now contains the separately releasable `identus-prism-did` crate at
merged revision `fe3a14b767a6b13f8aa2fb2acf92bc6e37d93c03`. It owns only the
portable method kernel: bounded syntax, exact signed-operation bytes, typed
CRUD operations, deterministic state reduction, document projection, and a
local long-form resolver. Private-key custody, Cardano, VDR storage behavior,
persistence, HTTP, and node lifecycle are outside that crate.

PR #324 already establishes the generic SDK compatibility edge for crypto,
DID values, DID resolution, and HTTP. Replacing NeoPRISM's method behavior in
the same PR would make review, rollback, and Preprod comparison too broad.
Issue #345 therefore needs a second, adapter-first slice.

NeoPRISM does not currently preserve the exact signed-operation bytes at that
boundary. Its Cardano sources decode `PrismObject`, and persistence re-encodes
each `SignedPrismOperation`. Protobuf duplicate singular fields and unknown
fields can therefore be lost before SDK parsing. NeoPRISM state also combines
SSI state, VDR storage state, protocol-version selection, and ledger timestamps,
while `PrismDidState` deliberately owns only the portable SSI substate. Both
differences must be resolved explicitly; a package substitution cannot do so.

## Decision

### Adopt one immutable method kernel

NeoPRISM will consume `identus-prism-did` from one immutable SDK-Rust commit.
The first implementation revision is selected only after PR #324 lands and
must be recorded in `Cargo.toml`, `Cargo.lock`, the PR, and the parity receipt.
All SDK crates used by one NeoPRISM revision must resolve from the same SDK-Rust
commit.

SDK-Rust is authoritative for the supported Rust method profile. The pinned
PRISM method specification is normative for protocol semantics. NeoPRISM,
SDK-TS, and historical Apollo/JVM behavior are compatibility evidence, not
authority over public Rust DTOs or errors.

### Preserve the NeoPRISM boundary during the canary

The existing `identus-did-prism` package remains NeoPRISM's public and runtime
facade during the canary. Its method-facing modules delegate inward to
`identus-prism-did` and translate explicitly at the boundary.

The facade may preserve deprecated NeoPRISM names temporarily, but no new
SDK-facing type may use Apollo, Castor, Mercury, or other historical domain
names. Generated protobuf types remain private on both sides and do not become
the compatibility API.

The initial mapping is:

| NeoPRISM responsibility | SDK-Rust owner | Canary disposition |
| --- | --- | --- |
| `did::{PrismDid, CanonicalPrismDid, LongFormPrismDid}` | `identus_prism_did::{PrismDid, CanonicalPrismDid, LongFormPrismDid}` | delegate syntax and conversion; retain deprecated facade types only where callers require them |
| SSI create/update/deactivate values and validation | typed SDK operation values and builders | translate at one fallible adapter; do not expose SDK private wire structs |
| signed envelope parsing, operation hash, operation ID | `RawPrismOperation` and `SignedPrismOperation` | add an observational NeoPRISM raw-envelope scanner before protobuf materialization; preserve legacy derived values while the SDK shadow uses the exact slice |
| SSI DID state transition | `PrismDidState` | use as the SSI substate inside a NeoPRISM-owned composite shadow reducer |
| DID document projection | SDK projection functions | project through SDK, then map to the existing NeoPRISM response facade while #324 remains active |
| local long-form resolution | `PrismDidResolver` | use SDK local resolution; short-form history continues through NeoPRISM adapters |
| storage/VDR operations | none in this slice | retain in NeoPRISM and evaluate them against the composite's current SDK SSI substate; later `identus-prism-vdr` adoption needs a separate issue |
| protocol-version updates | none in this slice | NeoPRISM retains processor selection; the SDK SSI substate rejects these as non-SSI operations |
| ledger timestamps and storage cascade | none in this slice | NeoPRISM composite sidecars retain key/service lifecycle metadata and revoke storage entries on DID deactivation |
| Cardano/indexing/persistence/runtime | none in this slice | retain ownership in NeoPRISM; add an in-memory SDK shadow index, build an isolated exact-byte candidate during rehearsal, and select it only at an approved cutover |

### Preserve exact bytes before SDK parsing

NeoPRISM will add a bounded observational scanner at the raw `PrismObject`
metadata boundary. In `legacy` and `shadow` modes, the current protobuf decoder
remains authoritative for the accepted `block_content`, operation set, and OSN
ordering. The scanner aligns exact signed-operation slices with that decoded
sequence. Duplicate `block_content`, empty/unknown operations, unknown fields,
and alignment failure must not change legacy acceptance or renumber operations;
they fall back to the current decode, mark the SDK shadow comparison
non-comparable, and emit a bounded divergence metric.

`sdk` mode never consumes a re-encoded fallback row. Its re-index and live
ingestion read raw chain-source bytes. An envelope that cannot be bounded,
decoded, or aligned receives a specifically classified envelope rejection or
the governance-approved catch-all rejection `unclassified-envelope`. The
catch-all covers oversized, malformed, unknown, and non-aligning envelopes not
matched by a more specific rule. Rejecting an envelope rejects all of its
operations. It records the bounded ledger coordinate, reason, and safe digest
but no derived operation/state mutation or unbounded attacker bytes.

Once an envelope is aligned, every exact signed-operation slice is parsed and
validated independently in OSN order. A stable operation-level rejection,
including duplicate signed fields, rejects only that operation; valid siblings
continue through the reducer. `ResourceLimitExceeded` is the exception defined
below: it quarantines that DID from its first failing operation onward. Envelope
and operation rejection both advance the observed ledger cursor, emit bounded
critical telemetry, and do not halt the indexer. Other valid transactions and
DIDs in the same block continue. Only an internal invariant failure may stop
ingestion; attacker-controlled syntax always maps to deterministic rejection.
The catch-all and all specific rejection categories require protocol-governance
approval before `sdk` mode can be promoted on a network.

For aligned operations, `legacy` mode continues to persist bytes produced by
`encode_to_vec`; upgrading or enabling the default mode therefore does not
silently change an existing storage/API contract. `shadow` keeps the exact
slice only in its bounded in-memory index. The legacy path continues to decode
and re-encode for its historical operation hash, operation ID, signature
verification, DID grouping, derived index rows, and indexer API output. The SDK
shadow alone uses exact bytes for those decisions. Any difference is evidence
to classify.

`sdk` mode persists the exact aligned slice in
`raw_operation.signed_operation_data`, but only in a clean candidate database
built by the offline full-chain rehearsal and later selected atomically. It must
not update production legacy rows through the current conflict-ignoring upsert,
because that would mix exact and re-encoded rows. Before promotion, the
implementation audits every reader of the field,
including raw-operation parsing, database views, hash derivation, and the
indexer HTTP API, and proves which representation each external response uses.
Fixtures prove that `legacy` output is unchanged and that `sdk` output is
reproducible from exact source bytes.

Exact-byte retention alone changes no database schema, but existing rows cannot
prove their original encoding. Promotion therefore requires a clean full
re-index from the immutable chain source. A database assembled from re-encoded
legacy rows is not valid parity evidence. Any durable profile-limit quarantine
marker introduced below uses a versioned migration when the existing schema
cannot represent it. Duplicate security-relevant fields must reach the SDK
scanner so its strict rejection remains effective. Fixtures cover duplicate
`block_content`, duplicate signed fields, unknown fields, empty/unknown
operations, scanner fallback, stable legacy OSN assignment, and clean cutover
without mixed raw-row encodings.

### Compose portable SSI state with NeoPRISM-owned state

`PrismDidState` replaces only the SSI method substate. NeoPRISM owns a composite
shadow state containing:

- the SDK SSI state;
- existing VDR storage entries and their terminal/head relationships;
- ledger-derived key/service creation and revocation timestamps; and
- the active protocol-version processor.

The composite applies operations in ledger order. SSI operations first update
a cloned SDK state and then update ledger metadata sidecars. Storage operations
remain NeoPRISM-owned and verify against the active VDR key exposed by the SDK
SSI state. DID deactivation commits atomically with the existing storage-entry
revocation cascade. Protocol-version updates change only the NeoPRISM processor.
Any failure leaves the whole composite unchanged.

This composite is an adopter adapter, not a second method implementation. It
may be deleted only after the corresponding SDK VDR/indexer capabilities are
adopted in later issues.

### Use dual-path differential execution before deletion

The first implementation keeps the legacy method path as the only writer of
production legacy tables. A `did-method-engine` configuration key accepts
`legacy`, `shadow`, or `sdk` and defaults to `legacy`. `shadow` reads the same exact raw operation
stream, executes the SDK composite in memory, records comparison telemetry,
and never mutates persisted derived state. It builds a separate in-memory
DID/operation/VDR index from exact bytes instead of querying legacy-derived DID
or hash rows.

The shadow runs in an isolated, supervised worker with bounded non-blocking
input, entry, raw-byte, and estimated retained-memory budgets. Budget accounting
happens before allocation. Queue saturation, budget exhaustion, worker failure,
or an unwinding panic disables comparison, marks the remaining range
non-comparable, and emits bounded critical telemetry; it never blocks, applies
backpressure to, or changes the result of legacy ingestion. Production builds
must preserve an isolation mechanism that cannot turn a shadow panic into a
process abort. Tests inject queue saturation, budget exhaustion, and worker
failure while proving that the legacy cursor continues.

Because this first shadow index is not persisted, every shadow start or late
enablement rebuilds it from the immutable chain source starting at the agreed
network origin before reporting comparisons. It never bootstraps from legacy
raw or derived rows. The #325 promotion receipt comes from one uninterrupted,
complete origin-to-tip scan; a restart invalidates that receipt and starts a
new run. Peak shadow memory and all configured budgets are part of its QoS
evidence.

Pre-promotion node startup rejects `did-method-engine=sdk`. After promotion,
`sdk` startup requires a durable selected-candidate record in guarded deployment
configuration outside the candidate database. It contains the candidate's
backend identity, network, SDK/NeoPRISM revisions, schema/migration digest,
immutable source range/cursor, and deterministic ordered-operation and
composite-state/projection manifest digests from #325. Absence or mismatch fails closed
before migrations, listeners, or ingestion.

The record is signed by a cutover authority whose verification key is pinned
outside the candidate database. Rehearsal cannot write guarded deployment
configuration. At cutover, the authority opens a read-only snapshot, recomputes
the identity and manifest digests, checks #325 and protected CI evidence,
revokes rehearsal write access, and signs the exact record. Only afterward is a
separate SDK runtime writer enabled. Node startup verifies the signature and
the immutable baseline record, so candidate contents alone cannot authorize
serving. Runtime rows after the signed source cursor are normal post-cutover
ingestion and do not rewrite that baseline.

The baseline revisions and schema digest identify the candidate that was
validated; they are not an accidental permanent ban on upgrades. Every later
runtime binary requires a separate signed runtime authorization that references
the immutable baseline receipt, exact allowed SDK/NeoPRISM revisions, network,
and the previous and target schema digests. Normal startup never applies an
unapproved migration. A guarded offline `sdk-upgrade` flow rehearses the
migration on a snapshot, captures backup/rollback and invariant evidence,
applies the approved chain with a migration-only role, and publishes a signed
migration receipt.

The migration executor cannot sign either artifact. After migration it loses
write access; an independent upgrade authority opens a read-only snapshot,
recomputes the schema and evidence digests, and verifies that the migrated
database through the pinned cursor equals the target runtime's signed manifests
before signing the receipt and runtime authorization with a separately pinned
key. For the bounded profile-remediation exception below, this pre-authorization
equality applies to unaffected DIDs; each affected DID must equal its target
full-replay per-DID manifest through the actual cutover cursor `C` inside the
atomic targeted cutover before commit. Authorizations carry a monotonic
sequence and superseded/revoked revisions in guarded external configuration.
Startup accepts only the latest non-revoked sequence and verifies the complete
signed authorization/receipt chain from the baseline to the current schema and
exact runtime revisions before listeners or ingestion. Starting an older
authorization requires an explicit signed rollback authorization and its
compatible restored snapshot; restoring a database backup alone is insufficient.

Every upgrade pins one immutable upgrade cursor and differentially replays both
the currently authorized and target runtimes from network origin through that
cursor on isolated clones. The upgrade authority signs the paired cursor,
ordered-operation/acceptance digest, composite-state digest, and projection
digest as a new link in the authorization chain; it does not compare a later
tip directly with the older cutover baseline. Exact acceptance and composite-
state equality permits an in-place behavior-neutral upgrade. A projection-only
difference may use an explicitly signed non-semantic authorization only after
classification as a presentation defect, API compatibility evidence, and
release notes; it cannot hide an acceptance or internal-state difference.

A change in ledger acceptance, exact operation IDs, or composite SSI/VDR/
timestamp/protocol state is otherwise not an in-place upgrade: it requires
protocol governance or version activation, a new isolated full re-index, and a
new promotion baseline/cutover. The sole exception is an approved SDK profile-
limit remediation for already quarantined DIDs. Its signed authorization names
the profile revision, affected DIDs and first failing operations, proves exact
manifest equality for every unaffected DID, and uses the targeted sole-writer
replay/cutover defined below for each affected DID. It cannot change acceptance
for a previously unquarantined DID. An unapproved build, broken chain,
unexpected schema, or unexplained manifest difference fails closed.

A separate offline `sdk-rehearsal` command is the only pre-promotion SDK writer.
It has no listener or service endpoint and can write only to a dedicated
candidate database, never merely another schema in the production database.
The command requires both production and candidate connections, compares
backend-owned immutable identities rather than connection strings, and refuses
equal identities. Rehearsal re-indexes and quarantine migrations never modify
production legacy tables. Backend isolation is explicit:

- **PostgreSQL:** the production connection uses a read-only identity role with
  no table, schema, migration, or role-management writes. A candidate writer
  role is provisioned out of band with privileges limited to the candidate
  database; rehearsal never holds cluster-global role-creation credentials.
  Identity is the tuple of PostgreSQL system identifier, database OID, and a
  generated database UUID, allowing another database in the same cluster while
  detecting endpoint aliases.
- **SQLite:** the production identity connection opens with read-only/immutable
  flags. The candidate is a separate file on a candidate-only volume; canonical
  path, device/inode identity, and a database UUID must all differ from the
  production file. Filesystem ownership permits rehearsal/SDK writes but denies
  the legacy deployment access to the candidate volume.

The rehearsal writer initializes only candidate-local identity and manifest
metadata; neither is a selection authority. When `sdk` is enabled for an
approved network, it serves only after that candidate has been rebuilt from
exact source bytes, validated, bound to the externally signed selection record,
and atomically selected. Selecting the candidate database is the
promotion/cutover event, not a pre-promotion evidence step. For SQLite the
selection record remains outside the database file, so the cutover authority
never needs write access to candidate rows.

Candidate endpoint/role credentials or SQLite volume access are never
provisioned to legacy deployments, including already-published binaries.
Network, database, and filesystem controls reject legacy access to the
candidate. The promotion and rollback receipts prove this separation; rollback
points the legacy deployment only at the restored or freshly rebuilt legacy
endpoint/file.

For every operation in the shared supported profile, the harness compares:

1. parse outcome and stable error category;
2. exact operation hash and operation ID;
3. ordered state transition outcome;
4. terminal composite state, including SSI state, storage state, timestamps,
   and protocol version, but excluding presentation projection;
5. projected DID document through its separate projection digest; and
6. deactivation and unsupported-version behavior.

Differences are classified as:

- **spec correction:** SDK behavior follows the pinned specification but changes
  ledger acceptance; promotion stops until PRISM protocol governance approves
  the rule for all implementations or activates it through a protocol version;
- **historical compatibility requirement:** ledger evidence requires a bounded
  compatibility rule in SDK-Rust;
- **NeoPRISM presentation defect/dead behavior:** projection, error text, or
  telemetry differences that cannot change whether an operation is accepted
  may be corrected with evidence and release notes. Every acceptance difference
  still requires protocol governance or version activation.
- **SDK profile limit:** finite SDK resources would make an otherwise accepted
  DID unresolvable. Promotion requires zero occurrences in the complete history
  of the target network and explicit network/profile approval. After promotion,
  any new occurrence fails closed, emits a critical metric/alert, and must not
  skip the operation or fall back to stale state. The failure is isolated to
  that DID: the raw operation and DID association are recorded, the ledger
  cursor advances, other DIDs continue indexing, and resolution returns a
  stable `profile-limit-exceeded` error from the first failing operation onward.
  Every later operation for the quarantined DID, including storage and
  deactivation operations, is retained as raw evidence but does not mutate its
  derived SSI or VDR state. The last good derived state is never served as
  current. A durable quarantine marker identifies the first failing operation;
  restart and re-index reproduce it deterministically. Raising a limit does not
  resume incrementally: a governed profile revision triggers a targeted
  origin-to-tip replay for that DID (or a full network re-index) using the
  recorded protocol-version timeline. Resolution continues returning the error
  while replay and live catch-up run. Final cutover executes inside the sole
  ingestion writer's ordering. It locks the DID and fixes the global indexer's
  committed cursor `C`. Before commit, an isolated target replay consumes every
  retained raw operation through that same `C`; its per-DID manifest must equal
  the staged replacement. Only then does the transaction replace every DID-
  derived SSI row, VDR row, key/service timestamp sidecar, and projection,
  record `C`, and clear the quarantine marker. A mismatch rolls back the whole
  transaction, retains quarantine, and emits a critical alert. An
  operation ordered after `C` is processed normally only after that transaction;
  no cursor gap can be skipped. Resolution resumes only after the complete
  cutover succeeds. If replay still exceeds the revised profile, no derived
  state is replaced and the marker atomically moves to the first operation that
  fails under that profile. The global protocol-version processor is not
  replaced by a DID replay.

No donor code is deleted merely because unit tests pass.

### Keep security and resource invariants at the SDK edge

The adapter must preserve the SDK invariants:

- signature verification and hashes consume the exact bounded nested operation
  bytes received on the wire;
- duplicate security-relevant protobuf fields are rejected;
- Ed25519 and X25519 public keys require exact widths;
- the Apollo/JVM secp256k1 compatibility fallback remains covered by the pinned
  50-vector suite;
- update actions are ordered and atomic, retired identifiers cannot be reused,
  and active limits do not count tombstones;
- long-form and projection limits are enforced before attacker-controlled
  allocation or work; and
- authentication, authorization, and previous-operation linkage are verified
  before a state-growth limit can quarantine a DID;
- `ResourceLimitExceeded` fails current resolution and shadow processing from
  the first failing operation onward; an immutable earlier version may resolve
  only when the public API explicitly supports versioned resolution. The error
  is never mapped to NeoPRISM's legacy skip-invalid-operation path; and
- errors crossing public/HTTP boundaries remain stable and redact sensitive
  bytes.

The adapter may narrow an SDK capability that NeoPRISM cannot honor. It must
never silently accept and ignore an SDK option or operation.

### Measure operational parity before switching the default

Issue #325 is the Preprod promotion gate. The published legacy node and the SDK-backed
candidate scan the complete agreed immutable Preprod history and produce comparable ordered
operation, composite-state, and projection manifests. The receipt also records scan duration,
CPU time/utilization, peak resident memory, exact binaries/images, revisions,
configuration, network, range, and host characteristics.

The SDK-backed path becomes the default for a specific network only when:

- operation, composite-state, and projection parity is exact at the agreed boundary;
- every non-comparable scanner occurrence in the complete target-network
  history is either eliminated by an exact-byte adapter improvement or
  classified and resolved through the same presentation, compatibility,
  protocol-governance, or profile-limit rules; unresolved occurrences block
  promotion;
- every acceptance difference is either removed or activated by an approved
  network-wide protocol-version decision, not a local NeoPRISM migration;
- QoS change is understood and accepted;
- exact-head protected CI and independent review are green;
- a rehearsal full-chain re-index has produced exact raw-operation storage and
  SDK-derived index rows in an isolated candidate database, plus a
  versioned quarantine migration when required, without modifying production
  legacy tables;
- a read-only candidate snapshot reproduces the #325 ordered-operation and
  composite-state/projection manifest digests, and the signed selection record binds those
  digests to the exact backend identity, schema, revisions, network, and source
  cursor after rehearsal write access is revoked;
- the complete pre-cutover legacy schema/data snapshot is captured and tested,
  or the clean legacy re-index route and immutable source are confirmed, before
  the candidate database can be selected; and
- the rollback receipt covers a code revert plus either restoration of the
  complete pre-cutover legacy schema/data snapshot followed by replay from that
  snapshot's cursor, or a clean legacy re-index from network origin into a fresh
  legacy schema. The legacy binary never opens or writes SDK-cutover tables.

The #325 receipt authorizes Preprod only. Mainnet remains on `legacy` until a
separate complete Mainnet shadow receipt and explicit promotion decision satisfy
the same operation, composite-state, profile-limit, QoS, re-index, and rollback
criteria.

## Consequences

- NeoPRISM can shrink toward Cardano adapters, persistence, service composition,
  and operations without making the migration irreversible.
- Temporary method duplication is intentional and measurable.
- The compatibility facade and composite shadow state add temporary translation
  code, but isolate product effects from portable method behavior.
- Exact-byte persistence and SDK-derived index rows require a full target-network
  re-index before promotion. Existing table definitions can be reused only in
  the isolated candidate database; a durable quarantine marker may
  require a versioned migration there.
- A protocol correction that changes accepted ledger history cannot be shipped
  as a local migration.
- `identus-prism-vdr` and `identus-prism-indexer` adoption remain independent
  milestones with their own operational evidence.

## Alternatives rejected

- **Replace the whole `did-prism` crate at once:** mixes portable method,
  storage, Cardano, and runtime behavior and makes parity failures hard to
  isolate.
- **Copy the SDK implementation back into NeoPRISM:** restores duplication and
  defeats independent SDK release/adoption evidence.
- **Expose generated protobuf DTOs as the adapter contract:** couples public
  APIs to one runtime and risks decode/re-encode security errors.
- **Treat SDK-TS or the old Rust implementation as normative:** freezes donor
  DTO/error choices and known deviations instead of following the specification.
- **Delete the legacy path before Preprod comparison:** removes the differential
  oracle and makes rollback operationally expensive.

## Verification

The implementation PR must attach:

- the exact SDK and NeoPRISM revisions and signed/DCO commit evidence;
- source-to-target and fixture provenance mappings;
- legacy/SDK differential vectors for syntax, exact wire bytes, CRUD/state,
  projection, limits, malformed input, and crypto compatibility;
- reader-audit and mixed-row-prevention evidence for
  `raw_operation.signed_operation_data`, including identical legacy API output
  before and after the adapter is installed;
- `sdk`-mode liveness fixtures proving that `profile-limit-exceeded` and
  `unclassified-envelope` advance the ledger cursor, preserve raw evidence,
  continue valid DIDs/transactions in the same block, emit critical alerts, and
  reproduce the same result after restart and full re-index;
- a concurrent-recovery fixture proving operations that arrive during replay
  are applied exactly once on the correct side of cutover cursor `C`, plus a
  failed-remediation fixture that retains quarantine at the newly computed
  first failing operation;
- aligned-operation fixtures proving one strict parse/validation rejection keeps
  valid sibling operations, while an envelope rejection drops the whole
  envelope with a stable category;
- a stable public `profile-limit-exceeded` error variant with specified HTTP
  status/body and redaction tests;
- shadow fault-injection evidence for queue saturation, entry/byte/memory budget
  exhaustion, worker failure, and panic isolation while legacy ingestion keeps
  advancing;
- startup/cutover evidence that pre-promotion `sdk`, absent or mismatched
  external selection records, and wrong database identity, manifest, schema,
  network, revision, or source receipts fail before migration, listeners, or
  ingestion;
- authority-separation evidence that rehearsal cannot write guarded selection
  configuration, its write access is revoked before signing, the cutover
  identity cannot rewrite candidate data, and startup rejects forged, unsigned,
  incorrectly signed, or digest-mismatched promotion receipts;
- post-cutover upgrade fixtures proving an exactly authorized binary and
  rehearsed migration chain can start, while an unapproved revision, unexpected
  schema, missing receipt, failed migration, or broken signature chain fails
  closed and preserves the rollback backup;
- migrated-snapshot evidence proving its rows through the pinned upgrade cursor
  reproduce the target runtime's signed acceptance, composite-state, and
  projection manifests before authorization, except that profile remediation
  compares unaffected DIDs before cutover and each affected DID against an
  isolated target replay through the same live cutover cursor `C` inside the
  transaction, with mismatch rollback evidence;
- upgrade authority-separation and anti-rollback evidence proving the migration
  executor cannot sign, write access is revoked before independent read-only
  attestation, superseded authorizations cannot start, and a database backup
  alone cannot roll back the authorized runtime sequence;
- origin-to-pinned-cursor differential evidence for every SDK upgrade, comparing
  current and target runtimes at the same cursor, with signed acceptance,
  composite-state, and projection digest pairs; exact acceptance/state equality
  for behavior-neutral upgrades; documented authorization for projection-only
  fixes; and a new governed promotion baseline for other semantic changes;
- profile-remediation evidence naming every quarantined DID/first failure,
  proving all unaffected manifests remain equal, and exercising atomic targeted
  replay without changing any previously unquarantined DID;
- isolation evidence that `sdk-rehearsal` rejects the production database by
  backend-owned identity even through endpoint/path aliases, and that candidate
  endpoint or file controls deny legacy deployment credentials/access;
- privilege-denial evidence that the rehearsal production identity role cannot
  write tables, schemas, migrations, or roles, and that its candidate writer
  role has no authority outside the candidate database, plus SQLite evidence
  for immutable production opens, distinct path/device/inode/UUID identities,
  and candidate-volume permissions;
- PostgreSQL identity evidence for the system-identifier/database-OID/database-
  UUID tuple, including same-cluster databases and endpoint aliases;
- NeoPRISM Nix check, strict Clippy, tests/doctests, conformance, coverage, and
  file-hygiene evidence;
- the issue #325 complete Preprod operation/state/profile-limit/QoS receipt;
- an independent exact-commit review receipt; and
- delivery metrics permitted by project policy.

## Rollback

Before promotion, select `did-method-engine=legacy` and remove the read-only
shadow adapter; no SDK-derived database rows or exact-slice replacements exist
in the production legacy tables, and legacy storage/output semantics remain
unchanged. The unselected rehearsal candidate is discarded or remains clearly
marked unselected and is never opened by the legacy binary; it must be rebuilt
or fully revalidated before any later selection. After promotion, rollback
never runs the reverted binary against the SDK-cutover database. It requires a
code revert plus either restoration of the complete pre-cutover legacy
schema/data snapshot followed by replay from its recorded cursor, or a clean
legacy re-index from network origin into a fresh legacy schema. Migration
forward/backward compatibility may be tested as supplemental evidence, but
never authorizes in-place rollback or replaces either clean rollback route.
Changing SDK public method semantics requires an SDK-Rust issue and ADR;
NeoPRISM must not carry an unreviewed local fork.
