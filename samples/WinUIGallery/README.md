# WinUI Gallery on FerroUI

This is the native FerroUI Gallery port and source browser for Microsoft WinUI Gallery
at commit `14061eeac21c38ae99f302edf3d9136d4f652c4c`.

The full pinned exported catalog is present: **126 pages and 350 sample definitions**.
Every page is searchable by name, stable ID, category, description and curated tags.
Every catalog definition retains the XAML/code from its original source file, its
path and the raw file's SHA-256 hash. The source sections preserve handlers,
bindings and substitution tokens; only line endings/trailing newlines are normalized.
Source and documentation links are available from the page. Back/forward navigation
and live theme selection use real FerroUI controls.

**This is not a completed port of every Gallery behavior.** `port-status.json`
separates source coverage from executable page coverage. The initial native subset
contains 70 explicit markup adaptations and a working ScratchPad editor, for 71 partial
pages. Each remains `partial`; it does not imply that
all examples, events, binding expressions, platform capabilities or exact pixels of
the original page have been reproduced. The generated framework parity reports are
under `docs/porting/winui-gallery/data/`.

The upstream exported catalog is not the entire executable Gallery: the scanner also
inventories **355 actual `ControlExample` instances**. Its source manifest contains
**521 tracked text files, including 355 catalog-page snippets**; the remaining text
files include supporting and legacy/unreferenced inputs. Five example instances are
absent from the exported catalog.
Those differences stay in the parity ledger and completeness gate; embedding the
catalog does not mark the remaining examples as ported.

## Run

```sh
cargo run -p winui-gallery --features desktop
cargo run -p winui-gallery --features desktop -- --page=Button
```

The `desktop` feature uses FerroUI's existing desktop host and the WinUI theme.
The library without this feature contains the entire source catalog, native gallery
view and examples without linking a desktop renderer. It can be hosted as a control
by an existing application. A browser package/launcher is not included in this sample.

```rust
let gallery = winui_gallery::GalleryView::new()?;
window.set_content(Some(std::rc::Rc::new(gallery.root())));
gallery.navigate("Slider");
// Retain `gallery` for the lifetime of the view. Its callbacks hold weak state references.
```

## Implementation

`Gallery.xaml` defines the native shell. `gallery.rs` supplies filtering, history,
page composition, theme changes and source inspection. Only the selected page is
created. The catalog stores a precomputed lowercase search index, so filtering does
not allocate strings for every row. Handlers use weak references; replacing a page
releases its control tree and callback state.

`Pages/*.xaml` contains reviewed adaptations to the existing framework API.
`examples.rs` records each adaptation and supplies Rust event handlers. Examples
include button click output and disabled options, original two/three-state and
select-all check-box handlers, password validation/reveal, text character counts,
sliders, selection, layout, dates, menus, flyouts,
scrolling, resources and element-name binding. The opt-in WinUI NumberBox, RadioButtons,
FontIcon, SymbolIcon, AutoSuggestBox, InfoBar, InfoBadge, ProgressRing, RatingControl
and Frame/Page contracts are exercised directly by examples
adapted from upstream. They use the framework's property, input, layout, drawing,
popup and automation infrastructure.

Both AutoSuggestBox examples keep the original event-reason checks, word matching,
chosen-suggestion precedence and no-results behavior. The detail view uses the
original title, subtitle and 108 embedded control PNGs through native resource URIs.
`assets-manifest.json` records exact bytes and the two explicit Windows path-case
aliases. Unicode lowercase comparison is an explicit approximation to the source's
CurrentCulture search. Mock-renderer tests verify asset paths/bytes and application
selection behavior; the capture host performs actual bitmap decoding.

`feedback_samples.rs` ports both ProgressRing examples, both RatingControl examples
and three of the four InfoBadge examples. Activation/background options, cleared
numeric input, rating captions, fractional placeholders, source badge styles and
integer badge-value guards run through the new native controls. The InfoBadge
NavigationView example remains pending. Named badge themes retain the upstream
resource keys and use the explicit Style-to-ControlTheme migration policy.

`sizing_sample.rs` preserves the CompactSizing page's standard/compact navigation
and all four text/password values plus its nullable selected date. Compact resources
are local to each compact page. PasswordBox and Header remain explicit native
TextBox/composition adaptations; changing the example does not change application
density.

`platform_samples.rs` ports the Popup open/close handlers, asynchronous clipboard
text/format/clear operations and the RefreshContainer request/deferral example.
Clipboard access comes from the containing window's existing platform service;
missing services and operation errors produce visible output. Refresh work runs on
the UI dispatcher and balances the native deferral even if the page is removed.
The Popup's placement and the refresh sample's ListBox replacement are explicit
adaptations. Bitmap/file clipboard operations, clipboard history and roaming,
custom refresh visualizers and full operating-system/input acceptance remain pending.

The color-contrast page ports the original relative-luminance calculation, contrast
ratio and pass thresholds to Rust. Its native RGB text inputs and reactive preview
are an explicit replacement for the upstream custom color-picker controls.

