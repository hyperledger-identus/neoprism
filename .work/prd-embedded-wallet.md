# PRD: Embedded Wallet Submitter

## Overview

Add a lightweight submitter mechanism using a pure Rust embedded wallet implementation. The embedded wallet eliminates the dependency on `cardano-node` infrastructure, requiring only `cardano-submit-api` for transaction submission. This enables production deployments with minimal infrastructure overhead.

## Goals

- Provide a lightweight alternative to `cardano-wallet` submitter
- Support BIP39 mnemonic-based key derivation
- Enable transaction building and signing using pure Rust (pallas library)
- Support stake key derivation for PRISM operations
- Maintain feature parity with existing PRISM metadata format

## Quality Gates

These commands must pass for every user story:
- `just format` — Format all sources (Rust, Nix, TOML, Python, SQL, Hurl)
- `just test` — Run all tests
- `just check` — Full validation (format + build + test + clippy)

## User Stories

### US-001: Add pallas dependencies to workspace
**Description:** As a builder, I want the pallas crates added to the workspace so that the embedded wallet can use them.

**Acceptance Criteria:**
- [ ] Add `pallas-txbuilder`, `pallas-primitives`, `pallas-addresses` (v1.0.0-alpha) to workspace `Cargo.toml`
- [ ] Add `bip39` (v2.0) with `rand_core` feature
- [ ] Add `ed25519-bip32` (v0.4) for HD key derivation

### US-002: Create embedded-wallet feature flag
**Description:** As a builder, I want a feature flag to enable embedded wallet compilation without requiring cardano-wallet support.

**Acceptance Criteria:**
- [ ] Add `embedded-wallet` feature to `did-prism-submitter/Cargo.toml`
- [ ] Feature enables pallas and bip39 dependencies
- [ ] Feature is mutually exclusive with `cardano-wallet` feature at runtime (document in docs)
- [ ] Default features remain empty

### US-003: Implement BIP39/BIP32 key derivation
**Description:** As an operator, I want to provide a BIP39 mnemonic and have the wallet derive Ed25519 keys for signing transactions.

**Acceptance Criteria:**
- [ ] Parse BIP39 mnemonic phrase using `bip39` crate
- [ ] Derive BIP32 private key from mnemonic (with optional password)
- [ ] Derive payment key at path `1852H/1815H/0H/0/0` (Shelley payment account)
- [ ] Derive stake key at path `1852H/1815H/0H/2/0` (Shelley stake account)
- [ ] Expose public keys for address generation
- [ ] Define custom error types in `Error` enum following project conventions

### US-004: Implement Blockfrost UTXO query
**Description:** As an operator, I want to configure a Blockfrost API endpoint and query UTXOs for the payment address.

**Acceptance Criteria:**
- [ ] Accept Blockfrost API URL and project ID in configuration
- [ ] Query UTXOs for payment address via Blockfrost API
- [ ] Parse UTXO response into internal representation
- [ ] Implement largest-first UTXO selection algorithm
- [ ] Handle API errors with appropriate error types

### US-005: Build transaction with PRISM metadata
**Description:** As the system, I want to submit PRISM operations and have the wallet build a valid Cardano transaction with the correct metadata format.

**Acceptance Criteria:**
- [ ] Reuse PRISM metadata format from `cardano_wallet.rs`
- [ ] Construct transaction using `pallas-txbuilder`
- [ ] Include selected UTXOs as inputs
- [ ] Include change address output
- [ ] Add PRISM metadata CBOR to transaction
- [ ] Calculate and set appropriate fees
- [ ] Handle minimum ADA requirements per output

### US-006: Sign transaction
**Description:** As the system, I want the wallet to sign transactions with the derived payment private key.

**Acceptance Criteria:**
- [ ] Sign transaction hash with derived Ed25519 private key
- [ ] Use `pallas-crypto` for signing operations
- [ ] Attach signature to transaction witnesses
- [ ] Encode signed transaction as CBOR

### US-007: Submit transaction via cardano-submit-api
**Description:** As the system, I want to submit the signed transaction to `cardano-submit-api` and receive the transaction ID.

**Acceptance Criteria:**
- [ ] Accept `cardano-submit-api` endpoint URL in configuration
- [ ] Submit CBOR-encoded transaction via HTTP POST
- [ ] Parse response to extract transaction ID
- [ ] Handle submission errors (network, validation failures)
- [ ] Return transaction ID on success

### US-008: Integrate embedded wallet into submitter module
**Description:** As a developer, I want the embedded wallet integrated into the existing submitter architecture alongside cardano-wallet.

**Acceptance Criteria:**
- [ ] Create `lib/did-prism-submitter/src/dlt/embedded_wallet.rs`
- [ ] Implement `Submitter` trait from existing module interface
- [ ] Add factory function to create embedded wallet submitter
- [ ] Document configuration requirements in module docs
- [ ] Export module in `dlt/mod.rs` with cfg(feature = "embedded-wallet")

## Functional Requirements

- FR-1: The system must accept BIP39 mnemonic phrases (12, 15, 24 words)
- FR-2: The system must derive payment and stake keys following Cardano Shelley derivation paths
- FR-3: The system must query UTXOs from Blockfrost API for a given payment address
- FR-4: The system must select UTXOs using largest-first algorithm
- FR-5: The system must build transactions with PRISM metadata matching existing format
- FR-6: The system must sign transactions with derived Ed25519 keys
- FR-7: The system must submit transactions to `cardano-submit-api`
- FR-8: The system must return transaction ID on successful submission

## Non-Goals

- Hardware wallet support (Ledger, Trezor) — out of scope
- Integration tests with real Blockfrost instances — out of scope
- Cardano-node based functionality — use cardano-wallet feature instead
- UTXO selection algorithms other than largest-first — out of scope
- Wallet recovery or key rotation features — out of scope
- UTXO query caching — deferred to future iteration
- Multiple Blockfrost API keys for key rotation — out of scope

## Technical Considerations

- All dependencies use `1.0.0-alpha` versions from pallas, which are pre-release
- The `ed25519-bip32` crate is used for Cardano-specific key derivation (different from standard BIP32)
- Feature flag must not conflict with `cardano-wallet` feature at compile time
- Follow existing error handling patterns in `did-prism-submitter` (custom `Error` enum with `derive_more`)
- Follow `StdExternalCrate` import grouping as per AGENTS.md

## Success Metrics

- Compilation succeeds with `--features embedded-wallet`
- All tests pass with embedded-wallet feature
- Transaction submission succeeds via `cardano-submit-api`
- Transaction appears on-chain with correct PRISM metadata
- No dependency on `cardano-node` or `cardano-wallet` infrastructure

## Open Questions

None — all questions resolved.