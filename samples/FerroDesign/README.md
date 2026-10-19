# FerroDesign

A design editor built with **FerroUI, Rust, compiled XAML and Vello**. The same editor, controls, document model and scene renderer run in a browser and in the native launcher.

The editor includes pages and layers, an infinite canvas, editable vector networks, rich text, image cropping, native effect controls, auto-layout child dragging, component variants and typed properties, visual local variables, animated prototypes, comments, undo/redo and portable export. The compiled XAML shell follows the navigation rail, contextual left sidebar, canvas, property sidebar and floating toolbar arrangement of Figma Design. [Feature coverage](docs/FEATURES.md) records the implementation and its limits; this repository does not claim complete Figma compatibility or verified pixel parity.

The editor is real FerroUI UI. `Editor.xaml` compiles into native FerroUI controls; `DesignCanvas` derives from `Control`; artwork is drawn through the public `DrawingContext`, geometry, brush, pen, text and imaging APIs. The HTML host only starts WebAssembly and provides browser storage, clipboard and downloads.

## Get the complete source

This sample is an independent Cargo workspace inside the FerroUI repository. Keep the repository around it: its path dependencies refer to FerroUI's actual source, not to a separate graphics implementation.

```sh
git clone --depth 1 --branch feat/ferrodesign-editor --single-branch https://github.com/wieslawsoltes/FerroUI.git
cd FerroUI/samples/FerroDesign
```

