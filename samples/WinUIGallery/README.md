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
contains 62 explicit markup adaptations and a working ScratchPad editor, for 63 partial
pages. Each remains `partial`; it does not imply that
all examples, events, binding expressions, platform capabilities or exact pixels of
the original page have been reproduced. The generated framework parity reports are
under `docs/porting/winui-gallery/data/`.

The upstream exported catalog is not the entire executable Gallery: the scanner also
inventories **355 actual `ControlExample` instances and 521 snippet files**, including
legacy/unreferenced files. Five example instances are absent from the exported catalog.
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
include button click output and disabled options, two/three-state check boxes,
text/password character counts, sliders, selection, layout, dates, menus, flyouts,
scrolling, resources and element-name binding. The opt-in WinUI NumberBox, RadioButtons,
FontIcon, SymbolIcon and Frame/Page contracts are exercised directly by examples
adapted from upstream. They use the framework's property, input, layout, drawing,
popup and automation infrastructure.

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
cargo test -p winui-gallery --lib
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
