# Combined Prism validation record (#381)

## Scope and status

The integration starts at `c19f399fb609ef1ab6b1f5930b3b59ff9d617545`, after
tickets #372–380 were merged, verified and closed. Visual authority remains
[PRISM-SPEC](../../designs/PRISM-SPEC.md), the editable Prism design and its pinned
previews. [Shipped system and interaction contracts](../../docs/src/references/prism-ui.md)
record tokens, responsive composition, safety boundaries and deliberate deviations.

**Independent rendered review and two-axis Standards/Spec code review remain
parent-owned and pending.** This record does not mark #381 complete. The exact
final implementation SHA, portable receipt names and external evidence location
are recorded in the implementer's handoff; reviewers must inspect the actual
merged diff and rendered crops, not treat this implementation record as approval.

## Approved seams and integration findings

- Rust HTTP/rendered HTML: actual routing, safe diagnostics, full captured data,
  mode-dependent availability, Scalar isolation and native GET markup. This does
  not establish visual fidelity.
- Browser workflow: actual HTML and local assets; deterministic test-only HTTP,
  time, visibility and clipboard boundaries for rare states. This does not
  establish backend/indexer correctness or OS/browser interoperability.
- Asset distribution: source-byte preservation, required files, original SVG
  hash and local delivery in development and Nix. This is not a browser test.

No production fixture route/data is introduced. Five opt-in Rust exporters plus
the ordinary cursor exporter generate fresh combined-branch fixtures outside the
checkout. Every earlier feature already had its own tests; these are integration
checks, not deferred feature implementation.

Test-first fixes and additional coverage:

1. **Copy DID** visibly wrapped into four lines inside a 48px square. The new
   browser test failed at 320/390/720/1024/1440px before a scoped identity-action
   width/padding correction. It now fits one line, remains 48px high, stacks on
   mobile and leaves square icon/Explorer-row targets unchanged. Existing exact
   copy/fallback/expanded-content tests still execute.
2. Regional rendered inspection found cursor **Copy** labels also wrapping in
   square targets. A new browser test failed at 320/390/801/1440px before those
   controls reused the existing copy SVG, matching the pinned expanded sync
   specimen. Accessible names, exact original values and 48 × 48px targets stay
   intact. Native clipboard success and denied-write fallback are retested.
3. The identity pending-geometry test still prevented the old native submit
   event, allowing enhanced GET to finish with a fixture-server 404. The full
   combined suite exposed this stale assumption. It now holds the fetch system
   boundary; the outcome suite independently holds real HTTP requests.
4. Dedicated malformed HTML-200 and **actual 30-second timeout** tests force
   resolver recovery. Both existing guards passed without production changes:
   controls/pending state reset, attempted input and labelled previous history
   survive, private response text is absent, URLs stay truthful and exact retry
   succeeds. The timeout is real elapsed time, not a shortened application timer.
5. Shell checks add whole-page axe/landmarks/focus, 200% CSS layout magnification
   and opaque-base capability fallback. Refresh fixtures accept an owned port so
   combined runs do not commandeer another test server.

## Reproduce from a checkout

Enter `nix develop` first. All following commands run inside that shell. Keep
Cargo artifacts on a filesystem with sufficient space; fixtures are small and
may use a temporary directory. Never put a Cargo target on a small fixture/cache
filesystem. Use a unique validation ID, ports and named browser sessions.

```sh
export VALIDATION_ID=prism-check
export CARGO_TARGET_DIR="$(git rev-parse --path-format=absolute --git-common-dir)/prism-build-cache/$VALIDATION_ID"
export CARGO_BUILD_JOBS=3
export FIXTURES="$(mktemp -d)"
export PRISM_FIXTURE_DIR="$FIXTURES/identity"
export PRISM_VDR_FIXTURES="$FIXTURES/vdr"
export PRISM_OPERATION_FIXTURES="$FIXTURES/operations"
export PRISM_OUTCOME_FIXTURES="$FIXTURES/outcomes"
export PRISM_CURSOR_FIXTURES="$FIXTURES/cursor"

bun install --frozen-lockfile
just build-assets
cargo test --workspace --all-features
cargo test -p neoprism-node --all-features export_identity_browser_fixtures -- --ignored
cargo test -p neoprism-node --all-features export_vdr_browser_fixtures -- --ignored
cargo test -p neoprism-node --all-features export_operation_browser_fixtures -- --ignored
cargo test -p neoprism-node --all-features export_outcome_browser_fixtures -- --ignored
PRISM_FIXTURE_DIR="$FIXTURES/explorer" cargo test -p neoprism-node --all-features export_explorer_browser_fixtures -- --ignored
# The cursor exporter executes in the ordinary all-feature node/workspace suite.
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
cargo build -p neoprism-node --all-features
python tools/verify_ui_assets.py bin/neoprism-node/assets
```

