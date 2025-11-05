# AGENTS.md — NeoPRISM Agent Guide

## Rust Guidelines

### Build, Lint, and Test Commands
- Enter dev shell: `nix develop`
- Build Rust workspace: `just build` (or `cargo build --all-features` inside dev shell)
- Run all Rust tests: `just test` (or `cargo test --all-features` inside dev shell)
- Run a single Rust test: `cargo test -p <crate> <test_name>` (or `cargo test --package <crate> <test_name>`)
- Format all sources: `just format` (formats Rust, Nix, TOML, Dhall, SQL)
- Start local database: `just db-up`
- Stop local database: `just db-down`

### Code Style Guidelines
- Imports: group by standard, external, then local; remove unused imports.
- Formatting: run `cargo fmt` for Rust; no trailing whitespace.
- Types: prefer explicit types in public APIs; use idiomatic Rust types and lifetime/ad hoc types sparingly.
- Naming: snake_case for Rust identifiers, CamelCase for Rust types; choose descriptive, unambiguous names.
- Dependency versions: for the root `Cargo.toml`, specify crate versions as normal. For workspace member `Cargo.toml`, always refer to the workspace dependencies version (do not specify a version, use `workspace = true`).
- Error handling: return `Result`/`Option` in Rust; map errors with context and log via `tracing`.
- Prefer defining custom error types in an `error.rs` file at the crate or module level as appropriate. Use `derive_more::Error` to derive the error trait when possible.
- Logging: use structured logs, control verbosity with `RUST_LOG`.
- Tests: place Rust tests in `tests/` or next to modules; use descriptive test names; run single tests via `cargo test <name>` inside dev shell.

### Error message style

- Tone: short, factual, lowercase start, no trailing period (eg. "did is not found")
- Placeholders: use named placeholders ({id}, {did}, {limit}, {actual}, {expected}, {location})
- Use derive_more #[display("...")] for enum/struct messages; prefer `{source}` when wrapping another error
- User-facing messages: no debug formatting ({:?}), avoid internal hashes or binary blobs
- Developer-facing messages: allowed to include {:?} or internal ids; keep them marked/internal (logs or debug-only)

Quick checklist
- starts lowercase
- no trailing period
- placeholder names are descriptive
- no {:?} in user-facing messages

Examples
- Good: #[display("public key id {id} is invalid")]
- Bad: #[display("entry with hash {initial_hash:?} already exists")] (avoid in API responses)

Lint suggestions
- Detect: display attributes that start with uppercase or end with period; detect {:?} inside displays

## Scala Guidelines

### Build, Lint, and Test Commands
- Format Scala sources: `just prism-test-build`
- Build and run conformance tests: `just prism-test-run`
- Build docs site: `nix build .#docs-site`

### Code Style Guidelines
- Imports: group by standard, external, then local; remove unused imports.
- Formatting: run `scalafmt` for Scala; no trailing whitespace.
- Types: prefer explicit types in public APIs; use idiomatic Scala types and lifetime/ad hoc types sparingly.
- Naming: camelCase for Scala; choose descriptive, unambiguous names.
- Error handling: return `Either`/`Try` in Scala.

## General Guidelines

### Commits
- Follow Conventional Commits (NO SCOPES), 72-char limit, no secrets.

### Cursor / Copilot rules
- Follow repository Copilot instructions in `.github/copilot-instructions.md` if present.
- Respect any Cursor rules under `.cursor/rules/` or `.cursorrules`.

### Quick verification
- Before PR: run `just format` and `just test`.

(Keep this guide short — agents should follow existing repo docs for deeper tasks.)
