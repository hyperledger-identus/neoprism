## Indexer Mode Configuration

The **Indexer node** monitors the Cardano blockchain for PRISM DID operations, validates and indexes them, and enables efficient lookup of DID Documents.  
It is typically used for DID resolution and verification services.

### DLT Source

The Indexer node supports multiple DLT sources for ingesting DID operations:

- **Oura:**  
  Connects to a Cardano relay node and streams block data in real time.
  - Key options: Cardano network, relay address.

- **DB-Sync:**  
  Connects to a Cardano DB-Sync instance and polls for new blocks and transactions.
  - Key options: DB-Sync URL, poll interval.

- **Common DLT Source Options:**  
  - Index interval: How often to check for unindexed operations.
  - Confirmation blocks: Number of blocks to wait before considering an operation final.

Choose the DLT source that best fits your infrastructure and reliability needs.

---

**Next Steps:**
- [CLI Options](../references/cli-options.md): Full list of flags and environment variables.
