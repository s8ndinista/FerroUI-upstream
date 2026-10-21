# WinUI Charts: public-contract port

## 1. Scope and source boundary

This work implements the Charts control introduced in
[WinUI discussion 12343](https://github.com/microsoft/microsoft-ui-xaml/discussions/12343)
as idiomatic Rust controls using FerroUI. The reference is pinned to
`microsoft/microsoft-ui-xaml` commit
[`9427b7216337ea3d7b19ac32d48c327286ee7138`](https://github.com/microsoft/microsoft-ui-xaml/tree/9427b7216337ea3d7b19ac32d48c327286ee7138).
The initial FerroUI base is `86b6b4ab41669bf830fba681504a4ee94752a4e6`.

**The public upstream tree does not contain the Charts implementation.** It
contains API tests, interaction tests, and engineering samples. The pinned
[ChartApp README](https://github.com/microsoft/microsoft-ui-xaml/blob/9427b7216337ea3d7b19ac32d48c327286ee7138/Samples/ChartApp/README.md)
states that Charts ships in `Microsoft.Internal.WinUIDetails`, that the public
build's pinned package does not include Charts, and that public pull-request
builds skip the sample. Inspection of the complete pinned Git tree found no
Charts implementation files outside these test and sample roots.

This is a port of the **published API and observable contract**, with translated
tests and adapted examples. Its rendering implementation uses FerroUI services;
it is not a translation of unpublished C++ source. Passing property, collection,
or layout stability tests does not prove pixel equality with Microsoft's
renderer or equivalence to unspecified private algorithms. This boundary
remains explicit even when every available test has a Rust counterpart.

The public API reference is
[Microsoft.UI.Xaml.Controls.Charts](https://learn.microsoft.com/en-us/windows/windows-app-sdk/api/winrt/microsoft.ui.xaml.controls.charts?view=windows-app-sdk-2.0-experimental).
The API is experimental. The pinned source tests are the reproducible
behavioral reference for this port.

## 2. Public surface and Rust map

The public namespace has 14 classes and seven enums. Rust files below are the
implementation destinations; file existence alone does not establish verified
parity. Section 5 describes the explicit verification map.

| Public type | File in `src/FerroUI.Controls.Charts/` | Responsibility |
| --- | --- | --- |
| `Chart` | `chart.rs` | Control lifecycle, layout, collections, legend, rendering and invalidation |
| `Axis` | `axis.rs` | Visibility, title and chart ownership |
| `CartesianAxis` | `cartesian_axis.rs` | Tick labels/marks, grid lines and brushes |
| `CategoryAxis` | `category_axis.rs` | Source-index/value ordering and direction |
| `DateTimeAxis` | `date_time_axis.rs` | Date bounds, calendar intervals and labels |
| `LinearAxis` | `linear_axis.rs` | Numeric bounds, spacing, validation and ticks |
| `CartesianSeries` | `cartesian_series.rs` | Abstract shared series properties, samples, axes and overrides |
| `LineSeries` | `line_series.rs` | Line presentation |
| `AreaSeries` | `area_series.rs` | Filled area presentation |
| `BarSeries` | `bar_series.rs` | Horizontal and vertical bar presentation |
| `Samples` | `samples.rs` | Shared source data and changes |
| `DataLabelOverride` | `data_label_override.rs` | Immutable text/brush descriptor |
| `DataMarkerOverride` | `data_marker_override.rs` | Immutable marker/brush descriptor |
| `XamlChartsResources` | Resource integration | Theme resources; adapter shape tracked by markup tests |

| Enum (`enums.rs`) | Values, in upstream order |
| --- | --- |
| `BarOrientation` | `Horizontal`, `Vertical` |
| `CategorySortKey` | `Index`, `Value` |
| `DateTimeIntervalType` | `Auto`, `Day`, `Week`, `Month`, `Year` |
| `GridLines` | `None`, `Major`, `Minor` |
| `MarkerShape` | `None`, `Square`, `Diamond`, `Triangle`, `X`, `Asterisk`, `ShortDash`, `LongDash`, `Circle`, `Plus` |
| `SortOrder` | `Ascending`, `Descending` |
| `StrokeDashStyle` | `Solid`, `Dash`, `Dot`, `DashDot`, `DashDotDot` |

`MarkerShape` has numeric values 0 through 9 in the order shown. Typed Rust
enums do not manufacture invalid discriminants to reproduce arbitrary managed
integer-to-enum casts.

### 2.1 Properties and defaults

| Owner | Contract |
| --- | --- |
| `Chart` | `ShowLegend = false`; `LegendTitle = ""`; `Series`, `Axes`, `Data` empty with stable collection identity; content property `Series` |
| `Axis` | `IsVisible = true`; `Label = ""` |
| `CartesianAxis` | `ShowTickLabels = true`; `ShowTickMarks = false`; `GridLines = None`; five brushes unset |
| `CategoryAxis` | `SortKey = Index`; `SortOrder = Ascending` |
| `LinearAxis` | `Minimum`, `Maximum`, `Spacing` unset, selecting automatic values |
| `DateTimeAxis` | Bounds unset; `IntervalType = Auto`; `LabelFormat = ""` |
| `CartesianSeries` | `Title = ""`; `IsVisible = true`; samples, axes and `Stroke` unset; `StrokeThickness = 1`; `StrokeDashStyle = Solid` |
| Series presentation | Labels/markers disabled; `MarkerShape = Circle`; label/marker brushes unset; override maps empty |
| `AreaSeries` | `Fill` unset |
| `BarSeries` | `Fill` unset; `Orientation = Horizontal` |
| `Samples` | `ItemsSource` unset; content property `ItemsSource` |

The X data role is always category/date data and Y is always numeric values,
including horizontal bars. Orientation changes display axes, not axis-type
compatibility. `XAxis` accepts `CategoryAxis` or `DateTimeAxis`; `YAxis`
accepts `LinearAxis`.

Automatic axes and brushes are effective rendering choices. They must not be
written back into public `XAxis`, `YAxis`, `Stroke`, or axis brush properties.
Upstream exposes axis references as plain properties without dependency
property identifiers and excludes both override maps from its XAML members.
Native metadata adaptations are recorded in section 4.

### 2.2 Validation and ownership

- A series or axis belongs to at most one chart and cannot occur twice in one
  chart. Removal releases ownership so it can be reused.
- Explicit axes must be present in the target chart before adding a series
  that references them. Rejected assignments preserve the old reference.
- Invalid indexes or ownership conflicts preserve the collection and emit no
  event. Setting an item to itself and clearing an empty collection are no-ops.
- Linear bounds are finite and strictly ordered. Spacing is finite, positive
  and no larger than a fully explicit range. Rejected values retain the old
  state; clearing a value selects automatic behavior.
- Date bounds are strictly ordered and support pre-1900 dates, including 1601.
  Calendar intervals select tick cadence; they do not aggregate samples.
- Upstream retains negative/nonfinite `StrokeThickness`. Rendering sanitizes
  the effective pen without mutating the public value.
- Overrides use zero-based source indexes. Out-of-range keys survive data
  growth, sample replacement and series removal.

### 2.3 Layout, resources and accessibility

Finite available dimensions are requested unchanged, including zero. Infinite
dimensions fall back to 160 by 96, subject to minimum-size constraints.

| Axis property | Resource key |
| --- | --- |
| `GridLineMajorBrush` | `ChartsGridLineMajorBrush` |
| `GridLineMinorBrush` | `ChartsGridLineMinorBrush` |
| `TickBrush` | `ChartsTickBrush` |
| `TickLabelBrush` | `ChartsTickLabelBrush` |
| `AxisLineBrush` | `ChartsAxisLineBrush` |

Resolution respects local/application resources and theme changes while
leaving unset local properties unset. Brushes use FerroUI's `IBrush` abstraction;
the chart does not reject a brush solely because it is not solid.

Published automation tests require `LabeledBy` naming, explicit-name
precedence, full-description precedence and peer queries that do not measure
or arrange an unloaded chart. Existing FerroUI automation is the integration
point. Those tests do not establish per-point selection, hover tooltip or
keyboard-navigation behavior.

## 3. Framework and dependency mapping

The implementation follows [PORTING-GUIDE.md](PORTING-GUIDE.md) and existing
external control libraries. No Windows runtime, COM, Win2D or backend-specific
renderer is required.

| Upstream responsibility | FerroUI integration |
| --- | --- |
| Dependency objects and properties | `FerroObject`, `ferro_class!`, `ferro_properties!`, class metadata |
| Lifecycle, layout and render | `Control`, `LayoutableImpl`, `VisualImpl`, property invalidation |
| Enumerable data | Existing items-source adapters and observable collections |
| Observable vectors/maps | Shared handles, change notifications and ownership validation |
| Reference identity | `Ref<T>` / `Rc<T>` identity; collections initialized per instance |
| Subscription lifetime | Weak captures and explicit detachment on source/series replacement |
| Plot and marker geometry | `DrawingContext`, `StreamGeometry`, pens, brushes and plot clips |
| Typography | Existing text formatting and text layouts |
| Palette/resources | Existing dictionaries and theme variants |
| Accessibility | Automation properties and control peers |
| XAML | Type registration, metadata and collection adapters |
| Validation exceptions | Rust `Result` without partial mutation |
| MSBuild and Windows deployment | Cargo and FerroUI platform hosts |

Geometry/tick construction is separated from drawing. Retained scenes can be
reused when data, axes, style and layout are unchanged; relevant changes
invalidate them through framework notifications. Core paths do not assume an
OS clock, filesystem or thread availability.

## 4. Adaptations and boundaries

1. **Unavailable rendering source.** Tick selection, labels and plot geometry
   are native implementations. Future publication of the original source would
   enable deeper algorithm and visual comparison. No pixel-parity claim is made.
2. **Typed inputs.** Nonnullable collection members and typed enums make managed
   null-insertion and arbitrary invalid-enum cases unrepresentable. Unsupported
   scalar sources likewise require an explicit adaptation where the Rust source
   adapter only accepts collections. Such cases are recorded, not faked.
3. **Enumeration.** FerroUI's snapshot collection conventions can differ from
   managed enumerator invalidation. The snapshot/borrow contract is an explicit
   adaptation, not the upstream mutation-during-enumeration exception.
4. **XAML ABI.** WinRT metadata providers and package resource URIs become native
   metadata/resources. Mapped tests establish inheritance, activation, enum
   conversion, content properties and collection population. Exact WinRT names
   and `ms-appx` identities are not FerroUI APIs.
5. **Sample hosts.** Four upstream Windows deployment variants share scenarios.
   MSIX manifests, logos and MSBuild files remain in the source inventory as
   context; FerroUI can share the scenarios across its own platform hosts.

The implementation is under active development. Implementation presence,
mapping completeness, test execution and visual verification are distinct.
Required integration areas include resource/theme changes, replacement and
detachment of live sources, shared axes, mixed series, marker/dash coverage,
clipping, XAML population and automation precedence. Pending tests remain
visible in the machine-readable map until verified counterparts exist.

Upstream [issue 12350](https://github.com/microsoft/microsoft-ui-xaml/issues/12350)
reports series drawing beyond the plot and across axes. A regression with
explicit bounds and out-of-range data should verify the plot clip. Labels and
legend occupy their own layout layers. This is a regression scenario, not a
requirement to reproduce the upstream bug.

Animations, zoom/pan, trackball and pie charts are requests in discussion
comments, not demonstrated released features. Stacked, spline, polar,
financial and logarithmic charts are likewise outside this pinned contract.

## 5. Source and test inventory

The [source manifest](data/winui-charts/source-manifest.json) lists all **88
files** under `controls/dev/Charts` and `Samples/ChartApp`, with Git blob SHAs
and roles. It includes test support, four sample hosts, deployment files and
assets. No nonexistent implementation source path is invented.

| Upstream test file | Tests | Rust responsibility |
| --- | ---: | --- |
| `APITests/ChartTests.cs` | 35 | Control, collections, lifecycle, XAML |
| `APITests/ChartAxisTests.cs` | 31 | Validation, properties, resources, live layout |
| `APITests/ChartSeriesTests.cs` | 35 | Series properties, brushes, overrides |
| `APITests/ChartSamplesTests.cs` | 25 | Source types, sharing, changes and replacement |
| `APITests/ChartAutomationTests.cs` | 4 | Automation precedence and no layout side effects |
| `InteractionTests/ChartsTests.cs` | 2 | Automation presence, live data/series/legend/theme changes |
| **Total** | **132** | **130 API tests and two interaction tests** |

Every method has an entry in [test-map.json](data/winui-charts/test-map.json),
keyed by its exact upstream file and name.

| Status | Meaning |
| --- | --- |
| `pending` | No reviewed Rust mapping yet |
| `mapped` | Named Rust `#[test]` function exists; execution evidence is separate |
| `adapted` | Rust test covers the alternative explained in its notes |
| `not-applicable` | Exact platform/type-system operation is absent, with rationale |

The scanner verifies unique/complete upstream identities, absence of invented
source tests, existence of mapped Rust tests and rationales for adaptations.
With an upstream checkout, it regenerates the pinned inventory and compares
it exactly. It does not equate similar names with semantic or visual parity.
`--require-complete` fails while any source method remains pending.

```sh
git clone --filter=blob:none https://github.com/microsoft/microsoft-ui-xaml.git /tmp/winui-charts-reference
git -C /tmp/winui-charts-reference checkout 9427b7216337ea3d7b19ac32d48c327286ee7138

# Regenerate inventory, preserving existing reviewed mappings.
python3 scripts/winui-charts/conformance.py inventory --upstream /tmp/winui-charts-reference

# Verify source and map integrity.
python3 scripts/winui-charts/conformance.py check --upstream /tmp/winui-charts-reference

# Associate exact translated method names, then review any adapted behavior.
python3 scripts/winui-charts/conformance.py bind-names
python3 scripts/winui-charts/conformance.py check --require-complete

# Execute behavior/integration tests separately.
cargo test -p ferroui-controls-charts
```

## 6. Verification record

| Gate | Recorded result |
| --- | --- |
| Public source audit | 88 files, 130 API tests, two interaction tests; implementation unpublished |
| Inventory generation | Generated from pinned revision above |
| Mapping integrity | Scanner validates inventory and existing mappings; pending entries stay explicit |
| Rust execution | Pending integration; no passing count claimed by this initial audit |
| Native/backend render | Pending integration |
| Browser build | Pending integration |
| Pixel comparison with internal implementation | Unverified; reference execution/source unavailable here |

## 7. Work log and attribution

- **2026-10-21:** Started `feat/winui-charts-port` from the FerroUI base above.
  Opened [draft PR 282](https://github.com/wieslawsoltes/FerroUI/pull/282) before
  implementation. Audited upstream, identified the internal-package boundary,
  and generated file/test inventories.
- **2026-10-21:** Began control/rendering, axis, sample and series implementation
  in parallel. Validation evidence is updated as commits land.

Public tests and samples are MIT licensed. The complete Microsoft notice and
license accompany the crate in
[`NOTICE.md`](../../src/FerroUI.Controls.Charts/NOTICE.md). The notice applies
to the published material used here and does not claim access to unpublished
implementation source.
