# NeoPRISM — Prism design specification

## Scope and status

**Static design deliverable, not application implementation.** Prism is the
selected visual direction for the NeoPRISM node UI. This specification records
the intended behavior and implementation constraints; it does not claim that the
mockups provide working interactions.

- Editable document: [neoprism.fig](neoprism.fig), opened in OpenPencil.
- Original logo: [neoprism.svg](neoprism.svg).
- Start on **Page 1**, which contains the design-library guide.

There are **26 artboards**: 12 desktop/mobile pairs, desktop foundations, and a
design-library guide. Desktop width is 1440 px; mobile width is 390 px. Heights
follow content. Tall artboards describe scrollable pages, not one device screen.
The mockups contain editable text/vector design data, not application source.

## Page and artboard inventory

| OpenPencil page | Desktop/mobile specimens |
| --- | --- |
| Resolver · desktop / Resolver · mobile | Entry; indexed identity; contexts/services/VDR; key and operation details; resolution-outcome gallery |
| Explorer · desktop + mobile | Populated page; empty/unavailable cursor; failed refresh retaining last data; expanded technical details |
| Components + states | Controls/feedback; data/disclosures; navigation/resilience; desktop foundations; native component masters |
| Page 1 | Design-library guide, scope and provenance |

On combined pages, desktop artboards are above mobile companions. Native
component sets are above the component specimen artboards. Layer names are the
stable way to find content; IDs can change when the file is reopened.

## Visual system

The Prism visual system uses:

- Poppins for UI text; JetBrains Mono for identifiers, coordinates and code.
- Violet-blue primary action `#5559F2`; primary hover `#4549DC`, pressed `#363AB8`.
- Navy foreground `#182033`, secondary text `#475467`, crisp data surface `#F9F9FD`.
- Pale lavender/cyan/rose illumination; frosted navigation and floating shell.
- Desktop shell: 32 px outer inset, 216 px sidebar, 32 px workspace padding.
- Identity dossier: 772 px content, 24 px gap, 300 px ledger-evidence rail.
- Mobile: 20 px inset, 350 px content, stacked panels, expandable navigation.
- Common spacing: 8/12/16/24/32 px. Controls: 48 px high. Radii: 8 px status,
  12 px controls/disclosures, 20 px panels, 24 px shell.
- Focus: external 2 px violet indicator beyond a 2 px light offset.
- Fine dividers; soft offset shell shadow. Never blur data or text. Use an opaque
  panel fallback when transparency is reduced or backdrop filtering is absent.

The document has **23 semantic color variables** and **six native component sets**, with 25
variants: Primary, Secondary, DIDField, Disclosure, Copy and Status. A separate
**Prism/Logo** component holds the supplied logo. Buttons in screen specimens
are linked instances with editable labels.
Not every repeated panel is a linked instance. Do not imply a fully wired
component application.

Use the original [NeoPRISM logo](neoprism.svg) for application implementation,
not an importer-specific adaptation. The mark occupies a 24 × 24 px slot;
preserve its colors, gradients and clipping rather than recoloring it to the UI
accent.

## Feature-parity traceability

The baseline UI uses Rust/Maud, Tailwind/DaisyUI and HTMX. Preserve its routes
and backend behavior when implementing.

