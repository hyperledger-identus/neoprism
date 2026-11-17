# Prism UI checks

Run all commands inside `nix develop`. The browser suite requires `agent-browser`
and Chromium on `PATH`; it chooses a named session scoped to the current worktree
and closes that session afterward. It does not use production data.

Clipboard integration checks also require Node 24 (provided by the dev shell).
The harness obtains only its owned session's loopback CDP WebSocket and grants
`clipboardReadWrite`/`clipboardSanitizedWrite` only to the fixture server origin.
It keeps that connection open during the test: Chromium revokes these overrides
on CDP disconnect. Closing the test connection/browser removes the permission;
no application code or global browser configuration changes. Permission failures
fail tests rather than skip them. Do not point these suites at a shared/user
browser. Run native clipboard suites serially to avoid system clipboard races.

```sh
just build-assets
cargo test -p neoprism-node
cargo build -p neoprism-node
target/debug/neoprism-node dev --db-url sqlite::memory: --port 18772
```

In another development shell:

```sh
python tools/ui-tests/prism_shell.py
python tools/verify_ui_assets.py bin/neoprism-node/assets
nix build .#neoprism-ui-assets --no-link
```

`PRISM_TEST_URL` selects another running node (default `http://127.0.0.1:18772`).
`PRISM_TEST_SESSION` optionally overrides the isolated browser session name.
To check Nix delivery in the browser, start the node with
`--assets-path "$(nix build .#neoprism-ui-assets --no-link --print-out-paths)/assets"`
and run the same suite against it.

## Explorer and shared copy

The reusable copy contract is a `type="button"` element with `class="prism-copy"`,
`data-copy-value` containing the complete original and a descriptive `aria-label`.
Keep the displayed value selectable. `copy.js` delegates events (including later
HTMX content), announces polite feedback without moving focus, and inserts an
adjacent labelled readonly textarea if clipboard writing fails. The enhancement
marks its parent `.prism-copy-container` to wrap flex children; the fallback spans
grid columns. This also supports constrained cursor and identity consumers.

Run the standalone copy browser fixtures:

```sh
PRISM_TEST_SESSION=prism-copy-check python tools/ui-tests/prism_copy.py
```

The replacement-content case verifies a real native write/read round trip.
Special-character/newline exact writes and missing/denied access are tested at
the clipboard boundary stub. Explorer's row test also uses an exact-write stub;
identity and cursor success tests use real native clipboard write/read.

Explorer checks export actual HTTP-rendered responses from deterministic SQLite
fixtures, then serve them and built assets on an ephemeral loopback port. No
production database or sample-data route is used:

```sh
export PRISM_FIXTURE_DIR="$(mktemp -d)"
cargo test -p neoprism-node export_explorer_browser_fixtures -- --ignored
just build-assets
PRISM_TEST_SESSION=prism-explorer-check python tools/ui-tests/prism_explorer.py
```

Set `PRISM_SCREENSHOT_DIR` to an existing external directory to save full-page
390px/1440px renders of all 100 rows and empty/invalid/out-of-range/failure states.
The browser suite blocks the HTMX CDN for deterministic rendering checks; polling
and stale-data recovery are the separate #380 seam. Tests cover 57px rows,
independent 48px actions, exact originals, keyboard/fallback behavior, 320–1440px
list bounds, centered equal-width pagination and fresh script-blocked native GET
navigation. Without copy enhancement narrow labels expose horizontally selectable
complete originals rather than only visual abbreviations.

Rust HTTP tests also cover storage ordering, the final partial page, normalized
zero, invalid/duplicate/overflowing queries, retrieval failures without leaked
infrastructure errors, and page-query retention on fragment refreshes.

## Resolver identity fixtures

The identity suite uses the real rendered Resolver page with deterministic test
fixtures, not a production fixture endpoint. It serves built assets and exported
HTML on loopback port 18773 (`PRISM_IDENTITY_PORT` overrides it).

```sh
export PRISM_FIXTURE_DIR="$(mktemp -d)"
cargo test -p neoprism-node export_identity_browser_fixtures -- --ignored
just build-assets
python tools/ui-tests/prism_identity.py
```

