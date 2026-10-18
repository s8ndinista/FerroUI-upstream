# FerroDesign

A design editor built with **FerroUI, Rust, compiled XAML and Vello**. The same editor, controls, document model and scene renderer run in a browser and in the native launcher.

The editor includes pages and layers, an infinite canvas, vector and text tools, property editing, layout, components, local variables, prototypes, comments, undo/redo and portable document export. The shell follows the navigation rail, contextual left sidebar, canvas, property sidebar and floating toolbar arrangement of Figma Design. [Feature coverage](docs/FEATURES.md) records the implementation and its limits; this repository does not claim complete Figma compatibility or verified pixel parity.

The editor is real FerroUI UI. `Editor.xaml` compiles into native FerroUI controls; `DesignCanvas` derives from `Control`; artwork is drawn through the public `DrawingContext`, geometry, brush, pen, text and imaging APIs. The HTML host only starts WebAssembly and provides browser storage, clipboard and downloads.

## Get the complete source

This sample is an independent Cargo workspace inside the FerroUI repository. Keep the repository around it: its path dependencies refer to FerroUI's actual source, not to a separate graphics implementation.

```sh
git clone --branch feat/ferrodesign-editor --single-branch https://github.com/wieslawsoltes/FerroUI.git
cd FerroUI/samples/FerroDesign
```

The implementation was based on upstream commit `325b3ce4b0fcf154eba79cbd92262536daf77e71`. The branch contains the complete application and the small browser feature-gating changes needed to use Vello without linking Skia. Nothing needs to be copied into a different framework.

## Build and run in a browser

Required host tools: a C/C++ build toolchain, `git`, `curl`, Python 3, and Node.js/npm. The setup script installs its own pinned Rust and Emscripten tools without editing shell profiles.

```sh
./scripts/setup.sh
source .tools/env.sh
./scripts/build-browser.sh
python3 -m http.server 8080 --directory dist
```

Open <http://localhost:8080>. The standard browser configuration requests **FerroUI Vello WebGL2 hybrid rendering** and falls back to **Vello CPU with Software2D presentation**. To select the CPU path explicitly, open <http://localhost:8080/?renderingMode=Software2D>.

The generated `dist` directory is a static distribution containing the application JavaScript, WebAssembly, FerroUI browser bridge, resources and dependency manifest. Serve it over HTTP; opening `index.html` through `file://` does not provide a usable WebAssembly application origin. This build does not enable the FerroUI service worker.

The pinned browser tools are Rust **1.99.0**, Emscripten **6.0.10** and `wasm-bindgen-cli` **0.2.129**. `Cargo.lock` and the browser validation package lock are committed. The build script checks the **active target dependency graph** and stops if `ferroui-skia`, `skia-safe` or `skia-bindings` appears.

The first build is substantial because it compiles FerroUI itself. The browser profile builds one job at a time, splits code generation and avoids LTO to control peak memory. Vello and its rasterization dependencies retain optimization level 3. Do not increase parallelism on a memory-constrained runner before measuring peak memory.

Useful build options:

```sh
./scripts/build-browser.sh --out /tmp/ferrodesign-dist
./scripts/build-browser.sh --skip-webapp  # when the unchanged FerroUI JS bridge is already built
./scripts/build-browser.sh --debug
```

## Native launcher

Use a supported Rust toolchain and the platform prerequisites documented by the parent FerroUI repository.

```sh
cargo run -p ferrodesign-app
cargo run -p ferrodesign-app -- --renderer-info
cargo run -p ferrodesign-app -- --render-smoke /tmp/ferrodesign.png
```

The native configuration uses Vello CPU and the platform framebuffer on Linux/X11 and Windows, and Vello hybrid/Metal with a CPU fallback on macOS. `--software` requests the CPU configuration. The headless smoke command draws the compiled-XAML application through a real Vello render target and writes PNG; it is not a screenshot of an HTML replacement.

## Editing

The initial document contains editable Orbit mobile banking screens, a component page, styles, variables, comments and a linked prototype. Click a layer on the canvas or in the left sidebar; the right sidebar edits the selected layer. Empty-canvas selection exposes the page background.

