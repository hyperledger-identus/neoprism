# NeoPRISM Architecture

NeoPRISM is a modular DID infrastructure for Cardano that can operate in multiple deployment modes:

- **Indexer**: Reads and validates DID operations from the Cardano blockchain, storing them in the configured database backend (PostgreSQL or the embedded SQLite engine). When queried, it reconstructs the current DID state by replaying relevant operations and returns DID documents via its REST API.

- **Submitter**: Receives signed DID operations and batches them into Cardano transaction metadata for publishing. This is a stateless process that does not manage private keys. It delegates transaction submission to a Cardano wallet component (currently `cardano-wallet`).

- **Standalone**: Runs both indexer and submitter in a single process for simplified deployments.

> Storage note: both the standalone and distributed layouts can point to PostgreSQL (recommended for production) or to the embedded SQLite file for local development. See [database configuration](../configuration/database.md) for trade-offs and CLI flags.

> **Note:** NeoPRISM does not manage keys for DID operations or Cardano wallets. Keys for DID operations are managed by the client that submits signed operations. Cardano wallet keys are managed by the cardano-wallet component.

## Standalone deployment

In this mode, both the indexer and submitter run in the same process, making it suitable for small and simple deployment setups.
You may also add a reverse proxy to handle authentication and routing for the submitter API paths.

```d2
did-controller: "DID controller"
verifier: "Verifier"
cardano-node: "Cardano relay node" {
  shape: cloud
}

internal: "Deployment" {
  neoprism: "NeoPRISM Standalone"
  cardano-wallet: "Cardano HTTP wallet"
  cardano-blockproducer: "Cardano block-producer node"
  db: "Database" {
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

## Distributed deployment

In this deployment mode, the indexer and submitter run as separate processes, which may be hosted on different machines. This separation allows for independent scaling of each component; for example, multiple indexer instances can be deployed to support high read volume.

- The **indexer** process reads, validates, and indexes DID operations from the Cardano blockchain, storing them in a shared database backend (PostgreSQL in production, SQLite for lightweight deployments).
- The **submitter** process is stateless and receives signed DID operations, batching and submitting them to the Cardano blockchain via the wallet component. It does not use the database.

A reverse proxy is recommended to route requests to the appropriate service, handling authentication and API path routing for both the indexer and submitter.

```d2
did-controller: "DID controller"
verifier: "Verifier"
cardano-node: "Cardano relay node" {
  shape: cloud
}

indexer-deployment: "Indexer Deployment" {
  indexer: "NeoPRISM Indexer"
  db: "Database" {
    shape: cylinder
  }

  indexer <-> db: read / write indexed operations
}

indexer-deployment.indexer <- cardano-node: stream operations using Oura
verifier -> indexer-deployment.indexer: resolve DID documents

submitter-deployment: "Submitter Deployment" {
  submitter: "NeoPRISM Submitter"
  cardano-wallet: "Cardano HTTP wallet"
  cardano-blockproducer: "Cardano block-producer node"
  submitter -> cardano-wallet: create transactions
  cardano-wallet -> cardano-blockproducer: submit transactions
}

did-controller -> submitter-deployment.submitter: submit signed PRISM operations
submitter-deployment.cardano-blockproducer -> cardano-node: propagate blocks
```

## Partial deployments

NeoPRISM also supports deploying only a subset of its components, depending on your use case and requirements:

- **Indexer-only deployment:**  
  Only the indexer process is deployed. This setup allows you to read, validate, and index DID operations from the Cardano blockchain, and serve DID resolution requests via the REST API. No submission of new DID operations to the blockchain is possible in this mode.

- **Submitter-only deployment:**  
  Only the submitter process is deployed. This setup allows you to batch and submit signed DID operations to the Cardano blockchain via the wallet component. DID resolution and indexing are not available in this mode.

These deployment options provide flexibility for scenarios where you may only need to resolve DIDs (indexer-only) or only need to submit new DID operations (submitter-only), without running the full NeoPRISM stack.

> **Note:** These modes are subsets of the distributed deployment, and can be scaled or combined as needed.
