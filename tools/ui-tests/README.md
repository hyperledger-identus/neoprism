# Prism shell checks

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
at 320–1440 CSS pixels. Copy success/failure checks require the shared `copy.js`
from #378; until integrated, they explicitly skip rather than duplicate it.
Rendered-HTML tests additionally cover backend publication/deactivation, all
usage enums, real counts, and selecting one latest applied operation for evidence
without claiming a chain tip or exposing synthetic local metadata.

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