Original source sections are also available for inspection. **Load XAML preview**
uses the public opt-in `FerroRuntimeXamlLoader::parse_winui_as` API; it does not delete
unknown handlers, substitution tokens, `x:Bind` expressions or unimplemented elements.
Unsupported constructs produce visible migration diagnostics. Successful construction
is labeled as an XAML preview; it is not automatically promoted to parity evidence.
Original C# remains visible as source and is not executed.

The desktop host explicitly selects `WinUIXamlOptions::for_winui_theme()` after
installing that theme. Library hosts use the strict default unless they pass their
own policy to `GalleryView::with_preview_options`. The UI shows the migration's
source-positioned adaptation diagnostics on a successful preview as well as errors.

The ScratchPad page provides an editable WinUI XAML document using the same strict
loader. A failed migration clears the preview and displays the error.

## Reproduce and validate

```sh
python3 samples/WinUIGallery/generate_catalog.py /path/to/pinned/WinUI-Gallery --check
python3 samples/WinUIGallery/generate_assets.py /path/to/pinned/WinUI-Gallery --check
cargo test -p winui-gallery --lib
cargo check -p winui-gallery --features desktop
```

The catalog generator refuses a different upstream commit or tracked local source
changes. To refresh
the snapshot, first review and update the pinned commit together with the framework
source pins and parity data, then regenerate the catalog. Source text is emitted as
Rust raw strings without interpreting XAML or C# escapes.

The tests cover catalog completeness/identity, token-preserving source wrapping,
navigation failure and forward-history preservation, category/token search,
construction of every native adaptation, the native gallery shell, strict WinUI
preview behavior and actual native event/property interactions. Tests that construct
controls do not claim an interactive desktop run, pixel equality, keyboard traversal,
screen-reader interoperability or verification on every host.

The current checkpoint passed **42 Gallery library tests**, **127 native control
tests**, **4,383 Base tests**, **41 XAML migration/runtime tests**, **13 binding-lifecycle
tests** and **three deferred-resource error-isolation tests**. Five compiled/interpreted
XAML tests passed, with one explicitly ignored fixture-regeneration task. The WinUI
theme passed **294 tests**, with two explicitly ignored regeneration/timing tasks;
its split reference preserves the exact bytes used by both compiler comparisons.
The view-builder and desktop checks passed. The source gates passed with **44
scanner/comparison regressions** and **two theme path-case tests**.
[VALIDATION.md](VALIDATION.md) records the scopes, commands and source hashes.

The separate
[theme resource/style ledger](../../docs/porting/winui-theme/TRACKING.md) records
6,912 ported, 1,991 waived and 70 pending entries: InfoBadge 27, ProgressRing 8 and
RatingControl 35 remain pending in the strict source-definition inventory.

At native source checkpoint `3c7ceb81dcc4948f374c05c4757c6cb72ed5de5d`, real
Skia/HarfBuzz captures rendered **all 126 pages in light** and **nine pages in dark**.
Every sweep output decoded as a nonblank 1280 × 900 PNG, with 126/9 distinct hashes.
The [eight reviewed current frames](../../docs/porting/winui-gallery/captures/README.md)
show InfoBadge, ProgressRing, RatingControl and CompactSizing in both themes. Their
light frames come from a separate four-page run recorded in the
[capture manifest](../../docs/porting/winui-gallery/captures/capture-manifest.json).
CompactSizing shows the initial Standard form; its compact-mode navigation and field
retention are tested separately. Rating stars and some icons visibly lack Segoe
glyphs on the recorded Linux host. These are native construction/rendering evidence;
complete visuals, real platform input and accessibility remain pending. Five unchanged
`d008845` frames are retained separately as historical evidence.

### Capture real rendered frames

The optional `capture` feature uses the existing headless platform with real Skia
drawing, HarfBuzz text shaping and the WinUI theme. It does not require a display
server. The following command captures actual native sample content in the Gallery
shell:

```sh
cargo run -p winui-gallery --features capture --bin winui-gallery-capture -- \
  --pages=InfoBadge,ProgressRing,RatingControl,CompactSizing \
  --output-dir=gallery-captures
```

Use `--dark` for the dark theme or `--all` to visit every catalog page. Capture is a
rendering smoke check: it does not compare against upstream Windows pixels and does
not turn a pending page into an implemented page. See `VALIDATION.md` for the tested
checkpoint, exact commands, source hashes and remaining acceptance boundaries.
Use `--pages=AutoSuggestBox --query=Button` to submit the source search example and
capture its native text and image details; the command fails if the selected image
cannot be decoded by the real renderer.

## Remaining acceptance

Each partial page still requires review of every original `ControlExample`, Rust
ports of its C# behavior and its complete source/API dependency map. Pages for native
windowing, notifications, islands, storage, media, WebView, composition, advanced text,
inking and controls still in separate PRs need their platform/control implementation
and integration. The catalog entry stays available while those contracts are pending.

WinUI-specific control wrappers must remain opt-in wherever a shared control's default
values or ownership semantics differ. Shared API names and behaviors are preserved;
an explicit adaptation (for example `ListView` to `ListBox`) is documented in this
sample and never counted as full WinUI API parity.
