# Rich text verification

## Source and tracking integrity

The inventory command reads the pinned Git tree. `--check` recomputes source
hashes, selected files, include edges, public member identifiers, and generated
reports, compares them byte for byte, and verifies recorded Rust targets and test
paths. A passing inventory check establishes traceability; it establishes no
runtime parity on its own.

```sh
python3 scripts/porting/winui-rich-text-inventory.py --reference /path/to/microsoft-ui-xaml --check
```

## Upstream test families to adapt

All paths below are relative to the pinned reference and appear in the source
inventory when selected. Preserve each original test's contract and assertions;
rewriting a test to assert the current implementation is not a parity check.

| Reference family | Coverage needed |
|---|---|
| `dxaml/xcp/components/text/unittests/TextHighlightMergeUnitTests.*` | Region precedence, splitting/merging and selection/highlighter overlaps. |
| `dxaml/xcp/components/text/unittests/TextBoxHelpersUnitTests.*` | Text navigation/classification helpers and boundary behavior. |
| `dxaml/test/native/external/foundation/text/RichEditBoxTOMTests.*` | RichEdit document/range/selection/format APIs, loading/saving, links and object-model geometry. |
| `dxaml/test/native/external/foundation/text/RtblHasContentOverflow.*` | Content overflow after formatting/size/content changes. |
| `dxaml/test/native/external/foundation/text/TextBidiTests.*` | Mixed-direction runs, paragraph flow, selection and rich/overflow control behavior. |
| `dxaml/test/native/external/foundation/text/TextBoxSelectionTests.*` | Programmatic and user selection, selection colors and focus effects. |
| `dxaml/test/native/external/foundation/text/TextBoxPointerTests.*`, `TextBoxkeyInput.*`, `TextBoxScrollingTests.*` | Pointer/key commands, caret positioning, scrolling, clipping and input integration. |
| `dxaml/test/native/external/foundation/text/HyperlinkInTextTests.*` | Link selection, navigation, focus, style and pointer behavior. |
| `dxaml/test/native/external/foundation/text/TextControlsRenderingTests.*`, `TextControlsSizingTests.*`, `TextFontFallBack.*` | Font fallback, constrained sizing, scale, glyph/raster and rendering regressions. |
| `dxaml/test/native/external/foundation/graphics/rendering/RichTextBlockTests.*`, `TextHighlighterTests.*` | Multi-paragraph layout, inline UI, formatting invalidation, highlights and visual reference comparisons. |
| `dxaml/test/managed/controls/RichTextBlock/`, `dxaml/test/managed/controls/TextBox/TextEditingTests.cs` | XAML-facing control properties, content, events and end-user editing flows. |

The generated snapshot inventories test source, not the native WinUI test runner
or every external Windows integration prerequisite. Original image masters are
reference evidence whose applicability depends on fonts, scale, OS and rendering
configuration. They are not assumed portable golden images.

## Rust test requirements

Document tests cover UTF-16 offsets around astral characters and combining
sequences; malformed ranges; run/paragraph boundary insertion and deletion;
format preservation; collection ownership/identity; structural positions and
gravity; snapshot consistency; notification reentrancy; and undo/redo.

Layout tests use fixed test fonts/shapers and cover wrapped mixed-format and bidi
paragraphs, zero/finite/infinite constraints, line spacing and indentation,
selection/hit-test geometry, overflow continuation, changes in linked controls,
embedded UI, and invalidation after inherited property changes. Fixed shapers
exercise deterministic geometry; separate real-font tests exercise shaping.

Parser tests include nested destinations, escapes, Unicode/fallback handling,
font/color tables, paragraph state, truncated input, oversized counts, bounded
nesting, and supported-content round trips. Unsupported content must have an
explicit result or preserved representation.

Control tests need a root and dispatcher scope, routed keys/pointers, read-only
commands, focus and selection changes, clipboard providers, template application,
runtime/compiled XAML, IME composition/commit/cancel, and no ownership cycles.
Automation tests need real text/range providers; an `IValueProvider` alone is
insufficient rich-text accessibility coverage.

## Recorded runs

