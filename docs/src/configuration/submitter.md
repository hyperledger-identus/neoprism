# Submitter Configuration

The **Submitter node** publishes PRISM DID operations to the Cardano blockchain.  
It is typically used for creating, updating, or deactivating DIDs.

## DLT Sink

The Submitter node requires a DLT sink to sign and submit transactions.  
Select the sink type with the `--dlt-sink-type` flag:

```
--dlt-sink-type <TYPE>    or    NPRISM_DLT_SINK_TYPE=<TYPE>
```

Supported values:

| Value | Description |
|-------|-------------|
| `cardano-wallet` | Uses an external Cardano wallet service to sign and submit transactions |
| `embedded-wallet` | Uses a built-in subprocess-based wallet — no external wallet service required |

---

## Cardano Wallet

Uses a [Cardano wallet](https://github.com/CardanoSolutions/cardano-wallet) service to sign and submit transactions containing DID operations.

| Flag | Environment Variable | Description |
|------|---------------------|-------------|
| `--cardano-wallet-base-url` | `NPRISM_CARDANO_WALLET_BASE_URL` | Base URL of the Cardano wallet service |
| `--cardano-wallet-wallet-id` | `NPRISM_CARDANO_WALLET_WALLET_ID` | Wallet ID to use for transactions |
| `--cardano-wallet-passphrase` | `NPRISM_CARDANO_WALLET_PASSPHRASE` | Passphrase for the wallet |
| `--cardano-wallet-payment-addr` | `NPRISM_CARDANO_WALLET_PAYMENT_ADDR` | Payment address for transactions |

> **Important:**
> When the submitter publishes a DID operation, it creates a transaction from the configured wallet to the specified payment address.
> Make sure you use your own payment address. Using an incorrect or third-party address may result in permanent loss of funds.

---

## Embedded Wallet

Uses a companion binary to sign and submit transactions without running a separate Cardano wallet service. The submitter spawns the binary as a subprocess for each transaction — it builds the transaction, signs it with the provided mnemonic, and submits it to the network.

Transactions are submitted via a **Cardano Submit API** endpoint (`--embedded-wallet-submit-api-url`) or through **Blockfrost** (default). Blockfrost is also used to resolve UTXOs during the build step.

| Flag | Environment Variable | Description |
|------|---------------------|-------------|
| `--embedded-wallet-bin` | `NPRISM_EMBEDDED_WALLET_BIN` | Path to the embedded wallet binary |
| `--embedded-wallet-mnemonic` | `NPRISM_EMBEDDED_WALLET_MNEMONIC` | Mnemonic phrase for the wallet (mutually exclusive with `--embedded-wallet-mnemonic-file`) |
| `--embedded-wallet-mnemonic-file` | `NPRISM_EMBEDDED_WALLET_MNEMONIC_FILE` | Path to a file containing the mnemonic phrase (mutually exclusive with `--embedded-wallet-mnemonic`) |
| `--embedded-wallet-submit-api-url` | `NPRISM_EMBEDDED_WALLET_SUBMIT_API_URL` | Cardano Submit API URL (omit to submit via Blockfrost) |
| `--embedded-wallet-blockfrost-url` | `NPRISM_EMBEDDED_WALLET_BLOCKFROST_URL` | Blockfrost API URL for private instances (default: `https://cardano-mainnet.blockfrost.io/api/v0`) |
| `--embedded-wallet-blockfrost-api-key` | `NPRISM_EMBEDDED_WALLET_BLOCKFROST_API_KEY` | Blockfrost API key for public instances (takes precedence over `--embedded-wallet-blockfrost-url` when set) |

> **Note:** When `--embedded-wallet-blockfrost-api-key` is set, it takes precedence over `--embedded-wallet-blockfrost-url` for Blockfrost requests. The submitter uses the default public Blockfrost URL for both UTXO resolution and submission in this case. To submit via a Cardano Submit API endpoint instead of Blockfrost, set `--embedded-wallet-submit-api-url`.

> **Note:** For network parameters (mainnet, preprod, etc.), use the shared `--cardano-network` flag documented in the [Indexer configuration](./indexer.md).

> **Security recommendation:** Prefer `--embedded-wallet-mnemonic-file` over `--embedded-wallet-mnemonic` in production deployments. Environment variables can leak via `/proc/<pid>/environ`, process listings, crash dumps, or log output. Loading the mnemonic from a file allows you to restrict file permissions (e.g., `chmod 600`) and use container secret mounts (Docker/Kubernetes secrets). Supplying both flags simultaneously is treated as a configuration error and will prevent the node from starting.

---

## DLT Sink Comparison

**Cardano Wallet**

The traditional approach that requires a full Cardano wallet service running alongside the node. The wallet handles key management, transaction construction, and submission. This is a good option when you already operate a Cardano wallet as part of your infrastructure, but it adds operational overhead of maintaining an additional service.

**Embedded Wallet**

A lightweight alternative that eliminates the need for an external wallet service. It uses a companion binary to construct and sign transactions as a subprocess. By default, transactions are submitted via Blockfrost; a Cardano Submit API endpoint can be configured as an alternative. This is the recommended option for most deployments due to its simpler operational model.

---

**Next Steps:**

- [CLI Options](../references/cli-options.md): Full list of flags and environment variables.
