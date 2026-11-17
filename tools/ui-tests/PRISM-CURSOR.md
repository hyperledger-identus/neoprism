# Cursor evidence checks (#379)

Run from the ticket worktree inside `nix develop`. The seams are real HTTP
responses/rendered HTML and browser workflows, not upstream-indexer health.

```sh
export CARGO_TARGET_DIR="$(git rev-parse --path-format=absolute --git-common-dir)/prism-build-cache/ui-cursor"
export CARGO_BUILD_JOBS=3
export PRISM_CURSOR_FIXTURES="$(mktemp -d)"
bun install --frozen-lockfile
just build-assets
cargo test -p neoprism-node cursor_tests
python tools/ui-tests/prism_cursor.py
```

The tests export deterministic full-page and fragment fixtures through the real
Explorer HTTP routes. The fixture-only Python server binds `127.0.0.1:18779`
(`PRISM_CURSOR_PORT` overrides it), and uses an isolated worktree browser session.
No fixture data or test endpoints ship in the application. `--serve` serves the
fixtures for manual review; open `/supplied.html`, `/missing-timestamp.html`,
`/waiting.html` or `/long.html`.

Coverage: reported slot and optional UTC timestamp, exact raw values including
fractional seconds and a 1024-character hash, native disclosure keyboard behavior,
script-blocked selectable evidence, 320–1440 px cursor bounds and target sizes,
receipt-versus-block-time semantics, repeated identical snapshots, and observable
pending/failed HTTP requests retaining the last received cursor.

Shared copy controls use #378's `data-copy-value` enhancement, not a second
clipboard implementation. Cursor request feedback does not infer upstream
freshness, source failure, chain-tip distance or PRISM-operation indexing.
Existing successful-response cursor polling stays at two seconds; polling
recovery and interaction preservation are #380's separate responsibility.

Square copy targets use the existing copy SVG with descriptive accessible names;
visible labels must not wrap inside 48px icon targets. Native success checks
retain exact originals and denied-write fallbacks without focus loss.

See [combined validation](PRISM-VALIDATION.md) for final integration evidence.
These checks do not certify visual fidelity, screen readers, other browser
engines or native mobile clipboard behavior.
