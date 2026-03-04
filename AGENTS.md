# AGENTS.md — NeoPRISM Agent Guide

## Rust Guidelines

### Build, Lint, and Test Commands
- Enter dev shell: `nix develop`
- Build Rust workspace: `just build` (or `cargo build --all-features` inside dev shell)
- Run all Rust tests: `just test` (or `cargo test --all-features` inside dev shell)
- Run a single Rust test: `cargo test -p <crate> <test_name>` (or `cargo test --package <crate> <test_name>`)
- Format all sources: `just format` (formats Rust, Nix, TOML, Python, SQL)
- Start local database: `just postgres-up`
- Stop local database: `just postgres-down`

### Code Style Guidelines
- Imports: group by standard, external, then local; remove unused imports.
- Formatting: run `cargo fmt` for Rust, `ruff format` for Python; no trailing whitespace.
- Types: prefer explicit types in public APIs; use idiomatic Rust types and lifetime/ad hoc types sparingly.
- Naming: snake_case for Rust identifiers, CamelCase for Rust types; choose descriptive, unambiguous names.
- Dependency versions: for the root `Cargo.toml`, specify crate versions as normal. For workspace member `Cargo.toml`, always refer to the workspace dependencies version (do not specify a version, use `workspace = true`).
- Error handling: return `Result`/`Option` in Rust; map errors with context and log via `tracing`.
- Prefer defining custom error types in an `error.rs` file at the crate or module level as appropriate. Use `derive_more::Error` to derive the error trait when possible.
- Logging: use structured logs, control verbosity with `RUST_LOG`.
- Tests: place Rust tests in `tests/` or next to modules; use descriptive test names; run single tests via `cargo test <name>` inside dev shell.

### Python Guidelines

#### Docker Configuration Generation
- Location: `tools/compose_gen/` contains Python-based Docker Compose configuration generation
- Run generation: `just build-config` (or `python -m compose_gen.main` from `tools/` directory)
- Format tools code: `just tools format` (runs ruff format and import sorting)
- Check tools code: `just tools check` (runs type checking and validation via Nix)
- Structure:
  - `models.py`: Pydantic models for type-safe Docker Compose schema
  - `services/`: Service builder modules (one per service)
  - `stacks/`: Stack composition modules (complex multi-service setups)
  - `main.py`: Entry point for generation

#### Code Style
- Formatting: use `ruff format` for Python files
- Type hints: use type hints everywhere; Pydantic models for data validation
- Imports: follow standard Python import order (stdlib, third-party, local)
- Naming: snake_case for functions/variables, PascalCase for classes
- Error handling: use Pydantic validation for configuration errors

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
- Build Scala conformance tests: `just e2e::build`
- Run conformance tests: `just e2e::run`
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

<!-- BEGIN BEADS INTEGRATION -->
## Issue Tracking with bd (beads)

**IMPORTANT**: This project uses **bd (beads)** for ALL issue tracking. Do NOT use markdown TODOs, task lists, or other tracking methods.

### Why bd?

- Dependency-aware: Track blockers and relationships between issues
- Git-friendly: Dolt-powered version control with native sync
- Agent-optimized: JSON output, ready work detection, discovered-from links
- Prevents duplicate tracking systems and confusion

### Quick Start

**Check for ready work:**

```bash
bd ready --json
```

**Create new issues:**

```bash
bd create "Issue title" --description="Detailed context" -t bug|feature|task -p 0-4 --json
bd create "Issue title" --description="What this issue is about" -p 1 --deps discovered-from:bd-123 --json
```

**Claim and update:**

```bash
bd update <id> --claim --json
bd update bd-42 --priority 1 --json
```

**Complete work:**

```bash
bd close bd-42 --reason "Completed" --json
```

### Issue Types

- `bug` - Something broken
- `feature` - New functionality
- `task` - Work item (tests, docs, refactoring)
- `epic` - Large feature with subtasks
- `chore` - Maintenance (dependencies, tooling)

### Priorities

- `0` - Critical (security, data loss, broken builds)
- `1` - High (major features, important bugs)
- `2` - Medium (default, nice-to-have)
- `3` - Low (polish, optimization)
- `4` - Backlog (future ideas)

### Workflow for AI Agents

1. **Check ready work**: `bd ready` shows unblocked issues
2. **Claim your task atomically**: `bd update <id> --claim`
3. **Work on it**: Implement, test, document
4. **Discover new work?** Create linked issue:
   - `bd create "Found bug" --description="Details about what was found" -p 1 --deps discovered-from:<parent-id>`
5. **Complete**: `bd close <id> --reason "Done"`

### Auto-Sync

bd automatically syncs via Dolt:

- Each write auto-commits to Dolt history
- Use `bd dolt push`/`bd dolt pull` for remote sync
- No manual export/import needed!

### Important Rules

- ✅ Use bd for ALL task tracking
- ✅ Always use `--json` flag for programmatic use
- ✅ Link discovered work with `discovered-from` dependencies
- ✅ Check `bd ready` before asking "what should I work on?"
- ❌ Do NOT create markdown TODO lists
- ❌ Do NOT use external issue trackers
- ❌ Do NOT duplicate tracking systems

For more details, see README.md and docs/QUICKSTART.md.

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd sync
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds

<!-- END BEADS INTEGRATION -->
