# Prism UI checks

Run all commands inside `nix develop`. The browser suite requires `agent-browser`
and Chromium on `PATH`; it chooses a named session scoped to the current worktree
and closes that session afterward. It does not use production data.

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
adjacent labelled readonly textarea if clipboard writing fails. Flex parents
should wrap; grid parents should let `.prism-copy-fallback` span their columns.

Run the standalone copy browser fixtures:

```sh
PRISM_TEST_SESSION=prism-copy-check python tools/ui-tests/prism_copy.py
```

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
