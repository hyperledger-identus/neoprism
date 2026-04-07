# Indexer Configuration

The **Indexer node** monitors the Cardano blockchain for PRISM DID operations, validates and indexes them, and enables efficient lookup of DID Documents.  
It is typically used for DID resolution and verification services.

## DLT Source

The Indexer node requires a DLT source to ingest DID operations from the Cardano blockchain.  
Select the source type with the `--dlt-source-type` flag:

```
--dlt-source-type <TYPE>    or    NPRISM_DLT_SOURCE_TYPE=<TYPE>
```

Supported values:

| Value | Description |
|-------|-------------|
| `oura` | Connects to a Cardano relay node and streams block data in real time |
| `dbsync` | Connects to a Cardano DB-Sync instance and polls for new blocks |
| `blockfrost` | Connects to the Blockfrost API for hosted Cardano blockchain data access |

In addition to source-specific options, all DLT sources share a common network setting and indexing configuration (see [Common Options](#common-dlt-source-options)).

---

## Oura

Connects to a Cardano relay node and streams block data in real time using the chainsync protocol.

| Flag | Environment Variable | Description |
|------|---------------------|-------------|
| `--cardano-relay-addr` | `NPRISM_CARDANO_RELAY_ADDR` | Address of the Cardano relay node (e.g. `backbone.mainnet.cardanofoundation.org:3001`) |

## DB-Sync

Connects to a Cardano DB-Sync instance and polls for new blocks and transactions.

| Flag | Environment Variable | Description |
|------|---------------------|-------------|
| `--cardano-dbsync-url` | `NPRISM_CARDANO_DBSYNC_URL` | DB-Sync URL (e.g. `postgres://user:pass@host:5432/db`) |
| `--cardano-dbsync-poll-interval` | `NPRISM_CARDANO_DBSYNC_POLL_INTERVAL` | Duration to wait before polling (e.g. `10s`, `1m`) |

## Blockfrost

Connects to the Blockfrost API for hosted Cardano blockchain data access. Requires a Blockfrost API key but eliminates the need to run your own Cardano infrastructure.

| Flag | Environment Variable | Description |
|------|---------------------|-------------|
| `--blockfrost-api-key` | `NPRISM_BLOCKFROST_API_KEY` | Blockfrost API key |
| `--blockfrost-base-url` | `NPRISM_BLOCKFROST_BASE_URL` | Blockfrost base URL |
| `--blockfrost-poll-interval` | `NPRISM_BLOCKFROST_POLL_INTERVAL` | Duration to wait between polls (e.g. `10s`, `1m`) |
| `--blockfrost-api-delay` | `NPRISM_BLOCKFROST_API_DELAY` | Throttling delay to respect rate limits |
| `--blockfrost-concurrency-limit` | `NPRISM_BLOCKFROST_CONCURRENCY_LIMIT` | API calls concurrency limit |

## Common DLT Source Options

These options apply regardless of the selected DLT source type:

| Flag | Environment Variable | Description |
|------|---------------------|-------------|
| `--cardano-network` | `NPRISM_CARDANO_NETWORK` | Cardano network (`mainnet`, `preprod`, `preview`, or `custom`; default: `mainnet`) |
| `--index-interval` | `NPRISM_INDEX_INTERVAL` | Duration between indexing cycles (e.g. `10s`, `1m`) |
| `--confirmation-blocks` | `NPRISM_CONFIRMATION_BLOCKS` | Number of confirmation blocks before considering a block valid (default: `112`) |

---

## DLT Source Comparison

**Oura**

Oura works by performing a chainsync protocol with a Cardano relay node.
This setup is quite lean, as you can connect to any available public relay.
The downside is that sync progress can be slow, since it performs a full sync from the blockchain.
If possible, connect to a Cardano node close to your location, as syncing across different geographic regions can be very slow.
The initial sync may take multiple days. The best option is to connect to your own Cardano node within the same network for optimal performance.

**DB Sync**

DBSync is a service that syncs the Cardano blockchain and writes the data to a PostgreSQL database.
DBSync is known to be resource-heavy and requires significant disk space.
The advantage is that sync speed is very fast, since NeoPRISM only needs to read the database tables and parse the operations.
If you can afford to run DBSync, it is recommended to use this option, as the initial sync is much faster compared to Oura.

**Blockfrost**

Blockfrost is a hosted API service that provides access to Cardano blockchain data without requiring you to run your own infrastructure.
This is the easiest option to get started with as it requires no Cardano node, DB-Sync instance, or relay connections.

To use Blockfrost, you need to obtain an API key from [blockfrost.io](https://blockfrost.io/).
The free tier is sufficient for most development and testing use cases.

---

## How Common DLT Source Configuration Works

NeoPRISM streams blocks from the Cardano blockchain and extracts PRISM metadata, which is then persisted to the database. These operations are initially stored as raw, unindexed data. At every configured interval (set by the index interval option), NeoPRISM wakes up and picks up unindexed operations from the database. It then runs the indexing logic, which extracts, validates, and transforms each raw operation into an efficient lookup data structure.

A faster index interval reduces the lag between when an operation is streamed and when it becomes indexed and available for fast lookup. However, setting a very short interval can put additional pressure on the database due to more frequent indexing cycles. NeoPRISM comes with a sensible default value for the index interval to balance performance and resource usage.

Choose the DLT source and interval settings that best fit your infrastructure and performance needs.

---

**Next Steps:**
- [CLI Options](../references/cli-options.md): Full list of flags and environment variables.
