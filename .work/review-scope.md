# Review Scope: Add embedded wallet DLT sink for Cardano transactions

**Commit:** `29179321dd88246a809e31c26bdff1f8c0aa6e4b`
**Author:** Pat Losoponkul
**Date:** Wed Mar 25 11:20:12 2026 +0700
**Files Changed:** 51 files (+2077/-245 lines)

---

## 1️⃣ TypeScript Embedded Wallet Package
*The wallet SDK/CLI implementation in TypeScript*

| Component | Files |
|-----------|-------|
| Core Sources | `packages/embedded-wallet/src/{cli,transaction,types}.ts` |
| Package Config | `packages/embedded-wallet/{package.json,tsconfig.json,.gitignore}` |
| Tests | `packages/embedded-wallet/tests/cli.test.ts`, `bun.lock` |

**Files:**
- `packages/embedded-wallet/src/cli.ts`
- `packages/embedded-wallet/src/transaction.ts`
- `packages/embedded-wallet/src/types.ts`
- `packages/embedded-wallet/package.json`
- `packages/embedded-wallet/tsconfig.json`
- `packages/embedded-wallet/.gitignore`
- `packages/embedded-wallet/tests/cli.test.ts`
- `packages/embedded-wallet/bun.lock`

---

## 2️⃣ Rust DLT Sink & CLI Integration
*The Cardano transaction sink implementation and neoprism-node CLI*

| Component | Files |
|-----------|-------|
| DLT Sink | `lib/did-prism-submitter/src/dlt/{embedded_wallet.rs,cardano_wallet.rs,mod.rs}`, `lib/did-prism-submitter/{lib.rs,Cargo.toml}` |
| CLI Binary | `bin/neoprism-node/src/{cli.rs,lib.rs}`, `bin/neoprism-node/Cargo.toml` |
| Lock File | `Cargo.lock` |

**Files:**
- `lib/did-prism-submitter/src/dlt/embedded_wallet.rs` (NEW)
- `lib/did-prism-submitter/src/dlt/cardano_wallet.rs`
- `lib/did-prism-submitter/src/dlt/mod.rs`
- `lib/did-prism-submitter/src/lib.rs`
- `lib/did-prism-submitter/Cargo.toml`
- `bin/neoprism-node/src/cli.rs`
- `bin/neoprism-node/src/lib.rs`
- `bin/neoprism-node/Cargo.toml`
- `Cargo.lock`

---

## 3️⃣ Build System & Nix Packaging
*How the embedded wallet is built, packaged, and integrated into the dev workflow*

| Component | Files |
|-----------|-------|
| Nix Packaging | `nix/embedded-wallet/{default,package}.nix`, `nix/neoprism/{binaries,images,packages/*}.nix`, `nix/devShells/default.nix`, `nix/checks/default.nix`, `flake.nix` |
| Compose Generator | `tools/compose_gen/{main.py, services/neoprism.py, stacks/prism_test.py}` |
| Build Scripts | `justfile`, `tools/just-recipes/{embedded-wallet.just, e2e.just}` |
| Lock Files | `bun.lock` (root), `package-lock.json` (deleted) |

**Files:**
- `nix/embedded-wallet/default.nix` (NEW)
- `nix/embedded-wallet/package.nix` (NEW)
- `nix/neoprism/binaries.nix`
- `nix/neoprism/images.nix`
- `nix/neoprism/packages/neoprism-docker.nix`
- `nix/neoprism/packages/neoprism-ui-assets.nix`
- `nix/devShells/default.nix`
- `nix/checks/default.nix`
- `flake.nix`
- `tools/compose_gen/main.py`
- `tools/compose_gen/services/neoprism.py`
- `tools/compose_gen/stacks/prism_test.py`
- `justfile`
- `tools/just-recipes/embedded-wallet.just` (NEW)
- `tools/just-recipes/e2e.just`
- `bun.lock` (NEW)
- `package-lock.json` (DELETED)

---

## 4️⃣ Docker & Deployment Configuration
*All Docker Compose files for various deployment scenarios*

| Component | Files |
|-----------|-------|
| New Compose | `docker/prism-test/compose-ci-embedded-wallet.yml` |
| Existing Compose | `docker/prism-test/{compose,compose-ci,compose-ci-sqlite,compose-ci-blockfrost,compose-sqlite}.yml`, `docker/prism-test/README.md` |
| Other Deployments | `docker/{mainnet-*,preprod-*,blockfrost-*}/**/compose.yml` (7 files) |

**Files:**
- `docker/prism-test/compose-ci-embedded-wallet.yml` (NEW)
- `docker/prism-test/compose.yml`
- `docker/prism-test/compose-ci.yml`
- `docker/prism-test/compose-ci-sqlite.yml`
- `docker/prism-test/compose-ci-blockfrost.yml`
- `docker/prism-test/compose-sqlite.yml`
- `docker/prism-test/README.md`
- `docker/blockfrost-neoprism-demo/compose.yml`
- `docker/mainnet-blockfrost/compose.yml`
- `docker/mainnet-dbsync/compose.yml`
- `docker/mainnet-relay/compose.yml`
- `docker/mainnet-universal-resolver/compose.yml`
- `docker/preprod-relay/compose.yml`

---

## 5️⃣ Tests & CI Metadata
*Integration tests and CI configuration*

| Component | Files |
|-----------|-------|
| Integration Tests | `tests/prism-test/src/test/scala/.../CreateDidOperationSuite.scala` |
| CI/Issue Tracking | `.beads/config.yaml`, `.beads/issues.jsonl`, `AGENTS.md` (deleted) |

**Files:**
- `tests/prism-test/src/test/scala/org/hyperledger/identus/prismtest/suite/CreateDidOperationSuite.scala`
- `.beads/config.yaml`
- `.beads/issues.jsonl`
- `AGENTS.md` (DELETED)

---

## Summary

| Area | Files | Focus |
|------|-------|-------|
| **1️⃣ TypeScript Wallet** | 8 | TS wallet SDK/CLI |
| **2️⃣ Rust DLT Sink** | 9 | Rust implementation + CLI |
| **3️⃣ Build & Packaging** | 16 | Nix, scripts, generators |
| **4️⃣ Docker Deployment** | 13 | Compose configurations |
| **5️⃣ Tests & CI** | 4 | Integration tests & metadata |
