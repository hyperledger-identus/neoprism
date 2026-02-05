# Implementation Plan: Blockfrost Documentation Updates

## Overview
Update all relevant documentation to reflect the new Blockfrost DLT source support introduced in commit 06b9ccd. The documentation needs to be updated across README, mdbook docs, and Docker compose READMEs to ensure users understand how to configure and use the Blockfrost data source.

## Requirements

**Functional Requirements:**
- Document Blockfrost as a third DLT source option alongside Oura and DB-Sync
- Document all Blockfrost-specific CLI options and environment variables
- Update duration format documentation (now uses humantime format like `10s`, `1m`)
- Add Blockfrost to all relevant feature lists and examples
- Document the new `compose-ci-blockfrost.yml` testing stack

**Non-Functional Requirements:**
- Maintain consistency with existing documentation style
- Keep documentation concise and actionable
- Ensure all code examples are accurate and tested
- Update architecture diagrams/descriptions to include Blockfrost

## Technical Decisions

**Architecture:**
- Blockfrost is treated as an interchangeable DLT source alongside Oura and DB-Sync
- Blockfrost requires API key authentication unlike Oura/DB-Sync
- Blockfrost has rate limiting considerations requiring `api_delay` and `concurrency_limit` options

**Documentation Approach:**
- README.md: High-level feature mention and quick-start example
- indexer.md: Detailed configuration options and comparison
- architecture.md: Mention as alternative data source
- useful-links.md: External reference link
- prism-test/README.md: Testing stack documentation

**Design Patterns:**
- Follow existing documentation structure for DLT sources
- Use consistent formatting for CLI options and environment variables
- Group related configuration options together

