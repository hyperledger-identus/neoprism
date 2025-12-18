# PRISM Test Docker Setup

Local PRISM testing environment with a local Cardano testnet, wallet services, and PRISM node implementations.

## Quick Start

Start the default environment (PostgreSQL backend):

```sh
cd docker/prism-test
docker compose up
```

The following services will be available:

> **Note:** Service availability varies by configuration. See [Compose Configurations](#compose-configurations) for details.

| Service                | URL                    | Remark                    |
|------------------------|------------------------|---------------------------|
| **NeoPRISM HTTP API**  | http://localhost:18080 |                           |
| **Cardano Wallet API** | http://localhost:18081 |                           |
| **Blockfrost API**     | http://localhost:18082 |                           |
| **PRISM Node gRPC**    | localhost:50053        |                           |

Stop the environment:

```sh
docker compose down      # Stop services
docker compose down -v   # Stop and remove volumes (clean restart)
```

## Compose Configurations

### compose.yml - Full Testing Environment

Complete PRISM testing stack with local Cardano testnet, Blockfrost API compatibility layer, and both NeoPRISM and legacy PRISM Node implementations. NeoPRISM uses PostgreSQL backend. Use this for comprehensive integration testing and conformance testing against both node implementations.

| Service | Port | Description |
|---------|------|-------------|
| **neoprism-standalone** | 18080 | NeoPRISM HTTP API (PostgreSQL backend) |
| **prism-node** | 50053 | PRISM Node gRPC API |
| **cardano-wallet** | 18081 | Cardano Wallet HTTP API |
| **bf-proxy** | 18082 | Blockfrost API proxy |
| **db-neoprism** | - | PostgreSQL database for NeoPRISM |
| **db-prism-node** | - | PostgreSQL database for PRISM Node |
| **db-dbsync** | - | PostgreSQL database for db-sync |

### compose-ci.yml - CI Testing Environment

Identical to compose.yml but uses locally built `identus-neoprism:latest` image instead of the released version. NeoPRISM uses PostgreSQL backend. Use this in CI pipelines or when testing local NeoPRISM builds against the full stack. Does not include Blockfrost services or PRISM Node.

| Service | Port | Description |
|---------|------|-------------|
| **neoprism-standalone** | 18080 | NeoPRISM HTTP API (PostgreSQL backend) |
| **cardano-wallet** | 18081 | Cardano Wallet HTTP API |
| **db-neoprism** | - | PostgreSQL database for NeoPRISM |
| **db-dbsync** | - | PostgreSQL database for db-sync |

### compose-sqlite.yml - Full Testing Environment (SQLite)

Complete PRISM testing stack with local Cardano testnet, Blockfrost API compatibility layer, and both NeoPRISM and legacy PRISM Node implementations. NeoPRISM uses in-memory SQLite backend. Use this for comprehensive integration testing with SQLite instead of PostgreSQL.

| Service | Port | Description |
|---------|------|-------------|
| **neoprism-standalone** | 18080 | NeoPRISM HTTP API (in-memory SQLite backend) |
| **prism-node** | 50053 | PRISM Node gRPC API |
| **cardano-wallet** | 18081 | Cardano Wallet HTTP API |
| **bf-proxy** | 18082 | Blockfrost API proxy |
| **db-prism-node** | - | PostgreSQL database for PRISM Node |
| **db-dbsync** | - | PostgreSQL database for db-sync |

### compose-ci-sqlite.yml - CI Testing Environment (SQLite)

Identical to compose-ci.yml but uses in-memory SQLite backend instead of PostgreSQL. Uses locally built `identus-neoprism:latest` image. Use this in CI pipelines for testing with SQLite backend. Does not include Blockfrost services or PRISM Node.

| Service | Port | Description |
|---------|------|-------------|
| **neoprism-standalone** | 18080 | NeoPRISM HTTP API (in-memory SQLite backend) |
| **cardano-wallet** | 18081 | Cardano Wallet HTTP API |
| **db-dbsync** | - | PostgreSQL database for db-sync |

### compose-sqlite-dev.yml - Minimal Development Environment

Lightweight setup with only NeoPRISM standalone service running in dev mode. Uses in-memory SQLite backend with no external dependencies. Uses locally built `identus-neoprism:latest` image. Ideal for quick local development and testing NeoPRISM features in isolation without full infrastructure overhead.

| Service | Port | Description |
|---------|------|-------------|
| **neoprism-standalone** | 18080 | NeoPRISM HTTP API (in-memory SQLite, dev mode) |

## Running Tests

Run normal conformance tests:

```sh
cd tests/prism-test
sbt test
```

## Pre-configured Entities

### Cardano Testnet

The local Cardano testnet is configured with custom parameters for testing.

| Property | Value |
|----------|-------|
| **Network Magic** | `42` |
| **Network** | Custom testnet |

### Wallet

A pre-configured wallet is available on the **cardano-wallet** service for testing purposes.

| Property | Value |
|----------|-------|
| **Wallet ID** | `9263a1248b046fe9e1aabc4134b03dc5c3a7ee3d` |
| **Payment Address** | `addr_test1qp83v2wq3z9mkcjj5ejlupgwt6tcly5mtmz36rpm8w4atvqd5jzpz23y8l4dwfd9l46fl2p86nmkkx5keewdevqxhlyslv99j3` |
| **Passphrase** | `super_secret` |
| **Mnemonic** | `mimic candy diamond virus hospital dragon culture price emotion tell update give faint resist faculty soup demand window dignity capital bullet purity practice fossil` |

### DID

A pre-configured DID is published to the **local testnet** and available for testing.

| Property | Value |
|----------|-------|
| **DID** | `did:prism:e3b675023ef13a2bd1b015a6b1c88d2bfbfbb09bed5b675598397aec361f0d6e` |
| **VDR Key Name** | `vdr-0` |
| **VDR Private Key (hex)** | `d3ed47189d10509305494d89e8f08d139beed1e8ed18c3cf6b38bc897078c052` |
| **PrismObject (hex)** | `22d50112d2010a086d61737465722d3012473045022100c921083f391f179c947a4e95f8ed226870c32557565f8adba52daebcf47ce5b3022019f8632237331c5183d5ee6d192b617637848e32ce5d26c12dd2a86890b8bd041a7d0a7b0a79123c0a086d61737465722d3010014a2e0a09736563703235366b31122103b20404f350d87eec98982131c176acfea520f26f8901fe08b619a56a0dd9e41712390a057664722d3010084a2e0a09736563703235366b31122102647aff70cfd5d510ec369c512da85faef95803db30bb47499a28c08a590186ac` |

## Generating Test Fixtures

Remove `TestAspect.ignore` from the `generateDidFixtureSpec` in `tests/prism-test/src/test/scala/org/hyperledger/identus/prismtest/MainSpec.scala`, then run:

```sh
cd tests/prism-test
sbt "testOnly -- -tags fixture"
```