| Existing feature | Design location / retained contract |
| --- | --- |
| `/resolver?did=…` GET form, Enter submission, retained query | Entry/indexed screens and invalid-result specimen; URLs remain bookmarkable |
| Empty entry screen | Entry pair adds format guidance and a link to indexed DIDs |
| Published/indexed state | Indexed pair shows copyable identity, counts and evidence |
| Unpublished long-form result | Outcome gallery: locally derived state, not ledger confirmation |
| Deactivated result | Outcome gallery: explicit status only when backend state supports it; history retained |
| Invalid input, not found, unexpected failure | Outcome gallery: preserved input, recovery, expandable technical diagnostics |
| Ordered `@context`, including empty | Detail pair lists contexts in order; overview/data sheets show compact empties |
| Keys sorted by ID: ID, Usage, Curve, X, Y | Indexed key rows and expanded key details; exact usage values retained |
| secp256k1, Ed25519, X25519; missing Y | Data pair shows absent Y as not applicable, never fabricated |
| Services: ID, Type, Endpoint | Detail pair demonstrates URI, object and multiple endpoints, single/multiple types, and raw representation |
| VDR: initial/last hashes and Data | Detail/data pairs show StatusList, bytes and IPFS, copyable hashes and original raw data |
| StatusList state/name/detail | Detail pair uses an uninterpreted numeric state, plus name and detail; no invented active-state mapping |
| All operation metadata | Diagnostics pair: block time, slot, block, block sequence `absn`, transaction ID, operation ID, operation sequence `osn` |
| Full signed payload and error chain | Expanded raw inspector includes the complete captured Debug representation, not a key-ID summary; rejected operations and technical causes remain inspectable |
| Debug on failed final-state resolution | Outcome gallery keeps operation diagnostics separate from the final-state failure |
| Resolver API external action | Open Resolver API remains a separate request, not proof of a DID-document response |
| `/explorer?page=…`, fixed 100/page | Explorer pair retains numbered pagination above and below rows and active-page selection |
| Latest first-indexed slot descending, DID tie-break | Explorer ordering is stated explicitly; no invented sortable columns or filters |
| DID links to Resolver | Each row opens the full identity; Copy always copies the complete DID |
| Cursor slot/hash/optional timestamp | Sync panel and missing-timestamp specimen; source identity and freshness remain distinct |
| Independent cursor/list polling | Cursor 2 seconds, list 5 seconds, current page retained; failed refresh does not erase the list |
| Missing cursor and empty list | Empty Explorer and navigation/resilience pair distinguish unavailable evidence from confirmed emptiness |
| Resolver/Explorer/API Docs navigation | Desktop sidebar and mobile expanded-menu specimen; GitHub, version and configured-network metadata retained |
| `/openapi` Scalar reference | Shared API Docs navigation link only; generated Scalar page is outside the design scope |
| Root redirect and node run modes | Routing behavior unchanged; hide unavailable destinations in submitter-only mode |

Baseline source references:

- [Shared components](../bin/neoprism-node/src/http/components.rs)
- [Routes](../bin/neoprism-node/src/http/urls.rs)
- [Resolver UI](../bin/neoprism-node/src/http/features/ui_resolver/)
- [Explorer UI](../bin/neoprism-node/src/http/features/ui_explorer/)
- [HTTP API](../bin/neoprism-node/src/http/features/api/)

## Compact Explorer list

Use a compact DID list for large datasets. Populated and refresh-failed screens
share the same pattern on desktop and mobile:

- 56 px row plus a 1 px divider: **57 px row pitch**, without inter-row gaps.
- Keep 14 px monospace identifiers; desktop shows the complete canonical DID.
  Mobile uses a single-line middle abbreviation, retaining the method, suffix
  prefix and final eight characters. Full values remain available through
  resolution and copy; copying must never use the abbreviated label.
- The identifier and chevron form one 48 px-high Resolver link. A separate,
  borderless 48 × 48 px copy target sits at the trailing edge. Do not nest copy
  inside the link or add a second open button/helper line to every row.
- Preserve a thin divider, visible keyboard focus and separate accessible names
  for resolution and copy. No smaller text or smaller touch targets are needed.
- The static excerpts show 12 desktop rows and 10 mobile rows. Page size
  remains 100, with the same ordering and polling. Both pagination groups and
  their page-count labels are centered on the list above and below it. Numeric
  page buttons have equal widths; Previous/Next remain wider on desktop.
- Cursor, empty-state, navigation and Scalar boundaries are unchanged.

## Explorer sync-status summary

Use a compact summary with technical details available on demand:

- Rename the panel **Sync status** and place the cursor-received, waiting or
  refresh-failed indicator alongside the heading.
- Show **Reported slot** and a human-readable **Block time · UTC**, in two
  unboxed columns on desktop and stacked on mobile. A missing block timestamp
  reads **Not provided**; never substitute the cursor receipt time.
- Keep cursor receipt age separate from block time. Cursor checks remain every
  2 seconds; DID list refresh remains every 5 seconds.
