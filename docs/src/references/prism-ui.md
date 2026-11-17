# Prism UI system and interaction contracts

The shipped Resolver and Explorer implement the approved light Prism direction.
[PRISM-SPEC.md](https://github.com/hyperledger-identus/neoprism/blob/main/designs/PRISM-SPEC.md) and `designs/neoprism.fig` remain
visual authority; this document records implementation, not a new design world.
Scalar's generated `/openapi` document is deliberately outside this system.

## Visual system

Source tokens and responsive rules live in
`bin/neoprism-node/ui-assets/prism.css`. Rust/Maud supplies semantic HTML;
Tailwind remains available. No frontend framework or dark theme is introduced.

| Role | Shipped value / use |
| --- | --- |
| UI typography | Self-hosted Poppins, 400/500/600; body 14px / 1.5 |
| Data typography | Self-hosted JetBrains Mono, 400/500; tabular numerals |
| Primary / hover / pressed | `#5559f2` / `#4549dc` / `#363ab8` |
| Foreground / secondary | `#182033` / `#475467` |
| Panel / data / divider | `#f9f9fd` / `#eff0f8` / `#d6d9eb` |
| Main heading | Desktop 30px/45px, mobile 26px/39px, weight 600 |
| Common spacing | 8, 12, 16, 24, 32px |
| Shapes | Status 8px; controls 12px; panels 20px; shell 24px |
| Focus | External 2px violet outline, 2px offset, light separation |
| Targets | At least 48px high; icon-copy targets 48 × 48px |
| Transition | Control colors, 150ms ease-out; removed for reduced motion |

Pale lavender/cyan/rose illumination is behind the shell, never on text or data.
The desktop shell uses a soft offset shadow, 32px outer inset, 216px sidebar and
32px workspace padding. At 800px and below it becomes normal document flow with
20px insets, a native expandable menu and stacked form controls. Pages scroll as
one document; tall disclosures are not nested scrolling application windows.

Resolver evidence sits in a 300px rail from 1280px upward; below that it follows
the dossier in reading order. Panels and data columns use shrinkable tracks and
visual wrapping, not inserted newlines. At 600px and below labelled identity
and recovery actions stack. The text-bearing **Copy DID** action sizes to its
label (full width when stacked); square icon-copy controls remain unchanged.

Explorer has a 57px row pitch, 14px monospace identifier, one 48px Resolver link
and a separate 48 × 48px copy target. Within narrow list containers the enhanced
label is middle-abbreviated; the original remains the link/copy value. Without
copy enhancement, full identifiers stay horizontally selectable. Pagination is
centered above and below the list; page numbers have equal-width targets.

Opaque shell/sidebar backgrounds are the default. Only a supported
`backdrop-filter` adds surface translucency; reduced transparency restores the
opaque backgrounds. Reduced motion removes transitions, not state information.
The supplied original multicolor SVG logo is not recolored. Fonts, scripts,
logo and font licenses ship locally in both development and Nix asset bundles.

## Shared controls and navigation

- One main landmark and one page heading; labelled navigation, form labels,
  native buttons and native `details`/`summary` disclosures retain reading order.
- Mobile navigation supports Enter/Space. Enhanced Escape closes it and returns
  focus to its summary. Native menu/navigation still work without JavaScript.
- Resolver/Explorer/API Docs destinations and root redirects follow run mode.
  Submitter-only mode has no Resolver/Explorer route and redirects to API Docs.
- Version and configured network describe actual runtime configuration, not
  sample telemetry. Cursor receipt does not establish network freshness.
- All full-value copy controls use the shared `copy.js` contract:
  `type="button"`, `data-copy-value`, descriptive accessible name and
  `prism-copy`. Delegation also handles swapped results. A successful write
  announces polite feedback without moving focus. Denied/missing clipboard
  access supplies an adjacent labelled readonly textarea with the exact value;
  readers can Tab to it and select manually. Values remain selectable without
  JavaScript. Copy controls do not become nested links.

## Resolver

The ordinary `/resolver?did=…` GET form remains the canonical interface. Enter
submission, retained input, required-field validation and bookmarkable URLs work
without scripts. Input is a single-line canonical value: presentation never
rewrites it or inserts line breaks. Composition events do not rewrite the value;
real OS IME interoperability requires separate testing.

`resolver.js` progressively fetches the same HTML GET, replacing only the result.
A real pending request disables Resolve, temporarily makes input readonly,
announces resolving and displays stable skeletons. Duplicate submissions are
ignored. Every terminal path resets controls. The bound is 30 seconds; HTTP,
network, malformed-success and timeout failures use safe retry feedback, never
the response body. Navigation restoration cancels obsolete delivery. Successful
resolution updates the URL; Back/Forward resolves the restored URL.

A failed delivery retains attempted input and the previous result/disclosure
state, visibly labelled **Previous result**. It does not present old history as
evidence for the failed attempt. Exact-value retry, correction and Explorer
links remain ordinary links. Without scripts, HTTP/network delivery failures use
native browser Back to recover the entered form, then a native GET retry.

Outcomes distinguish invalid syntax, invalid encoded initial state, no indexed
state on this node, unusable stored operations, retrieval failure, locally
derived state, indexed state and deactivation. Absence on one node is not proof
of on-chain nonexistence. Locally derived results suppress synthetic ledger
evidence; deactivation preserves actual history even with empty current keys.

Details retain all original fields:

- Keys sorted by ID; exact usage/curve/coordinates, with absent Y explicitly not
  applicable. Counts come from state, not mockup examples.
- Ordered contexts and explicit compact empties. Services preserve single and
  multiple types, URI/object/array endpoints and captured fields. Readable JSON
  is a parsed-state projection, not reconstructed authored formatting.
- VDR initial/last hashes, Bytes, IPFS and numeric StatusList state/name/detail.
  No invented active-state interpretation or external IPFS fetch.
- Applied/rejected operation history, every ledger coordinate and signer,
  separately labelled safe domain diagnostics, and the complete captured signed
  message Debug representation. Debug is not original wire bytes. Capture occurs
  before evidence hashing because protobuf serialization mutates cached fields.

Internal infrastructure causes are logged server-side and replaced with generic
public feedback, including HTML-200 error outcomes. The typed diagnostic
projection does not walk arbitrary error-source chains. Public protocol failures
and complete captured payload inspection remain available when safe.

**Open Resolver API** deliberately opens a separate request in a new tab with
`noopener noreferrer`. Indexed-state success is not a DID-document API response
and never fabricates an API success/failure notice.

## Explorer

`/explorer?page=…` retains 100 rows per page, descending first-indexed slot with
DID tie-break and ordinary GET pagination. No invented sorting/filtering controls
or fabricated source/health metrics are added.

Sync status separates reported slot, optional block time (UTC), browser receipt
age and **Distance to chain tip unknown**. Missing timestamp means **Not
provided**, never receipt time. Technical details expose complete original slot,
hash and timestamp with exact copy controls. Missing cursor is waiting for sync
information, not an invented empty record or proof of a configured source.

`explorer.js` alone owns independent 2-second cursor and 5-second selected-page
list refreshes. It pauses while hidden, refreshes on return, bounds hung requests
at 15 seconds, and rejects obsolete delivery. HTMX does not also replace these
resources. Pagination remains native navigation and wins over old requests.

Failed refresh retains last successful evidence and names its retry cadence.
Empty list, out-of-range page, waiting cursor and retrieval failure are distinct.
Disclosure expansion, focus, DOM selection and manual-copy selection survive
refresh. Changed evidence waits until interaction ends; only the latest staged
snapshot applies. Feedback explicitly distinguishes received from displayed data.
A successful receipt is not proof of cursor advancement, chain tip, node health
or the last indexed PRISM operation.

## Production corrections and boundaries

The pinned prototype supplies composition, not production data or backend truth.
The implementation intentionally omits mockup notices, illustrative content,
fake telemetry and unearned API-result banners. Safe diagnostics replace internal
cause disclosure. Real counts, available fields, content lengths and complete
100-row pages determine actual document height; static specimen excerpts do not
limit data. Native disclosure height can exceed the specimen's compact controls
when exact labels or safe explanatory copy wrap.

No fixture routes/data, wallet/editor workflow, resolution/indexing/storage
semantic changes, generated Scalar redesign or embedded-executable asset work
are part of Prism. Runtime asset distribution remains the existing model.

## Validation and remaining review

See [the reproducible validation record](https://github.com/hyperledger-identus/neoprism/blob/main/tools/ui-tests/PRISM-VALIDATION.md)
for commands, seams, browser/assistive-technology limitations and evidence names.
Automated axe and keyboard checks are evidence toward WCAG 2.2 AA, not a WCAG
certification. Independent rendered review and two-axis code review are
**parent-owned and pending**; integration ticket #381 is not complete until those
reviews and any required repairs have been independently verified.