Checks include all three key curves, full-value copy controls, native keyboard
and no-script disclosures, stable pending controls, and expanded-content bounds
at 320–1440 CSS pixels. Shared `copy.js` is required: copy tests no longer skip
when it is absent. Success checks read native clipboard values for the complete
DID, key coordinate, transaction ID and operation ID. Failure uses a denied-write
boundary stub and checks the adjacent selectable fallback without moving focus,
including the mobile column action layout.
Rendered-HTML tests additionally cover backend publication/deactivation, all
usage enums, real counts, and selecting one latest applied operation for evidence
without claiming a chain tip or exposing synthetic local metadata.

The same fixture exporter/suite also covers ordered contexts and services with
single/multiple types, URI/object/array endpoints, nested/null/boolean values,
long identifiers and URLs, plus compact empty collections. Clipboard boundary
stubs verify exact captured fields, full typed Debug text, readable parsed-state
JSON, denied/missing writes, polite feedback and keyboard-selectable fallbacks.
The full typed text is captured independently during fixture setup. Readable JSON
is explicitly not a reconstruction of authored wire formatting; captured fields
retain their spacing/order even when the readable projection changes them.
Native script-blocked disclosures and expanded 320–1440px bounds are checked.

## Resolver operation fixtures

```sh
export PRISM_OPERATION_FIXTURES="$(mktemp -d)"
cargo test -p neoprism-node export_operation_browser_fixtures -- --ignored
just build-assets
PRISM_TEST_SESSION=prism-operations-check python tools/ui-tests/prism_operations.py -v
```

`PRISM_OPERATION_PORT` overrides private loopback port 18776. The suite verifies
applied/rejected disclosures, real protocol failure to initialize final state,
explicit empty history, 320–1440px expanded-content bounds, scoped axe checks,
keyboard/no-script disclosures, and exact complete Debug inspection/copy/manual
fallback. Clipboard success and denial use boundary stubs, not the OS-global
clipboard, so this suite can run concurrently with the native clipboard suites.
`PRISM_SCREENSHOT_DIR` optionally saves 390px/1440px page renders.

The Rust rendered-HTML tests cover every metadata field, HTML-200 masking and
full server-side logging of unexpected internal causes, and safe nested size,
crypto/protobuf, service and state-conflict diagnostics. Client-safe projection
lives in `ui_resolver/diagnostics.rs`; never replace it with a full error-chain
formatter. Full signed payloads intentionally retain all captured Debug fields,
including protobuf fields; they are not reconstructed authored wire bytes.

## Explorer cursor fixtures

```sh
export PRISM_CURSOR_FIXTURES="$(mktemp -d)"
cargo test -p neoprism-node export_cursor_browser_fixtures
just build-assets
PRISM_TEST_SESSION=prism-cursor-check python tools/ui-tests/prism_cursor.py -v
```

`PRISM_CURSOR_PORT` overrides loopback port 18779. The previously deferred copy
case must execute: it reads complete slot, hash (including 1024 characters) and
nanosecond UTC timestamp values from the native clipboard, checks focus, and
uses a denied-write boundary stub for exact readonly fallback bounds at 320px.
The suite also covers native/no-script disclosures, responsive evidence bounds,
scoped accessibility audits and observable receipt/request states. It blocks
HTMX only during the copy case; polling interaction preservation remains #380.

## Resolver VDR fixtures

The VDR suite exports actual Resolver rendering with deterministic typed storage
fixtures (Bytes, IPFS and uninterpreted numeric StatusList). Normal, long-content
and empty pages exist only in test output; no production sample route is added.
Hashes use the complete 64-character SHA-256 values supported by the backend.

```sh
export CARGO_TARGET_DIR="$(git rev-parse --path-format=absolute --git-common-dir)/prism-build-cache/375"
export CARGO_BUILD_JOBS=2
export PRISM_VDR_FIXTURES="$(mktemp -d)"
cargo test -p neoprism-node vdr_tests
cargo test -p neoprism-node export_vdr_browser_fixtures -- --ignored
bun install --frozen-lockfile
just build-assets
PRISM_TEST_SESSION=prism-vdr-375 python tools/ui-tests/prism_vdr.py -v
```

