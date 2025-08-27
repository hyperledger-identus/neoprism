# Product Requirement Document (PRD)
## Feature: Integrate did-midnight-serde CLI for ContractStateDecoder

---

### Overview

This feature enables the Rust implementation of the `ContractStateDecoder` trait to delegate decoding logic to the external `did-midnight-serde` CLI.
The integration will be encapsulated in a new Rust module, `serde_cli`, within `lib/did-midnight-sources/`.
The module will invoke the CLI as a subprocess, parse its JSON output, and return a Rust `DidDocument`. The subprocess invocation may use blocking code (e.g., `std::process::Command` or `tokio::task::spawn_blocking`), as long as the `ContractStateDecoder` trait remains synchronous and its interface is unchanged.
This approach allows leveraging existing CLI logic for contract state decoding.

---

### Goals

- Enable decoding of `ContractState` via the `did-midnight-serde` CLI.
- Provide a robust, testable Rust interface for subprocess invocation and output parsing. Subprocess invocation may be blocking, but must not change the interface of `ContractStateDecoder` (remains sync).
- Ensure errors are handled and reported according to NeoPRISM repo guidelines.
- Make the integration easy to maintain and extend.

---

### User Stories

- **As a developer**, I want to decode a `ContractState` using the existing CLI logic, so I can avoid duplicating code and ensure consistency.
- **As a system integrator**, I want errors from the CLI or parsing to be reported clearly, so I can diagnose issues quickly.
- **As a tester**, I want to verify the integration works end-to-end, including subprocess invocation and output parsing.

---

### Functional Requirements

1. **Module Creation**
   - Create a new Rust module: `lib/did-midnight-sources/src/serde_cli.rs`.

2. **Function Implementation**
   - Implement an async function (suggested name: `decode_contract_state_via_cli`) with the following signature:
     ```rust
     pub async fn decode_contract_state_via_cli(
         did: &str,
         network_id: MidnightNetwork,
         contract_state: &ContractState
     ) -> Result<DidDocument, SerdeCliError>
     ```
     - Accepts `did` as a string.
     - Accepts `network_id` as `MidnightNetwork`, converted to number via `.as_u8_repr()`.
     - Accepts `contract_state` as `ContractState`, converted to hex string.
     - CLI executable path is hardcoded as `"did-midnight-serde"`.

   - The function must:
     - Build CLI argument list: `<did> <network_id> <hex>`.
     - Spawn the CLI subprocess asynchronously using `tokio::process::Command`.
     - Capture stdout (JSON-encoded `DidDocument`).
     - Parse stdout into the Rust `DidDocument` type.
     - Return the parsed document or an error.

3. **Trait Implementation**
    - In `lib/did-midnight/src/dlt.rs`, implement `ContractStateDecoder` for a type that delegates to `decode_contract_state_via_cli`.
    - The trait method must remain synchronous. If subprocess invocation is blocking, use `tokio::task::spawn_blocking` or similar to avoid blocking the main thread, but do not change the trait interface.

4. **Error Handling**
   - Define a custom error type `SerdeCliError` in `serde_cli.rs` (derive `Error` as per repo guidelines).
   - Handle:
     - Subprocess launch failures.
     - Non-zero exit codes.
     - Empty or malformed stdout.
     - JSON parsing errors.
   - Use clear, lower-case, placeholder-based error messages.

5. **Testing**
   - Add integration tests in `lib/did-midnight-sources/tests/serde_cli.rs`:
     - Test successful decoding with valid input (behind a feature flag).
     - Test error cases: CLI not found, bad output, invalid JSON, etc.
     - Use test doubles/mocks if CLI is unavailable.

---

### Non-Functional Requirements

- **Performance:** Subprocess invocation should be fast; blocking code is allowed for CLI calls, but must not block the main thread if possible (e.g., use `tokio::task::spawn_blocking`). The trait interface for `ContractStateDecoder` must remain synchronous.
- **Configurability:** CLI path is hardcoded for now.
- **Maintainability:** Code should be modular, with clear separation of concerns.
- **Logging:** Log errors using `tracing` as per repo standards.

---

### Acceptance Criteria

- [ ] `serde_cli` module exists and exposes async `decode_contract_state_via_cli`.
- [ ] Function correctly invokes CLI, parses output, and returns `DidDocument`.
- [ ] Errors are handled and reported per repo guidelines.
- [ ] `ContractStateDecoder` trait is implemented using the new function and remains synchronous (interface unchanged, may use blocking code internally).
- [ ] Tests cover success and failure scenarios, with real CLI tests behind a feature flag.
- [ ] Code passes formatting and lint checks (`cargo fmt`, etc.).

---

### Implementation Steps

1. **Module Setup**
   - Create `lib/did-midnight-sources/src/serde_cli.rs`.
   - Define `SerdeCliError` enum with variants for subprocess, output, and parsing errors.

2. **Function Implementation**
   - Implement async `decode_contract_state_via_cli`:
     - Accept `did`, `network_id`, and `contract_state` as arguments.
     - Convert `network_id` via `.as_u8_repr()`.
     - Convert `contract_state` to hex string.
     - Build CLI argument list.
     - Use `tokio::process::Command` to invoke CLI asynchronously.
     - Capture and check exit status.
     - Read stdout; parse as JSON.
     - Map errors to `SerdeCliError`.

3. **Trait Integration**
    - In `lib/did-midnight/src/dlt.rs`, implement synchronous `ContractStateDecoder` for a type that calls `decode_contract_state_via_cli`. If subprocess invocation is blocking, use `tokio::task::spawn_blocking` or similar to avoid blocking the main thread, but do not change the trait interface. Blocking code is acceptable for CLI invocation as long as the trait remains sync.

4. **Testing**
   - Add integration tests in `lib/did-midnight-sources/tests/serde_cli.rs`.
   - Test with valid and invalid inputs.
   - Real CLI tests must be behind a feature flag.
   - Mock CLI if needed.

5. **Documentation**
   - Document public APIs and error types.
   - Add usage examples in module-level docs.

---

### Edge Cases & Error Handling

- **CLI Not Found:** Return error if CLI executable is missing.
- **Non-zero Exit Code:** Return error if process fails.
- **Malformed Output:** Return error if stdout is not valid JSON.
- **Timeouts:** (Optional) Consider adding a timeout for subprocess execution.
- **Argument Conversion:** Return error if arguments cannot be converted as required.

---

### Dependencies

- Rust standard library (`tokio::process`, `serde_json`).
- Existing types: `ContractState`, `DidDocument`, `MidnightNetwork`.
- External CLI: `did-midnight-serde` (must be installed and in PATH).

---

### Out-of-Scope

- Rewriting or extending the CLI itself.
- Supporting non-JSON output formats.
- Advanced error recovery or retry logic.

---

### Example Usage

```rust
let did = "did:midnight:...";
let network_id = MidnightNetwork::Mainnet;
let contract_state = /* ... */;
let did_doc = decode_contract_state_via_cli(did, network_id, &contract_state).await?; // under the hood may offload blocking code to tokio blocking tasks
```

---

### Self-Review Checklist

- [x] All requirements are actionable and testable.
- [x] Error handling aligns with repo guidelines.
- [x] Steps are broken down for junior devs/AI agents.
- [x] Async and feature flag requirements are included.
- [x] Formatting and structure follow AGENTS.md standards.
