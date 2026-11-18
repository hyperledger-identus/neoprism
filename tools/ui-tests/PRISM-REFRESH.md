# Explorer refresh delivery checks

Run in `nix develop` from your worktree. These tests use the approved browser
workflow and real HTTP-rendered HTML seams, not production fixture routes.

```sh
export CARGO_TARGET_DIR="$(git rev-parse --path-format=absolute --git-common-dir)/../target-prism/ui-refresh"
export CARGO_BUILD_JOBS=2
export PRISM_FIXTURE_DIR="$(mktemp -d)"
export PRISM_CURSOR_FIXTURES="$(mktemp -d)"
cargo test -p neoprism-node --all-features
cargo test -p neoprism-node export_explorer_browser_fixtures -- --ignored
cargo test -p neoprism-node export_cursor_browser_fixtures
just build-assets
PRISM_TEST_SESSION=prism-380-refresh python tools/ui-tests/prism_refresh.py -v
```

`prism_refresh.py` owns loopback port 18880 by default (`PRISM_REFRESH_PORT`
overrides it) and closes its named browser/server. Use a unique session and port
when adapting this suite for concurrent work. See [combined validation](PRISM-VALIDATION.md)
for final integration evidence and the explicit unexecuted browser/AT matrix.
Fixtures are generated through the actual Explorer document/fragment routes,
backed by deterministic SQLite records and cursor watch snapshots. Responses
contain no production data. A test-only HTTP server delivers those exact HTML
responses and local built assets; it returns actual HTTP 503s, closes sockets
for network failures, and can hold a response for timeout checks.

The test document installs a deterministic Date/timer boundary before application
scripts. The fetch boundary still calls real HTTP; it can delay delivery and
ignore cancellation deliberately to exercise ordering guards. Visibility tests
control the document visibility API and dispatch its native notification, not
obsolete HTMX lifecycle events. Clipboard denial is a boundary stub; native
positive clipboard checks remain in `prism_cursor.py` and `prism_copy.py`.
`prism_explorer.py` checks exact row writes through the clipboard boundary stub.

Coverage includes:

- Independent 2-second cursor and 5-second selected-page list cadences.
- No phantom initial loading; real, independently pending requests and retries.
- Hidden suspension, immediate return, repeated visibility changes, one in-flight
  request per resource, cancellation/timeout, stale response exclusion and native
  pagination winning over a pending old request.
- Repeated HTTP/network failures, independent browser receipt ages and retry
  cadences, retained last successful evidence, recovery and safe public feedback.
- Missing cursor versus confirmed empty list versus out-of-range page versus
  failed requests, including first success deferred by interaction then failure.
- Expanded native disclosures, focused controls, DOM text selection, receipt
  selection, readonly clipboard fallback selection and keyed DID-row retention.
  Changed evidence waits until focus/selection leave; only the newest received
  snapshot is staged. Receipt/status feedback states explicitly when displayed
  values have not yet changed.
- Script-blocked native GET pagination, deliberate refresh and retry links;
  responsive retry/feedback bounds at 320–1440px and scoped accessibility audits.

Delivery is owned by `ui-assets/explorer.js` only. Explorer no longer arms HTMX
outerHTML polling or loads the old `cursor.js`. HTMX remains available to other
features. Resource roots and feedback controls are stable; changed evidence is
reconciled in place only outside active interaction. UI scheduling does not
change indexing, storage, cursor semantics, or generated Scalar pages.

These tests do not certify visual fidelity, assistive technology, other browser
engines, mobile Safari, or upstream/source health. A successful browser receipt
never means the cursor advanced or the node reached the chain tip.
