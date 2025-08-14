# NeoPRISM Architecture

NeoPRISM supports multiple roles within its system architecture:
- **Indexer**: Reads, validates, and indexes DID operations from the Cardano blockchain.
- **Submitter**: Batches and submits signed DID operations to the Cardano blockchain.
- **Resolver**: Resolves and returns the current state of a DID.

NeoPRISM acts as an indexer by reading DID operations from the Cardano blockchain, validating them, and storing them in a local PostgreSQL database.
It organizes these operations using keys such as DIDs or storage hashes.
When a user requests the current state of a DID, NeoPRISM replays the relevant operations to reconstruct and return the latest state via its REST API.

In its role as a submitter, NeoPRISM receives signed DID operations and batches them into Cardano transaction metadata.
It does not manage private keys for either DID operations or Cardano wallets.
The metadata is sent to the wallet component (currently supporting only `cardano-wallet`) for publishing.
This process is stateless and requires the wallet passphrase, along with other related wallet configurations, which are provided through CLI options or environment variables.

## Closed-loop standalone deployment

In this mode, both the indexer and submitter run in the same process, which is suitable for a small and simple deployment setup.
You may also add a reverse proxy to handle authentication and routing for the submitter API paths.

```d2
did-controller: "DID controller"
verifier: "Verifying client"
cardano-node: "Cardano relay node" {
  shape: cloud
}

internal: "Deployment" {
  neoprism: "NeoPRISM standalone node"
  cardano-wallet: "Cardano HTTP wallet"
  cardano-blockproducer: "Cardano block-producer node" {
    shape: cloud
  }
  db: "PostgreSQL" {
    shape: cylinder
  }

  neoprism -> cardano-wallet: create transactions
  neoprism <-> db: read / write indexed operations
  cardano-wallet -> cardano-blockproducer: submit transactions
}

internal.neoprism <- cardano-node: stream operations using Oura
did-controller -> internal.neoprism: submit signed PRISM operations
verifier -> internal.neoprism: resolve DID documents
internal.cardano-blockproducer -> cardano-node: propagate blocks
```
