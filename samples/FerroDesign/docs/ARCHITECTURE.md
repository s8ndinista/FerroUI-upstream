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

The benchmark in `ferrodesign-core/examples/performance.rs` measures document creation, indexing, hit tests, viewport queries, drag refresh and undo/redo for a synthetic grid. It does not measure the canvas renderer, the layout of the sidebar control tree, GPU time or display frame rate.

## Editing and history

An `Editor` owns a document, the active page, selection, clipboard, spatial scene and history. A pointer gesture starts one transaction, applies any number of intermediate updates, then commits one undo entry. Escape cancels the transaction and restores its initial state. Nested editing helpers participate in the active transaction rather than adding one history item per pointer event.

History records changed nodes and document metadata. Unchanged nodes are compared without cloning them into the committed delta. Grouping, reparenting and ungrouping preserve world geometry; duplication remaps internal node and prototype references. World movement is converted through the parent inverse transform so that children of rotated containers move correctly.

Constraint resizing updates children relative to the old and new container extent. Auto layout implements row, column, wrapping, fixed/fill/hug sizing, padding, spacing, alignment, min/max limits, absolute children and grid placement. The visual property panel exposes common settings; the structured layer editor exposes the remaining stored settings.

## Prototypes and local services

The core prototype player is a deterministic state machine for navigation history, overlays, variables, pending URLs and scroll targets. The shell hosts click navigation, presentation, overlays and return to editing. Presentation keeps an original document snapshot so runtime variable mutations do not overwrite authored values. Stored transition data is richer than the current presentation host; see the feature matrix before relying on animation or non-click triggers.

All persistence is local. JSON is validated before replacing a document. The browser host stores an autosave and downloads portable files. Comments, components, variables and history are local concepts; no authentication, collaboration transport or hosted sharing service is implied by their presence in the UI.

## Extending the editor

Add document semantics to the core with a focused test for the invariant being changed. Add a command to the shell and bind it through the XAML controls or quick actions. Keep scene drawing in `ScenePainter` and use public FerroUI media types so that browser, desktop and PNG export retain a common rendering path. Keep host-only operations in `platform.rs`; browser DOM should not acquire a second editor implementation.
