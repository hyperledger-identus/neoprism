# SDK-Rust adoption

NeoPRISM reuses generic cryptography, DID syntax, DID resolution, and HTTP
resolver mechanics from
[SDK-Rust](https://github.com/hyperledger-identus/sdk-rust). NeoPRISM remains the
owner of PRISM operations, Cardano integration, persistence, service runtime,
OpenAPI composition, and product policy.

This boundary reduces duplicate security-sensitive code without turning the
generic SDK into a Cardano or NeoPRISM-specific component.

## Ownership boundary

```d2
sdk: "SDK-Rust (generic, reusable)" {
  crypto: "identus-crypto\nkeys, signatures, hashes"
  did: "identus-did\nDID/URI types and resolution contract"
  http: "identus-did-resolver-http\ngeneric HTTP resolver mechanics"
}

neoprism: "NeoPRISM (product and PRISM ownership)" {
  facade: "compatibility facades\nlegacy API and DTO projection"
  prism: "PRISM protocol\noperations and DID state"
  cardano: "Cardano\nindex, submit, ledger"
  runtime: "service runtime\nstorage, routing, OpenAPI"
}

sdk.crypto -> neoprism.facade: reusable primitives
sdk.did -> neoprism.facade: bounded domain types
sdk.http -> neoprism.runtime: transport mechanics
neoprism.facade -> neoprism.prism: stable local contract
neoprism.prism -> neoprism.cardano: operation lifecycle
neoprism.prism -> neoprism.runtime: resolved state
```

| Capability | SDK-Rust owns | NeoPRISM owns |
| --- | --- | --- |
| Cryptography | Generic key, signature, hash, and JWK primitives | Compatibility facade and PRISM-specific key policy |
| DID model | Bounded DID, DID URL, URI, document, and resolution types | Projection to legacy DTOs and PRISM state |
| Resolution | Generic resolver contract and result validation | PRISM resolution and error mapping |
| HTTP | Generic DID resolver request/response mechanics | Router mount path, dynamic state, OpenAPI, and deployment policy |
| Ledger/runtime | Nothing Cardano-specific | Cardano indexer, submitter, ledger, storage, node, and UI |

The reusable-module criteria are governed by
[SDK-Rust ADR 0110](https://github.com/hyperledger-identus/sdk-rust/blob/develop/docs/adr/0110-define-reusable-module-extraction-criteria.md).
The extraction inventory and rationale are maintained in
[NeoPRISM discussion 320](https://github.com/hyperledger-identus/neoprism/discussions/320).

## Compatibility contract

The SDK is authoritative for generic validation. NeoPRISM exposes explicit,
fallible adapters where its established API differs:

- empty optional arrays from legacy DID documents are omitted before SDK
  validation;
- only the `accept` resolution option is supported; unsupported or unknown
  options fail with `INVALID_OPTIONS` instead of being ignored;
- successful resolution responses honor the requested media type;
- mismatched result and document DIDs are rejected;
- Ed25519 public keys must have their exact encoded width;
- the established Apollo/JVM secp256k1 fallback remains covered by compatibility
  vectors;
- unknown extension error URIs use the SDK safe HTTP 500 fallback.

Adapters are migration boundaries, not copies of generic SDK behavior. New
generic functionality belongs in SDK-Rust; PRISM, Cardano, storage, and product
behavior belongs in NeoPRISM.

## Dependency policy

SDK-Rust Git dependencies are declared once in the workspace and pinned to an
immutable reviewed commit. Member crates inherit those declarations. Updating
the pin is a deliberate integration change and requires the same local,
protected-CI, review, and parity evidence as other adoption changes.

NeoPRISM's Rust/Nix toolchain may remain newer than SDK-Rust's baseline when the
application requires it. The SDK baseline must not force the application to
downgrade its own development environment.

## Verification and acceptance

```d2
candidate: "exact candidate commit"
local: "local gates\nfmt, check, Clippy, focused tests"
nix: "reproducible Nix check\nimmutable vendoring"
review: "independent agent review\nClaude or Codex"
ci: "protected hosted CI\nchecks, coverage, conformance"
image: "multi-architecture image\nbuild and runtime"
parity: "fixed-boundary Preprod parity\nold vs SDK-backed"
approval: "independent human approval"
merge: "guarded merge"

candidate -> local -> nix -> review -> ci -> image -> parity -> approval -> merge
```

Acceptance requires all of the following against the exact candidate:

1. formatting, workspace checks, strict Clippy, and focused crypto/DID/resolver
   tests pass locally;
2. the repository Nix check passes without mutable dependency resolution;
3. an independent Claude or Codex review has no unresolved blocker, high, or
   medium finding;
4. every required protected GitHub check is green;
5. native and cross-platform container builds are reproducible and runnable;
6. the published baseline and SDK-backed candidate scan the same fixed Preprod
   boundary and produce equivalent indexed operations;
7. scan duration, CPU, and memory evidence is recorded so QoS regression is
   visible;
8. an independent engineer approves the pull request.

The reproducible old/new harness, commands, artifacts, and metric definitions
are tracked in
[issue 325](https://github.com/hyperledger-identus/neoprism/issues/325). The
deterministic embedded-wallet packaging prerequisite is tracked in
[issue 326](https://github.com/hyperledger-identus/neoprism/issues/326) and its
[implementation PR 348](https://github.com/hyperledger-identus/neoprism/pull/348).
The SDK adoption itself is reviewed in
[PR 324](https://github.com/hyperledger-identus/neoprism/pull/324).

## Rollback

Revert the adoption pull request. The compatibility facades keep the local API
boundary explicit, and immutable SDK revisions make the previous dependency
graph reproducible. A rollback must retain the parity artifacts and incident
evidence rather than deleting them.
