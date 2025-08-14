# NeoPRISM Architecture

NeoPRISM supports multiple roles within its system architecture:
- **Indexer**: Reads, validates, and indexes DID operations from the Cardano blockchain.
- **Submitter**: Batches and submits signed DID operations to the Cardano blockchain.
- **Standalone**: Runs both indexer and submitter in a single process for simple deployments.

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
verifier: "Verifier"
cardano-node: "Cardano relay node" {
  shape: cloud
}

internal: "Deployment" {
  neoprism: "NeoPRISM Standalone"
  cardano-wallet: "Cardano HTTP wallet"
  cardano-blockproducer: "Cardano block-producer node"
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

## Closed-loop indexer - submitter deployment

In this deployment mode, the indexer and submitter run as separate processes, which may be hosted on different machines. This separation allows for independent scaling of each component; for example, multiple indexer instances can be deployed to support high read volume.

- The **indexer** process reads, validates, and indexes DID operations from the Cardano blockchain, storing them in a shared PostgreSQL database.
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
  db: "PostgreSQL" {
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

## Open-loop Indexer-only and Submitter-only deployments

NeoPRISM also supports deploying only a subset of its components, depending on your use case and requirements:

- **Indexer-only deployment:**  
  Only the indexer process is deployed. This setup allows you to read, validate, and index DID operations from the Cardano blockchain, and serve DID resolution requests via the REST API. No submission of new DID operations to the blockchain is possible in this mode.

- **Submitter-only deployment:**  
  Only the submitter process is deployed. This setup allows you to batch and submit signed DID operations to the Cardano blockchain via the wallet component. DID resolution and indexing are not available in this mode.

These deployment options provide flexibility for scenarios where you may only need to resolve DIDs (indexer-only) or only need to submit new DID operations (submitter-only), without running the full NeoPRISM stack.

> Note: These modes are subsets of the closed-loop indexer–submitter deployment, and can be scaled or combined as needed.
