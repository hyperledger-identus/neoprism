# Combined Prism validation record (#381)

## Scope and status

The integration starts at `c19f399fb609ef1ab6b1f5930b3b59ff9d617545`, after
tickets #372–380 were merged, verified and closed. Visual authority remains
[PRISM-SPEC](../../designs/PRISM-SPEC.md), the editable Prism design and its pinned
previews. [Shipped system and interaction contracts](../../docs/src/references/prism-ui.md)
record tokens, responsive composition, safety boundaries and deliberate deviations.

**The three independent reviews at integration `0309ec3` are complete.** Standards
reported one documented Pydantic breach and seven duplication findings; Spec
reported one lossless-copy defect; rendered acceptance reported no material
visual blocker. Their recovered summaries are in
`.git/prism-run-artifacts/repair-context.md`; the earlier raw reports and images
were lost with ephemeral session storage. They are not newly executed reviews.

This replacement resumes the interrupted single repair pass, not another review
round. All nine findings are addressed below. Parent verification and ticket
closure remain separate: this record does not itself close #381. Durable repair
logs, fixtures, renders, detector receipts, bundle and exact-SHA handoff live in
`.git/prism-run-artifacts/review-repair/` in the common repository, not HOME/tmp.

### Nine finding dispositions

| Finding | Repair |
| --- | --- |
| Spec: captured CRLF/bare CR altered by HTML and textarea normalization | Central conditional JSON transport/decode; faithful text-node fallback with native full-selection copy |
| Standards: unchecked external CLI envelope | Strict Pydantic envelope and failing-before-fix validation checks |
| S1: duplicate browser/evaluate adapter | Shared session-parameterized `browser_adapter.Browser` |
| S2: repeated fixture server lifecycle | Shared `fixture_server` context manager; specialized failure handlers stay local |
| S3: repeated published-history recovery checks | Focused setup/assertion helpers; HTTP, malformed-200 and real timeout mechanisms remain explicit |
| S4: repeated long-form identity fixture | Shared HTTP test-only constant/constructor |
| S5: repeated section splitting | Shared test-only section helper with descriptive failures |
| S6: repeated HTTP oneshot/body decoding | Reuse existing HTTP `get`; status assertions remain local |
| S7: repeated chevron SVG | Shared render helper with identical visual markup |

The copy regression first failed on real captured valid JSON containing CRLF,
bare CR, Unicode and markup characters. Transport then passed before a separate
manual-fallback regression exposed textarea normalization. Final tests compare
native clipboard, fallback text, selection and native keyboard copy against the
known original literal, including missing and denied async clipboard access.
No existing assertions were weakened and no browser checks were skipped.

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
| Rustfmt / changed Python Ruff / Pydantic boundary tests | Passed; two envelope tests include invalid response subcases |
| Shell / shared copy / identity | 8 / 5 / 14 passed |
| VDR / operations / outcomes | 4 / 5 / 13 passed |
| Refresh / Explorer / cursor | 15 / 5 / 7 passed |
| All nine browser suites | 76 passed, zero skips; fresh fixtures, serial native clipboard suites |
| Nix-delivered shell | Eight additional browser checks passed |
| Development + Nix assets | Required scripts/fonts/logo/licenses preserved; byte checks passed |
| Mode/Scalar preservation | HTTP tests passed; generated API sources unchanged from review baseline |
| Changed-file Ruff, Markdownlint, EditorConfig | Passed |
| Repository-wide `just lint-text` | Existing vendored `.pi` failures; see below |
| Final mechanical design detector | One authorized post-repair run, 30 changed text/SVG UI targets; raw `[]`, exit 0 |
| Independent rendered + Standards/Spec review | Completed at `0309ec3`; recovered findings repaired, parent merge verification remains |

The detector target manifest includes changed shared shell, Resolver/Explorer
renderers and tests, Tailwind entry, SVG, CSS and all four browser scripts.
Binary fonts/licenses are verified by asset delivery instead. The prior
pre-repair #381 detector returned `[]`, according to the recovered parent
receipt, but its raw artifact was lost; it did not verify the repaired bytes.
The one authorized final scan ran after all copy/icon changes. Its exact targets
and hashes, command, raw output and exit code are preserved as
`detector-{targets,command,raw,exit}.json` in the durable repair directory.
No rules were disabled or suppressed. Neither scan is a visual review or
accessibility proof.

The full text-lint command reports **261 Markdown findings in 18 vendored skill
files and 28 EditorConfig errors**, all under `.pi/skills/impeccable`. YAML and
shell checks pass. Those pre-existing failures are separate from changed-file
checks; skills/config exclusions were not edited or suppressed.

## Rendered and accessibility evidence

Evidence provenance matters: the earlier independent rendered review inspected
1440/390px entry, identity/actions/keys/evidence/services, VDR/operations,
Explorer/pagination, expanded Sync, 320px long/pending/failure, copy fallback and
opaque/recovery states. It found no blocking/material visual issue. Those raw
images were lost with ephemeral HOME storage; the recovered report is not a
claim that they remain available.

Recovered independent contrast sampling used the darkest background pixel in
each whole label rectangle and WCAG sRGB math. Explorer heading contrast was
11.12/10.43 at 1440/390px; intro 5.25/4.94; top page count 5.12/4.67; bottom count
5.37/5.02; list note 5.34/5.03. Mobile resolver guidance/label samples were at
least 4.59. These are sampled normal-text AA results, not universal certification.
Axe's gradient-backed contrast rule remains incomplete, not an automated pass.

Fresh **post-repair** receipts are retained under
`.git/prism-run-artifacts/review-repair/repair-evidence/`:

- `capture-index.json` identifies actual pages, selectors, widths and filenames.
  Focused faithful fallback is captured at 320/390/1440px, with raw text,
  readonly role/name, outline and overflow proofs plus accessibility snapshots.
- `fallback-*-axe.json` retains scoped service-region audits: zero violations.
  This does not certify browser/AT combinations or all gradients.
- `{key,context,evidence}-icon-{390,1440}.png` shows the extracted chevrons in
  their actual key, context and evidence regions. Shared helper markup is
  unchanged. Fallback wrapping, focus ring and icon appearance were inspected
  at native capture widths; no cosmetic redesign was made.
- `logs/copy-transport-{red,green}.log` and
  `logs/copy-fallback-{red,green2}.log` retain the two failing-before-fix
  regressions and successful reruns. An intermediate green-attempt log records
  a test-script global-const redeclaration error, subsequently corrected without
  weakening any assertion. Final full-suite logs are authoritative.
- `validate.sh`, all nine `logs/browser-*.log`, fresh exporters, workspace,
  Clippy, format, envelope validation and development/Nix asset receipts are
  durable. Five opt-in exporters still carry intentional Rust ignore markers
  and are explicitly executed; browser suites have zero skips.

The existing workflow suites cover native GET/no-script disclosures, exact
values, keyboard focus, 48px targets, pending/recovery, responsive long content,
reduced motion, CSS layout magnification and opaque capability fallback.
Synthetic IME events, owned CDP preferences and deterministic clipboard denial
are boundary tests, not OS/browser/AT interoperability certification.

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
keyboard checks. Outstanding browser/AT checks and parent merge verification remain explicit
release/review limitations; they must not be silently converted into passes.
