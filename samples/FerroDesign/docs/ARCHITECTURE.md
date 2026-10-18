# Architecture

## Runtime boundaries

`ferrodesign-core` is a UI-independent Rust library. It owns document semantics and can be tested or used by another client without initializing FerroUI. `ferrodesign-app` supplies the native control tree, gesture handling, property editors and rendering. Browser and native entry points configure platform services, then construct the same application.

`Editor.xaml` is compiled by the `ferroui-build` build task and loaded through a `ferres://FerroDesign/ui/Editor.xaml` resource URI. The shell uses actual FerroUI `Grid`, `StackPanel`, `ScrollViewer`, `Button`, `TextBox`, `ComboBox`, `Border` and related controls. The application registers the Inter font and the FerroUI Fluent theme. Control events and the document state are wired with FerroUI `Ref<T>` handles and weak `Rc` controller references.

The custom `DesignCanvas` extends FerroUI `Control` with `ferro_class!` and implements the normal rendering and input contracts. It handles routed pointer/key events, pointer capture, focus, clipping and visual invalidation. No HTML element substitutes for a design layer or a property editor. Browser JavaScript is limited to the runtime bridge and host capabilities.

## Rendering through FerroUI

`ScenePainter` accepts a `DrawingContext`, a `Document`, the root node IDs and a document-space viewport. It knows no Vello, Skia, OpenGL, WebGPU or platform framebuffer types. Its primitives include:

| Document feature | Public FerroUI API |
| --- | --- |
| Paths, polygons, curves and ellipses | `StreamGeometry`, `IGeometryContext`, `EllipseGeometry`, `RectangleGeometry`, `GeometryGroup` |
| Boolean combinations | `CombinedGeometry` and `GeometryCombineMode` |
| Solid and gradient fills | Immutable solid, linear-gradient and radial-gradient brushes |
| Raster image fills | `Bitmap`, `ImmutableImageBrush`, stretch/alignment/tile modes |
| Strokes | `ImmutablePen`, dash style, line caps/joins and widened geometry for alignment |
| Text and rich text runs | `TextLayout`, run properties/cache, glyph runs and a text drawing sink |
| Nested geometry | `push_transform`, clip and opacity state on the drawing context |
| Shadows and layer blur | `BoxShadow`, `BoxShadows`, geometry and `ImmutableBlurEffect` |
| PNG export | `RenderTargetBitmap` and the FerroUI bitmap encoder |

The browser app disables FerroUI.Browser's default features and enables only `vello`. The upstream browser change makes its existing Skia dependency optional and gates the Skia-specific GL implementation. Default upstream consumers continue to select Skia unless they opt out; this application does opt out. The build checks the resolved target graph rather than grepping `Cargo.lock`, which can contain optional packages that are not linked.

FerroUI's browser Vello backend selects WebGL2 hybrid rendering when available, with Vello CPU/Software2D as fallback. On native platforms the launcher configures Vello directly and avoids the desktop convenience package that would introduce Skia.

## Retained scene and invalidation

The document has stable node IDs and parent-relative affine geometry. The editor updates a scene index after a document change. Pure paint/text changes preserve the spatial index; geometric changes update the affected subtree; hierarchy, visibility or stacking changes rebuild traversal where required. Tests compare incremental results to a full rebuild, including clipped transformed descendants and root-level reorder.

The renderer caches immutable geometries, brushes, pens, shaped text and image assets. A prepared traversal stores subtree bounds and skip ranges. Camera-only changes reuse the prepared scene; offscreen subtrees can be skipped without rebuilding path or text data. A document change refreshes prepared state and reuses unchanged visual cache entries. Missing images and effects that the public API cannot express appear in render diagnostics.

`DesignCanvas` owns the camera. World/screen conversion and zoom anchoring live in the core `Viewport` type. The canvas adds selection bounds, resize/rotate handles, hover outlines, alignment guides, frame labels, rulers and local comment pins after the artwork pass.

