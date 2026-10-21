# Gallery validation checkpoint

This ledger records bounded evidence for the native FerroUI sample. Page and API
completion remains governed by `port-status.json` and the framework parity scanner.
The catalog contains 126 pages; 67 have partial native implementations, including
the ScratchPad. No page is marked `ported` or `verified`.

## Reproducible source inputs

The source repository is [Microsoft WinUI Gallery](https://github.com/microsoft/WinUI-Gallery),
pinned to `14061eeac21c38ae99f302edf3d9136d4f652c4c`. The catalog generator checks
that exact commit and refuses tracked modifications to its inputs.

| Input or generated evidence | SHA-256 / location |
| --- | --- |
| Original `WinUIGallery/SampleSupport/Data/ControlInfoData.json` | `334896cc85905de1afe64bdac7717218dc6f60607c132443312d1e9d37b0e27b` |
| Generated `catalog_data.rs` | `451aa227d421cab6eae1a76573ed5c32a8ac55bed6965041bbdffaa850d26416` |
| Original sample files for 350 catalog definitions | Raw source hashes are retained in every `SampleDefinition` in `catalog_data.rs`. |
| Explicit mappings for 27 partial source examples | Source paths, raw hashes and named test anchors are in `port-status.json`. |
| Original control images | `assets-manifest.json` records the source path, original filename, byte count, SHA-256 and registered URI paths of every PNG. |

The source browser preserves original handlers, binding expressions and substitution
tokens. Exported catalog transformations are not used to sanitize the displayed
source. The framework scanner separately inventories 355 actual `ControlExample`
instances and 521 snippet files, including instances/files outside the exported
catalog. Source inventory is not evidence that every example executes.

Both generation checks passed for this checkpoint:

```sh
python3 samples/WinUIGallery/generate_catalog.py ../upstream-gallery --check
python3 samples/WinUIGallery/generate_assets.py ../upstream-gallery --check
```

Results: 126 pages, 350 source definitions, 97 original cat names and 108 original
PNG files. The images total 525,150 bytes. They are registered under 110 paths:
the original Windows metadata spells `CheckBox.png` and `AnnotatedScrollbar.png`
differently from the tracked filenames `Checkbox.png` and `AnnotatedScrollBar.png`.
Both spellings are explicit aliases; original filenames and image bytes are retained.

## Native behavior tests

The library suite passed **33 tests, 0 failures** on Linux x86_64 with
`rustc 1.99.0 (b940084d7 2026-09-28)`:

```sh
cargo test -p winui-gallery --lib --offline
```

The tests use the framework's existing native property, control, event, dispatcher,
mock-window and service test infrastructure. Their source is `tests.rs`, SHA-256
`134db8a064d68e3dc2d07f7e7ee037c98d21a55dfdaeffbbd92e5651f9785bab`.

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
| Popup | Real native popup host opens/closes; source option bindings update horizontal/vertical offsets. | Source acrylic/absolute placement and platform input/appearance. |
| Clipboard | Existing host service performs text copy/paste, format listing and clear; missing service/access-denied outcomes are visible. | Real operating-system clipboard integration, bitmaps/files, history, roaming and source accessibility announcements. |
| Pull to refresh | Existing RefreshContainer requests refresh; a UI dispatcher continuation inserts the source item and completes its native deferral. | Custom visualizer, gestures, pixels and the original ListView contract. |
| Color contrast | Original luminance/contrast calculation, thresholds, symmetry/endpoints and invalid RGB input feedback. | The upstream custom color-picker controls and full accessibility acceptance. |
| Lifetime | Dropping a native page releases weak callback targets. | Broader application and platform lifetime stress testing. |

Mock-renderer construction does not establish bitmap decoding, rendered pixels,
keyboard traversal, screen-reader interoperability or real operating-system service
behavior. Each partial mapping retains that distinction.

## Rendering and desktop host

The current desktop application checked successfully with the existing desktop host,
X11, Skia, HarfBuzz and WinUI theme. The optional capture executable built and ran
with the existing headless window platform, real Skia drawing and HarfBuzz text
shaping, without a display server.

| Gate | Result |
| --- | --- |
| `cargo check -p winui-gallery --features desktop --offline` | Passed on the current source. This is compilation evidence; no interactive desktop session was claimed. |
| `cargo run -p winui-gallery --features capture --bin winui-gallery-capture --offline -- --pages=NumberBox,AutoSuggestBox,RadioButton,WebView2 --query=Button --output-dir=/tmp/ferroui-gallery-captures` | Built and rendered all four pages. The AutoSuggestBox source search decoded and displayed the original Button PNG through the native asset service. |
| Capture executable with `--all --query=Button` | All 126 catalog pages rendered in the light theme. All outputs decoded as nonblank 1280 × 900 PNGs, with 126 distinct frame hashes. |
| Capture executable with `--pages=NumberBox,AutoSuggestBox,RadioButton,InfoBar,WebView2 --query=Button --dark` | All five selected pages rendered in the dark theme. |
| Visual review | Five representative retained images were inspected: NumberBox light, AutoSuggestBox light, RadioButton dark, InfoBar dark and the pending WebView2 source browser in light. |

The [capture README](../../docs/porting/winui-gallery/captures/README.md) contains
the exact follow-up commands, reviewed images and their SHA-256 hashes. The capture
host logs each stable page ID before rendering, fails on native construction errors,
and rejects an AutoSuggestBox detail image that fails actual bitmap decoding.

The host does not provide Segoe Fluent Icons or Segoe MDL2 Assets. AutoSuggestBox's
Find glyph and InfoBar's status/close glyphs show missing-font boxes in these Linux
frames. Their API/glyph values are preserved, but font presentation remains pending
on such hosts. These are rendering smoke artifacts, not upstream Windows pixel-parity
baselines. Rendering a pending page's source browser does not implement its control.

For the constrained build environment, a public rust-skia binary archive was downloaded
once and selected through the build script's existing `SKIA_BINARIES_URL` option:

```text
https://github.com/rust-skia/skia-binaries/releases/download/0.153.3/skia-binaries-b7f043e0b1e2a850e702-x86_64-unknown-linux-gnu-ganesh-gl-jpegd-jpege-pdf-vulkan.tar.gz
SHA-256: 9ebe4c448cc9f789bad241cbd11739cdf759e27db5719c23a7635e096d5807b3
```

This selects the same public dependency artifact; no dependency sources or toolchain
checks were patched. The local build used one Cargo job, no incremental compilation
and disabled development/test debug information to bound resource usage.

## Completion boundary

The sample remains a partial native port with a complete pinned source catalog.
Rendering an existing native adaptation or a pending page's source browser does not
promote that page's status. Full completion still requires the missing framework and
platform APIs, every original example and code-behind behavior, compiled/deferred
XAML features, cross-platform input and accessibility acceptance, and reviewed visual
comparison against the pinned upstream Gallery.
