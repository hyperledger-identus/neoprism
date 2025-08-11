# NeoPRISM Node

![Rust Edition](https://img.shields.io/badge/edition-2024-blue)
[![Unit tests](https://github.com/hyperledger-identus/neoprism/actions/workflows/checks.yml/badge.svg)](https://github.com/hyperledger-identus/neoprism/actions)
[![PRISM tests](https://github.com/hyperledger-identus/neoprism/actions/workflows/integration-test.yml/badge.svg)](https://github.com/hyperledger-identus/neoprism/actions)
[![GitHub release](https://img.shields.io/github/release/hyperledger-identus/neoprism.svg)](https://github.com/hyperledger-identus/neoprism/releases)
[![GitHub tag](https://img.shields.io/github/tag/hyperledger-identus/neoprism.svg)](https://github.com/hyperledger-identus/neoprism/tags)
[![License](https://img.shields.io/github/license/hyperledger-identus/neoprism.svg)](./LICENSE)
[![Docker Pulls](https://img.shields.io/docker/pulls/hyperledgeridentus/identus-neoprism)](https://hub.docker.com/r/hyperledgeridentus/identus-neoprism)

## Overview

NeoPRISM is an open-source project for managing [PRISM Decentralized Identifiers (DIDs)](https://www.w3.org/TR/did-core/) on the Cardano blockchain. PRISM DIDs provide unique, verifiable identifiers for people, organizations, and devices, with their associated DID Documents securely anchored on-chain.

NeoPRISM is designed to be extensible and is not limited to a single service. In addition to providing networked services, NeoPRISM aims to offer SDKs, WASM/FFI libraries, and other bindings to support a wide range of application integrations.

NeoPRISM supports two main roles:

- **Indexer:** Continuously monitors the Cardano blockchain for PRISM DID operations, validates them, and maintains an up-to-date index of all active DIDs and their current documents. This enables fast, reliable DID resolution and retrieval of W3C-compliant DID Documents via a simple API.

- **Submitter:** Provides an interface for clients to create, update, or deactivate PRISM DIDs by submitting new operations to the Cardano blockchain. The submitter ensures that all operations are properly formatted, signed, and compliant with the [PRISM DID method specification](https://github.com/input-output-hk/prism-did-method-spec/blob/main/w3c-spec/PRISM-method.md).

By separating these roles, NeoPRISM Node enables both robust DID resolution (read) and secure DID management (write), making it easy for applications to integrate decentralized identity on Cardano without handling blockchain details directly.

For a more detailed protocol overview, see the [PRISM DID method specification](https://github.com/input-output-hk/prism-did-method-spec/blob/main/w3c-spec/PRISM-method.md#high-level-protocol-description).

# Quickstart

## Public demo instance

A public instance of neoprism is hosted at [https://neoprism.patlo.dev](https://neoprism.patlo.dev).  
A public preprod instance is also available at [https://neoprism-preprod.patlo.dev](https://neoprism-preprod.patlo.dev).

## Self-hosting

Start the node and sync block metadata from the relay node `backbone.mainnet.cardanofoundation.org:3001`

```bash
cd docker/mainnet-relay
docker-compose up
```

The WebUI is available at `http://localhost:8080`

The resolver endpoint is available at `http://localhost:8080/api/dids/<did>`


# Development guide

This project uses Nix for the local development environment and artifact packaging.
Follow the instructions [here](https://nixos.org/download/#download-nix) to install Nix—it's all you need!

__Entering the development shell__

If you already have `cargo` and other required dependencies (e.g. `protoc`) installed, you can use your own environment.
Feel free to check the [nix shell](./nix/devShells/neoprism.nix) to see the required dependencies and port them to your own environment.

A recommended approach is to use `nix develop` command to enter the development shell.
This way, the development shell is consistent and the same version of the libraries are used to build and test.


```bash
nix develop

# you can now run command like "cargo version"
```
Note that you may need to enable experimental flake commands. Please follow the instructions [here](https://nixos.wiki/wiki/Flakes).

Additionally, you can use `--unset <ENV>` to disable host environment variable to make development shell more pure.
For example:

```bash
nix develop --unset PATH
```

to disable all binaries available on host `PATH`.


## Development quickstart

Spinning up services in dev shell

```bash
nix develop --unset PATH
npm install
dbUp
runNode
```

Cleaning up services in dev shell

```bash
dbDown
```

## Frequently used commands

These are commands you can run outside the development shell

| command                                                 | description                                                        |
|---------------------------------------------------------|--------------------------------------------------------------------|
| `nix build .#neoprism-docker`                           | Use nix to build the docker image (output available at `./result`) |
| `nix build .#neoprism-docker && docker load < ./result` | Use nix to build the docker image and load it using docker         |

Assuming you are in the development shell, these are frequently used commands.

| command                          | description                                    |
|----------------------------------|------------------------------------------------|
| `npm install`                    | Install the npm dependencies (first time only) |
| `cargo build`                    | Build the cargo workspace                      |
| `cargo clean`                    | Clean the cargo workspace                      |
| `cargo r -p neoprism-node -- -h` | See `neoprism-node` service CLI options        |
| `cargo test --all-features`      | Run tests that enable all crate features       |

These are some scripts provided by the shell to automate the local development workflow:

| command                                 | description                                                      |
|-----------------------------------------|------------------------------------------------------------------|
| `format`                                | Run formatter on everything                                      |
| `build`                                 | Building the whole project                                       |
| `buildAssets`                           | Building the WebUI assets (css, javascript, static assets)       |
| `buildConfig`                           | Building the generated config                                    |
| `dbUp`                                  | Spin up the local database                                       |
| `dbDown`                                | Tear down the local database                                     |
| `pgDump`                                | Dump the local database to `postgres.dump` file                  |
| `pgRestore`                             | Restore the local database from `postgres.dump` file             |
| `runNode indexer`                       | Run the indexer node connecting to the local database            |
| `runNode indexer --cardano-addr <ADDR>` | Run the indexer node connecting to the cardano relay at `ADDR`   |
| `runNode indexer --dbsync-url <URL>`    | Run the indexer node connecting to the DB Sync instance at `URL` |
