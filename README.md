# NeoPRISM

![Rust Edition](https://img.shields.io/badge/edition-2024-blue)
[![Unit tests](https://github.com/hyperledger-identus/neoprism/actions/workflows/checks.yml/badge.svg)](https://github.com/hyperledger-identus/neoprism/actions)
[![PRISM tests](https://github.com/hyperledger-identus/neoprism/actions/workflows/integration-test.yml/badge.svg)](https://github.com/hyperledger-identus/neoprism/actions)
[![GitHub release](https://img.shields.io/github/release/hyperledger-identus/neoprism.svg)](https://github.com/hyperledger-identus/neoprism/releases)
[![GitHub tag](https://img.shields.io/github/tag/hyperledger-identus/neoprism.svg)](https://github.com/hyperledger-identus/neoprism/tags)
[![License](https://img.shields.io/github/license/hyperledger-identus/neoprism.svg)](./LICENSE)
[![Docker Pulls](https://img.shields.io/docker/pulls/hyperledgeridentus/identus-neoprism)](https://hub.docker.com/r/hyperledgeridentus/identus-neoprism)

## Overview

NeoPRISM is an open-source implementation of a PRISM node, providing a robust and extensible platform for managing [PRISM Decentralized Identifiers (DIDs)](https://www.w3.org/TR/did-core/) anchored on the Cardano blockchain.

A PRISM node continuously monitors the Cardano blockchain for DID operations, validates and indexes them, and enables efficient lookup of DID Documents. It also allows users to create, update, or deactivate DIDs by submitting operations, ensuring all protocol rules and security checks are enforced. By running a PRISM node, organizations and individuals can independently manage and verify DIDs, supporting a decentralized identity system without reliance on a central authority.

NeoPRISM supports two primary roles, which can be deployed independently or together in the same process:

- **Indexer:** Monitors the Cardano blockchain for PRISM DID operations, validates them, and maintains an up-to-date index for efficient DID resolution.
- **Submitter:** Provides an interface for clients to create, update, or deactivate PRISM DIDs by submitting operations to the Cardano blockchain.

## Features

1. **🔗 PRISM DID Indexing and Resolution**
   - Monitors the Cardano blockchain for PRISM DID operations, parses and validates them, and provides a W3C-compliant DID document resolution API.

2. **🌐 Web User Interface (WebUI)**
   - User-friendly web interface to explore DIDs and interact with the resolver endpoint, available both locally and via public demo instances.

3. **📚 API Explorer and Documentation**
   - Integrated OpenAPI/Swagger UI for programmatic access and interactive documentation of all available REST endpoints.

4. **🚀 Operation Submission**
   - Supports submitting PRISM DID operations via the API, including integration with cardano-wallet for operation submission.

5. **⚙️ Configurable Infrastructure**
   - Easily configurable to connect with Cardano mainnet, pre-production, or local testnets, with support for multi-architecture Docker images.

6. **🧪 Development and Testnet Support**
   - Tools and scripts for spinning up local development environments and testnets, including automated database management and test suites for integration and interoperability.

7. **🔐 CORS and Security**
   - CORS enabled for secure cross-origin requests in API integrations.

## Introduction to PRISM DID

[PRISM Decentralized Identifiers (DIDs)](https://github.com/input-output-hk/prism-did-method-spec/blob/main/w3c-spec/PRISM-method.md) are unique, verifiable identifiers anchored on the Cardano blockchain. Each PRISM DID is linked to a DID Document, which contains public keys and service endpoints, allowing for secure and decentralized digital identity.

PRISM DIDs can be either short-form (anchored on the blockchain) or long-form (containing all necessary information within the identifier itself). This approach provides flexibility for both quick, off-chain use and strong, on-chain trust.


# Quickstart

## Public demo instance

A public instance of neoprism is hosted at [https://neoprism.patlo.dev](https://neoprism.patlo.dev).  
A public preprod instance is also available at [https://neoprism-preprod.patlo.dev](https://neoprism-preprod.patlo.dev).

## Self-hosting

Start the node and sync block metadata from the relay node at `backbone.mainnet.cardanofoundation.org:3001`:

```bash
cd docker/mainnet-relay
docker-compose up
```

The Web UI is available at `http://localhost:8080`.

The resolver endpoint is available at `http://localhost:8080/api/dids/<did>`.


# Development guide

This project uses Nix for the local development environment and artifact packaging.
Follow the instructions [here](https://nixos.org/download/#download-nix) to install Nix—it's all you need!

__Entering the development shell__

If you already have `cargo` and other required dependencies (e.g., `protoc`) installed, you can use your own environment.
Feel free to check the [nix shell](./nix/devShells/neoprism.nix) to see the required dependencies and port them to your own environment.

A recommended approach is to use the `nix develop` command to enter the development shell.
This ensures that the development environment is consistent and that the same versions of the libraries are used to build and test.


```bash
nix develop

# you can now run commands like "cargo version"
```
Note that you may need to enable experimental flake commands. Please follow the instructions [here](https://nixos.wiki/wiki/Flakes).

Additionally, you can use `--unset <ENV>` to disable host environment variables and make the development shell more pure.
For example:

```bash
nix develop --unset PATH
```

to disable all binaries available on the host `PATH`.


## Development quickstart

Starting services in the development shell

```bash
nix develop --unset PATH
npm install
dbUp
runNode
```

Cleaning up services in the development shell

```bash
dbDown
```

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