| Action | Shortcut |
| --- | --- |
| Select / frame / rectangle / ellipse / line | `V` / `F` / `R` / `O` / `L` |
| Pen / freehand pencil / text | `P` / `Shift+P` / `T` |
| Hand / comment / section | `H` / `C` / `Shift+S` |
| Auto layout / component / place image | `Shift+A` / `Ctrl`/`Cmd+Alt+K` / `Ctrl`/`Cmd+Shift+K` |
| Pan | Hold `Space` and drag, middle-button drag, or scroll |
| Zoom around the pointer | `Ctrl`/`Cmd` + scroll |
| Fit page / fit selection | `1` / `2` |
| Undo / redo | `Ctrl`/`Cmd+Z` / `Ctrl`/`Cmd+Shift+Z` |
| Copy / cut / paste / duplicate | `Ctrl`/`Cmd+C` / `X` / `V` / `D` |
| Group / ungroup | `Ctrl`/`Cmd+G` / `Ctrl`/`Cmd+Shift+G` |
| Move selection | Arrow keys; hold `Shift` for 10 units |
| Quick actions | `K` or `Ctrl`/`Cmd+K` |
| Save / open | `Ctrl`/`Cmd+S` / `Ctrl`/`Cmd+O` |
| Cancel gesture / leave presentation | `Escape` |

Shortcuts are handled when the canvas has focus; text fields retain their normal text-editing keys. Shift constrains drawing and resizing, locks a move to an axis, and snaps rotation to 15-degree increments. Dragging snaps to pixels and nearby object edges/centers; quick actions can toggle snapping. The pen creates editable vector paths; finish with Enter or close the path by clicking its first point.

Use the property sidebar for geometry, fills, strokes, typography and layout. Quick actions also expose grouping, layer order, alignment/distribution, boolean combinations, components, pages, comments, prototype presentation and export. `Layer data…` exposes validated structured properties that do not yet have a dedicated visual control, including path segments, rich text runs and component metadata.

## Files and local persistence

The native editable format is a versioned `.ferrodesign` JSON document. It stores the complete document graph, paints, layout, components, variable collections, comments and prototype links. Browser edits are autosaved in local storage; the native launcher uses `ferrodesign-autosave.json` in its working directory. Export a document to make a portable copy.

`Open` also imports SVG into a selected editable frame. `Place image` accepts PNG, JPEG, GIF or BMP; FerroUI decodes the image and its data is embedded in the portable document. The native Import dialog accepts JSON, SVG markup or a raster image data URI.

Export options include editable JSON, SVG, PNG rendered through FerroUI's `RenderTargetBitmap`, basic FerroUI XAML, and CSS inspection. SVG/XAML/CSS output have format-specific limits described in the feature matrix. A JSON round trip is the complete fidelity-preserving format for this editor. Figma's proprietary `.fig` file format is not implemented.

## Validation

```sh
cargo test --locked -p ferrodesign-core
cargo check --locked -p ferrodesign-app
cargo run --release -p ferrodesign-core --example performance -- 10000
npm ci
npx playwright install chromium
npm run test:browser
```

`tests/browser.mjs` serves the actual `dist` build and exercises both requested browser rendering modes. It checks nonblank native pixels, performs a real pointer drag to create a rectangle, uses keyboard undo/redo, deletes and reimports the document, compares JSON, and checks startup errors and render-thread panics. It writes PNG screenshots, traces and JSON/HTML reports to `browser-report`.

The GitHub Actions workflow `.github/workflows/ferrodesign.yml` runs the locked core tests, the real WebAssembly build, browser interaction validation and native compilation. Build and validation artifacts are attached to each workflow run. See [validation and measurements](docs/VALIDATION.md) for observed results and the distinction between document-engine timings and browser rendering performance.

## Source map

| Path | Responsibility |
| --- | --- |
| `crates/ferrodesign-core/src/model.rs` | Document graph, paints, text, components, variables and validated serialization |
| `crates/ferrodesign-core/src/editor.rs` | Transactions, history and editing operations |
| `crates/ferrodesign-core/src/geometry.rs` | Affine geometry, vector paths and viewport math |
| `crates/ferrodesign-core/src/layout.rs` | Constraints and row/column/wrapped/grid auto layout |
| `crates/ferrodesign-core/src/spatial.rs` | Spatial index, clipping and geometry-aware hit testing |
| `crates/ferrodesign-app/ui/Editor.xaml` | Compiled FerroUI editor shell and control styling |
| `crates/ferrodesign-app/src/shell.rs` | Control events, property editing and document commands |
| `crates/ferrodesign-app/src/canvas.rs` | FerroUI canvas control, input gestures and selection overlays |
| `crates/ferrodesign-app/src/renderer.rs` | Cached public-API scene renderer |
| `crates/ferrodesign-app/src/platform.rs` | Browser/native storage, clipboard and downloads |
| `crates/ferrodesign-app/src/desktop.rs` | Vello native configuration and headless rendering |
| `web` / `scripts` / `tests` | Browser host, reproducible build and interaction validation |

See [architecture](docs/ARCHITECTURE.md) for rendering, invalidation and transaction details.

## License

New FerroDesign source is MIT-licensed, consistent with the parent repository. Refer to the repository license and individual dependency licenses. The sample uses its own name and sample artwork; it is not an official Figma product.
