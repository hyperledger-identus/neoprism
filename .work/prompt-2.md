## Overview

Support a new submitter mechanism using a pure Rust embedded wallet. The embedded wallet will be implemented using [pallas](https://github.com/txpipe/pallas) library, specifically the `pallas-txbuilder` and `pallas-crypto` crates, along with the BIP32 HD wallet functionality from the `pallas/examples/wallet` example.

## Goal

Provide a lightweight submitter option for neoprism. The only currently supported submitter is `cardano-wallet`, which is heavy as it requires `cardano-node` infrastructure. With the new embedded wallet, the only required dependency is `cardano-submit-api`.

## Input / Output

The wallet takes:
- mnemonic phrase (BIP39)
- payment address
- Blockfrost provider config

And builds a transaction to be submitted via `cardano-submit-api`.

## Architecture

The embedded wallet is a **pure Rust implementation** within the `did-prism-submitter` crate. No subprocess or stdio communication is required.

### Key Components

1. **Key Derivation** — Use BIP32 HD key derivation from mnemonic (based on `pallas/examples/wallet`)
   - `Bip32PrivateKey::from_bip39_mnemonic(mnemonic, password)`
   - Derive payment/stake keys following Cardano's derivation path

2. **Transaction Building** — Use `pallas-txbuilder` crate
   - Construct transaction with metadata (PRISM operations)
   - Handle UTXO selection from Blockfrost query

3. **Signing** — Use `pallas-crypto` for Ed25519 signatures
   - Sign transaction with derived private key

4. **Submission** — Submit CBOR-encoded transaction to `cardano-submit-api`

### Implementation Details

```
did-prism-submitter/
├── src/
│   ├── dlt/
│   │   ├── mod.rs
│   │   ├── cardano_wallet.rs      # existing
│   │   └── embedded_wallet.rs     # NEW
│   └── lib.rs
```

### New Feature Flag

Add `embedded-wallet` feature flag (similar to existing `cardano-wallet`):
```toml
[features]
default         = []
cardano-wallet  = ["dep:reqwest"]
embedded-wallet = ["dep:pallas-txbuilder", "dep:pallas-primitives", "dep:pallas-addresses"]
```

### Dependencies

Add to workspace `Cargo.toml`:
```toml
pallas-txbuilder   = { version = "1.0.0-alpha" }
pallas-primitives  = { version = "1.0.0-alpha" }
pallas-addresses   = { version = "1.0.0-alpha" }
bip39              = { version = "2.0", features = ["rand_core"] }
ed25519-bip32      = "0.4"
```

## User Stories

1. **Derive Keys from Mnemonic** — As an operator, I can provide a BIP39 mnemonic and have the wallet derive the necessary Ed25519 keys for signing.

2. **Query UTXOs via Blockfrost** — As an operator, I can configure a Blockfrost API endpoint and the wallet queries UTXOs at the payment address.

3. **Build and Sign Transaction** — As the system, I can submit PRISM operations and have the wallet build a valid Cardano transaction with the PRISM metadata.

4. **Submit Transaction** — As the system, I can submit the signed transaction to `cardano-submit-api` and receive the transaction ID.

5. **Feature Flag Toggle** — As a builder, I can compile neoprism with `embedded-wallet` feature without requiring `cardano-wallet` support.