The layer sidebar is an ItemsControl with a VirtualizingStackPanel and a flattened hierarchy of lightweight row values. Native control containers are realized for the viewport. Selection-only updates reuse the item source; metadata changes rebuild it. The shell keeps weak row references so scrolling can release offscreen controls.

The benchmark in `ferrodesign-core/examples/performance.rs` measures document creation, indexing, hit tests, viewport queries, drag refresh and undo/redo for a synthetic grid. It does not measure the canvas renderer, the layout of the sidebar control tree, GPU time or display frame rate.

## Editing and history

An `Editor` owns a document, the active page, selection, clipboard, spatial scene and history. A pointer gesture starts one transaction, applies any number of intermediate updates, then commits one undo entry. Escape cancels the transaction and restores its initial state. Nested editing helpers participate in the active transaction rather than adding one history item per pointer event.

History records changed nodes and document metadata. Unchanged nodes are compared without cloning them into the committed delta. Grouping, reparenting and ungrouping preserve world geometry; duplication remaps internal node and prototype references. World movement is converted through the parent inverse transform so that children of rotated containers move correctly.

Constraint resizing updates children relative to the old and new container extent. Auto layout implements row, column, wrapping, fixed/fill/hug sizing, padding, spacing, alignment, min/max limits, absolute children and grid placement. The visual property panel exposes common settings; the structured layer editor exposes the remaining stored settings.

Interactive vector editing uses stable path-command/point identifiers within a gesture. Curve splitting preserves cubic and quadratic shape; nearest-point queries isolate polynomial stationary points instead of using display tessellation. Recomputing tight bounds compensates the node transform, including the center-of-rotation change and reflected/sheared parents.

The on-canvas text editor is a FerroUI TextBox transformed into the selected layer coordinate system. The scene temporarily omits that text layer without changing its authored visibility. Synchronous text-changing events update the open transaction, and focus loss/Ctrl+Enter commits it; Escape cancels it. A minimal Unicode character diff remaps rich-run UTF-8 byte ranges. The native control owns caret navigation, selection, IME and text clipboard handling.

## Prototypes and local services

The core prototype player is a deterministic state machine for navigation history, modal overlays, session variable overrides and external requests. Pointer events and normalized key chords dispatch through the active root; after-delay rules use native FerroUI DispatcherTimer subscriptions. Activation tokens prevent stale timers from firing after navigation or overlay closure, and unrelated overlay changes do not rearm an existing timer. Long delays are split into cancellable chunks; zero delays yield a UI turn.

The runtime snapshots the authored document, selection, active page, camera and tool. Presentation works on a disposable document and restores the snapshot on exit. Serialization during presentation reads the authored snapshot, so variable bindings cannot autosave preview values into the design. Stored transition definitions are preserved; animated transition interpolation and Smart Animate are still absent. External links are validated before the platform opener is invoked. Content-scroll requests are reported rather than silently substituted with camera movement.

All persistence is local. JSON is validated before replacing a document. The browser host uses a coalescing IndexedDB save queue and downloads portable files. Current/previous records commit atomically, failure status remains visible, and revision comparisons prevent a stale tab from overwriting another tab. Status notifications are posted asynchronously to avoid JavaScript-to-Rust reentry while a shell callback is active. Visibility/page lifecycle events flush pending work; unsaved changes use a before-unload indication. A storage failure never reports a successful save. Native saves use a temporary file and atomic replacement. Comments, components, variables and history are local concepts; no authentication, collaboration transport or hosted sharing service is implied by their presence in the UI.

## Extending the editor

Add document semantics to the core with a focused test for the invariant being changed. Add a command to the shell and bind it through the XAML controls or quick actions. Keep scene drawing in `ScenePainter` and use public FerroUI media types so that browser, desktop and PNG export retain a common rendering path. Keep host-only operations in `platform.rs`; browser DOM should not acquire a second editor implementation.
