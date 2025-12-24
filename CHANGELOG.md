# Changelog

All notable changes to this project will be documented in this file.

## [0.9.0](https://github.com/hyperledger-identus/neoprism/releases/tag/v0.9.0) - 2025-12-24

### 🚀 Features

- Sqlite backend #108 ([#158](https://github.com/hyperledger-identus/neoprism/pull/158))
- Add transaction ID indexing and query endpoint ([#184](https://github.com/hyperledger-identus/neoprism/pull/184))
- Add operation ID indexing and query endpoint ([#185](https://github.com/hyperledger-identus/neoprism/pull/185))

### 🚜 Refactor

- Streamline development tooling and configuration ([#181](https://github.com/hyperledger-identus/neoprism/pull/181))

### 📚 Documentation

- Correct the broken link to the nix shell ([#175](https://github.com/hyperledger-identus/neoprism/pull/175))
- Update e2e commands and stack naming conventions ([#189](https://github.com/hyperledger-identus/neoprism/pull/189))

### ⚙️ Miscellaneous Tasks

- Add external URL config to neoprism service in test setup ([#174](https://github.com/hyperledger-identus/neoprism/pull/174))
- Update zio-http from 3.7.0 to 3.7.1 ([#176](https://github.com/hyperledger-identus/neoprism/pull/176))
- Update zio-http from 3.7.1 to 3.7.3 ([#178](https://github.com/hyperledger-identus/neoprism/pull/178))
- Update zio-http from 3.7.3 to 3.7.4 ([#179](https://github.com/hyperledger-identus/neoprism/pull/179))

## [0.8.0](https://github.com/hyperledger-identus/neoprism/releases/tag/v0.8.0) - 2025-12-02

### 🚀 Features

- Add in-memory ledger for testing purposes ([#155](https://github.com/hyperledger-identus/neoprism/pull/155))

### 💼 Other

- Support cross-platform docker build for macOS ([#164](https://github.com/hyperledger-identus/neoprism/pull/164))

### 🚜 Refactor

- Migrate docker compose config generation from dhall to python ([#159](https://github.com/hyperledger-identus/neoprism/pull/159))

### 📚 Documentation

- Add blockfrost deployment example with neoprism integration ([#152](https://github.com/hyperledger-identus/neoprism/pull/152))

### 🎨 Styling

- Remove unnecessary future annotations imports ([#162](https://github.com/hyperledger-identus/neoprism/pull/162))

### 🧪 Testing

- Streamline e2e testing workflow and build commands ([#161](https://github.com/hyperledger-identus/neoprism/pull/161))

### ⚙️ Miscellaneous Tasks

- Update sbt-scalafmt from 2.5.5 to 2.5.6 ([#150](https://github.com/hyperledger-identus/neoprism/pull/150))
- Replace nix shell scripts with justfile task runner ([#154](https://github.com/hyperledger-identus/neoprism/pull/154))
- Remove scala-did implementation and references ([#160](https://github.com/hyperledger-identus/neoprism/pull/160))
- Update grpc-netty-shaded from 1.76.0 to 1.76.1 ([#166](https://github.com/hyperledger-identus/neoprism/pull/166))
- Update grpc-netty-shaded from 1.76.1 to 1.77.0 ([#167](https://github.com/hyperledger-identus/neoprism/pull/167))
- Update zio-http from 3.5.1 to 3.7.0 ([#171](https://github.com/hyperledger-identus/neoprism/pull/171))
- Update zio, zio-test, zio-test-magnolia, ... from 2.1.22 to 2.1.23 ([#170](https://github.com/hyperledger-identus/neoprism/pull/170))
- Update scalafmt-core from 3.10.1 to 3.10.2 ([#169](https://github.com/hyperledger-identus/neoprism/pull/169))
- Add Scorecard supply-chain security workflow ([#172](https://github.com/hyperledger-identus/neoprism/pull/172))

## [0.7.0](https://github.com/hyperledger-identus/neoprism/releases/tag/v0.7.0) - 2025-10-30

### 🚀 Features

- Add dynamic port and external URL for OpenAPI server ([#145](https://github.com/hyperledger-identus/neoprism/pull/145))

### 🐛 Bug Fixes

- Correct typo in error message and remove unused files ([#144](https://github.com/hyperledger-identus/neoprism/pull/144))

### 🧪 Testing

- Integrate blockfrost ryo backend for prism-test environment ([#148](https://github.com/hyperledger-identus/neoprism/pull/148))

### ⚙️ Miscellaneous Tasks

- Update scalafmt-core from 3.9.6 to 3.9.10 ([#146](https://github.com/hyperledger-identus/neoprism/pull/146))
- Update scalafmt-core from 3.9.10 to 3.10.1 ([#147](https://github.com/hyperledger-identus/neoprism/pull/147))
- *(release)* Prepare the next release ([#149](https://github.com/hyperledger-identus/neoprism/pull/149))

## [0.6.2](https://github.com/hyperledger-identus/neoprism/releases/tag/v0.6.2) - 2025-10-27

### 🐛 Bug Fixes

- Correct service id display and improve page layout with footer ([#142](https://github.com/hyperledger-identus/neoprism/pull/142))

### ⚙️ Miscellaneous Tasks

- *(release)* Prepare the next release ([#143](https://github.com/hyperledger-identus/neoprism/pull/143))

## [0.6.1](https://github.com/hyperledger-identus/neoprism/releases/tag/v0.6.1) - 2025-10-20

### 🐛 Bug Fixes

- Add DID prefix to service IDs in DID documents ([#135](https://github.com/hyperledger-identus/neoprism/pull/135))

### ⚙️ Miscellaneous Tasks

- Update grpc-netty-shaded from 1.75.0 to 1.76.0 ([#134](https://github.com/hyperledger-identus/neoprism/pull/134))
- Update scala3-library from 3.3.6 to 3.3.7 ([#137](https://github.com/hyperledger-identus/neoprism/pull/137))
- Update zio, zio-test, zio-test-magnolia, ... from 2.1.21 to 2.1.22 ([#138](https://github.com/hyperledger-identus/neoprism/pull/138))
- Add version bump workflow automation ([#139](https://github.com/hyperledger-identus/neoprism/pull/139))

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
