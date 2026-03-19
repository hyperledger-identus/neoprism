## Overview

Support a new submitter mechanism.
This submitter will use the MeshSDK to act as an embedded wallet.
The embedded wallet will be implemented using typescript with bun.
The responsibility of the embedded wallet is to create a transaction for the submission operation.

## Goal

The goal is to provide the neoprism more lightweight options.
The only supported summitter option is to use `cardano-wallet` which is quite heavy as it relies on the `cardano-node` infrastructure.
With the new embedded wallet, the only required dependency is the `cardano-submit-api`.

## Input / Output

The wallet should take the following input

- mnemonic phrase
- payment address
- blockfrost provider config

and build a transaction to be submitted via cardano submit-api.

## Submitter crate

The embedded wallet will be invoked as a subprocess and communicate via `stdin` and `stdout`.
There should be also a new feature flag to represent this similar to the `cardano-wallet` flag.

