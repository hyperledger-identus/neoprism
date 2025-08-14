## Indexer Configuration

The **Indexer node** monitors the Cardano blockchain for PRISM DID operations, validates and indexes them, and enables efficient lookup of DID Documents.  
It is typically used for DID resolution and verification services.

### DLT Source

The Indexer node supports multiple DLT sources for ingesting DID operations:

- **Oura:**  
  Connects to a Cardano relay node and streams block data in real time.
  - Key options: 
    - Cardano network: `--cardano-network` or `NPRISM_CARDANO_NETWORK`
    - Relay address: `--cardano-relay-addr` or `NPRISM_CARDANO_RELAY_ADDR`

- **DB-Sync:**  
  Connects to a Cardano DB-Sync instance and polls for new blocks and transactions.
  - Key options: 
    - DB-Sync URL: `--db-sync-url` or `NPRISM_DB_SYNC_URL`
    - Poll interval: `--db-sync-poll-interval` or `NPRISM_DB_SYNC_POLL_INTERVAL`

- **Common DLT Source Options:**  
  - Index interval: `--index-interval` or `NPRISM_INDEX_INTERVAL`
  - Confirmation blocks: `--confirmation-blocks` or `NPRISM_CONFIRMATION_BLOCKS`

---

### DLT Source Comparison

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

---

### How Common DLT Source Configuration Works

NeoPRISM streams blocks from the Cardano blockchain and extracts PRISM metadata, which is then persisted to the database. These operations are initially stored as raw, unindexed data. At every configured interval (set by the index interval option), NeoPRISM wakes up and picks up unindexed operations from the database. It then runs the indexing logic, which extracts, validates, and transforms each raw operation into an efficient lookup data structure.

A faster index interval reduces the lag between when an operation is streamed and when it becomes indexed and available for fast lookup. However, setting a very short interval can put additional pressure on the database due to more frequent indexing cycles. NeoPRISM comes with a sensible default value for the index interval to balance performance and resource usage.

Choose the DLT source and interval settings that best fit your infrastructure and performance needs.

---

**Next Steps:**
- [CLI Options](../references/cli-options.md): Full list of flags and environment variables.
