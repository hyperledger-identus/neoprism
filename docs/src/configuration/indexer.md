## Indexer Mode Configuration

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

Choose the DLT source that best fits your infrastructure and reliability needs.

---

**Next Steps:**
- [CLI Options](../references/cli-options.md): Full list of flags and environment variables.