- Keep **Distance to chain tip unknown** visible. Receipt of a cursor is not
  proof of advancement, a current chain tip, node health or PRISM indexing.
  Do not add a percentage, progress bar, ETA or fully-synced status.
- A 48 px **Technical details** disclosure reveals the unformatted slot,
  complete block hash and exact UTC timestamp, each with a full-value copy
  control when supplied. The original values must not acquire formatting or
  line breaks when copied. Explain cursor-versus-PRISM-operation semantics here.
- Preserve expansion, focus and selection during polling; use Enter/Space and
  `aria-expanded`/`aria-controls` when implementing. These scenes are static,
  so both collapsed and expanded states are shown as separate Explorer
  artboards, with additional component specimens. This does not claim a working
  click interaction.
- Failed refresh retains the last values with **Couldn’t refresh · showing last
  received cursor**, receipt age and retry cadence. The unavailable-cursor
  specimen reads **Waiting for sync information**, without invented values or
  an empty technical-details control. This does not prove a configured source.
- Explorer expanded desktop/mobile examples show every hidden field in page
  context, alongside the default collapsed examples. Navigation/resilience
  specimens also show expanded details and a missing timestamp.
  All existing Explorer outcomes and the compact DID list remain present.

## API Docs boundary

**API-page mockups are excluded.** Keep the shared API Docs navigation link to
`/openapi`. The page itself remains generated by Scalar from OpenAPI; do not
redesign, wrap, theme or replace it as part of this work.

## Proposed interaction improvements

These are design proposals, not existing implemented capabilities:

- Summary first, details on demand. Preserve all original fields and complete
  diagnostics behind labelled disclosures. Empty sections stay compact but
  explicit; do not hide empty key or operation state.
- Required DID field, useful format guidance, retained invalid value and
  actionable recovery. No wallet connection or DID authoring workflow.
- Copy controls for complete values, success feedback and a selectable manual
  fallback. Render wrapping visually: never insert newlines into raw JSON,
  URLs, DIDs, coordinates or hashes. Exact captured payload text is preserved.
- The DID form may expand its visual field for long-form values. Preserve Enter
  submission, IME composition and the canonical value; input presentation must
  not introduce line breaks into a DID. Resolve is disabled while its request
  is pending, with a meaningful loading label and stable skeletons.
- Separate published/indexed, locally derived, deactivated and failed states.
  No invented health badge, sync percentage, chain tip or freshness guarantee.
- Network labels identify configuration. Cursor receipt time is not block time,
  and the DLT cursor is not the last indexed PRISM operation.
- Initial loading, explicit empty, stale/failed refresh retaining prior data,
  missing timestamp, out-of-range page and deliberate retry specimens.
- API notices depend on actual requests. Indexed-state success alone never
  triggers a success or failure notice for the separate DID resolution API.
- Semantic headings, landmarks and buttons; reading-order tab flow; visible
  focus; Enter/Space disclosures; Escape closes menu and restores focus;
  polite copy/loading announcements without stealing focus.
- Short state transitions (150–200 ms), no decorative page-load choreography.
  Reduced motion removes transitions without removing state information.

## Data provenance and safety

The sample DID, key coordinates, transaction ID, operation ID, timestamp and
signed-operation Debug representation derive from a captured local-testnet
response. The design is not live telemetry. Non-empty service/VDR/other-curve
examples and the rejected operation are illustrative fixtures, not claims about
that DID.

Controls do not navigate, copy, send requests or submit operations in these
static scenes.

## Validation expectations

When updating the design:

- Run OpenPencil's recommended and accessibility lint presets. Resolve errors
  and warnings without disabling rules; review informational findings separately.
- Check desktop/mobile content bounds, legibility and focus treatment. Background
  illumination may be clipped intentionally; content must not overflow.
- Check raw JSON validity and exact full-payload preservation. Copy labels and
  abbreviated identifiers must not alter the underlying value.
- Verify that Poppins and JetBrains Mono are available when rendering.

Static design lint does not establish runtime accessibility, browser
interoperability, real-device behavior or backend correctness. Implementation
must separately test keyboard and assistive-technology behavior, responsive
layouts, request outcomes and state retention during polling.
