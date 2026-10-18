# Feature coverage

This is an implementation inventory, not a claim of complete Figma parity. “UI” means there is an editing path in this application. “Structured” means the document model and renderer support the data but editing requires the validated layer/collection JSON dialog or a client using the core API. Actual build and interaction evidence is recorded separately in [VALIDATION.md](VALIDATION.md).

## Editor and documents

| Area | Implemented | Limit |
| --- | --- | --- |
| Shell | XAML navigation rail, pages/layers, canvas, property sidebar, bottom toolbar, modal actions and toasts | Visual arrangement is Figma-inspired; pixel parity has not been established |
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
| Vectors | Pen click/drag tangents, freehand pencil, anchor/control-point editing, exact curve picking/splitting, anchor removal and corner/smooth/symmetric conversion | No vector-network topology, bend/erase or variable-width tool; smooth/symmetric conversion is geometric, not a persistent tangent constraint |
| Booleans | Non-destructive union/subtract/intersect/exclude; operand rendering and hit testing | Flattening text/boolean results into exact editable outlines is not implemented |
| Paints | Stacked solids, linear/radial gradients, image paints, opacity/visibility | Native paint dialog edits stack order, visibility/opacity, solid/linear/radial types, gradient stops/geometry and image scale; no color-profile or image-crop-handle parity |
| Strokes | Width, color, alignment, cap/join, dashes and arrow geometry | Native controls for alignment, cap, join and dash lengths; no variable-width stroke editing |
| Corners | Uniform and per-corner radii in the model and renderer | Native uniform/per-corner fields; no continuous-corner smoothing parity |
| Text | Font family/size/weight/style, alignment, line/letter spacing, wrapping, decoration and rich-run rendering | Native on-canvas TextBox caret, selection, IME, one-transaction editing and UTF-8 run preservation; active editing displays base style, and rich-run style authoring remains structured |
| Images | Place PNG/JPEG/GIF/BMP from the browser picker, FerroUI decoding, embedded data-URI persistence and image brush fills | No remote asset service, animated video/GIF authoring or crop-handle UI |
| Effects | Shadow stacks, inner/drop shadows and layer blur through FerroUI | Background blur and full group blend compositing require public API support; diagnostics report unsupported effects |
| Auto layout | Row/column, wrap, grid, fixed/fill/hug dimensions, gap, padding, alignment, absolute children, min/max | Common controls are visual; no complete Figma grid/span and drag-reorder interaction parity |
| Constraints | Start/end/center/stretch/scale behavior when resizing containers | Native horizontal/vertical choices; a graphical constraint diagram is not implemented |
| Export | JSON, SVG, PNG via FerroUI, basic FerroUI XAML, CSS inspection | XAML/CSS are useful approximations, not complete round-trip representations of every paint/layout/effect |

## Components, variables and prototyping

| Area | Implemented | Limit |
| --- | --- | --- |
| Components | Create component, instantiate, detach, synchronize instances while preserving overrides | No remote library publishing, complete variants/slots/property editor or library analytics |
| Styles | Visual creation/application of local fill, text and effect styles; bindings and serialization; grid style data in the core | No full style-definition editor or remote style libraries; grid style application remains unsupported |
| Variables | Collections, modes, typed values, aliases with cycle detection, bindings and mode application | Collection editing is structured; no complete visual variable table or conditional-expression builder |
| Prototypes | Native add/edit/remove interaction dialog, all core triggers/actions, flow starts, presentation, navigate/back, modal overlays and session variable state | Transition definitions are stored but animated interpolation/Smart Animate is not hosted |
| Additional prototype triggers | Browser/native input dispatches hover, press, drag and key rules; native DispatcherTimers fire activation-scoped after-delay rules | Hover fires on entry and drag after a movement threshold; Figma while-hover/while-press/release reversal semantics are not implemented |
| URL/scroll prototype actions | Validated HTTP(S)/mailto/tel links use browser/native host openers; blocked requests are reported | Scroll targets remain explicit unsupported requests; prototype scroll containers are not implemented |
| Comments | Local comment threads, pins, resolve/reopen and serialization | No network delivery, notifications, mentions or concurrent commenting |
| Inspect | Selected-layer CSS and structured properties | No full Figma Dev Mode, code connector, repository mapping or ready-for-dev workflow |

## Features outside this implementation

### SVG import limits

The importer accepts up to 16 MiB of SVG source and validates the resulting document before adding it. An error leaves the document and history unchanged. Unsupported visual constructs are rejected explicitly: arbitrary clip paths/masks, filters, pattern paints, markers, nonzero dash offset, non-scaling strokes, repeat/reflect gradients, offset-focus or unsupported elliptical radial gradients, positioned `tspan`, text paths, text-length constraints, per-character rotations, vertical/explicit RTL text and complex CSS selectors. Rectangular clipping for SVG viewports is supported. Script/event content, DTDs, external resources, reference cycles and duplicate SVG IDs are rejected. Text remains editable; the renderer uses actual FerroUI baseline/anchor metrics, while core text selection bounds are estimated.

### Other products and services

The source does not contain Figma's multiplayer service, accounts, team permissions, live presence, audio calls, plugin/widget runtime, community marketplace, AI services, billing, cloud libraries, full proprietary file interoperability or the separate FigJam/Slides/Sites/Make/Draw product feature sets. A portable design download is the sharing mechanism. Local comments and components do not imply those server products.

The visual reference was Figma's public design-file documentation: [Explore design files](https://help.figma.com/hc/en-us/articles/15297425105303-Explore-design-files), [navigation and left sidebar](https://help.figma.com/hc/en-us/articles/360039831974-Explore-the-navigation-bar-and-left-sidebar), [right sidebar](https://help.figma.com/hc/en-us/articles/360039832014-Design-prototype-and-explore-layer-properties-in-the-right-sidebar), and [toolbar](https://help.figma.com/hc/en-us/articles/360041064174-Access-design-tools-from-the-toolbar). The application uses its own name and editable sample document. There is no same-document reference screenshot test against Figma, so no numerical visual-parity score is asserted.