Authoritative command/result records are stored in `mapping.json` under
`validation`, including the checkpoint they cover. Generated [PROGRESS.md](PROGRESS.md)
shows those records. A later test definition or source change is not implicitly
covered by an earlier successful run.

### Current integration checkpoints

The [completed full workflow at 41ea4f1b](https://github.com/wieslawsoltes/FerroUI/actions/runs/39604251320)
passed both Linux and Windows. Linux totals were Base 4,412; Controls 4,915;
Win32 308; WinUI 31; Fluent 210 with two ignored; Simple 213 with three ignored;
WinUI theme 296 with two ignored; XAML 605 with 17 ignored. All five reference
generators and the exact file comparison passed. Windows passed adapter
compilation, 35 WinUI, two exact HSTRING, two native probe and 474 rich Controls
tests. Those results describe the preceding published checkpoint.

At `92140e7c`, the source-identical public harness passed **203 tests with zero
failures/ignored** against matching rebuilt ordinary libraries. The final scoped
Cargo build completed the Controls artifact, preserved Base's exact hash/mtime,
and was stopped before downstream compilation. It is not a full Cargo no-run
claim. Exactly three existing private startup tests remain excluded whole from
the local harness and retained in repository CI.

The coordinated CoreText gate passed **15 portable tests**. Six new Windows-only
layout-adapter tests still require a successful native run. The full XAML suite
passed **611 tests with 17 ignored** before the final runtime geometry changes;
after rebuilding that graph, **18 relevant XAML checks passed**. Rust-path and
View-builder generation checks and Simple/Fluent reference comparisons passed.

Pre-fix regressions independently demonstrated three metadata receiver errors,
two same-view column losses, an RTL structural-marker geometry defect, stale
cursor publication, reflected popup-edge placement, ordinary dismissal session
retirement and redirected story-edge count error. Their corrected repository
fixtures are included in the completed gates. A separate template-replacement
probe reproduced connection retirement after that 203-test checkpoint. It became
a public regression and is repaired in the later 235-case checkpoint below.

The [new full workflow at 92140e7c](https://github.com/wieslawsoltes/FerroUI/actions/runs/39612123175)
is separate. Its Windows job finished with two compile errors: the native
LayoutRequest.IsCanceled getter returns ABI `u8`, and two new predicates expected
Rust `bool`. The narrow fix uses explicit nonzero/zero comparisons and awaits a
new native run. Independent later steps passed two exact HSTRING, two native
probe and **529 rich Controls tests**, with no failures/ignored in those tests.
The Linux job completed successfully: Base 4,413, Controls 4,970, Win32 308,
WinUI host 39, Fluent 210/2 ignored, Simple 213/3, WinUI theme 296/2, XAML
611/17, and the separate ignored doctest. All five reference generators and
exact checked-in-file comparisons passed. Neither this Windows job nor the six
new Windows-only layout tests are recorded as passing.

The subsequent [focus handoff](https://github.com/wieslawsoltes/FerroUI/commit/b16a5ce14a7bdebfed0752254f5b5d2eb9c3db42)
and [TOM clipboard](https://github.com/wieslawsoltes/FerroUI/commit/132c7853a17553e475e842d8c7c3327a60a37e6e)
source passed **235 original public tests, zero failures/ignored**, including
four focus and 28 clipboard cases. Both ordinary Base variants and Controls
were rebuilt in the established feature graph; hashes were unchanged during
build and execution. Cargo completed the Controls artifact in 222.97 seconds
and was intentionally stopped before downstream linking. The direct harness
then ran in 2.88 seconds. This is scoped compilation plus source-identical
runtime evidence, with the same three whole private-startup exclusions.

Before the focus repair, the same four public fixtures produced two expected
remount failures and two cancellation passes against the old libraries. After
the 235 gate, five separate native-column fixtures produced three expected
failures (15 versus 12, 14 versus 10, and -3 versus -2) and two passes against
those exact libraries. They establish the next correction's starting point;
they are not included in the 235-test passing claim.

### Earlier checkpoints

The restored core checkpoint
[`38c934480ca20192746a363670c12eac47d67edf`](https://github.com/wieslawsoltes/FerroUI/commit/38c934480ca20192746a363670c12eac47d67edf)
passed `cargo test --locked --offline -p ferroui-controls --lib rich_ --no-fail-fast`:
**97 passed; 0 failed; 0 ignored; 0 measured; 4426 filtered out**. That run includes
21 document, 45 editor, 16 view and 15 portable automation tests. The subsequently
added view/highlighter tests, four portable follow-ups, native bridge tests and
markup/catalog tests need their own later records.

The continuation checkpoints added the following completed runs (the authoritative
records and exact revisions are also in `mapping.json`):

| Checkpoint | Command scope | Result |
|---|---|---|
| `871d566d25aab3e89021d7fc5db6edf32c07267e` | controls `--lib rich_` | 181 passed; 0 failed; 4434 filtered out. |
| `f1a051f8eaddd2d9b651a731206eebd9fa3a252a` | Win32 `--lib text_` | 34 passed; 0 failed; 234 filtered out, including ten text pattern/range bridge cases on the Linux-hosted test harness. |
| `92551b30a05c4766594efb6c112e56cdf3464f1a` | controls `--lib rich_` | 246 passed; 0 failed; 4434 filtered out, including persistent presenter/scrolling, streams, native-valued units, live-pointer batching, language and automation owner/geometry tests. |
| `ed836e831fc8af5114e2ce8ae7632867c7a2332f` | full controls `--lib`; Win32 `--lib text_`; source scanner regression | 4706 controls, 41 Linux-hosted Win32 and one source-scanner test passed. The full controls run had loopback access. Exact scope and the equivalent post-run type-registration annotation are recorded in mapping.json. |

The new paragraph tests cover all numeric spacing rules, point conversions,
actual line/overflow behavior, signed indents and RTF sl/slmult ordering. The
27 focused document-service tests include the adapted UseLfAndUseCrLf fixture;
all other additions are independent contract regressions. The control additions
exercise cancellation/reentrancy, deferred notification lifetime, real content
presenters, navigation and IME state. Accessibility additions exercise caret
focus, stable weak hyperlink peers, TextChild, native marshalling and overflow
geometry. The 92551b30 checkpoint verifies the subsequent presenter/stream additions. Later
composition/TextEdit and context-flyout changes require their own recorded run.

## Implementation test map

### Versioned native TOM observations

The [first system MSFTEDIT run](https://github.com/wieslawsoltes/FerroUI/actions/runs/39600670835/job/119224339597)
at `eff6f53ade273677db8679d2c6089909fcf7a886` passed its Home/End probe:
60 initial commands and 60 repeated no-ops, on Windows 10.0.26100 with
`Msftedit.dll` 6.2.26100.33438. The exact fixture text, native story lengths,
normalized and directed endpoints, flags, signed results and HRESULTs are retained
in [native-msftedit-home-end.json](native-msftedit-home-end.json), together with
the probe's Git blob and CI provenance. This is an observation of that system
RichEdit version; it does not establish external WinUI engine equivalence.

The recorded multiline fixture `abc\rdefg\rhij` demonstrates a concrete distinction:
starting with anchor 1 and active end 6, collapsed Line Home reaches 0 and returns
-6, while extended Line Home retains anchor 1, reaches 4 and returns -2. Collapsed
Line End reaches 8 and returns 2. The native Story End extension can include the
synthetic final paragraph mark; FerroUI's normalized model still excludes that
mark under RT-I01/RT-I08. These facts drove the later implementation correction;
the successful probe itself does not change the Rust behavior at its checkpoint.

To retain a later completed observation, use the strict extractor with the exact
tested commit, probe blob and run/job URLs:

```sh
python3 scripts/porting/rich-text-tom-observations.py \
  --log /path/to/completed-windows-job.log \
  --commit <tested-full-commit-id> --probe-blob <tested-probe-git-blob-id> \
  --run-url <actions-run-url> --job-url <actions-job-url> \
  --output /path/to/native-observations.json
```

The extractor rejects unknown probe rows, incomplete command/repeat totals,
inconsistent engine identities, malformed fixture lengths and contradictory
directed-state fields. It preserves observations without comparing them to the
Rust implementation. The later vertical probe has its own execution record;
adding its source does not make it part of this first native run.

The [second record](native-msftedit-navigation.json) comes from
[Windows at 41ea4f1b](https://github.com/wieslawsoltes/FerroUI/actions/runs/39604251320/job/119236263239):
the same 60 Home/End commands/repeats plus 264 vertical commands, using MSFTEDIT
6.2.26100.33648. Its Home/End rows exactly match the earlier engine version.
Vertical collapse applies the full requested distance after choosing the
directional endpoint; a collapse alone reports one. For example, in
`abcdef\rxy\rabcdef\rpqrs`, selection 2→12 followed by collapsed Line Down1
reaches 19 and returns one, while Paragraph Down1 reaches 17. The sequential
long→short→long collapse sequence retains positions 5→8→14→8→5.

The [third record](native-msftedit-navigation-isolated.json) comes from
[Windows at 92140e7c](https://github.com/wieslawsoltes/FerroUI/actions/runs/39612123175/job/119263416996):
60 Home/End commands, 60 repeats and 282 vertical commands on MSFTEDIT
6.2.26100.33438. The original 264 vertical and all Home/End rows exactly match
the 6.2.26100.33648 record. Eighteen additional commands use fresh controls or
explicit sequences. They confirm extended-column clamping: caret 5, Line Down2
reaches 15 for collapse but 12 for extension; Up(-2) agrees. In the isolated
long/short/long sequence collapse visits 5→8→14→8→5, while extension visits
5→8→10→8→1 with anchor 5 retained. Fresh final-empty-line Up1 and Down(-1)
both reach 4; Up2 and Down(-2) reach 3. Thus the earlier negative-direction
difference was reused state, but extended-column behavior is a real remaining
Rust correction. The two native probe tests passed; this does not turn the
separate failing CoreText compilation into a passing Windows job.

### Rust families

| Rust family | Concrete evidence and current limits |
|---|---|
| [Document tests](../../../src/FerroUI.Controls/documents/rich_text_document_tests.rs) | UTF-16 surrogate rejection, grapheme/word boundaries, nested semantic containers, snapshot formatting, paragraph splicing, batched revisions, reentrancy, pointer gravity, weak ownership, hyperlink cancellation/release semantics and embedded controls. Native synthetic element markers and every upstream element type are not represented by this suite. |
| [Rich view tests](../../../src/FerroUI.Controls/rich_text_block_tests.rs) | Measure/wrap, inherited formatting invalidation, selection geometry/direction, linked reflow and ownership, embedded controls, text-pointer insertion geometry, hyperlink host integration, and later real-ellipsis/finite-height/paragraph-indent/visible-range regressions. Check the validation record to determine which additions were executed in a particular run. |
| [Editor tests](../../../src/FerroUI.Controls/rich_edit_box_tests.rs) | Rich undo/redo, typing coalescing, read-only/max-length policy, failed transaction rollback, IME preview/cancel/commit and surrounding text, stale async clipboard guards, native transfer policies, bounded RTF fixtures, live ranges/search, mixed formatting, shared control geometry and caret blinking. These do not constitute execution of a real platform IME or unavailable native TOM engine. |
| [Live pointer batch tests](../../../src/FerroUI.Controls/documents/rich_text_pointer_tests.rs) | Immediate ordered rebasing, pointers created midway through a batch, nested rollback, reentrant notification order, formatting-only changes and weak lifetime. |
| [TOM unit tests](../../../src/FerroUI.Controls/rich_text/rich_edit_text_units_tests.rs) | Native values for eight units, scalar Character, actual shaped Cluster, Word/Sentence distinctions, soft/hard paragraphs, CRLF, native collapse counts, one-based/end-relative indices, saturation and nonmutation, weak measured Line association and overflow/trimmed coverage. Cluster requires complete untrimmed layout and tests ligatures, scripts, bidi, wrapped origins, omitted glyphs and reentrancy. |
| [TOM Delete/Character tests](../../../src/FerroUI.Controls/rich_text/rich_edit_text_range_operations_tests.rs) | Signed deletion, selected-span counts, paragraph joins, shaped reflow after first deletion, inherited formatting, scalar-safe Character replacement, independent directed selection and live range rebasing. Callback tests distinguish ordinary rollback from proven external model writes during layout, document publication and editor publication, including nested groups and held outer document batches. |
| [Selection Home/End tests](../../../src/FerroUI.Controls/rich_text/rich_edit_text_selection_navigation_tests.rs) | Twenty-two cases cover directed UTF-16 movement/extension, shaped and headless lines, CRLF/soft breaks, wrapping/RTL/overflow, layout rejection, read-only/composition/history retention, cancellation and selection/model reentry. Six native-derived cases add normalized collapse edges, real paragraph marks, repeated End affinity, empty paragraphs, cancellation and nested/view ownership. Synthetic terminal EOP and further engine affinity remain separate in RT-I21. |
| [Selection vertical and accepted-movement tests](../../../src/FerroUI.Controls/rich_text/rich_edit_text_selection_movement_tests.rs) | Shaped Line and headless Paragraph fixtures cover directed collapse/extension, accepted unit counts, negative/extreme/zero requests, short-line column retention, mixed heights, bidi/UTF-16, wrapping, empty lines, overflow and stale/trimmed layout rejection. Callback cancellation/redirection, nested same-offset navigation, view replacement and external document/publication edits preserve accepted state. Read-only, preview, redo and logical move_by regressions share the publication path. Runtime execution records remain separate from test definitions. |
| [Input policy manager tests](../../../src/FerroUI.Base/input/text_input/text_input_method_options_tests.rs), [rich owner tests](../../../src/FerroUI.Controls/rich_edit_input_options_tests.rs) and [CoreText cache tests](../../../src/Windows/FerroUI.WinUI/core_text_input_options.rs) | Seven Base lifecycle/inheritance/disposal/reentrancy cases, two focused/flyout semantic-owner cases, and three lazy-context/nested-setter/retry cases. These distinguish shared policy delivery from native OS prediction behavior. Each suite has its own actual execution record. |
| [TOM clipboard tests](../../../src/FerroUI.Controls/rich_text/rich_edit_text_clipboard_tests.rs) | Twenty-eight public cases cover copy permission, exact representation policies, metadata-only CanPaste, rich/plain formatting, one-group undo, UTF-16 limits, preview ordering, delayed target ownership, held batches, independent clones, read-only completion, retrieved-transfer disposal and callback publication. Real pending futures exercise cancellation rather than immediately ready test doubles. Defined source and completed execution are tracked separately; native protection, formats and ABI remain partial under RT-I24. |
| [Focus handoff tests](../../../src/FerroUI.Controls/rich_edit_input_focus_tests.rs) | Four public cases cover actual template replacement during native/overlay popup focus return, nested Lost/Got redirection and same-owner return, canceled focus and ordinary post-event client installation. Two remount regressions failed against pre-fix libraries while both cancellation cases passed. All four pass in the 235-case source-identical gate; arbitrary asynchronous platform focus order remains open. |
| [Context-menu tests](../../../src/FerroUI.Controls/rich_edit_context_menu_tests.rs) | Eighteen cases drive real pointer, Apps/Shift+F10 and holding requests, shared arguments, root DIPs at 2x scale, normalized/RTL/collapsed/wrapped selection anchors and actual popup positioner calls. Cancellation/precedence, menu installation/replacement/removal, nested requests, detach/reattach, weak anchor ownership, repeated positioning, reflection-correct point transforms and live IME/focus before/after native/overlay popup dismissal have explicit coverage. A separate focus-redirection fixture verifies retirement of the original composition. Native automatic selection floaties and legacy asynchronous PopupMenu are separate under RT-I22. |
| [Cursor publication tests](../../../src/FerroUI.Controls/rich_edit_input_geometry_tests.rs) | Three public input-manager cases deliberately return an old rectangle after getter callbacks change owner/client, reset, leave/return, or publish newer geometry for the same client. The accepted backend rectangle, connection and history are asserted without exposing private manager APIs. |
| [CoreText geometry tests](../../../src/Windows/FerroUI.WinUI/core_text_layout.rs) and [native adapter tests](../../../src/Windows/FerroUI.WinUI/win_ui_text_input_method_reentrancy_tests.rs) | Seven portable geometry cases cover complete rectangle transforms, reflection/rotation, scaling/origin, caret-only minimum, unavailable geometry, cancellation/reentry and native setter failure. Six Windows-only cases cover semantic ownership, accessor/revision replacement, deferred mutation, nested/focus/reset invalidation and a real attached owner resized by the final revision callback. Arbitrary requested-range and candidate-window geometry remain pending; native execution is recorded separately. |
| [Shared formatter marker regression](../../../src/FerroUI.Base/media/text_formatting/text_line_tests.rs) | The direct structural-terminator case covers LTR/RTL caret-distance/hit inverses, selected fragment bounds, nonzero source starts, one/two-unit line/paragraph markers and trailing marker positions. It failed against the preceding Base library and passed after the shared correction. |
| [System MSFTEDIT probe](../../../src/Windows/FerroUI.Win32/rich_text_tom_oracle_tests.rs) | Windows-only native TOM setup, 60 Home/End commands plus 60 repeated no-ops, and 282 Line/Paragraph MoveUp/MoveDown commands in the current source. The versioned records contain 264 and then 282 commands; the latter adds 18 fresh-control/sequence observations and confirms an extended-column difference still requiring a Rust correction. The vertical matrix includes directed selections, zero/positive/negative counts, empty and UTF-16 stories, boundary no-ops and retained-column sequences. Logs Windows/DLL versions, exact UTF-16 fixture text, native story length, directed endpoints/flags, HRESULT and signed deltas. Initial destination/count rows are observations; the probe is not a WinUI-engine conformance fixture. |
| [Text stream tests](../../../src/FerroUI.Controls/rich_text/rich_edit_text_stream_tests.rs) | UTF-8/UTF-16 and BOM handling, raw RTF, validation before I/O/mutation, exact byte bounds, interrupted and partial I/O, prefix failure, caller position/ownership, transactional range replacement and reentrant output snapshots. |
| [Language tests](../../../src/FerroUI.Controls/documents/rich_language_tests.rs) | Source validation grammar, inheritance into shaping, local overrides, snapshots/history, RTF default/scope/plain reset, representability errors and owner/presenter identity. |
| [Inherited presentation tests](../../../src/FerroUI.Controls/documents/rich_presentation_invalidation_tests.rs) | Theme/owner presentation preserves semantic revision, mutation identity, undo and active composition; font/language/flow and inherited mutable collections still invalidate layout. Explicit local formatting and clearing to inheritance remain semantic changes. The Base inheritance tests distinguish actual notification origin from the resulting inherited priority and preserve it in owned event arguments. |
| [Composition cause tests](../../../src/FerroUI.Controls/rich_text/rich_text_composition_tests.rs) | Accepted Unicode deltas, cursor filtering/normalization, one finalization/undo unit, explicit cancellation, read-only/reentrant rejection, group event ordering/rollback, stale preview invalidation inside held model batches and weak disposable observers. Routed argument ranges have the separate tests below; actual backend event order remains a platform gate. |
| [Composition event tests](../../../src/FerroUI.Controls/rich_edit_composition_event_tests.rs) and [begin lifecycle tests](../../../src/FerroUI.Controls/rich_edit_composition_start_tests.rs) | Native unavailable defaults and whole UTF-16 ranges; accepted filtered/limited completion payloads; no cursor-only changed events; exact disjoint/empty replacements; explicit begin before text without selected-text deletion/history; empty completion/cancel; legacy implicit start; reentrant caret/model/start/end callbacks and stale acknowledgement rejection. |
| [RTF style tests](../../../src/FerroUI.Controls/rich_text/rtf_styles_tests.rs) and [field tests](../../../src/FerroUI.Controls/rich_text/rtf_field_tests.rs) | Literal grammar fixtures for forward/cyclic inheritance, additive/scoped character formatting, font charset selection, direct overrides, names/metadata isolation, metrics/tabs/empty paragraphs and expansion limits. Multi-paragraph fields preserve targets, surrounding content and formatting, exclude invisible empty links, count separators, bound cumulative metadata expansion and reject before mutation during streamed import. Canonical export retains resolved appearance rather than style/field identity. |
| [Highlighter tests](../../../src/FerroUI.Controls/documents/text_highlighter_tests.rs) | All ten literal upstream overlap fixtures plus an independent per-character layering oracle, signed range value copies, markup boxing, empty/invalid clipping, brush defaults, mutable brush invalidation and weak/reentrant subscriptions. Native raster/high-contrast comparisons remain separate. |
| [Portable text automation tests](../../../src/FerroUI.Controls/automation/provider/rich_text_automation_tests.rs) | Six Text/eighteen TextRange operations, Text2 caret and TextChild contracts over the live model, scalar-safe text, range identity/rebasing/lifetime, selection policies, format and text search, unit normalization, 25 typed attributes/sentinels, stable hyperlink peers, children, events and shaped/clipped overflow geometry. The 92551b30 checkpoint covers scroll alignment and presenter ownership. TextEdit composition, payload and subscription additions require the later corresponding record. |
| [Native text pattern tests](../../../src/Windows/FerroUI.Win32/automation/text_pattern_tests.rs) and [range identity lifetime test](../../../src/Windows/FerroUI.Win32/automation/text_range_node.rs) | Native provider slots, worker-to-UI dispatch, BSTR/SAFEARRAY/VARIANT/HRESULT values, correct reserved sentinels/COLORREFs, geometry/children, document replacement, foreign ranges, text/selection events and delayed COM release. Test definitions and a cross-compile are not evidence of an actual Windows UI Automation client or screen-reader run. |

The source overlap fixtures preserved by
`text_highlight_merge_matches_all_source_overlap_cases` are **NonOverlapped,
OverlapNextPartial, OverlapPrevPartial, OverlapNextFull, OverlapEqualFull,
OverlapEqualPartial, OverlapEqualMultiple, OverlapInner, Overlap1 and
OverlapMultiple**. Their source test files remain `reference` entries with
`adapted_test_cases`; they are not counted as completed runtime source ports.

The `rich_` filter includes document, rich view/editor and portable automation
modules, but does not select the `text_highlighter_tests` module. Use a
`text_highlight` filter or the full controls suite for those tests. Native text
tests live in the Win32 crate and require their own command/result record.

Runtime/compiled XAML, catalog interactions, native Windows/macOS/Linux/browser
input and accessibility, exact font/DPI rendering comparisons, large-document
performance and externally implemented service conformance are additional gates
in [PLAN.md](PLAN.md). They remain unverified until recorded with their actual
commands or platform procedure and result.

## Continuation: shaped units, paragraph tabs, public peer events and casing

The published branch already includes the following focused fixtures. Their
source/member mappings are now reconciled with those actual implementations;
a historical gate is not evidence that every later checkpoint has run them.

- `rich_edit_text_units_tests.rs`: actual ligature/script Cluster boundaries,
  bidi logical order, wrapped buffer origins, embedded objects/soft breaks,
  omitted glyphs, linked pages, reflow and reentrant/malformed layout rejection.
- `rich_paragraph_tabs_tests.rs` and Base `text_tab_layout_tests.rs`: native enum
  values, sorted/replaced stops, points conversion, 63-stop validation, immutable
  cloning, rich snapshots/undo, default interval ownership, RTF/clipboard context,
  shaped fields across runs and objects, RTL decimal alignment, wrapping,
  justification, leaders, bars and cached shaping reuse.
- `automation_peer_text_edit_tests.rs` and Win32 `text_pattern_tests.rs`: the
  public event path without TextEdit patterns, runtime metadata, one-hop weak
  EventsSource, cycles/lifetime, UTF-16/NUL payloads, lazy provider identity and
  reentrant source changes.
- New `rich_edit_text_case_tests.rs`: native LetterCase values, Unicode
  expansions/directed selection, contextual sigma across formatted runs,
  paragraph/link/control preservation, astral/interior-caret positions, no-op
  revision/redo, read-only/limit rejection, nonexpanding over-limit documents,
  committed-content recapture after preedit cancellation and grouped rollback.

The focused case-conversion filter is `rich_text_case_`. Execution results for
this continuation are recorded only after a completed gate in the journal and
`mapping.json`; the fixture list alone does not assert a passed run.

### Exact input and CoreText state fixtures

`rich_edit_exact_input_tests.rs` has targeted `rich_exact_` regressions for
replacement ranges independent of selection, empty deletion, invalid UTF-16 and
policy rejection, active composition span, original-selection cancellation,
selection-only preview updates, intercepted completion, selection cancellation,
reentrant application edits and identical-text redo/format preservation.
`text_input_method_edit.rs` exercises the portable request's scalar boundaries.
`core_text_synchronization.rs` covers failed text/selection notification retry,
last-acknowledged length, reentrant state and replacement-client fences. The
CoreText state harness is compiled on the host; it is not an actual native
CoreText/IME execution or clause-format rendering test.

### Actual theme, command, markup and catalog fixtures

The theme tests use each production theme's compiled resources. Their common
six scenarios verify live template parts and one presenter, complete-story
scrolling and end-caret reveal, theme changes during history/composition,
retemplating, an opened owner command flyout with live enablement, and local
placeholder-brush precedence across pointer/focus/disabled states.

| Fixture | Evidence defined |
|---|---|
| [WinUI theme](../../../src/FerroUI.Themes.WinUI/tests/rich_edit_box_tests.rs) | Six actual-theme tests, including light/dark and high-contrast variants. |
| [Fluent theme](../../../src/FerroUI.Themes.Fluent/tests/rich_edit_box_tests.rs) | The same six scenarios against the Fluent dictionary. |
| [Simple theme](../../../src/FerroUI.Themes.Simple/tests/rich_edit_box_tests.rs) | The same six scenarios against the Simple dictionary. |
| [Formatting commands](../../../src/FerroUI.Controls/rich_edit_format_command_tests.rs) | Seven regressions for independent mixed state, uniform toggle/directed selection/undo, unrelated decorations, collapsed typing format, read-only/accelerator policy, reentrancy and preedit cancellation. |
| [Runtime XAML](../../../tests/FerroUI.Markup.Xaml.UnitTests/xaml/rich_text_tests.rs) | Four tests for nested document/highlighter values, actual-owner command bindings, two-way text binding and linked overflow. |
| [Compiled-emitter fixture](../../../tests/FerroUI.Markup.Xaml.UnitTests/emitter/fixtures/rich_text_document.xaml) | Shared document fixture registered in the existing runtime-versus-compiled differential corpus. |
| [Catalog page](../../../samples/ControlCatalog/Pages/RichTextPage.xaml) | Registered Text navigation page plus the sample's generated document tests and shown-page corpus. The page demonstrates editing, shared content and linked columns. |

These are authored FerroUI regressions and examples tied to the mapped native
contracts. They are not native source test cases copied verbatim. Runtime or
compiled-source fixture registration does not assert a passed run; completed
gates are recorded separately in the journal and validation records.

### RTF encoding fixtures

[rtf_encoding_tests.rs](../../../src/FerroUI.Controls/rich_text/rtf_encoding_tests.rs)
defines fifteen focused regressions. Literal byte fixtures cover all seventeen
supported code pages and canonical-writer text round trips. Other cases cover
multibyte characters split across raw/escaped tokens, escaped delimiter bytes,
font/page group state and `plain`, `cpg` precedence, DEFAULT versus ANSI charset,
font names/field instructions, Unicode fallback, incomplete sequences at semantic
boundaries, invalid UTF-8 without replacement, UTF-16 limits, repeated output
buffers and ignored field destinations. Native code-page/vendor behavior and
unsupported RTF content require separate conformance tests. A source fixture is
not a recorded runtime pass.

### Native counted-string and callback fixtures

Two new `exact_hstring_` tests in
[win_rt_tests.rs](../../../src/Windows/FerroUI.Win32/win_rt/win_rt_tests.rs)
cover astral/NUL preservation, borrowed ownership and strict rejection of isolated
surrogates. The WinUI host fixture
`native_text_request_preserves_exact_code_units_and_snapshot_lifetime` checks
that native queries retain the requested raw UTF-16 slice in an owned snapshot.
Four [native adapter regressions](../../../src/Windows/FerroUI.WinUI/win_ui_text_input_method_reentrancy_tests.rs)
cover client replacement from disposal, connection access, subscription
registration and state queries. Those Windows-only tests exercise the actual
adapter without a COM context or desktop window; their definitions are not
proof of execution or a real platform IME integration pass.
