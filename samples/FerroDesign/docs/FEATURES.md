# Feature coverage

This is an implementation inventory, not a claim of complete Figma parity. “UI” means there is an editing path in this application. “Structured” means the document model and renderer support the data but editing requires the validated layer/collection JSON dialog or a client using the core API. Actual build and interaction evidence is recorded separately in [VALIDATION.md](VALIDATION.md).

## Editor and documents

| Area | Implemented | Limit |
| --- | --- | --- |
| Shell | Compiled XAML navigation rail, pages/layers, contextual page/layer inspector, bottom toolbar, native modal actions and toasts | Visual arrangement follows the public design-file reference; a same-document pixel-parity comparison has not been established |
| Pages | Add and switch pages; separate roots, backgrounds and flow starts | No cloud projects or team file browser |
| Layers | Select, search, expand/collapse, rename, hide, lock and change stacking order | Native ItemsControl/VirtualizingStackPanel realizes viewport rows; selection refresh retains the item source. Browser tests check bounded rows for 10,000 layers |
| Canvas | Pan, pointer-anchored zoom, fit page/selection, grid/ruler overlay | No multiplayer cursors or synced views |
| Selection | Shape-aware hits, clipping, shift selection, marquee, root selection normalization | Advanced Figma selection cycling and all selection preferences are absent |
| Transform | Move, corner resize, rotation, flip, keyboard nudge, pixel/object snapping with guides | No general visual skew tool; model can preserve affine transforms |
| History | Transactional undo/redo, gesture cancellation, history labels | Local session history, not shared server version history |
| Clipboard | Copy/cut/paste/duplicate with remapped internal references | No Figma clipboard binary interoperability |
| Organization | Group/frame selection, ungroup, align/distribute, layer order | No collaborative branches, review or merge workflow |
| Files | Complete native JSON serialization, validation, autosave, import/export | IndexedDB current/previous snapshots, migration and conflict detection; no `.fig` reader/writer or hosted file service |
| SVG import | Editable frame import, nested viewports/groups, arbitrary affine transforms, shapes, all path commands, supported gradients, Unicode text/runs, internal reuse and embedded images | Unsupported constructs fail atomically; detailed limits are below |

## Artwork and layout

| Area | Implemented | Limit |
| --- | --- | --- |
| Shapes | Frames, groups, rectangles, ellipses, lines, polygons, stars, sections | Device-preset picker and every Figma shape property are not implemented |
| Vectors | Pen click/drag tangents, freehand pencil, persistent corner/smooth/symmetric constraints, exact curve picking/splitting/bending, anchor and segment erasure | Native point/bend/erase tools preserve one-gesture history and affine geometry; variable-width strokes are absent |
| Vector networks | Stable vertices and edges, branches, self-loops, parallel edges, explicit filled regions/holes, connection, splitting, welding, bending and erasure | Native conversion and topology controls; regions share the layer paint stack. Automatic face extraction at arbitrary intersections and separate region paints are absent |
| Booleans | Non-destructive union/subtract/intersect/exclude; operand rendering and hit testing | Flattening text/boolean results into exact editable outlines is not implemented |
| Paints | Stacked solids, linear/radial gradients, image paints, opacity/visibility, blend-mode selection | Native paint dialog edits order, visibility/opacity, paint types, stops/geometry, image scale and normalized source window; no complete color-profile, mesh-gradient or angular-gradient workflow |
| Strokes | Width, color, alignment, cap/join, dashes and arrow geometry | Native controls for alignment, cap, join and dash lengths; no variable-width stroke editing |
| Corners | Uniform and per-corner radii in the model and renderer | Native uniform/per-corner fields; no continuous-corner smoothing parity |
| Text | Font family/size/weight/style, alignment, line/letter spacing, wrapping, decoration and rich-run rendering | Native on-canvas TextBox caret, selection, IME, one-transaction editing and UTF-8 run preservation. A native character-selection dialog applies and clears range styling with a rich preview. Active on-canvas editing still displays the base layer style |
| Images | Place PNG/JPEG/GIF/BMP, public FerroUI decoding, embedded persistence, all four image scale modes and cropped image brushes | Native source-window fields work with Fill/Fit/Stretch/Tile. Direct pan/corner crop handles support Fill/Stretch leaf shapes, affine pixel anchoring, cancel/capture handling and one-gesture history; no remote asset service or animated media authoring |
| Effects | Shadow stacks, inner/drop shadows, layer blur, isolated group opacity/blend, cached background blur through public FerroUI APIs | Background blur samples document content within the current isolated group. Its bounded fallback can decline oversized/deep effects with diagnostics; see rendering limits below. Pass-through groups and complete Figma effect/color-management parity are absent |
| Auto layout | Row/column, wrap, grid, fixed/fill/hug dimensions, gap, padding, alignment, absolute children, min/max; explicit/automatic grid cells and row/column spans | Native child sizing/placement dialog and Move before/after controls follow reversed flow. Grid tracks use this engine's deterministic sizing rules; direct canvas drag-reordering and complete Figma track-sizing parity are absent |
| Constraints | Start/end/center/stretch/scale behavior when resizing containers | Native horizontal/vertical choices; a graphical constraint diagram is not implemented |
| Export | JSON, SVG, PNG via FerroUI, basic FerroUI XAML, CSS inspection | XAML/CSS are useful approximations, not complete round-trip representations of every paint/layout/effect |

## Components, variables and prototyping

