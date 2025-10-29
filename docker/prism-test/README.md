# PRISM Test Docker Setup

Local PRISM testing environment with Cardano testnet, wallet services, and PRISM node implementations.

## Quick Start

Start the environment:

```sh
cd docker/prism-test
docker compose up
```

The following services will be available:

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

## Running Tests

Run normal conformance tests:

```sh
cd tests/prism-test
sbt test
```

## Variants

- **compose.yml**: Standard environment
- **compose-ci.yml**: CI environment (use CI locally built image)

## Pre-configured Wallet

**Wallet ID:** `9263a1248b046fe9e1aabc4134b03dc5c3a7ee3d`

**Payment Address:** `addr_test1qp83v2wq3z9mkcjj5ejlupgwt6tcly5mtmz36rpm8w4atvqd5jzpz23y8l4dwfd9l46fl2p86nmkkx5keewdevqxhlyslv99j3`

**Passphrase:** `super_secret`

**Mnemonic (24 words):**
```
mimic candy diamond virus hospital dragon culture price emotion tell update give
faint resist faculty soup demand window dignity capital bullet purity practice fossil
```

## Generating Test Fixtures

Remove `TestAspect.ignore` from the `generateDidFixtureSpec` in `tests/prism-test/src/test/scala/org/hyperledger/identus/prismtest/MainSpec.scala`, then run:

```sh
cd tests/prism-test
sbt "testOnly -- -tags fixture"
```

### Initial DID

**DID:** `did:prism:e3b675023ef13a2bd1b015a6b1c88d2bfbfbb09bed5b675598397aec361f0d6e`

**VDR Key Name:** `vdr-0`

**VDR Private Key (hex):** `d3ed47189d10509305494d89e8f08d139beed1e8ed18c3cf6b38bc897078c052`

**PrismObject (hex):**
```
22d50112d2010a086d61737465722d3012473045022100c921083f391f179c947a4e95f8ed226870c32557565f8adba52daebcf47ce5b3022019f8632237331c5183d5ee6d192b617637848e32ce5d26c12dd2a86890b8bd041a7d0a7b0a79123c0a086d61737465722d3010014a2e0a09736563703235366b31122103b20404f350d87eec98982131c176acfea520f26f8901fe08b619a56a0dd9e41712390a057664722d3010084a2e0a09736563703235366b31122102647aff70cfd5d510ec369c512da85faef95803db30bb47499a28c08a590186ac
```