The implementation began from upstream commit `325b3ce4b0fcf154eba79cbd92262536daf77e71` and incorporates upstream `3d8b473d95662c74d7bed4874a861233a09f3f05` (FerroUI 0.3.0-preview.1). The branch contains the complete application, Vello browser feature gating, native text-input fixes and the public FerroUI isolated-compositing capability. [PR #254](https://github.com/wieslawsoltes/FerroUI/pull/254) collects the branch changes.

## Build and run in a browser

Required host tools: a C/C++ build toolchain, `git`, `curl`, Python 3, and Node.js 22/npm (the CI configuration). The setup script installs its own pinned Rust and Emscripten tools without editing shell profiles.

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

Use Rust **1.99.0**, as required by this sample's Cargo manifest, and the platform prerequisites documented by the parent FerroUI repository. The setup environment above supplies the pinned toolchain for native builds as well.

```sh
cargo run -p ferrodesign-app
cargo run -p ferrodesign-app -- --renderer-info
cargo run -p ferrodesign-app -- --render-smoke /tmp/ferrodesign.png
```

The native configuration uses Vello CPU and the platform framebuffer on Linux/X11 and Windows, and Vello hybrid/Metal with a CPU fallback on macOS. `--software` requests the CPU configuration. The headless smoke command draws the compiled-XAML application through a real Vello render target and writes PNG; it is not a screenshot of an HTML replacement.

## Editing

The initial document contains editable Orbit mobile banking screens, a component page, styles, variables, comments and a linked prototype. Click a layer on the canvas or in the left sidebar; the right sidebar edits the selected layer. Empty-canvas selection exposes the page background.

Select an auto-layout frame or one of its children, then choose **Layout and grid placement…** in the inspector. Native controls edit child sizing, min/max limits, growth, absolute positioning, one-based grid cells and row/column spans. **Move before/after** follows the visual flow, including reversed layouts; pinned grid children keep their cells.

Select one automatic child and drag it along its existing auto-layout frame to reorder it. An outline and insertion line preview the destination in rows, columns, wrapped rows and grids, including reversed flow and transformed frames. The document stays unchanged during the preview; releasing the pointer commits the reorder and sibling layout as one undo step. Returning to the original slot, Escape or lost pointer capture cancels without a document edit. Dragging stays within the original parent. Use the placement dialog for pinned grid coordinates; moving several layout children together or dragging them into another frame is not implemented.

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

In presentation, an authored Escape key rule takes priority; otherwise Escape returns to the editor.

Shortcuts are handled when the canvas has focus; text fields retain their normal text-editing keys. Shift constrains drawing and resizing, locks a move to an axis, and snaps rotation to 15-degree increments. Dragging snaps to pixels and nearby object edges/centers; quick actions can toggle snapping. The pen creates editable vector paths: click for corners, drag to create Bézier tangents, and finish with Enter or by clicking the first point. Select a vector and press Enter or double-click it to edit anchors. Drag anchors or control handles, double-click a curve to split it, and use Corner/Smooth/Mirror in the sidebar. An entire drag is one undo step.

The vector sidebar also offers Bend and Erase tools. Smooth and mirrored tangents remain constrained during later edits. Convert a path to a vector network to connect branches, shift-select vertices in boundary order, fill a region, join or weld vertices. Split edges retain stable identities and preserve their curves; the complete operation participates in document history.

Double-click a text layer to edit directly on the canvas with a native FerroUI caret, selection, clipboard and IME. Ctrl/Cmd+Enter or moving focus commits one text transaction; Escape restores the original content. Pending IME preedit stays outside the document draft, so lifecycle saves retain the last confirmed text and formatting. Rich-text ranges remain valid UTF-8 byte ranges across edits. **Style selected characters…** opens a native range-selection editor with character styles and a rich preview. Active on-canvas editing currently displays the base layer style.

Use the property sidebar for geometry, fills, strokes, typography and layout. With no layer selected it shows the page background and local styles, variables and components. The paint-stack dialog edits fill order, visibility/opacity, gradients, stops and image source windows. **Crop image…** provides native pan and corner gestures for Fill/Stretch images; Escape cancels the active gesture, and Undo restores the whole gesture. Numeric crop windows are also available for Fit and Tile.

The **Effects** editor provides native controls for drop shadow, inner shadow, layer blur and background blur. Edit blur radius, signed shadow offsets/spread, RGB and separate opacity; show or hide a row, move it up/down, duplicate, remove or clear effects. Enter or leaving a field applies its value, and Escape restores the committed value. Numeric arrow keys step by 1, with Shift for 10 and Alt for 0.1. Multi-selection changes matching effect rows on unlocked layers; missing rows remain unchanged. Each accepted command is one history step, and invalid or unchanged input preserves history and bound styles.

The Appearance blend selector controls isolated layer compositing. Layer opacity applies once to the completed group. The first listed shadow of each kind paints above later shadows of that kind. Leaf shapes paint inner shadows between fills and strokes; container inner shadows sit above their compound content. Background blur samples earlier document content within the isolated group and caches its filtered result; [documented bounds](docs/FEATURES.md#rendering-and-authoring-limits) keep its allocations predictable. Shapes, ordinary fills, strokes and text continue to use native vector rendering.

**Properties and variants…** creates component sets, assigns variant combinations and exposes text, visibility and nested instance-swap properties. Switching a variant preserves matching layer IDs and explicit overrides. The native variable editor manages collections, modes, typed values, aliases and layer bindings. Invalid authoring changes are rejected atomically.

In the Prototype tab, **Edit interactions…** adds trigger/action rules and transitions. Presentation supports animated transitions, Smart Animate, reversible hover/press interactions, scroll containers, wheel scrolling and ScrollTo. Preview changes operate on disposable state; the authored design is restored when presentation ends and remains the source for autosaves. `Layer data…` remains available for validated structured properties without a dedicated control.

## Files and local persistence

The native editable format is a versioned `.ferrodesign` JSON document. It stores the complete document graph, paints, layout, components, variable collections, comments and prototype links. Edits coalesce before serializing the full document as compact JSON for local autosave; portable downloads use the readable pretty format. Both read the authored snapshot during presentation. The browser queues snapshots in IndexedDB. A completed transaction updates the current and previous versions together; the status changes to saved only after completion. Lifecycle saves finish on-canvas text edits and image cropping, then serialize committed document changes. Legacy localStorage documents migrate on a successful write. Conflicting saves from another tab are reported instead of overwritten, and a failed save leaves the document available for export. The native launcher atomically replaces `ferrodesign-autosave.json` and flushes committed document changes before closing. Export a document to make a portable copy.

Native inspector fields commit on Enter or focus loss; numeric arrow keys in Effects also commit. Committed values participate in autosave, but page or window close does not flush a focused, uncommitted field draft. Press Enter or move focus before leaving to apply that draft.

`Open` also imports SVG into a selected editable frame. `Place image` accepts PNG, JPEG, GIF or BMP; FerroUI decodes the image and its data is embedded in the portable document. The native Import dialog accepts JSON, SVG markup or a raster image data URI.

Export options include editable JSON, SVG, PNG rendered through FerroUI's `RenderTargetBitmap`, basic FerroUI XAML, and CSS inspection. SVG/XAML/CSS output have format-specific limits described in the feature matrix. A JSON round trip is the complete fidelity-preserving format for this editor. Figma's proprietary `.fig` file format is not implemented.

## Validation

Implementation `945283ef` passed the complete [FerroDesign workflow](https://github.com/wieslawsoltes/FerroUI/actions/runs/39306364660): 219 core tests, 17 host tests, 64 native application tests, the actual Vello WebAssembly build, 5 Chromium IndexedDB checks, and 62 Software2D / 59 WebGL2 interaction checks with no page or console errors. The separate [framework CI](https://github.com/wieslawsoltes/FerroUI/actions/runs/39306373798) also passed all 14 jobs. [Source-tagged screenshots and raw evidence](docs/evidence/2026-10-19-945283ef/README.md) and [measurement details](docs/VALIDATION.md) preserve the exact test scope and source revision.

```sh
cargo test --locked -p ferrodesign-core
cargo check --locked -p ferrodesign-app
cargo test --locked -p ferrodesign-app --lib -- --test-threads=1
cargo run --release -p ferrodesign-core --example performance -- 10000
npm ci
npx playwright install chromium
node --test tests/document-store.test.mjs
node tests/document-store-browser.mjs
npm run test:browser
```

`tests/browser.mjs` serves the actual `dist` build and exercises both requested browser rendering modes. It checks native pixels, pointer drawing, vector-point and pen-handle gestures, on-canvas Unicode text editing, paint/effect controls, auto-layout drag previews and drops, prototype pointer/key/delay interactions, keyboard history, document import/export, durable reload, and a 10,000-layer document larger than the usual localStorage allowance. ComboBox checks verify the actual native popup item focus, and selection-dependent interactions wait for a committed native frame. The suite also checks startup exceptions and render-thread panics, and writes PNG screenshots, traces and JSON/HTML reports to `browser-report`. The separate IndexedDB test verifies real transaction rollback, migration and conflicting tab writes.

The GitHub Actions workflow `.github/workflows/ferrodesign.yml` runs locked core tests, paired release benchmarks, focused native TextBox preedit tests, public compositing recording/Vello pixel tests, native compilation and the complete native application test suite before the real WebAssembly build and browser interaction validation. The native suite contains pixel comparisons for incremental preparation, spatial root culling and shadow order, plus bounded background blur, gesture ownership and authoring parser regressions. Browser checks also cover networks, components, variables, rich text, crop gestures, page controls, grid spans/reorder, effect transactions and 10,000-layer preparation/root-candidate counts. Build and validation artifacts are attached to each workflow run. See [validation and measurements](docs/VALIDATION.md) for observed results; source coverage and added assertions do not establish a passing run on their own.

## Rendering performance

The renderer retains immutable geometry, paints, text, image assets and prepared subtree bounds. Scoped changes refresh affected nodes and ancestors, and the layer sidebar reuses unchanged row projections. Scenes with at least 32 prepared roots also use a balanced bounds hierarchy to find paint candidates for the viewport. Candidates return to document paint order before compositing, so spatial partitioning preserves stacking. Unchanged viewport queries reuse their candidate buffer; scoped root-bound changes refit the affected tree branch. A view containing the entire root bounds uses the retained paint-order list directly.

These mechanisms reduce preparation, bounds testing and control allocation. Root signatures, visible artwork drawing, wide sibling aggregation, structural layout commits and complete serialization still have size-dependent costs. The report distinguishes those operations from device-specific GPU time and display frame rate. See [architecture](docs/ARCHITECTURE.md#retained-scene-and-invalidation) for cache boundaries and conservative fallbacks.

## Source map

| Path | Responsibility |
| --- | --- |
| `crates/ferrodesign-core/src/model.rs` | Document graph, paints, text, components, variables and validated serialization |
| `crates/ferrodesign-core/src/editor.rs` | Transactions, history and editing operations |
| `crates/ferrodesign-core/src/vector_edit.rs` | Bézier editing, exact curve picking/splitting and affine bounds normalization |
| `crates/ferrodesign-core/src/text_edit.rs` | Unicode-safe rich-text range preservation |
| `crates/ferrodesign-core/src/prototype.rs` | Trigger rules, activation tokens, overlays and variable overrides |
| `crates/ferrodesign-core/src/geometry.rs` | Affine geometry, vector paths and viewport math |
| `crates/ferrodesign-core/src/layout.rs` | Constraints and row/column/wrapped/grid auto layout |
| `crates/ferrodesign-core/src/layout_drag.rs` | Immutable sibling geometry, indexed insertion targets and validated reorder commits |
| `crates/ferrodesign-core/src/effects_edit.rs` | Typed effect commands, atomic batch validation and scoped history |
| `crates/ferrodesign-core/src/spatial.rs` | Spatial index, clipping and geometry-aware hit testing |
| `crates/ferrodesign-app/ui/Editor.xaml` | Compiled FerroUI editor shell and control styling |
| `crates/ferrodesign-app/src/shell.rs` | Control events, property editing and document commands |
| `crates/ferrodesign-app/src/canvas.rs` | FerroUI canvas control, input gestures and selection overlays |
| `crates/ferrodesign-app/src/shell/` | Native authoring dialogs and the cached layer projection |
| `crates/ferrodesign-app/src/prototype.rs` | Presentation snapshots and cancellable FerroUI dispatcher timers |
| `crates/ferrodesign-app/src/renderer.rs` | Cached public-API scene renderer |
| `crates/ferrodesign-app/src/renderer/root_index.rs` | Prepared-root bounds hierarchy, refitting and retained ordered viewport candidates |
| `crates/ferrodesign-app/src/platform.rs` | Browser/native storage, clipboard and downloads |
| `crates/ferrodesign-app/src/desktop.rs` | Vello native configuration and headless rendering |
| `web` / `scripts` / `tests` | Browser host, reproducible build and interaction validation |

See [architecture](docs/ARCHITECTURE.md) for rendering, invalidation and transaction details.

## License

New FerroDesign source is MIT-licensed, consistent with the parent repository. Refer to the repository license and individual dependency licenses. The sample uses its own name and sample artwork; it is not an official Figma product.
