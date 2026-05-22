# AGENTS.md — NeoPRISM Agent Guide

## Project Overview

NeoPRISM is a Rust implementation of a PRISM node for managing Decentralized Identifiers (DIDs) anchored on the Cardano blockchain. It is a Hyperledger Identus project under the Linux Foundation Decentralized Trust.

Workspace members:

- `bin/neoprism-node/` — Main binary, PRISM node executable
- `lib/did-prism/` — Core PRISM DID implementation (operations, protocol)
- `lib/did-core/` — W3C DID Core types and traits
- `lib/apollo/` — Cryptographic primitives (secp256k1, Ed25519, X25519)
- `lib/did-prism-indexer/` — Cardano blockchain indexer (Oura, DBSync, Blockfrost)
- `lib/did-prism-ledger/` — In-memory ledger implementation
- `lib/did-prism-submitter/` — DID operation submission to blockchain
- `lib/did-resolver-http/` — HTTP DID resolver
- `lib/node-storage/` — PostgreSQL and SQLite storage layer

## Development Environment

**Nix is required.** All commands must be run inside `nix develop`:

```bash
nix develop                    # Enter dev shell
nix develop --unset PATH       # Enter pure dev shell
```

## Documentation

Documentation is built using [mdBook](https://rust-lang.github.io/mdBook/).

- Source: `docs/src/`
- Config: `docs/book.toml`
- Build: `nix build .#docs-site`
- Output: `docs/book/` (HTML)

The docs use two preprocessors:

- `cmdrun` — run commands in markdown
- `mdbook-d2` — render D2 diagrams

## Rust Guidelines

### Build Commands

- Build: `just build` or `cargo build --all-features`
- Build assets only: `just build-assets` (Tailwind CSS)
- Build Docker configs: `just build-config`

### Test Commands

- Run all tests: `just test` or `cargo test --all-features`
- Run single test in crate: `cargo test -p <crate> <test_name>`
- Run single test with full path: `cargo test -p <crate> <module>::test_fn`
- Run integration tests: `cargo test --all-features --test <test_file>`
- Coverage: `just coverage` (LCOV) or `just coverage-html` (HTML report)

### Lint and Format Commands

- Format all: `just format` (Rust, Nix, TOML, Python, SQL, Hurl)
- Format Rust only: `cargo fmt`
- Clippy: `cargo clippy --all-targets -- -D warnings`
- Full check: `just check` (format + build + test + clippy)
- Pre-PR check: `just full-check` (includes E2E tests)

### Text File Linting

This project uses **nix-provided linters** instead of `npx`. All linter
binaries are declared in `nix/devShells/default.nix` and available inside
`nix develop`. This ensures reproducible versions across all developers
without requiring Node.js, Python, or Homebrew for linting.

**Commands** (run inside `nix develop`):

- Lint all text files: `just lint-text`
- Auto-fix markdown: `just lint-text-fix`

**Tools and config files:**

| Tool | Config | Purpose |
| --- | --- | --- |
| markdownlint-cli2 | `.markdownlint.yml`, `.markdownlint-cli2.yaml` | Markdown formatting |
| yamllint | `.yamllint.yml` | YAML validation |
| editorconfig-checker | `.editorconfig`, `.editorconfig-checker.json` | Charset (UTF-8 no BOM), line endings (LF), indent |
| shellcheck | — | Shell script analysis |

**CI vs nix version gap:**

The CI workflow (`file-hygiene.yml`) uses GitHub Actions with independently
pinned versions that may differ from nixpkgs. When a rule passes locally
but fails in CI (or vice versa), the cause is usually a version mismatch.

Current known gaps:

- **markdownlint**: nix has v0.38.0, CI has v0.40.0. Rule MD060 (table
  column style) exists only in CI — disabled in config since it cannot
  be verified locally.
- **yamllint**: nix has v1.37.1, CI has v1.35.1. Minor rule behavior
  differences on `colons` — relaxed in config.

**Managing linter dependencies:**

Linter packages are declared in `nix/devShells/default.nix` under the
`# text linters` comment block. Their versions come from the nixpkgs
input pinned in `flake.lock`. The CI versions come from the reusable
workflow in `hyperledger-identus/.github` (pinned action SHAs in
`.github/workflows/lint-files.yml`).

To check current nix versions:

```bash
nix develop --command bash -c "markdownlint-cli2 --help | head -1; yamllint --version; editorconfig-checker --version; shellcheck --version | head -2"
```

To check CI versions, inspect the GitHub Action source for each tool:

- markdownlint: `DavidAnson/markdownlint-cli2-action` → check tag
- yamllint: `frenck/action-yamllint` → check `src/requirements.txt`
- editorconfig-checker: `editorconfig-checker/action-editorconfig-checker`
- shellcheck: `ludeeus/action-shellcheck` → check tag

**When versions diverge:**

1. Try `nix flake update` to pull newer nixpkgs — may close the gap
2. If nixpkgs is still behind CI, disable the new rule in config and
   add a comment explaining why (e.g., `# MD060 disabled — nix v0.38.0
   does not have this rule, CI v0.40.0 does`)
3. If nix is ahead of CI, relax the rule or exclude affected files in
   config (e.g., vendored Cardano YAML files in `.yamllint.yml` ignore)
4. Never fix issues that cannot be verified locally — disable in config
5. Update the "Current known gaps" list above when gaps change

**Key files for linter management:**

| What | Where |
| --- | --- |
| Nix linter packages | `nix/devShells/default.nix` (`# text linters` block) |
| Nix input versions | `flake.lock` (update with `nix flake update`) |
| CI workflow caller | `.github/workflows/file-hygiene.yml` |
| CI reusable workflow | `hyperledger-identus/.github/.github/workflows/lint-files.yml` |
| Markdownlint rules | `.markdownlint.yml` |
| Markdownlint ignores | `.markdownlint-cli2.yaml` |
| Yamllint rules + ignores | `.yamllint.yml` |
| EditorConfig rules | `.editorconfig` |
| EditorConfig excludes | `.editorconfig-checker.json` |
| Just recipes | `justfile` (`lint-text`, `lint-text-fix`) |

### Code Style

#### Imports

Use `StdExternalCrate` grouping with module-level granularity:

```rust
// Standard library
use std::collections::HashMap;
// External crates
use axum::Json;
use serde::Deserialize;
// Local crates
use crate::error::Error;
```

Run `cargo fmt` to auto-format imports.

#### Formatting

- Line width: 120 characters max
- Edition: Rust 2024
- Format doc comments: enabled

#### Naming Conventions

- Identifiers: `snake_case`
- Types: `CamelCase`
- Choose descriptive, unambiguous names

#### Dependency Versions

In workspace member `Cargo.toml`, always use workspace dependencies:

```toml
[dependencies]
serde = { workspace = true }
tokio = { workspace = true }
```

Do not specify versions in member crates.

#### Error Handling

- Return `Result`/`Option` in public APIs
- Define custom error types in `error.rs` at crate or module level
- Use `derive_more::Error` and `derive_more::Display` for error types
- Map errors with context using `.map_err()`; log via `tracing`

Example error definition:

```rust
#[derive(Debug, derive_more::Display, derive_more::Error)]
pub enum Error {
    #[display("did suffix {suffix} is invalid")]
    InvalidSuffix { suffix: String },
    #[display("failed to parse key data for id {id}")]
    KeyParseError { id: PublicKeyId },
}
```

#### Error Message Style

- **Tone**: short, factual, lowercase start, no trailing period (e.g., "did is not found")
- **Placeholders**: use named placeholders (`{id}`, `{did}`, `{limit}`, `{actual}`, `{expected}`, `{location}`)

**User-facing errors** (HTTP API and service layers):

- Use `derive_more #[display("...")]` with descriptive placeholders
- Applies to: `ApiError`, `ResolutionError`, and any error that propagates to HTTP responses
- BadRequest (400): preserve error message to help users fix their input, debug formmating (`{:?}`) allowed
- Internal (500): show generic "internal server error", log full error chain via `tracing::error!`

**Developer-facing errors** (library layer, internal processing):

- Debug formatting (`{:?}`) allowed for hashes, binary data, internal IDs
- These appear in logs but are masked before reaching HTTP clients
- Applies to: `did-prism/*`, `did-core`, `did-prism-indexer`, `node-storage`, `apollo`

**Examples**:

```rust
// User-facing (ApiError, ResolutionError)
#[display("public key id {id} is invalid")]           // ✅ descriptive placeholder
#[display("did {did} is not found")]                   // ✅ descriptive placeholder

// Developer-facing (library errors, logged only)
#[display("entry with hash {initial_hash:?} exists")] // ✅ {:?} ok for hashes in logs
#[display("block {block_hash:?} tx {tx_idx:?}")]       // ✅ {:?} ok for internal IDs
```

**Quick checklist**:

- starts lowercase
- no trailing period
- placeholder names are descriptive
- `{:?}` only in developer-facing errors (library layer)

#### Logging

Use structured logging via `tracing`:

```rust
tracing::info!(did = %did, "resolving did document");
tracing::error!(error = %e, "failed to connect");
```

Control verbosity with `RUST_LOG=debug` environment variable.

#### Tests

- Place tests in `tests/` directory or next to modules with `#[cfg(test)]`
- Use descriptive test names: `test_create_did_with_valid_input`
- For async tests: `#[tokio::test]`
- Run single test: `cargo test -p did-prism storage_operation`

### SQL Migrations

- Location: `lib/node-storage/migrations/postgres/`
- Format with: `sqlfluff fix . && sqlfluff lint .`
- Dialect: PostgreSQL
- Keywords: UPPERCASE

## Python Guidelines

### Docker Configuration Generation

- Location: `tools/compose_gen/`
- Run: `just build-config` or `python -m compose_gen.main`
- Format: `just tools::format`
- Type check: `just tools::check`
- **Important**: All `docker/*/compose*.yml` files are auto-generated. Do not edit directly.

### Code Style

- Formatter: `ruff format`
- Type hints: Required everywhere; use Pydantic models for data validation
- Imports: stdlib → third-party → local; auto-sorted by ruff
- Naming: `snake_case` for functions/variables, `PascalCase` for classes

## Scala Guidelines

### Build Commands

- Build tests: `just e2e::build`
- Run tests: `just e2e::run` (runs multiple configurations)

### Code Style

- Formatter: `scalafmt`
- Imports: stdlib → external → local
- Naming: `camelCase` for identifiers
- Error handling: return `Either`/`Try`

## General Guidelines

### Database Operations

- PostgreSQL: `just postgres-up` / `just postgres-down`
- SQLite: `just sqlite-init` / `just sqlite-clean`

### Commit Conventions

- Format: [Conventional Commits](https://www.conventionalcommits.org/) (scopes optional, e.g. `chore(release): prepare for the next release`)
- Limit: 72 characters
- No secrets in commits
- Examples:
  - `feat(indexer): add postgres connection pooling`
  - `fix(did-prism): fix did resolution for revoked keys`
  - `chore(deps): update cargo dependencies`

### Pre-Commit Checklist

Before submitting a PR:

1. `just format` — format all sources
2. `just lint-text` — lint markdown, YAML, editorconfig, shell scripts
3. `just test` — run all tests
4. `just check` — full validation (optional but recommended)

### File Generation Warnings

- Docker Compose files (`docker/*/compose*.yml`): auto-generated from Python
- Bindings (`bindings/ts-types/`): auto-generated from TypeScript
- Do not edit these files directly; modify source generators instead

### Cursor / Copilot Rules

If `.github/copilot-instructions.md` or `.cursor/rules/` exist, follow those additionally.
