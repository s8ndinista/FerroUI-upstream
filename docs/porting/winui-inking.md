# WinUI inking port

This is the source, architecture and compatibility record for the inking work in
`src/FerroUI.Controls/inking`. The goal is the full WinUI control surface with
FerroUI input, rendering, properties, layout and accessibility. The work includes
portable replacements for services supplied by Windows rather than present in
the WinUI source. Those replacements have explicit compatibility boundaries;
this document does not treat a matching API name as proof of OS engine parity.

The source/member ledger is authoritative for the remaining work. In particular,
the complete WinUI templates, all localized resources, native ISF/clipboard
interoperability and independent Windows ink-thread behavior are not established
by the portable control implementation. The PR remains a draft while those
requirements and the recorded validation gaps are unresolved.

## 1. Pinned sources and accounting

- WinUI repository: <https://github.com/microsoft/microsoft-ui-xaml>.
- WinUI revision: `9427b7216337ea3d7b19ac32d48c327286ee7138`, committed
  2026-10-21 at 04:22:04 UTC.
- FerroUI starting revision: `86b6b4ab41669bf830fba681504a4ee94752a4e6`.
- Branch: `feat/winui-inking-port`.
- Review: <https://github.com/wieslawsoltes/FerroUI/pull/283>.

The exact source roots are
[`controls/dev/InkCanvas`](https://github.com/microsoft/microsoft-ui-xaml/tree/9427b7216337ea3d7b19ac32d48c327286ee7138/controls/dev/InkCanvas)
and
[`controls/dev/InkToolbar`](https://github.com/microsoft/microsoft-ui-xaml/tree/9427b7216337ea3d7b19ac32d48c327286ee7138/controls/dev/InkToolbar).
They contain 164 files, totaling 2,040,931 bytes at this revision:

| Category | Files | Accounting decision |
|---|---:|---|
| Control implementation/header files | 32 | Mapped individually; small related C++ declarations may share a Rust file. |
| Presenter/service implementation/header files | 2 | Split into presenter, input, events, container and synchronizer modules. |
| Public IDLs | 6 | All 45 types and 181 declared members retained in the member ledger. |
| Automation implementation/header files | 9 | Dedicated or inherited FerroUI peers identified separately from visual controls. |
| Button manager, color names and flyout support | 4 | Mapped to the equivalent control orchestration and helpers. |
| XAML templates/theme resources | 2 | Full upstream visual templates remain separately tracked. |
| Localized string resources | 86 | Every locale remains in the inventory; English labels do not count as locale parity. |
| API-test suites | 2 | All 65 test methods enumerated, with exact Rust test mappings where available. |
| Interaction-test suites | 2 | Kept separate from in-process API tests. |
| Test-application XAML/code-behind | 4 | Samples do not count as implemented controls or tests. |
| Diagnostics header | 1 | Windows telemetry infrastructure is tracked separately from control behavior. |
| C++/MSBuild project files | 14 | Not applicable to the existing Cargo crate. |

The data is under [`data/winui-inking`](data/winui-inking/README.md):

- `source-map.json` keeps every path, Git blob hash, byte size, category and
  disposition.
- `api-map.json` retains every declared type/member, enum value, target file,
  symbol and behavioral note.
- `tests-map.json` keeps the upstream test scenarios and local evidence.
- `dependencies.json` distinguishes required OS dependencies from adjacent
  recognition/analysis services.
- `upstream/*.idl` are exact snapshots of all six IDLs, verified using their
  original Git blob hashes.

The implementation follows [`PORTING-GUIDE.md`](PORTING-GUIDE.md). The source is
MIT licensed; the complete Microsoft notice is retained beside the implementation
in [`NOTICE.md`](../../src/FerroUI.Controls/inking/NOTICE.md). No OS implementation
or Windows binary is copied into the portable library.

## 2. What WinUI actually supplies

WinUI's `InkCanvas` hosts an external composition visual. It constructs the COM
`InkDesktopHost`, creates an OS `Windows.UI.Input.Inking.InkPresenter` on that
host's ink thread, and attaches it through DirectComposition/SystemVisualLink.
Its `InkPresenter` is a proxy: UI-thread properties are cached, mutations are
queued, thread-affine service calls are marshaled, and input/stroke events are
posted back to the UI dispatcher.

The source of the stroke renderer, curve-fitting algorithm, native pencil
texture, ISF serializer, handwriting recognizer and OS stencil implementation is
not part of these WinUI directories. `InkStroke` and `InkDrawingAttributes`
remain Windows Runtime types in the pinned IDL. A portable port therefore needs
both the visible WinUI controls and an independently implemented service layer.

FerroUI replaces the host integration with ordinary framework drawing and
pointer routing. The controls can use whichever backend the application already
uses. The portable engine has no COM dependency and does not create a second
windowing, property or layout system. This also means that passing portable tests
does not establish Windows independent-thread latency or identical OS pixels.

## 3. Implementation organization

All paths below are relative to `src/FerroUI.Controls/inking`. Their exact status
and individual missing members are in the ledger.

| Upstream responsibility | Rust organization | Framework services |
|---|---|---|
| InkCanvas visual and input host | `ink_canvas.rs` | `Control`, routed pointer events, capture, layout, visual invalidation and drawing context |
| Presenter proxy and collected/erased args | `ink_presenter.rs` | Shared service identity, event subscriptions and invalidation |
| Processed/unprocessed input and configuration | `ink_input.rs`, `ink_events.rs` | FerroUI pointer types and explicit `IDisposable` subscriptions |
| Wet-to-dry handoff | `ink_synchronizer.rs` | Explicit Rust transaction state instead of an OS thread-affine proxy |
| OS point/drawing-attribute value contracts | `ink_point.rs`, `ink_drawing_attributes.rs` | `Point`, `Size`, `Matrix`, `Color` and validation errors |
| OS strokes and construction | `ink_stroke.rs`, `ink_stroke_builder.rs` | Cached portable geometry and immutable input samples |
| OS stroke collection and persistence | `ink_stroke_container.rs`, `persistence.rs` | Observable changes, selection, transforms and bounded byte parsing |
| Ruler/protractor state and constraints | `ink_presenter_stencil.rs` | FerroUI geometry/transforms, with native manipulation parity tracked separately |
| Toolbar orchestration and button manager | `ink_toolbar.rs` | Existing panel/children collection, styled properties and events |
| Tool/pen/toggle/menu subclasses | `ink_toolbar_buttons.rs` | FerroUI radio, toggle and checkbox control bases |
| Public toolbar enums | `ink_toolbar_types.rs` | Rust enums preserving upstream values and XAML names |
| Color labels/palettes | `color_names.rs` and pen-button code | FerroUI brushes and colors; localization completeness tracked separately |

The shared toolbar module is an intentional file mapping: the small C++ headers
share control inheritance and boilerplate, while the source ledger retains their
individual identities. The host's formerly combined presenter header is split by
service responsibility. Neither organization removes an upstream member from
the completeness accounting.

### Ownership and reentrancy

| Object | Rust representation and lifetime |
|---|---|
| Device samples and drawing defaults | Validated value snapshots. Later default changes do not mutate completed ink. |
| Stroke | Shared `Rc<InkStroke>` identity with immutable raw points, selected state, attributes and transform. |
| Presenter and service objects | Shared `Rc` values; child services hold weak presenter references. Repeated access returns stable service identity. |
| Canvas and toolbar controls | FerroUI `Ref<T>`/`WeakRef<T>`, class macros and registered property metadata. |
| Event subscription | Explicit disposable handle; callbacks run without a live mutable `RefCell` borrow. |

The code reuses existing FerroUI patterns rather than carrying C++ reference
tracking, HRESULT, boxing or COM apartment semantics into Rust. Input validation
and malformed persisted data return `InkError`. Core algorithms avoid filesystem
access, blocking waits and an assumed thread runtime.

### Input processing

The actual upstream default is **pen only**. Mouse and touch are opt-in through
`CoreInputDeviceTypes`; a comment in the upstream test page saying that all three
work by default does not match its presenter initialization.

Mode starts at `Inking`; `RightDragAction` starts at `LeaveUnprocessed`. Barrel
and eraser input are enabled. Processed strokes expose started, continued,
ended and canceled events. Unprocessed input exposes entered, hovered, exited,
pressed, moved, released and lost events. Capture loss, disabled input and
detachment must cancel pending input without committing a stray stroke.

Input samples retain their pressure, tilt, twist and timestamps. Availability of
actual device measurements depends on the existing FerroUI platform backend;
constructing a sample with these fields is not evidence that every platform
driver supplies them. Coalesced sample delivery and hardware pen testing remain
separate validation requirements.

### Stroke geometry and rendering

The portable renderer constructs pressure-dependent circular or rectangular nib
geometry using FerroUI points and matrices. It preserves raw samples, caches
derived smoothing/outline/bounds data and invalidates those caches when geometry
inputs change. Selection alone does not need to rebuild tessellation.

Highlighter contours need to be filled as one stroke geometry, so overlapping
pieces of the same stroke are composited once. Separate highlighter strokes may
still overlap. Pencil opacity and tilt response are portable approximations;
native pencil grain and DirectInk's exact fit/rasterization are not claimed.

The geometry and persistence tests must include isolated points, repeated
samples, pressure zero, extreme valid coordinates, transforms, sparse segment
eraser intersections and malformed data. Rendering every captured sample as an
unconnected dot would not preserve the stroke hit-testing or appearance contract.

### Custom drying

`activate_custom_drying` returns a stable synchronizer. `begin_dry` snapshots a
batch of committed strokes; `end_dry` releases only that batch from the wet
presentation. Strokes arriving during an open transaction remain pending for the
next one. Invalid nesting/order returns an error instead of silently dropping
ink.

The upstream OS rejects activation after the first nonzero size activates its
input. Any portable timing difference is called out in the API record. Several
upstream comments mention replacing a presenter for runtime activation, but the
pinned executable implementation performs the stricter OS call. Comments alone
are not used as behavioral evidence.

### Persistence and clipboard

The portable format and native Windows formats are distinct compatibility
contracts. A bounded binary stroke document can preserve IDs, attributes,
samples and transforms without being ISF. Unsupported native formats must fail
explicitly. Loads must validate the complete document before replacing visible
ink, and reject duplicate identities, malformed sizes and nonfinite geometry.

Native ISF and GIF-with-embedded-ISF need a codec or a Windows adapter and
Windows-produced fixtures. Clipboard support must state which data format it
publishes; internal stroke copies and a FerroUI payload do not prove a Windows
application can paste the data. The
[Microsoft ISF specification](https://learn.microsoft.com/en-us/uwp/specifications/ink-serialized-format)
and [WPF's serializer](https://github.com/dotnet/wpf/tree/main/src/Microsoft.DotNet.Wpf/src/PresentationCore/MS/internal/Ink/InkSerializedFormat)
are possible additional references, not code already imported by this port.

## 4. Toolbar behavior and visual scope

The toolbar has independent per-pen palettes and widths. `All` creates ballpoint,
pencil, highlighter, eraser and stencil controls; `PensOnly`, `AllExceptPens` and
`None` retain their upstream meanings. The first available pen becomes active.
Pen tools select Inking mode, eraser selects Erasing, and custom non-pen tools
select None so the previous pen does not continue drawing.

An explicitly set `TargetInkPresenter` wins over `TargetInkCanvas`. Clearing it
falls back to the canvas presenter. Target changes must remove stale stencil
state from the previous target. Color/width changes update the active presenter's
defaults. `EraseAllClicked` fires before clearing to let an undo handler snapshot
the collection, and the toolbar restores the most recently used non-eraser tool.

The intended highlighter uses the six-color upstream palette, starts at yellow
`#FFE600`, has selected width 16, and uses a rectangle nib `(width / 3, width)`.
The pinned button helper implements this, but its toolbar code builds generic
square attributes without dispatching to the helper. Using the chisel helper
and the custom-pen virtual is recorded as a deliberate correction.

The full upstream `InkToolbar.xaml` is 130,900 bytes. Reusing FerroUI control
bases provides usable framework controls, but it does not port every visual
state, pen glyph, flyout, popup placement rule, animation or template part in that
file. The theme records remain open until those elements have concrete mappings
and visual comparisons. Likewise, default English names do not stand in for the
86 localized resources.

## 5. Accessibility and upstream limitations

The canvas peer should report a Pane even when no strokes exist. Its visible
bounds must follow layout, its own clip, ancestor clip transforms, scroll
viewports and root clipping. A fully clipped or hidden canvas is offscreen;
partially visible ink remains discoverable. The upstream tests specifically
exercise these cases because a composition-only canvas otherwise reports no
rendered content.

Toolbar accessibility includes tool selection, toggle state, expand/collapse
for configuration flyouts, accessible numeric width values and localized color
help. Reusing a base peer covers only the patterns that base actually implements.
Dedicated upstream peers and their tests remain explicit ledger records.

The pinned upstream toolbar itself documents lift gaps: `IsCustomDry` returns
false, and target changes omit mid-stroke tracking, an internal high-contrast
notification and independent input inspection. A FerroUI correction is useful
but must be documented as a correction, not attributed to unavailable upstream
implementation. Native recognizers and `InkAnalyzer` are adjacent Windows Ink
services; neither is called by the two pinned control directories.

## 6. Verification and completion gates

The ledger checker runs without Rust or platform services:

```sh
python scripts/inking/check_port.py
python scripts/inking/check_port.py --json
python scripts/inking/check_port.py --require-complete
```

It verifies all 164 source identities are retained, the six IDL snapshots retain
their Git blob hashes, every IDL type/member is represented, and mapped files,
symbols and tests exist. A successful ordinary check means the record is
internally consistent. `--require-complete` additionally rejects partial,
unported and external entries. Neither mode executes behavior or compares pixels.

Rust validation is targeted at the controls crate and inking modules:

```sh
cargo check -p ferroui-controls
cargo test -p ferroui-controls --lib inking::
```

Execution results belong in the PR work log after the commands complete. A test
listed in `tests-map.json` is a scenario mapping, not an assertion that a test
run happened. Do not count a constructor test as coverage for an upstream
clipping, localization or accessibility test.

The following evidence is needed before a full compatibility claim:

1. All required public members and service adaptations reconciled against the
   pinned IDL and source, with no unrecorded omissions.
2. Pointer lifecycle, reentrancy, selection, erasing, persistence and custom-dry
   transaction tests passing on the portable implementation.
3. XAML loading/type registration and a real interactive canvas/toolbar sample.
4. Render comparisons for tools, highlighter overlap, themes, flyouts, stencils
   and high contrast; accessibility pattern tests and locale resource coverage.
5. Native-format fixtures proving the advertised ISF and clipboard interop, if
   those formats are exposed as supported.
6. Windows and browser builds plus physical pen/coalesced-input evidence for any
   device or latency guarantee.

## 7. Work log

1. Pinned WinUI and FerroUI revisions, inspected source/IDL/test ownership, and
   separated the OS service dependencies from the available control sources.
2. Created `feat/winui-inking-port`, the initial tracking document and draft
   PR #283 before implementing the control family.
3. Recorded the complete 164-file inventory, six immutable IDL snapshots,
   45-type/181-member map, 65-test map and dependency ledger.
4. Added Microsoft attribution and a reproducible ledger check; implementation
   and test mappings are updated with each verified batch.
