# Rich text implementation journal

This journal records how the port is developed. [PROGRESS.md](PROGRESS.md) and
the reviewed mapping remain the authority for implementation status; a checkpoint
preserves work and does not establish complete parity.

## Reference and first published batches — 2026-10-17

The source reference is fixed at
[`fe244ca6ac4d482712b234a9de3cf771cc6481de`](https://github.com/microsoft/microsoft-ui-xaml/tree/fe244ca6ac4d482712b234a9de3cf771cc6481de).
The inventory retains all 230 files in `core/text`, 62 in `core/native/text`, and
23 in `components/text`. Repository inspection established that the public source
contains the RichEdit host and LineServices adapters, while the implementations
of `WinUIEdit.dll`, the external text object model, and the line engine behind
`Ptls-min.h` are external. Those boundaries are documented in [DEPENDENCIES.md](DEPENDENCIES.md).

The initial tracking batch was published as
[`656509591a045f50abb86807da7ecf11ad6375ce`](https://github.com/wieslawsoltes/FerroUI/commit/656509591a045f50abb86807da7ecf11ad6375ce).
It retained 609 source/reference files and 1,079 exact public XML identifiers.
Its reviewed map was empty; no runtime or parity success was asserted.

The core document, display/overflow, editor, RTF/clipboard, input-method and Rust
text object model batch was published as
[`432d10fb0335be3fb586290bf7467126a790646f`](https://github.com/wieslawsoltes/FerroUI/commit/432d10fb0335be3fb586290bf7467126a790646f).
The follow-up
[`fb15fa7bf618e21233edaf567a9ed0c0a5096e74`](https://github.com/wieslawsoltes/FerroUI/commit/fb15fa7bf618e21233edaf567a9ed0c0a5096e74)
corrected the hyperlink test to follow the source's Enter-on-key-release behavior.
Those published test files contain 21 document, 37 editor and 16 view test
definitions. Their presence is separate from the command/result records in
`mapping.json`.

## Workspace reconstruction and expanded audit — 2026-10-18

The execution environment disconnected before later work was checkpointed.
The restored environment did not contain that working tree. Work resumed by
cloning the published branch and the pinned source again. Later implementation,
mapping and documentation are reconstructed and reviewed against the actual
restored files; they are not represented as byte-for-byte recovery of unpublished
files. Previously observed local totals are not reused as current completion
claims.

The next inventory revision makes the block declaration, caret browsing,
hyperlink peer and text-options dependencies explicit. It also adds the native
UI Automation wrapper implementation/header, the generated native enum reference,
and the three automation model inputs. The public inventory now includes the
Text/Text2/TextRange/TextRange2/TextEdit/TextChild interfaces, their range units and
selection policy, text-attribute and annotation types, and text-specific pattern
and event dispatch members. Unsupported patterns and attributes stay pending;
expanding the denominator does not implement them.

The map is updated in implementation batches. Each reviewed entry names the
actual Rust symbol, its test evidence, and any observable translation. Runtime
test results are recorded only after their commands finish. Native Windows
execution, real input-method sessions, screen-reader interaction, exact rendering,
and external-engine contracts remain separate acceptance gates in [PLAN.md](PLAN.md).

## Continued source review and service contracts — 2026-10-19

The published branch was recovered at `2489efdcc60229a78eadbb52bcf4bc07bc5a3f86`
in an isolated checkout. Its GitHub CI failures were traced to a missing unsafe
block around an owned-BSTR COM test call; the repair was published as
`1b4704b2f96ac844dbfbe48b96fa5621e12e3ed3`.

Paragraph spacing and signed indentation were published as
`32b19d14facbf06a94c9715f1120d8e2a5b75a90`. This batch adds native-valued spacing
rules, point conversion, actual line/overflow behavior, signed indent storage,
RTF multiple spacing and eleven independent regressions. The enum values have
complete member mappings; the broader paragraph/native formatter contracts do
not inherit that completion status.

The document-service, edit-event and accessibility batch was published as
`871d566d25aab3e89021d7fc5db6edf32c07267e`. It adds shared default formats,
explicit get/set/find flags, native newline fixture assertions, UTF-32 and range
operations, cancellable/reentrant selection, synchronous TextChanging/deferred
TextChanged, framework content presenters, Text2 caret and TextChild hyperlink
providers. The integrated rich-text filter passed all 181 tests. Initial fixture
failures were corrected by using the existing per-thread test scope, installing
the framework keyboard device for focus tests and performing layout before
inspecting the automation visual tree. Production behavior was not weakened to
make those assertions pass.

The Win32 text-related filter subsequently passed 34 tests, including ten text
pattern/range bridge cases. Its first compilation caught a pattern-array arity
mismatch after TextChild was added; `f1a051f8eaddd2d9b651a731206eebd9fa3a252a`
repairs the complete twelve-interface registration. These runs occurred on a
Linux host using the portable Win32 test harness; they are not Windows
screen-reader or real native accessibility-client execution.

The mapping audit also found three historical test symbols absent from the
published sources. The ellipsis entry now points to an existing executed test,
and two absent scroll-test references were removed pending real regression
coverage. The generator checks every retained target and test symbol. No
unpublished test definition is treated as recovered or executed evidence.

Further source review established that real templates require RichEditBox to use
FerroUI TemplatedControl with a persistent rich presenter, not inherit the display
control's finite-height pagination behavior. It also distinguished display
batching from model notification batching, and native tab stops from the current
formatter's fixed-width tabs. These are implementation tasks, not stored-property
substitutes. The next batch preserves one document/editor while completing the
template/presenter, scrolling and stream-service boundaries.

## Persistent editor view, streams, units and language — 2026-10-19

The next implementation checkpoint makes RichEditBox a FerroUI TemplatedControl
with one persistent RichEditBoxTextPresenter. The model, directed selection,
composition and undo history survive template replacement. Header, placeholder
and description use ordinary framework presenters; document extent, scroll
offset, caret, IME, hit testing and automation share the presenter's coordinates.
Automation retains the semantic editor owner and exposes no duplicate document
through its raw presenter peer. Both physical and logical scroll alignment use
the last selected line and the established viewport.

Document and range streaming use bounded, explicit UTF-8/UTF-16 Read/Write
operations and the existing RTF parser, with complete validation before editing.
TOM Character, Word, Sentence, Paragraph, Line, Story and HardParagraph units now
have separate native-valued service contracts; Unicode navigation/search remains
distinct. An unmeasured editor falls back from Line to Paragraph, while a
measured presenter supplies current display-line boundaries. Cluster and other
unimplemented native units are not aliases for Unicode graphemes.

Inherited TextElement.Language now reaches FerroUI shaping and fallback through
CultureInfo, with the pinned source's XML-language validation and application
language default. Character-format snapshots, editor history and supported RTF
language controls preserve the tag. Windows locale resolution, the complete
LCID namespace and dual proofing languages remain separate native contracts.

The integrated command
`cargo test --locked --offline -p ferroui-controls --lib rich_ --no-fail-fast`
completed with **246 passed, 0 failed, 4434 filtered out**. New coverage consists
of 15 stream, 19 TOM unit, 8 live-pointer, 6 language, 10 presenter/control and 7
automation tests. A real double-rebase bug in batched range replacement was fixed
by updating internal weak live positions at each model replacement while keeping
public document notifications batched. Tests cover nested edits, rollback,
undo/redo, reentrant callbacks and weak lifetime. Geometry fixtures now execute
the actual layout manager, and the unmeasured Line fixture uses wrapping rather
than a line break that was also a TOM paragraph boundary.

This checkpoint does not assert complete native parity. Theme templates,
compiled markup, TextEdit composition notifications and the remaining external
engine contracts continue in subsequent implementation and validation batches.

## Accepted composition, TextEdit and owned flyout focus — 2026-10-19

Published as [`ed836e831fc8af5114e2ce8ae7632867c7a2332f`](https://github.com/wieslawsoltes/FerroUI/commit/ed836e831fc8af5114e2ce8ae7632867c7a2332f).
The editor publishes accepted preview deltas and one finalization, after its
normalization, filtering and UTF-16 limits. Cursor-only movement emits no text
cause. Explicit cancellation finalizes with empty text; successful commit creates
one history unit without an intermediate cancel. Nested groups queue causes and
discard them on rollback; an internal mutation identity rejects stale composition
even before a surrounding model batch publishes its revision.

Portable TextEdit inherits Text independently of Text2, exposes the full live
composition range and disposes/rewires observations with the owner/document.
A conversion target remains absent until a backend provides that distinct range.
The Windows bridge adds the SDK interface and dispatcher-based BSTR SAFEARRAY
payloads. Array creation now cleans up initialized native slots on conversion
failure and releases locks during unwinding. Length-aware BSTR allocation retains
embedded NUL text. Public arbitrary-peer raising, EventsSource and lazy native
activation are a subsequent contract, not inferred from editor-provider events.

RichEditBox tracks the actual focused descendant and current target of its owned
context flyout. Active selection/composition survive that transition; focus
return, closure, replacement or genuine blur clears weak subscriptions. Generic
content hosts reveal the caret through the presenter's own logical scrolling.
The control's retained visual/model state does not alone retain a native IME
transport across a different popup root; the framework owner protocol is next.

The full controls command passed **4706 tests, zero failures**. The initial run
had six remote display failures because sandboxed loopback listening was denied;
all eight remote tests passed with loopback access, followed by the successful
full-suite rerun. One new caret-boundary assertion exposed only roundoff:
60.00000000000001 versus a viewport height of 60. The test now uses FerroUI's
existing geometry comparison tolerance and retains diagnostic values; production
scroll geometry was not padded to satisfy that assertion.

The Win32 `--lib text_` command passed **41 tests, zero failures** on Linux,
including four TextEdit and three array cleanup cases. CI on the previous
92551b30 checkpoint identified the missing published themes, stale generated Rust
path fixture and a scanner-unresolved `Self` reference in a highlighter type
registration. The equivalent explicit `TextHighlightRange` registration passed
the source scanner regression. Theme and generated-path verification follow in
the next checkpoint. Native OS/client and pixel-conformance evidence remains
separate from these results.

The reviewed map now includes the AutomationPeer partial and UIAWindow source
pairs: **630 selected sources, 919 additional framework dependencies, 806 include
boundary edges and 1258 exact public identifiers**. New mappings cover the
published presenter, streams, native-valued units, immediate pointers, inherited
language and TextEdit behavior. The previous Cluster/grapheme equivalence claim
has been removed. Twenty complete entries are enum fields; no source file or
entire rich-text subsystem is declared complete by that count.

## Inherited presentation versus document edits — 2026-10-19

The first complete WinUI theme run reached 293 passing tests but failed the new
opened rich-editor command-flyout test: undo was disabled after the theme was
applied. A diagnostic assertion localized the loss before the flyout opened.
The source trace was `TextElement` inherited font/foreground invalidation through
`BlockInlineHost`, followed by a document revision and invalidation of editor
history. The same path could invalidate an accepted composition when focus or
theme presentation changed.

The fix adds inheritance provenance to borrowed and owned property-change
arguments, independently of resulting binding priority. Ancestor propagation and
inheritance-parent changes set this flag; clearing a local document value does
not, even when the new effective value is inherited. A separate inline-host
layout route now reaches the view without notifying document observers. Local
formatting remains a semantic mutation. Inherited feature and decoration
collections use the same distinction. Existing snapshot capture already used
base values, so it retains inheritance across undo rather than freezing the
previous theme into restored runs.

`cargo test --locked --offline -p ferroui-controls --lib rich_ --no-fail-fast`
passed **276 tests, zero failures**, including four new regressions covering
preview/history retention, actual layout refresh, inherited mutable formatting,
and local formatting inside a model batch. The Base `--lib inheritance` command
passed **34 tests, zero failures**, including assertions distinguishing parent
propagation from clearing a local value and checking owned arguments. Only
formatting of the new test file followed these runs. The real-theme rerun and
the remaining theme/markup/workspace gates are recorded in their own subsequent
checkpoint; these focused results do not establish those gates.

## 2026-10-20 — Published-state recovery, TOM casing and map reconciliation

Recovered PR #243 at `86ee377d`. Earlier unpublished local files were not
available in the new workspace, so new work is built from the published source.
Reconciled the source/member map with the already published shaped Cluster,
explicit/default paragraph tab, and public AutomationPeer TextEdit/EventsSource
implementations. Source files remain partial; the map retains the full pinned
inventory and does not turn lexical counts into parity percentages.

Added a dedicated structured `ChangeCase` implementation and native-valued
`LetterCase`. Complete selected-string Unicode casing is partitioned back into
existing inline structure, preserving cross-run final-sigma context, expanding
characters, paragraph/link/control content, and directed UTF-16 selection.
No-op edits preserve history/revision; successful edits form one undo unit.
Over-limit expansions fail before truncation; nonexpanding changes preserve an
already-over-limit document. Changed programmatic operations recapture committed
content after preedit cancellation. RT-I15 records the exact portable casing and
native locale/position boundaries.

Validation is pending at the time of this journal entry. The task-owned baseline
build was stopped to release memory while unrelated compiler jobs used the shared
environment; the stopped run is neither a pass nor a failure.
Focused and final integration results will be appended after they complete.

The same continuation adds exact UTF-16 range/text/resulting-selection input
requests, independently retained composition spans and generation/revision-aware
CoreText acknowledgement recovery. Empty replacement deletion, selection-only
updates, cancelled/reentrant acknowledgement and last-successful native lengths
have dedicated fixtures. Unsupported native clause formatting explicitly fails;
the legacy TextBox route and native OS execution remain tracked separately.

Source-map reproduction completed successfully after fetching the missing pinned
Git blobs in one batch: 630 source/reference files, 1,258 public identifiers, 919
framework dependencies and 806 include boundaries. A subsequent `--check` was
clean. These are source integrity/reconciliation gates, not runtime test results.

The continuation also supplies compiled RichEditBox themes for WinUI, Fluent and
Simple; owner-bound formatting commands preserve independent mixed state,
unrelated underline decorations and one history transaction. New actual-theme,
runtime/compiled XAML and catalog fixtures cover the framework integration.
`TextCommandBarFlyout.cpp` and its header are newly selected pinned source inputs,
so their command behavior and transitive include boundaries are now part of this
rich-text map. The native query/set branches were read directly before recording
the mapping. RT-I17 records the MenuFlyout/palette translation and remaining
native touch, lazy-part, localization and appearance contracts. Runtime gates
for these additions are pending until their completed results are appended.

The RTF reader now shares an incremental, non-replacing byte decoder across body
text, font names and field instructions. Seventeen portable code pages have
literal multilingual fixtures; font charset/code-page precedence, default-font
resets, Unicode fallback, malformed boundary handling and decoded UTF-16 bounds
are separately covered. `encoding_rs` is pinned to 0.8.42 with default features
disabled. The public Text XML remains a reference-only specification because the
native RTF engine is external; the reader/writer/stream API rows now link the
actual codec implementation and fixtures. RT-I18 preserves Symbol/OEM/Johab,
unlisted pages, vendor/native mappings, original-byte roundtrip and unsupported
full-content boundaries. Runtime gate results follow only when completed.

The native boundary follow-up retains exact counted UTF-16 query slices and
embedded NULs in owned HSTRING values while strictly rejecting malformed edit
payloads. Callback generation fences cover client access, subscription setup,
disposal and state queries. Host fixtures and Windows-only actual-adapter
regressions are linked separately from native execution evidence.

After adding the native text-command source/header and all continuation anchors,
the regenerated record and `--check` passed with **632 selected files, 1,258
public API identifiers, 942 additional framework dependencies and 826 include
boundary edges**. The map records 36 complete API entries, 428 partial and
794 pending entries; no runtime source file is marked complete. These numbers
measure the mapped inventory, not a percentage of functional parity. The source
integrity command and its actual result are recorded in `mapping.json`.
