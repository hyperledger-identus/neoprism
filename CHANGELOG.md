# Changelog

All notable changes to this project will be documented in this file.

## [0.6.0](https://github.com/hyperledger-identus/neoprism/releases/tag/v0.6.0) - 2025-10-07

### 🚀 Features

- Remove midnight resolver and provide reusable did resolver http binding ([#127](https://github.com/hyperledger-identus/neoprism/pull/127))
- Add alsoKnownAs did document properties ([#132](https://github.com/hyperledger-identus/neoprism/pull/132))

### 🐛 Bug Fixes

- Remove midnight-did related crates ([#131](https://github.com/hyperledger-identus/neoprism/pull/131))

### ⚙️ Miscellaneous Tasks

- Update zio-http from 3.5.0 to 3.5.1 ([#126](https://github.com/hyperledger-identus/neoprism/pull/126))
- Update sbt, scripted-plugin from 1.11.6 to 1.11.7 ([#129](https://github.com/hyperledger-identus/neoprism/pull/129))

## [0.5.0](https://github.com/hyperledger-identus/neoprism/releases/tag/v0.5.0) - 2025-09-10

### 🚀 Features

- Experimental did midnight resolver ([#117](https://github.com/hyperledger-identus/neoprism/pull/117))
- Add endpoint to resolve VDR data ([#120](https://github.com/hyperledger-identus/neoprism/pull/120))

### 🚜 Refactor

- Split AppState and unify HTTP router struct ([#115](https://github.com/hyperledger-identus/neoprism/pull/115))

### 📚 Documentation

- Fix broken link in documentation site and add linkcheck config ([#105](https://github.com/hyperledger-identus/neoprism/pull/105))
- Refactor error messages for consistency ([#116](https://github.com/hyperledger-identus/neoprism/pull/116))

### 🎨 Styling

- Align and format TOML files; add taplo config for formatting [skip ci] ([#107](https://github.com/hyperledger-identus/neoprism/pull/107))

### ⚙️ Miscellaneous Tasks

- Update grpc-netty-shaded from 1.74.0 to 1.75.0 ([#112](https://github.com/hyperledger-identus/neoprism/pull/112))
- Update compilerplugin, scalapb-runtime, ... from 0.11.19 to 0.11.20 ([#113](https://github.com/hyperledger-identus/neoprism/pull/113))
- Update sbt, scripted-plugin from 1.11.4 to 1.11.5 ([#114](https://github.com/hyperledger-identus/neoprism/pull/114))
- Update dependency versions ([#118](https://github.com/hyperledger-identus/neoprism/pull/118))
- Use statix to lint nix expressions ([#119](https://github.com/hyperledger-identus/neoprism/pull/119))
- Update zio-http from 3.4.0 to 3.4.1 ([#121](https://github.com/hyperledger-identus/neoprism/pull/121))
- Update zio, zio-test, zio-test-magnolia, ... from 2.1.20 to 2.1.21 ([#122](https://github.com/hyperledger-identus/neoprism/pull/122))
- Update zio-http from 3.4.1 to 3.5.0 ([#123](https://github.com/hyperledger-identus/neoprism/pull/123))
- Update sbt, scripted-plugin from 1.11.5 to 1.11.6 ([#124](https://github.com/hyperledger-identus/neoprism/pull/124))
- *(release)* Prepare the next release ([#125](https://github.com/hyperledger-identus/neoprism/pull/125))

## [0.4.0](https://github.com/hyperledger-identus/neoprism/releases/tag/v0.4.0) - 2025-08-19

### 🚀 Features

- Universal resolver compatibility and driver endpoint ([#94](https://github.com/hyperledger-identus/neoprism/pull/94))

### 🐛 Bug Fixes

- Improving indexing logic, validation and prevent flaky tests ([#84](https://github.com/hyperledger-identus/neoprism/pull/84))
- Use correct permission in deploy-docs action ([#90](https://github.com/hyperledger-identus/neoprism/pull/90))

### 📚 Documentation

- Update project readme and improve clarity ([#87](https://github.com/hyperledger-identus/neoprism/pull/87))
- Add content for documentation site ([#89](https://github.com/hyperledger-identus/neoprism/pull/89))
- Add documentation about logging config ([#91](https://github.com/hyperledger-identus/neoprism/pull/91))
- Add missing page about logging ([#92](https://github.com/hyperledger-identus/neoprism/pull/92))
- Add documentation page about prism-test ([#93](https://github.com/hyperledger-identus/neoprism/pull/93))
- Fix PRISM tests badge link to use conformance-test.yml workflow ([#95](https://github.com/hyperledger-identus/neoprism/pull/95))

### 🧪 Testing

- Run prism test on github action ([#83](https://github.com/hyperledger-identus/neoprism/pull/83))

### ⚙️ Miscellaneous Tasks

- Add scala-steward config ([#96](https://github.com/hyperledger-identus/neoprism/pull/96))
- Fix scala-steward config ([#97](https://github.com/hyperledger-identus/neoprism/pull/97))
- Fix scala-steward config ([#98](https://github.com/hyperledger-identus/neoprism/pull/98))
- Fix token issue for scala-steward action ([#102](https://github.com/hyperledger-identus/neoprism/pull/102))
- Update monocle-core, monocle-macro to 3.3.0 ([#99](https://github.com/hyperledger-identus/neoprism/pull/99))
- Update sbt, scripted-plugin to 1.11.4 in main ([#101](https://github.com/hyperledger-identus/neoprism/pull/101))
- Update zio-http to 3.4.0 in main ([#100](https://github.com/hyperledger-identus/neoprism/pull/100))

## [0.3.1](https://github.com/hyperledger-identus/neoprism/releases/tag/v0.3.1) - 2025-08-07

### 🐛 Bug Fixes

- Add test coverage for prism specs and bug fixes ([#79](https://github.com/hyperledger-identus/neoprism/pull/79))
- Add VDR tests and expose storage entry in DIDData protobuf ([#80](https://github.com/hyperledger-identus/neoprism/pull/80))

### ⚙️ Miscellaneous Tasks

- Make docker setup use published image ([#81](https://github.com/hyperledger-identus/neoprism/pull/81))

## [0.3.0](https://github.com/hyperledger-identus/neoprism/releases/tag/v0.3.0) - 2025-07-31

### 🚀 Features

- Support operation submission using cardano-wallet ([#75](https://github.com/hyperledger-identus/neoprism/pull/75))
- Add indexer integration test suite ([#77](https://github.com/hyperledger-identus/neoprism/pull/77))

### 🐛 Bug Fixes

- Optimize testnet genesis block ([#72](https://github.com/hyperledger-identus/neoprism/pull/72))

### 🚜 Refactor

- Split release script into its own devshell ([#71](https://github.com/hyperledger-identus/neoprism/pull/71))

### 🧪 Testing

- Prepare devshell and packages for interop testing ([#73](https://github.com/hyperledger-identus/neoprism/pull/73))

### ⚙️ Miscellaneous Tasks

- Update healthcheck for docker setup ([#76](https://github.com/hyperledger-identus/neoprism/pull/76))

## [0.2.0](https://github.com/hyperledger-identus/neoprism/releases/tag/v0.2.0) - 2025-07-19

### 🚀 Features

- Enable CORS ([#64](https://github.com/hyperledger-identus/neoprism/pull/64))
- Add healthcheck and build metadata endpoint ([#68](https://github.com/hyperledger-identus/neoprism/pull/68))
- Configurable confirmation blocks ([#69](https://github.com/hyperledger-identus/neoprism/pull/69))

### 💼 Other

- Support multi-arch testnet image ([#67](https://github.com/hyperledger-identus/neoprism/pull/67))

### 🧪 Testing

- Add local testnet infrastructure ([#66](https://github.com/hyperledger-identus/neoprism/pull/66))

## [0.1.1](https://github.com/hyperledger-identus/neoprism/releases/tag/v0.1.1) - 2025-07-15

### 🐛 Bug Fixes

- Update the new VDR protobufs ([#61](https://github.com/hyperledger-identus/neoprism/pull/61))

### 💼 Other

- Generate docker configs from single source of truth ([#62](https://github.com/hyperledger-identus/neoprism/pull/62))

### ⚙️ Miscellaneous Tasks

- Update version in docker compose

## [0.1.0](https://github.com/hyperledger-identus/neoprism/releases/tag/v0.1.0) - 2025-07-10

### ⚙️ Miscellaneous Tasks

- Add PR title check ([#56](https://github.com/hyperledger-identus/neoprism/pull/56))
- Add release action and improve packaging ([#57](https://github.com/hyperledger-identus/neoprism/pull/57))
- Fix release action unable to checkout ([#58](https://github.com/hyperledger-identus/neoprism/pull/58))
- Release docker image to hyperledgeridentus dockerhub ([#59](https://github.com/hyperledger-identus/neoprism/pull/59))

<!-- generated by git-cliff -->
