## Indexer Mode Configuration

When running NeoPRISM in indexer mode, the following configuration options are available. You can set these using command-line flags or environment variables.

### Server Settings

See [Common Configuration](./common.md) for server settings.

### Database Settings

- `--db-url` / `NPRISM_DB_URL`  
  Database URL (e.g., `postgres://user:pass@host:5432/db`)
- `--skip-migration` / `NPRISM_SKIP_MIGRATION`  
  Skip database migration on node startup

### DLT Source Settings

Supported DLT sources:
- Oura
- DB-Sync

#### Common Options

- `--index-interval` / `NPRISM_INDEX_INTERVAL`  
  Number of seconds to wait before checking for unindexed operations (default: `10`)
- `--confirmation-blocks` / `NPRISM_CONFIRMATION_BLOCKS`  
  Number of confirmation blocks to wait before considering a block valid (default: `112`)

#### Oura Options

- `--cardano-network` / `NPRISM_CARDANO_NETWORK`  
  Cardano network to sync from (`mainnet`, `preprod`, or `preview`)
- `--cardano-relay-addr` / `NPRISM_CARDANO_RELAY_ADDR`  
  Address of the Cardano relay node to sync from (e.g., `backbone.mainnet.cardanofoundation.org:3001`)

#### DB-Sync Options

- `--cardano-dbsync-url` / `NPRISM_CARDANO_DBSYNC_URL`  
  DB-Sync URL (e.g., `postgres://user:pass@host:5432/db`)
- `--cardano-dbsync-poll-interval` / `NPRISM_CARDANO_DBSYNC_POLL_INTERVAL`  
  Number of seconds to wait before polling DB-Sync for the next update (default: `10`)

---

You can use either command-line flags or the corresponding environment variables to configure indexer mode. Adjust these options to fit your deployment and operational requirements.