Start an actual empty development node (not a fixture server):

```sh
"$CARGO_TARGET_DIR/debug/neoprism-node" dev --db-url sqlite::memory: --port 18772
```

In another development shell, retain the exported fixture variables. The following
ports are examples: choose free owned ports when other agents/tests are active.
Run **serially**: native clipboard is OS-global even across isolated sessions.

```sh
export PRISM_TEST_URL=http://127.0.0.1:18772
export PRISM_IDENTITY_PORT=18773 PRISM_VDR_PORT=18775
export PRISM_OPERATION_PORT=18776 PRISM_OUTCOME_PORT=18777
export PRISM_CURSOR_PORT=18779 PRISM_REFRESH_PORT=18880
PRISM_TEST_SESSION="$VALIDATION_ID-shell" python tools/ui-tests/prism_shell.py -v
PRISM_TEST_SESSION="$VALIDATION_ID-copy" python tools/ui-tests/prism_copy.py -v
PRISM_TEST_SESSION="$VALIDATION_ID-identity" python tools/ui-tests/prism_identity.py -v
PRISM_TEST_SESSION="$VALIDATION_ID-vdr" python tools/ui-tests/prism_vdr.py -v
PRISM_TEST_SESSION="$VALIDATION_ID-operations" python tools/ui-tests/prism_operations.py -v
PRISM_TEST_SESSION="$VALIDATION_ID-outcomes" python tools/ui-tests/prism_outcomes.py -v
export PRISM_FIXTURE_DIR="$FIXTURES/explorer"
PRISM_TEST_SESSION="$VALIDATION_ID-refresh" python tools/ui-tests/prism_refresh.py -v
PRISM_TEST_SESSION="$VALIDATION_ID-explorer" python tools/ui-tests/prism_explorer.py -v
PRISM_TEST_SESSION="$VALIDATION_ID-cursor" python tools/ui-tests/prism_cursor.py -v
```

`clipboard_permissions.py` requires Node 24 and grants only the owned loopback
browser session's fixture origin. Its WebSocket stays alive during native
write/read checks and closes afterward. No global permission, application
clipboard bypass or shared/user browser is used. Failure/missing-access checks
stub the clipboard boundary; they do not certify OS permission-dialog behavior.

For Nix delivery, verify bytes and start a second node with its asset directory;
run the same shell suite against that node's owned port:

```sh
ASSETS="$(nix build .#neoprism-ui-assets --no-link --print-out-paths)/assets"
python tools/verify_ui_assets.py "$ASSETS"
"$CARGO_TARGET_DIR/debug/neoprism-node" dev --db-url sqlite::memory: \
  --port 18782 --assets-path "$ASSETS"
# In another shell:
PRISM_TEST_URL=http://127.0.0.1:18782 PRISM_TEST_SESSION="$VALIDATION_ID-nix" \
  python tools/ui-tests/prism_shell.py -v
```

Stop only the servers you started. Browser suites close their owned sessions.
[README](README.md), [cursor checks](PRISM-CURSOR.md) and
[refresh checks](PRISM-REFRESH.md) describe individual fixture boundaries.

## Executed checks

| Check | Integration result |
| --- | --- |
| Workspace tests, all features | Passed; 106 node tests, five opt-in exporters separately executed |
| Workspace Clippy, all targets/all features | Passed with `-D warnings` |
| Rustfmt | Passed |
| Shell / shared copy / identity | 8 / 5 / 12 passed |
| VDR / operations / outcomes | 4 / 5 / 13 passed |
| Refresh / Explorer / cursor | 15 / 5 / 7 passed |
| All nine browser suites | 74 passed, zero skips |
| Nix-delivered shell | Eight additional browser checks passed |
| Development + Nix assets | Required scripts/fonts/logo/licenses preserved; byte checks passed |
| Mode/Scalar preservation | HTTP tests passed; generated API sources unchanged from review baseline |
| Changed-file Ruff, Markdownlint, EditorConfig | Passed |
| Repository-wide `just lint-text` | Existing vendored `.pi` failures; see below |
| Final mechanical design detector | One run, 29 complete changed text/SVG UI targets; raw `[]`, exit 0 |
| Independent rendered + Standards/Spec review | Pending, parent-owned |

The detector target manifest includes changed shared shell, Resolver/Explorer
renderers and tests, Tailwind entry, SVG, CSS and all four browser scripts.
Binary fonts/licenses are verified by asset delivery instead. No rules were
disabled and no pinned-design warnings were suppressed: this run emitted no
findings. That mechanical result is not a visual review or accessibility proof.

