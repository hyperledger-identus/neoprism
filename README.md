# NeoPRISM

![Rust Edition](https://img.shields.io/badge/edition-2024-blue)
[![Unit tests](https://github.com/hyperledger-identus/neoprism/actions/workflows/checks.yml/badge.svg)](https://github.com/hyperledger-identus/neoprism/actions)
[![PRISM tests](https://github.com/hyperledger-identus/neoprism/actions/workflows/conformance-test.yml/badge.svg)](https://github.com/hyperledger-identus/neoprism/actions/workflows/conformance-test.yml)
[![GitHub release](https://img.shields.io/github/release/hyperledger-identus/neoprism.svg)](https://github.com/hyperledger-identus/neoprism/releases)
[![GitHub tag](https://img.shields.io/github/tag/hyperledger-identus/neoprism.svg)](https://github.com/hyperledger-identus/neoprism/tags)
[![License](https://img.shields.io/github/license/hyperledger-identus/neoprism.svg)](./LICENSE)
[![Docker Pulls](https://img.shields.io/docker/pulls/hyperledgeridentus/identus-neoprism)](https://hub.docker.com/r/hyperledgeridentus/identus-neoprism)
[![Discord](https://img.shields.io/discord/905194001349627914?label=discord)](https://discord.com/channels/905194001349627914/1230596020790886490)

---

**📚 [Documentation](https://hyperledger-identus.github.io/neoprism/)**

---

# Overview

NeoPRISM is an open-source implementation of a PRISM node, providing a robust and extensible platform for managing [PRISM Decentralized Identifiers (DIDs)](https://www.w3.org/TR/did-core/) anchored on the Cardano blockchain.

A PRISM node continuously monitors the Cardano blockchain for DID operations, validates and indexes them, and enables efficient lookup of DID Documents.
It also allows users to create, update, or deactivate DIDs by submitting operations, ensuring all protocol rules and security checks are enforced.
By running a PRISM node, organizations and individuals can independently manage and verify DIDs, supporting a decentralized identity system without reliance on a central authority.

## Features

- **🛠️ Multiple Deployment Modes**
  - Supports three operational modes:
    - **Indexer:** Resolves and indexes DIDs for verification services.
    - **Submitter:** Publishes DID operations to the Cardano blockchain.
    - **Standalone:** Combines indexing and submission capabilities into a single process.

- **🔗 Cardano Data Source Integration**
  - Ingests DID operations from various Cardano data sources, including [Oura](https://github.com/txpipe/oura) and [DBSync](https://github.com/input-output-hk/cardano-db-sync).

- **🆔 W3C-Compliant DID Resolution**
  - Provides a universal-resolver compatible HTTP endpoint.
  - Resolves PRISM DIDs to DID Documents according to the W3C DID specification.

- **📤 DID Operation Publishing**
  - Publishes PRISM DID operations to the Cardano blockchain.
  - Integrates with Cardano-wallet; future support for additional publishing methods.

- **📚 OpenAPI Documentation**
  - Documents all HTTP endpoints using the OpenAPI specification.

- **🗄️ Verifiable Data Registry (VDR) Support**
  - Enables reading and writing arbitrary data to the Cardano blockchain using transaction metadata.
  - Supports indexing and retrieval of data from transaction metadata for verifiable data use cases.

- **🦀 Rust Implementation**
  - Developed in Rust for efficient resource usage and reliable performance.

## Introduction to PRISM DID

The [PRISM DID method](https://github.com/input-output-hk/prism-did-method-spec) (`did:prism`) is a protocol for creating and managing Decentralized Identifiers (DIDs) built on the Cardano blockchain.
This specification defines the operations, serialization formats, and rules for managing the lifecycle of DIDs and their associated DID documents.

At its core, PRISM works by encoding identity operations (Create, Update, Deactivate) as Protocol Buffer messages that are signed, batched into blocks, and published as metadata in Cardano transactions.
PRISM nodes continuously monitor the blockchain, processing these operations to maintain an up-to-date map of DIDs and their states.
The method requires a "secure depth" of 112 block confirmations before considering operations final.
During this confirmation period, users can leverage long form DIDs immediately without waiting for on-chain anchoring, providing flexibility while maintaining the ability to later publish these DIDs to the blockchain.

The protocol defines clear rules for constructing and validating operations, translating internal state to W3C-compliant DID Documents, and resolving DIDs.
Security is enforced through cryptographic signatures, with each DID having at least one master key for operation signing.
PRISM is designed to be scalable and secure, with support for various verification methods, services, and key types including secp256k1, Ed25519, and X25519.

**Examples:**

Short-form DID:
```
did:prism:9b5118411248d9663b6ab15128fba8106511230ff654e7514cdcc4ce919bde9b
```

Long-form DID:
```
did:prism:9b5118411248d9663b6ab15128fba8106511230ff654e7514cdcc4ce919bde9b:Cj8KPRI7CgdtYXN0ZXIwEAFKLgoJc2VjcDI1NmsxEiEDHpf-yhIns-LP3tLvA8icC5FJ1ZlBwbllPtIdNZ3q0jU
```

# Quickstart

## Public Demo Instance

- Mainnet: [https://neoprism.patlo.dev](https://neoprism.patlo.dev)
- Preprod: [https://neoprism-preprod.patlo.dev](https://neoprism-preprod.patlo.dev)

## Self-hosting

This example setup runs a NeoPRISM node that connects to the Cardano mainnet public relay using Oura.
It fetches DID operations from the blockchain, synchronizes and indexes them into a local PostgreSQL database.
Once operations are indexed, you can browse them using the Explorer page in the Web UI.

**Prerequisites:**
- [Docker](https://docs.docker.com/get-docker/)

**Supported Platforms:**
- The official NeoPRISM Docker image supports both x86_64 and arm64 architectures.
- Compatible with Linux, macOS, and Windows hosts that can run Docker.

**Steps:**

1. Clone the repository and navigate to the example directory:
   ```bash
   cd docker/mainnet-relay
   ```
2. Start the node and sync block metadata:
   ```bash
   docker-compose up
   ```
3. Access the Web UI at [http://localhost:8080](http://localhost:8080). The Explorer page allows you to browse indexed DID operations.
4. Use the resolver endpoint to resolve DIDs:
   ```bash
   curl http://localhost:8080/api/dids/<did>
   ```

# Development guide

This project uses Nix to provide a consistent local development environment and to package build artifacts.
To get started, install Nix by following the instructions [here](https://nixos.org/download/#download-nix).

__Entering the development shell__

If you already have `cargo` and other required dependencies (such as `protoc`) installed, you can use your own environment.
You can review the [nix shell](./nix/devShells/neoprism.nix) file to see the required dependencies and adapt them to your setup.

We recommend using the `nix develop` command to enter the development shell.
This ensures a consistent environment and uses the same library versions for building and testing.

```bash
nix develop

# You can now run commands like "cargo version"
```
Note: You may need to enable experimental flake commands. Please follow the instructions [here](https://nixos.wiki/wiki/Flakes).

You can also use `--unset <ENV>` to disable host environment variables and make the development shell more pure.
For example:

```bash
nix develop --unset PATH
```

This disables all binaries available on the host `PATH`.


## Development quickstart

To set up and run NeoPRISM for development, follow these steps:

1. **Enter the development shell:**
   ```bash
   nix develop --unset PATH
   ```
2. **Install web UI dependencies** (only required the first time):
   ```bash
   npm install
   ```
3. **Start the local PostgreSQL database** (required for NeoPRISM to store data):
   ```bash
   dbUp
   ```
4. **Run the NeoPRISM node** (this will automatically generate all required assets):
   ```bash
   runNode
   ```
5. **Access the Web UI and API** at [http://localhost:8080](http://localhost:8080).

**Cleaning up services**

- To stop and remove the local database:
  ```bash
  dbDown
  ```

**Notes**
- Default port used is `8080`.
- No need to run `buildAssets` manually; `runNode` handles asset generation automatically.
- You can run `buildAssets` separately if you only want to generate web UI assets without starting the node.

## Frequently used commands

These are commands you can run outside the development shell:

| command                                                 | description                                                        |
|---------------------------------------------------------|--------------------------------------------------------------------|
| `nix build .#neoprism-docker`                           | Use nix to build the docker image (output available at `./result`) |
| `nix build .#neoprism-docker && docker load < ./result` | Use nix to build the docker image and load it using docker         |

Assuming you are in the development shell, here are some frequently used commands:

| command                          | description                                    |
|----------------------------------|------------------------------------------------|
| `npm install`                    | Install the npm dependencies (first time only) |
| `cargo build`                    | Build the cargo workspace                      |
| `cargo clean`                    | Clean the cargo workspace                      |
| `cargo r -p neoprism-node -- -h` | See `neoprism-node` service CLI options        |
| `cargo test --all-features`      | Run tests that enable all crate features       |

The following scripts are provided by the shell to automate the local development workflow:

| command                                 | description                                                        |
|-----------------------------------------|--------------------------------------------------------------------|
| `format`                                | Run the formatter on everything                                    |
| `build`                                 | Build the whole project                                            |
| `buildAssets`                           | Build the Web UI assets (CSS, JavaScript, static assets)           |
| `buildConfig`                           | Build the generated config                                         |
| `dbUp`                                  | Spin up the local database                                         |
| `dbDown`                                | Tear down the local database                                       |
| `pgDump`                                | Dump the local database to the `postgres.dump` file                |
| `pgRestore`                             | Restore the local database from the `postgres.dump` file           |
| `runNode indexer`                       | Run the indexer node, connecting to the local database             |
| `runNode indexer --cardano-addr <ADDR>` | Run the indexer node, connecting to the Cardano relay at `<ADDR>`  |
| `runNode indexer --dbsync-url <URL>`    | Run the indexer node, connecting to the DB Sync instance at `<URL>`|
