# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Product Purpose

NeoPRISM is a Rust PRISM node for managing decentralized identifiers anchored on Cardano. Its existing web UI provides DID resolution, exploration of indexed DIDs, and access to API documentation.

## Capabilities and Constraints

- Existing interfaces include Resolver, Explorer, and API Docs.
- DID inspection exposes public keys, services, VDR entries, and operation/block information.
- The application uses Rust/Maud server-rendered views, Tailwind CSS, and DaisyUI.
- Local blockchain testing is available through `docker/prism-test/`.
- The user selected **B — Prism** from the three OpenPencil mockups, then requested full desktop/mobile feature-parity designs for every existing page and component.
- The expanded static library is `designs/neoprism.fig` (26 artboards; the generated Scalar API page is excluded at the user’s request). Scope, parity and proposed interactions are recorded in `designs/PRISM-SPEC.md`. Application implementation has not started.

## Brand Commitments

- Preserve the NeoPRISM name and its connection to Hyperledger Identus.
- Use the supplied `designs/neoprism.svg` logo in the current Prism library;
  fingerprint branding is historical concept artwork only.
- The user requested glass-style alternatives referencing `designs/workspace/51f7f90560d2dec85033d2deb13a90f4.jpg` and `designs/workspace/9976dce1cf96a85d3f5352633c333cf8.jpg`.
- The user supplied <https://identus.io/> as an additional related-project style reference.

## Evidence on Hand

- Reviewed v0.15.1 test-stack UI and source under `bin/neoprism-node/src/http/`.
- Screenshots and measurements: `designs/workspace/ui-review/`.
- Seeded local-testnet DID: `did:prism:e3b675023ef13a2bd1b015a6b1c88d2bfbfbb09bed5b675598397aec361f0d6e`.
- During review, this DID's UI displayed indexed keys while its Resolver API returned HTTP 500. Mockups must not imply a successful standards-compliant API response.

## Open Decisions

Primary user roles and usage frequency remain unconfirmed. The visual direction is **B — Prism**, selected by the user. The full static desktop/mobile design scope is complete for review; expanded interaction proposals and production implementation still need approval. Continuation instructions and saved designs are under `designs/workspace/` (start with `designs/workspace/neoprism-ui-handoff.md`).