`PRISM_VDR_PORT` overrides private loopback port 18775. Browser checks cover
320–1440px expanded bounds, exact typed/raw copy writes and denied-write manual
fallbacks without lost focus, native keyboard/script-blocked disclosures,
selectable captured Debug text and scoped accessibility audits. Clipboard checks
use deterministic boundary stubs, not the OS-global clipboard. Rust checks cover
all typed shapes, numeric extremes, escaping, empty typed fields, ordering and a
real native GET resolution with compact empty VDR state. Raw data is the complete
captured Debug representation, explicitly not a reconstruction of wire bytes.

## Agreed seams

- Rust HTTP tests cover root redirects, mode-dependent destinations, runtime
  metadata, escaped retained query values, real long-form resolution handoff,
  native GET markup, Scalar isolation, and local asset content types.
- Browser checks cover required validation, Enter/GET submission, synthetic IME
  composition events without value rewriting, pending feedback and back/forward
  recovery, keyboard menu navigation and Escape focus restoration, touch targets,
  responsive bounds at 320–1440 CSS pixels, local fonts, and reduced motion.
  A fresh script-blocked browser context checks native navigation and long-form
  submission without application scripts. The long-form fixture comes from the
  repository README, and is never displayed as production entry data.
- Asset verification compares delivered files with source bytes and checks the
  original logo hash. Nix runs this verification as an install check.

These checks do not certify visual fidelity, assistive technology, OS-level IME,
other browser engines, or mobile Safari. The approved composition remains in
`designs/neoprism.fig` and `designs/PRISM-SPEC.md`. Shared styles live in
`bin/neoprism-node/ui-assets/prism.css`; copy that entire directory when packaging
runtime assets, rather than only the generated Tailwind stylesheet.

## Resolver outcomes and recovery (#377)

`prism_outcomes.py` serves exported Maud pages at a private HTTP boundary. It
covers invalid syntax/initial state, node-local absence, unusable stored
operations, genuine retrieval failure, locally derived state, published
long-form input and deactivation. No fixture route or sample data is added to
the node. Publication/deactivation fixtures describe captured state at the
rendering seam; real SQLite HTTP tests additionally distinguish empty storage
from retrieval failure.

Run inside `nix develop`:

```sh
export CARGO_TARGET_DIR="$(git rev-parse --path-format=absolute --git-common-dir)/prism-build-cache/377"
export CARGO_BUILD_JOBS=3
export PRISM_OUTCOME_FIXTURES="$HOME/.cache/neoprism-prism-implementation/377-browser-fixtures"
export PRISM_TEST_SESSION=prism-outcomes-377
export PRISM_OUTCOME_PORT=18777
cargo test -p neoprism-node
cargo test -p neoprism-node export_outcome_browser_fixtures -- --ignored
bun install --frozen-lockfile
just build-assets
python tools/ui-tests/prism_outcomes.py -v
```

Enhancement fetches the existing ordinary GET page, swaps only the result and
updates the bookmarkable URL. It never changes API or resolution semantics.
Pending skeletons exist only while that HTTP request is active. A 30-second
request bound, network/HTTP/invalid-response recovery and bfcache restoration
recover controls. Failed delivery preserves previous state/disclosures and labels
them as the previous request, not evidence from the failed attempt. Retry links
carry the entire submitted value; correction and Explorer links also work
without scripts. Without JavaScript, native browser Back restores the entered
value after network/HTTP delivery failure, followed by an ordinary GET retry.

Browser checks cover every outcome with and without scripts, real delayed GET
and repeat protection, HTTP/network failure and retry, keyboard/focus/value
retention, back/forward URLs, desktop evidence expansion, shared copy after a
result swap, expanded bounds at 320–1440px and scoped accessibility audits.
Clipboard uses a deterministic system-boundary stub. Set
`PRISM_SCREENSHOT_DIR` to capture outcome pages at 390/1440px for rendered review.
No browser suite claims real assistive-technology or Safari certification.