**Integration Points:**
- Blockfrost API (https://cardano-mainnet.blockfrost.io/api/v0)
- Blockfrost Rust client library (blockfrost crate)

## Context & Assumptions

**Constraints:**
- CLI options auto-generate via `cmdrun` directives in mdbook
- Docker compose files in `docker/prism-test/` are auto-generated from Python sources
- Must maintain backward compatibility with existing Oura/DB-Sync documentation

**Assumptions:**
- Users are familiar with basic NeoPRISM concepts (indexer, submitter, standalone modes)
- Users understand what Blockfrost is (hosted Cardano API service)
- Documentation readers have access to a Blockfrost API key

**Risks:**
- CLI options may change if the implementation is modified
- Docker compose stack documentation may become outdated if Python sources change

**Scope Boundaries:**
- **In scope:** README.md, docs/src/configuration/indexer.md, docs/src/architecture/README.md, docs/src/useful-links.md, docker/prism-test/README.md, docker/blockfrost-neoprism-demo/README.md
- **Out of scope:** CHANGELOG.md (auto-generated from releases), code implementation changes, Docker compose YAML files (auto-generated)

---

## Milestone 1: Update README.md

**Goal:** Add Blockfrost to the main project README as a supported DLT source and provide a quick-start example.

### Implementation Steps

1. Update line 35 in README.md to mention Blockfrost alongside Oura and DB-Sync:
   - Change: "- Ingests DID operations from various Cardano data sources, including [Oura](https://github.com/txpipe/oura) and [DBSync](https://github.com/input-output-hk/cardano-db-sync)."
   - To: "- Ingests DID operations from various Cardano data sources, including [Oura](https://github.com/txpipe/oura), [DBSync](https://github.com/input-output-hk/cardano-db-sync), and [Blockfrost](https://blockfrost.io/)."

2. Add Blockfrost `just run` command example to the frequently used commands table around line 372:
   - Add new row: `| just run indexer --blockfrost-api-key <KEY> | Run the indexer node, connecting to Blockfrost API with your API key |`

3. Verify the Blockfrost link uses the correct URL format consistent with other links in the document.

### Verification
- Review the README.md changes to ensure:
  - Blockfrost is mentioned in the features section
  - The new `just run` command example is properly formatted in the table
  - All links are correctly formatted

---

## Milestone 2: Update docs/src/configuration/indexer.md

**Goal:** Add comprehensive Blockfrost DLT source documentation including all configuration options, usage examples, and comparison with other sources.

### Implementation Steps

1. Add Blockfrost as third option under "DLT Source" section (after Oura and DB-Sync):
   ```markdown
   - **Blockfrost:**
     Connects to the Blockfrost API for hosted Cardano blockchain data access.
     Requires a Blockfrost API key but eliminates the need to run your own Cardano infrastructure.
     - Key options:
       - API key: `--blockfrost-api-key` or `NPRISM_BLOCKFROST_API_KEY`
       - Base URL: `--blockfrost-base-url` or `NPRISM_BLOCKFROST_BASE_URL`
       - Poll interval: `--blockfrost-poll-interval` or `NPRISM_BLOCKFROST_POLL_INTERVAL` (duration format, e.g., `10s`, `1m`)
       - API delay: `--blockfrost-api-delay` or `NPRISM_BLOCKFROST_API_DELAY` (throttling to respect rate limits)
       - Concurrency limit: `--blockfrost-concurrency-limit` or `NPRISM_BLOCKFROST_CONCURRENCY_LIMIT`
   ```

2. Update the duration format note for existing options:
   - Line 20: Change "(duration format, e.g., `10s`, `1m`)" to ensure consistency
   - Line 23: Update index interval to show duration format

3. Add Blockfrost section under "DLT Source Comparison" after the DB Sync section:
   ```markdown
   **Blockfrost**

   Blockfrost is a hosted API service that provides access to Cardano blockchain data without requiring you to run your own infrastructure.
   This is the easiest option to get started with as it requires no Cardano node, DB-Sync instance, or relay connections.
   
   To use Blockfrost, you need to obtain an API key from [blockfrost.io](https://blockfrost.io/).
   The free tier is sufficient for most development and testing use cases.
   
   Note that Blockfrost has rate limits depending on your plan. Use the `api_delay` option to add throttling
   between API calls if you encounter rate limiting issues. The default concurrency limit of 4 helps prevent
   overwhelming the API with too many concurrent requests.
   ```

4. Verify that all Blockfrost configuration options are documented with correct environment variable names and CLI flags.

### Verification
- Read the updated indexer.md file to verify:
  - Blockfrost section is added in the correct location
  - All five configuration options are documented
  - Duration format examples are consistent
  - The comparison section provides useful context about rate limits and API keys

---

## Milestone 3: Update docs/src/architecture/README.md

**Goal:** Update architecture documentation to mention Blockfrost as an alternative DLT source alongside Oura and DB-Sync.

### Implementation Steps

1. Update line 40 to mention Blockfrost:
   - Change: `internal.neoprism <- cardano-node: stream operations using Oura`
   - To: `internal.neoprism <- cardano-node: stream operations using Oura, DB-Sync, or Blockfrost`

2. Update line 71 similarly:
   - Change: `indexer-deployment.indexer <- cardano-node: stream operations using Oura`
   - To: `indexer-deployment.indexer <- cardano-node: stream operations using Oura, DB-Sync, or Blockfrost`

3. Add a note after the deployment diagrams explaining that the three DLT sources are interchangeable:
   ```markdown
   > **Note:** The indexer can use any of three data sources: Oura (streaming from Cardano node), 
   > DB-Sync (querying PostgreSQL database), or Blockfrost (hosted API service). Choose the source 
   > that best fits your infrastructure and operational requirements.
   ```

### Verification
- Review the architecture/README.md to ensure:
  - Both diagram descriptions mention all three DLT sources
  - The explanatory note is clear and helpful
  - Formatting is consistent with the rest of the document

---

## Milestone 4: Update docs/src/useful-links.md

**Goal:** Add Blockfrost to the external resources section for easy reference.

### Implementation Steps

1. Add Blockfrost link after the Cardano Wallet link (around line 24):
   ```markdown
   - [Blockfrost](https://blockfrost.io/)
   ```

2. Verify the link format matches other entries (uses markdown list format with `- [Name](URL)`).

### Verification
- Check that the useful-links.md file includes Blockfrost in the External Resources section
- Verify the link format is consistent with other entries

---

## Milestone 5: Update docker/prism-test/README.md

**Goal:** Document the new `compose-ci-blockfrost.yml` testing stack and update the configuration matrix.

### Implementation Steps

1. Read the current docker/prism-test/README.md to understand the existing structure and format.

2. Add documentation for the new `compose-ci-blockfrost.yml` stack:
   - Add it to the overview section if there's a list of compose files
   - Document what services it includes (similar to existing compose files)
   - Note that it uses Blockfrost API instead of direct DB-Sync connection

3. Update the configuration matrix table to include the Blockfrost variant:
   - Add new row for `compose-ci-blockfrost.yml`
   - Columns: Mode, Backend, Compose file, `just` target, Recommended usage, Pros, Trade-offs
   - Mode: `ci-blockfrost`, Backend: `PostgreSQL`, Compose file: `docker/prism-test/compose-ci-blockfrost.yml`
   - Just target: `just e2e::up ci-blockfrost`
   - Recommended usage: CI testing with Blockfrost as the DLT source
   - Pros: Tests Blockfrost integration without requiring full Cardano infrastructure
   - Trade-offs: Requires Blockfrost API key; rate limits may affect test speed

4. Verify the Blockfrost stack documentation follows the same format as existing compose file documentation.

### Verification
- Review the docker/prism-test/README.md to ensure:
  - The new compose-ci-blockfrost.yml is documented
  - The configuration matrix includes the Blockfrost variant
  - All descriptions are accurate and helpful

---

## Milestone 6: Update docker/blockfrost-neoprism-demo/README.md (Optional Enhancement)

**Goal:** Clarify the distinction between using Blockfrost as a DLT source vs the Blockfrost Ryo demo setup.

### Implementation Steps

1. Read the current docker/blockfrost-neoprism-demo/README.md to understand its structure.

2. Add a note in the Overview section clarifying that this demo uses Blockfrost Ryo (a self-hosted Blockfrost-compatible API) and requires a DB-Sync database, whereas using Blockfrost directly as a DLT source requires only an API key:
   ```markdown
   > **Note:** This demo setup uses Blockfrost Ryo, a self-hosted Blockfrost-compatible API that 
   > requires a DB-Sync database. For a simpler setup without DB-Sync, you can configure NeoPRISM 
   > to use the hosted Blockfrost API directly by providing a Blockfrost API key. See the 
   > [Indexer Configuration](../../docs/src/configuration/indexer.md) documentation for details.
   ```

3. Verify the clarification is helpful and doesn't confuse users.

### Verification
- Review the blockfrost-neoprism-demo/README.md to ensure:
  - The clarification note is present and helpful
  - It correctly distinguishes between the two Blockfrost integration approaches

---

## Milestone 7: Verify CLI Options Documentation

**Goal:** Ensure that the auto-generated CLI options documentation includes the new Blockfrost options.

### Implementation Steps

1. Check the docs/src/references/cli-options.md file structure.

2. Verify it uses `cmdrun` directives that will auto-generate help output:
   - Confirm the file contains: `<!-- cmdrun neoprism-node indexer -h -->`
   - This will automatically include the new Blockfrost options when the book is built

3. Note: No manual edits needed to this file as it's auto-generated during mdbook build.

### Verification
- Confirm the cli-options.md file contains the `cmdrun` directives
- Build the documentation (if possible) to verify the Blockfrost options appear correctly:
  ```bash
  nix build .#docs-site
  ```

---

## Summary

This plan covers all documentation updates needed for the Blockfrost DLT source feature:

1. **README.md** - High-level feature mention and quick-start
2. **docs/src/configuration/indexer.md** - Detailed configuration documentation
3. **docs/src/architecture/README.md** - Architecture diagram updates
4. **docs/src/useful-links.md** - External resource link
5. **docker/prism-test/README.md** - Testing stack documentation
6. **docker/blockfrost-neoprism-demo/README.md** - Clarification (optional)
7. **docs/src/references/cli-options.md** - Auto-generation verification

All changes maintain consistency with existing documentation style and provide clear, actionable information for users wanting to use Blockfrost as their DLT source.