The full text-lint command reports **261 Markdown findings in 18 vendored skill
files and 28 EditorConfig errors**, all under `.pi/skills/impeccable`. YAML and
shell checks pass. Those pre-existing failures are separate from changed-file
checks; skills/config exclusions were not edited or suppressed.

## Rendered and accessibility evidence

Evidence is external, under the handoff's `381-evidence` directory, not shipped
as UI content. `renders/capture-index.json`, `references/crop-index.json` and the
handoff crop index identify original captures, exact regions and pinned sources.

- Actual 1440/390px entry, identity, expanded keys/contexts, VDR, operations,
  Explorer list/pagination and expanded sync pages are captured. Additional
  320px long VDR/pending/failure and 801px identity captures are retained.
- `renders/*-identity-panel.png`, `*-entry-panel.png`, `*-key.png`, `*-context.png`,
  `*-vdr.png`, `*-operation.png`, `*-sync.png`, `*-row.png` and `*-pagination.png`
  are actual region renders, not enlarged thumbnails. Corresponding pinned
  reference crops are retained in `references/` and were inspected directly.
- Entry shell, typography, illumination, 20px mobile inset, data surfaces, row
  density and copy/action geometry track the pinned world. Exact real content
  changes heights: long-form input, multiple curves/services and full payloads
  exceed the static illustrative examples. The single-line native DID field
  scrolls horizontally rather than inserting mockup line breaks. Identifiers
  remain contiguous selectable monospace text rather than splitting method and
  suffix into separately styled blocks. Resolve has a stable 132px minimum to
  accommodate the pending label. Native disclosures keep browser markers.
- Cursor **snapshot supplied** is intentionally not a green proof-of-freshness
  badge. Receipt text and explicit native refresh/retry controls add truthful
  delivery state and document height absent from static specimens. API-result
  notices/mockup footers/sample telemetry are deliberately omitted.
- Scripted keyboard checks cover navigation/menus/Escape focus return, labelled
  disclosures, separate copy/link targets, manual fallback, pending/recovery,
  exact GET retry and no focus theft. Fresh script-blocked contexts cover native
  GET navigation, disclosure access and recovery. IME events are synthetic only.
- Expanded layouts test 320/390px, intermediate widths around 600/800/801px,
  1024/1280/1440px and long data. CSS layout magnification is 200%; native browser
  chrome zoom UI and physical-device text scaling were not certified.
- Whole entry page and scoped feature axe 4.12.1 audits report zero violations;
  manual rendered-region inspection complements them. Focus is a visible 2px
  external ring. Visible controls are at least 48px high; independent row/icon
  actions retain 48px squares. Status announcements are polite and do not steal
  focus; reading order follows document order.
- Computed representative contrast ratios: body/panel 15.46:1, secondary/panel
  7.32:1, white/primary 5.13:1, secondary action/panel 6.17:1. These are samples,
  not a claim that every possible browser/data/assistive-technology state was
  manually certified. Feature axe audits additionally check displayed contrast.
- Reduced motion is tested. Owned CDP emulation of reduced transparency/motion
  verifies opaque shell/sidebar, no backdrop filter, no text filter and zero
  transition duration (`preferences-contrast.json`). The shell suite also drops
  the enhancement `@supports` block at the stylesheet capability boundary to
  exercise shipped opaque defaults, not a different engine's implementation.
- `pre-fix/cursor-wrapped-copy-390.png` and red/green logs preserve the integration
  regression evidence. Final renders show the corrected icon targets.

## Honest browser / assistive-technology matrix

| Environment | Executed / limitation |
| --- | --- |
| Installed Linux Chromium 157.0.8103.38 | Automated workflows, native clipboard round trips, axe, keyboard, rendered regions and preference emulation executed |
| Stable Chrome / Edge | Not executed; validation-time warning reported stable Chrome 158.0.8127.36 ahead of installed Chromium; no branded Chrome/Edge certification |
| Firefox / Gecko | Not available, not executed |
| Safari / WebKit on macOS | Not available, not executed |
| Mobile Safari / physical iOS or Android | Not available, not executed; responsive desktop emulation only |
| Real screen reader / platform AT | Not available, not executed; no NVDA/JAWS/VoiceOver/TalkBack certification |
| OS IME / OS clipboard permission dialogs | Not executed; synthetic composition and deterministic denial/missing-access boundaries only |
| Native zoom UI / OS transparency preference | Not certified; CSS layout magnification and owned CDP preference emulation only |

No system packages were installed and no shared browser was updated. WCAG 2.2 AA
is the validation target, **not a conformance certification** from axe plus
keyboard checks. Outstanding browser/AT checks and parent reviews remain explicit
release/review limitations; they must not be silently converted into passes.