| Area | Implemented | Limit |
| --- | --- | --- |
| Components | Create/instantiate/detach, component sets, named variant combinations, stable-ID variant switching, text/boolean/nested-instance properties, reset and override-preserving synchronization | Native component/property/variant dialogs; definitions target source layers. No remote library publishing, full slot semantics, analytics or cross-file library subscription |
| Styles | Visual creation/application of local fill, text and effect styles; bindings and serialization; grid style data in the core | No full style-definition editor or remote style libraries; grid style application remains unsupported |
| Variables | Native collection/mode/value/alias/binding controls, typed color/number/string/boolean values, cycle detection and mode application | Invalid authoring operations are atomic. Legacy imported variable data is preserved and diagnosed; no remote collections or complete expression-builder interface |
| Prototypes | Native interaction authoring, flow starts, navigate/back/overlays, session variable state, transition interpolation and Smart Animate | Matching layers interpolate supported geometry/paint/text properties; incompatible artwork uses a crossfade. No complete Figma timeline, spring/physics or video playback system |
| Additional prototype triggers | Hover, press, drag, key and activation-scoped native delay timers, reversible while-hover/while-press state | Reversal restores state across temporary frame/variant IDs; no shared presentation session or every Figma gesture preference |
| URL/scroll prototype actions | Validated HTTP(S)/mailto/tel openers, scroll-container bounds, ScrollTo actions and clamped native wheel scrolling | Scroll updates the disposable prototype document, retaining authored positions and saves. No full momentum/touch scroll physics or device emulation |
| Comments | Local comment threads, pins, resolve/reopen and serialization | No network delivery, notifications, mentions or concurrent commenting |
| Inspect | Selected-layer CSS and structured properties | No full Figma Dev Mode, code connector, repository mapping or ready-for-dev workflow |

## Features outside this implementation

### Rendering and authoring limits

Normal artwork remains vector geometry, brushes, pens and text submitted through FerroUI. Background blur is the bounded raster operation: each padded surface supports at most 2048 pixels per dimension and 4 million pixels, with 32 MiB of retained backdrop cache, 128 MiB of conservative working reservations and eight nested filters. Canvas sampling follows zoom and display scaling; PNG sampling follows export resolution. Panning reuses cached filtered output. A declined filter increments explicit renderer diagnostics and renders the remaining layer content; it is not silently presented as a successfully rendered blur.

The new `DrawingContext::try_push_compositing_layer` capability records/replays isolated blend and opacity scopes through FerroUI. Vello implements it; an unsupported backend returns `None` without changing the state stack. Group opacity is applied to completed overlapping children once. Background replacement operates on premultiplied pixels within document isolation, so it does not copy the editor canvas background into a transparent export.

Vector-network regions are explicit topological contours. Branching edges may exist without fills, and implicit fill-closing edges can remain unstroked. Stable IDs survive splitting, movement and undo. Geometry normalization preserves world placement, including reflected or sheared parents. These capabilities do not imply automatic curve-intersection topology or variable-width geometry.

Component variant changes match source identity, then layer name/kind and sibling occurrence. Matching materialized layer IDs and explicit overrides survive a switch. Exposed property controls display effective direct descendant overrides; setting or resetting that property clears its corresponding direct flags. Component synchronization can affect dependent instances and layout, so it still uses full document history snapshots. The performance benchmark reports this cost separately from scoped single-node editing.

Scoped canvas edits use an editor change feed to refresh affected render data and ancestor bounds. Camera frames retain prepared geometry, text and paints. Structural, component, variable and layout transactions retain full preparation; missing history or unusual dependency sets also fall back to a complete refresh. Preparation counters distinguish this work from ordinary drawing: root signatures and paint traversal still visit root IDs, and wide ancestor bounds still inspect siblings. A one-leaf preparation result is not a constant-time full frame or an FPS measurement.

Grid placement is zero-based in the serialized model and one-based in native fields; empty/Auto leaves a coordinate unspecified. Pinned children reserve cells before automatic children, while hidden and absolute children do not occupy tracks. Invalid spans and occupied-cell collisions are rejected atomically by authoring commands. Connected groups of overlapping row spans use equal-height tracks for stable repeated Hug/Fill layout. Existing grids without nondefault placement metadata retain their earlier layout path.

### SVG import limits

The importer accepts up to 16 MiB of SVG source and validates the resulting document before adding it. An error leaves the document and history unchanged. Unsupported visual constructs are rejected explicitly: arbitrary clip paths/masks, filters, pattern paints, markers, nonzero dash offset, non-scaling strokes, repeat/reflect gradients, offset-focus or unsupported elliptical radial gradients, positioned `tspan`, text paths, text-length constraints, per-character rotations, vertical/explicit RTL text and complex CSS selectors. Rectangular clipping for SVG viewports is supported. Script/event content, DTDs, external resources, reference cycles and duplicate SVG IDs are rejected. Text remains editable; the renderer uses actual FerroUI baseline/anchor metrics, while core text selection bounds are estimated.

### Other products and services

The source does not contain Figma's multiplayer service, accounts, team permissions, live presence, audio calls, plugin/widget runtime, community marketplace, AI services, billing, cloud libraries, full proprietary file interoperability or the separate FigJam/Slides/Sites/Make/Draw product feature sets. A portable design download is the sharing mechanism. Local comments and components do not imply those server products.

The visual reference was Figma's public design-file documentation: [Explore design files](https://help.figma.com/hc/en-us/articles/15297425105303-Explore-design-files), [navigation and left sidebar](https://help.figma.com/hc/en-us/articles/360039831974-Explore-the-navigation-bar-and-left-sidebar), [right sidebar](https://help.figma.com/hc/en-us/articles/360039832014-Design-prototype-and-explore-layer-properties-in-the-right-sidebar), and [toolbar](https://help.figma.com/hc/en-us/articles/360041064174-Access-design-tools-from-the-toolbar). The application uses its own name and editable sample document. There is no same-document reference screenshot test against Figma, so no numerical visual-parity score is asserted.
