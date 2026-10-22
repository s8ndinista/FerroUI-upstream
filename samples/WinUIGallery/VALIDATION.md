# Gallery validation checkpoint

This ledger records bounded evidence for the native FerroUI sample. Page and API
completion remains governed by `port-status.json` and the framework parity scanner.
The catalog contains 126 pages: **71 have partial native implementations**, including
the ScratchPad, and **55 remain pending**. There are **35 explicit partial mappings to
original source examples**. No page is marked `ported` or `verified`.

The current [generated API summary](../../docs/porting/winui-gallery/data/summary.json)
records 1 verified type, 36 partial types, 202 verified members and 172 partial members.
The complete `Symbol` enum contributes 197 of those verified members; these counts do
not certify complete control, page, visual or platform parity.
The source candidate map separately identifies 297 types and 1,413 members without
certifying their signatures, defaults, behavior or WinMD ABI compatibility.

## Reproducible source inputs

The source repository is [Microsoft WinUI Gallery](https://github.com/microsoft/WinUI-Gallery),
pinned to `14061eeac21c38ae99f302edf3d9136d4f652c4c`. The catalog generator checks
that exact commit and refuses tracked modifications to its inputs.

| Input or generated evidence | SHA-256 / location |
| --- | --- |
| Original `WinUIGallery/SampleSupport/Data/ControlInfoData.json` | `334896cc85905de1afe64bdac7717218dc6f60607c132443312d1e9d37b0e27b` |
| Generated `catalog_data.rs` | `451aa227d421cab6eae1a76573ed5c32a8ac55bed6965041bbdffaa850d26416` |
| Original sample files for 350 catalog definitions | Raw source hashes are retained in every `SampleDefinition` in `catalog_data.rs`. |
| Explicit mappings for 35 partial source examples | Source paths, raw hashes and named test anchors are in `port-status.json`. |
| Original control images | `assets-manifest.json` records the source path, original filename, byte count, SHA-256 and registered URI paths of every PNG. |

The source browser preserves original handlers, binding expressions and substitution
tokens. Exported catalog transformations are not used to sanitize the displayed
source. The framework scanner separately inventories 355 actual `ControlExample`
instances. Its [source-file manifest](../../docs/porting/winui-gallery/data/source-files.json)
contains **521 tracked text files, including 355 catalog-page snippets**, with
supporting and legacy/unreferenced files counted separately from executable examples.
Source inventory is not evidence that every example executes.

Both generation checks passed again for the current checkpoint; the pinned catalog
and asset inputs retain the hashes listed above:

```sh
python3 samples/WinUIGallery/generate_catalog.py ../upstream-gallery --check
python3 samples/WinUIGallery/generate_assets.py ../upstream-gallery --check
```

Results: 126 pages, 350 source definitions, 97 original cat names and 108 original
PNG files. The images total 525,150 bytes. They are registered under 110 paths:
the original Windows metadata spells `CheckBox.png` and `AnnotatedScrollbar.png`
differently from the tracked filenames `Checkbox.png` and `AnnotatedScrollBar.png`.
Both spellings are explicit aliases; original filenames and image bytes are retained.

## Current framework regression checkpoint

The following suites passed on the current source on Linux x86_64 with
`rustc 1.99.0 (b940084d7 2026-09-28)`:

| Suite | Measured result | Scope |
| --- | --- | --- |
| Native WinUI control integration | **127 passed, 0 failed** | 77 existing tests plus InfoBadge 13, ProgressRing 16 and RatingControl 21. |
| Base library | **4,383 passed, 0 failed** | Includes the additive local-value query; existing priority and clearing semantics remain unchanged. |
| XAML migration/runtime | **41 passed, 0 failed** | Includes namespace-isolated feedback controls, actual native construction and audited InfoBadge style-key projection. |
| Compiled/interpreted XAML | **5 passed, 0 failed, 1 ignored** | The ignored task explicitly regenerates a reviewed fixture; emitted and interpreted root/template binding tests pass. |
| WinUI binding lifecycle | **13 passed, 0 failed** | Explicit initialization, source/target changes, template recycling, error scope and subscription lifetime. |
| Deferred-resource error isolation | **3 passed, 0 failed** | Typed nested error scopes preserve inner errors; unrelated panics resume and ordinary throwing retains its prior behavior. |
| Native view builders | **`cargo check -p ferroui-view` passed** | Generated typed builders compile against the current native controls. |
| Native Gallery library | **42 passed, 0 failed** | Application interactions and mock-service acceptance described below. |
| WinUI theme library | **294 passed, 0 failed, 2 ignored** | The explicit ignored tasks regenerate the reference and measure load time. Both compiler-output comparisons and four reference-storage tests pass. |
| Scanner/comparison regression | **44 passed** | API, glyph, namespace-builder and baseline comparison regressions. |
| Theme path-case regression | **2 passed** | Source dictionary path handling remains explicit and deterministic. |
| Source and inventory gates | **Passed** | API/event/symbol/builder freshness, existing API preservation, full theme inventory, and exact catalog/asset regeneration. |
| Desktop host | **Compilation passed** | Existing desktop/X11/Skia/HarfBuzz integration compiles; no interactive desktop session is claimed. |

The native-control [execution record](../../docs/porting/winui-gallery/data/executions/native-controls-003.json)
stores the exact command, passing names for all 50 new focused tests, implementation/test
hashes and the run-log hash. New API contracts remain `partial`; absent ProgressRing
animated-source members and RatingItem state branches without direct assertions remain
unverified. These recorded tests use actual native objects with mock host services.

The current runs used `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`,
`CARGO_PROFILE_DEV_DEBUG=0` and `CARGO_PROFILE_TEST_DEBUG=0`. The Base, XAML, theme
and desktop commands were:

```sh
cargo test --locked --offline -p ferroui-base \
  --features testing,compiler-metadata,markup-functions --lib
cargo test --locked --offline -p ferroui-markup-xaml-loader \
  --features emitter,ferroui-base/markup-functions --test winui_migration --test winui_compiled
cargo test --locked --offline -p ferroui-themes-winui --lib
cargo check --locked --offline -p winui-gallery --features desktop
```

The binding-lifecycle, deferred-resource isolation, view-builder, theme and desktop
checks have all been rerun on the current source. The checked-in compiled-theme
reference is now stored in five parts, with its former path retained as a short
guide. Tests join the parts byte for byte before both existing build-time/runtime
compiler comparisons; the production emitter is unchanged. Four additional tests
cover reference ordering, chunk boundaries, byte-preserving joining and unexpected
files. The two ignored theme tasks are explicit regeneration and timing operations.

The existing-API gate preserves 3,170 namespaced types, 28,435 declared member names
and 458 source registrations against the starting commit, with 82 additive types.
This source-declaration result does not establish every signature, default or
behavioral equivalence.

The separate [theme resource/style ledger](../../docs/porting/winui-theme/TRACKING.md)
records **6,912 ported, 1,991 waived and 70 pending entries** across scoped resources
and styles. The 70 pending entries comprise InfoBadge 27, ProgressRing 8 and
RatingControl 35. The current inventory has zero unresolved references and zero
duplicate keys. Its declaration accounting uses the theme's own source pin and
does not itself certify native behavior or visual equivalence; compiled-theme
execution is reported separately above.

## Native Gallery behavior tests

The library suite passed **42 tests, 0 failures** on Linux x86_64 with
`rustc 1.99.0 (b940084d7 2026-09-28)`:

```sh
cargo test --locked --offline -p winui-gallery --lib
```

The tests use the framework's existing native property, control, event, dispatcher,
mock-window and service test infrastructure. Test sources at this checkpoint are:

| Source | SHA-256 |
| --- | --- |
| `tests.rs` | `1475fcaadfa9e4745eaefdd9539641556693e06143e9db3899f0b82d1cd9b70a` |
| `feedback_samples_tests.rs` | `523fbcde28227291e93fd490eab592e2025fe91130bbe35c3c1eaff96c72b15d` |

| Area | Evidence exercised by the tests | Remaining acceptance |
| --- | --- | --- |
| Catalog and shell | Unique stable IDs; all 126 pages selectable; filtering and navigation history; native adaptation construction; original source preservation. | Every upstream page's complete example and application behavior. |
| XAML preview | Public opt-in WinUI migration; strict unsupported-binding diagnostics; typed missing-resource error returned without unwinding. | Executing original C# and the full upstream binding/template/state feature set. |
| Buttons, text and selection | Click/disabled output; original two-state, three-state and select-all CheckBox handlers; Unicode scalar text counts; Slider/ComboBox outputs; command-bar events. | All source options, inputs, accessibility and visual states. |
| Password examples | Source rejection/clearing of `Password`, valid/empty transitions and reveal option, using an explicit native TextBox adaptation. | A distinct WinUI PasswordBox API contract and full source visual behavior. |
| NumberBox | Original native values, expression evaluation, NaN/empty state and ValueChanged observation. | All source formatters, localization, input and appearance. |
| Radio buttons and icons | Original group/color selection, Symbol enum and FontIcon glyph values. | Platform font availability, full keyboard/automation acceptance and every icon subtype. |
| Frame/Page | Native page types, navigation, BackStack/ForwardStack and journal behavior. | Page transitions, animation effects and journal persistence. |
| AutoSuggestBox | All 97 source cat names; user-input-only filtering; word matching; selected-item output; title filtering and prefix ordering; chosen suggestion before free query; unchanged details on no result. | Exact CurrentCulture search semantics, full platform input/automation and source appearance. |
| Control detail images | All 126 source image URIs resolve to the original embedded PNG bytes, including the two explicit path aliases. | Real decoding is established separately by the renderer capture command. |
| InfoBar | All three source compositions: severity, short/long message, Button/HyperlinkButton action and open/closable/icon options. | Complete native visual states, icons and live-region acceptance. |
| InfoBadge | Three source examples: selection of the original severity theme keys, Refresh-button composition with the original glyph, and the source integer guard for dynamic values. | NavigationView.InfoBadge and the remaining scoped source-resource definitions are pending; installed-font rendering, exact visuals and Windows accessibility require separate acceptance. |
| ProgressRing | Both source examples use the native control, activation/background options and finite/empty NumberBox handling. | Custom DeterminateSource/IndeterminateSource, exact Lottie motion and platform UIA/pixel acceptance remain pending. |
| RatingControl | Both source examples retain user-only caption changes, live clear/read-only options and fractional placeholder binding in both directions. | Locale formatting, raw gamepad mappings, text-scale/custom-template details, installed glyph fonts and Windows visual/accessibility acceptance. Native Value/PlaceholderValue reject NaN as an explicit validation exception. |
| CompactSizing | Navigation between distinct native page types preserves all four text/password fields and the nullable date; compact resources remain local and pages release after removal. | PasswordBox and Header use explicit native TextBox/composition adapters; full original control contracts and rendered density/input acceptance remain partial. |
| Popup | Real native popup host opens/closes; source option bindings update horizontal/vertical offsets. | Source acrylic/absolute placement and platform input/appearance. |
| Clipboard | Existing host service performs text copy/paste, format listing and clear; missing service/access-denied outcomes are visible. | Real operating-system clipboard integration, bitmaps/files, history, roaming and source accessibility announcements. |
| Pull to refresh | Existing RefreshContainer requests refresh; a UI dispatcher continuation inserts the source item and completes its native deferral. | Custom visualizer, gestures, pixels and the original ListView contract. |
| Color contrast | Original luminance/contrast calculation, thresholds, symmetry/endpoints and invalid RGB input feedback. | The upstream custom color-picker controls and full accessibility acceptance. |
| Lifetime | Dropping a native page releases weak callback targets. | Broader application and platform lifetime stress testing. |

Mock-renderer construction does not establish bitmap decoding, rendered pixels,
keyboard traversal, screen-reader interoperability or real operating-system service
behavior. Each partial mapping retains that distinction.

## Current rendering and desktop host

The current native Gallery source checkpoint is
`3c7ceb81dcc4948f374c05c4757c6cb72ed5de5d`. The optional capture executable built
and ran using the existing headless window platform, real Skia drawing, HarfBuzz
text shaping and WinUI theme without a display server. The desktop application
also compiled successfully with its existing desktop, X11, Skia and HarfBuzz host.
No interactive desktop session is claimed.

| Gate | Measured result |
| --- | --- |
| Desktop host | `cargo check --locked --offline -p winui-gallery --features desktop` passed. |
| Dedicated light capture | InfoBadge, ProgressRing, RatingControl and CompactSizing rendered; four decoded, nonblank 1280 × 900 PNGs with four distinct hashes. |
| Full light sweep | All 126 catalog pages rendered; 126 decoded, nonblank 1280 × 900 PNGs with 126 distinct hashes. |
| Selected dark sweep | All nine requested pages rendered; nine decoded, nonblank 1280 × 900 PNGs with nine distinct hashes. |
| Visual review | Eight retained frames were inspected: InfoBadge, ProgressRing, RatingControl and CompactSizing in both themes. |

All **139 output PNGs** were decoded and checked for pixel variation. The
[capture manifest](../../docs/porting/winui-gallery/captures/capture-manifest.json)
records source, binary and log hashes, every sweep page ID/hash and the retained
image hashes. The four retained light frames come from the separate dedicated run;
they are recorded separately from the same pages in the full sweep. Three differ
in bytes between runs, so the hashes identify these observations rather than a
deterministic pixel baseline for future builds.

The exact current capture commands were:

```sh
cargo run --locked --offline -p winui-gallery --features capture --bin winui-gallery-capture -- \
  --pages=InfoBadge,ProgressRing,RatingControl,CompactSizing \
  --output-dir=/tmp/ferroui-gallery-current-light
target/debug/winui-gallery-capture \
  --all --query=Button --output-dir=/tmp/ferroui-gallery-current-all
target/debug/winui-gallery-capture \
  --pages=NumberBox,RadioButton,AutoSuggestBox,InfoBar,InfoBadge,ProgressRing,RatingControl,CompactSizing,WebView2 \
  --dark --query=Button --output-dir=/tmp/ferroui-gallery-current-dark
```

The [capture README](../../docs/porting/winui-gallery/captures/README.md) records the
full toolchain/build environment and reviewed frames. The capture host logs each
stable page ID before rendering, fails on native adaptation construction errors,
and rejects an AutoSuggestBox detail image that fails actual bitmap decoding.
Both sweeps submitted the source `Button` search through the real renderer.

The recorded Linux host does not provide Segoe Fluent Icons or Segoe MDL2 Assets.
Rating stars and some icon glyphs visibly appear as missing-font boxes. Their
API/glyph values are retained; installed-font presentation remains pending.
CompactSizing frames show its initial **Standard** form, so compact-mode switching
and value retention are established by native behavior tests rather than these
frames. ProgressRing's static frames do not establish animation behavior. These
captures establish native construction and rendering; upstream Windows pixels,
real platform input, UIA and screen-reader acceptance remain unverified. Rendering
a pending page's source browser does not implement its control.

### Historical frames

Five unchanged captures remain from the **prior `d008845` checkpoint**: NumberBox
light, AutoSuggestBox light, RadioButton dark, InfoBar dark and WebView2's pending
source browser in light. Their commands and hashes remain in the capture README's
historical section. They are separate evidence for that earlier checkpoint and do
not depict the newly added controls or CompactSizing implementation.

### Renderer dependency

The constrained build environment reused the following public rust-skia binary
archive through the build script's existing `SKIA_BINARIES_URL` option. Its local
bytes were checked against this SHA-256 for the current run:

```text
https://github.com/rust-skia/skia-binaries/releases/download/0.153.3/skia-binaries-b7f043e0b1e2a850e702-x86_64-unknown-linux-gnu-ganesh-gl-jpegd-jpege-pdf-vulkan.tar.gz
SHA-256: 9ebe4c448cc9f789bad241cbd11739cdf759e27db5719c23a7635e096d5807b3
```

The current setting was `SKIA_BINARIES_URL=file:///tmp/ferroui-skia.tar.gz`.
This selects the same public dependency artifact; no dependency sources or toolchain
checks were patched. The local build used one Cargo job, no incremental compilation
and disabled development/test debug information to bound resource usage.

## Completion boundary

The sample remains a partial native port with a complete pinned source catalog.
Rendering an existing native adaptation or a pending page's source browser does not
promote that page's status. The current strict completeness gate reports 1,544 API
types, 7,264 API members and all 126 Gallery pages without complete behavioral
execution evidence. Partial contract tests do not satisfy that completion requirement.
Full completion still requires the missing framework and
platform APIs, every original example and code-behind behavior, compiled/deferred
XAML features, cross-platform input and accessibility acceptance, and reviewed visual
comparison against the pinned upstream Gallery.
