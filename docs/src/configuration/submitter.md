## Submitter Mode Configuration

When running NeoPRISM in submitter mode, the following configuration options are available. You can set these using command-line flags or environment variables.

### Server Settings

See [Common Configuration](./common.md) for server settings.

### Database Settings

- `--db-url` / `NPRISM_DB_URL`  
  Database URL (e.g., `postgres://user:pass@host:5432/db`)
- `--skip-migration` / `NPRISM_SKIP_MIGRATION`  
  Skip database migration on node startup

### DLT Sink Settings

Currently, the supported DLT sink is:
- Cardano wallet

#### Cardano Wallet Options

- `--cardano-wallet-base-url` / `NPRISM_CARDANO_WALLET_BASE_URL`  
  Base URL of the Cardano wallet
- `--cardano-wallet-wallet-id` / `NPRISM_CARDANO_WALLET_WALLET_ID`  
  Wallet ID to use for making transactions
- `--cardano-wallet-passphrase` / `NPRISM_CARDANO_WALLET_PASSPHRASE`  
  Passphrase for the wallet
- `--cardano-wallet-payment-addr` / `NPRISM_CARDANO_WALLET_PAYMENT_ADDR`  
  Payment address for making transactions

---

You can use either command-line flags or the corresponding environment variables to configure submitter mode. Adjust these options to fit your deployment and operational requirements.
