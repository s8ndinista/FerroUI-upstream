# WinUI API compatibility and Gallery port

This work adds an **opt-in** WinUI API and XAML projection to FerroUI while preserving the
existing Avalonia-derived API, property defaults, resource precedence and control behavior.
The acceptance target is the complete pinned Microsoft WinUI Gallery, including its XAML,
code-behind interactions, navigation, accessibility and platform services. A catalog entry,
an API with the same name, or a successful parse alone does not establish behavioral parity.

## Reference revisions

| Project | Revision | Purpose |
| --- | --- | --- |
| [FerroUI](https://github.com/wieslawsoltes/FerroUI) | `86b6b4ab41669bf830fba681504a4ee94752a4e6` | Starting framework API |
| [Microsoft WinUI](https://github.com/microsoft/microsoft-ui-xaml) | `a71c42bd3e352060e5f5ecf3b5e1a98f8a745809` | Public API declarations and implementation |
| [Microsoft WinUI Gallery](https://github.com/microsoft/WinUI-Gallery) | `14061eeac21c38ae99f302edf3d9136d4f652c4c` | Actual application, page catalog and sample requirements |

The source repositories are read-only references. Their commit IDs and source hashes belong
in every generated inventory so that an upstream update cannot silently change the denominator.

## Architecture

1. **Shared framework services.** Reuse FerroUI's object/property system, collections, layout,
   routed input, text, rendering, styles, resources, bindings, automation, dispatcher and
   platform interfaces. There is one visual tree and one renderer.
2. **Compatible controls.** Use existing controls directly when their contracts match. Add
   missing general-purpose members without changing existing defaults. When the WinUI
   contract conflicts with an existing control, expose an explicit control in
   `FerroUI.Controls.WinUI` / `ferroui_controls::winui` rather than replacing the original.
3. **Explicit XAML migration.** An opt-in, diagnostic-producing migration step converts
   audited namespace/type/member equivalents. Its output is ordinary FerroUI XAML for the
   existing runtime loader or compiler. Unsupported syntax and semantic differences remain
   diagnostics; no missing property, event or child may be silently discarded.
4. **Native Gallery.** The port is a FerroUI application. It preserves canonical upstream
   page IDs, categories, descriptions and source references. Interactive examples use real
   FerroUI controls. Catalog coverage, executable examples and verified behavior are reported
   separately.
5. **Evidence-driven parity.** Generated source/API/usage maps record the exact upstream
   declaration or use, the target API, status, tests and remaining differences. Strict gates
   reject unresolved requirements; inventories do not masquerade as implementation evidence.

## Compatibility rules

| Concern | Rule |
| --- | --- |
| Existing markup | The existing FerroUI namespace and loader behavior remain the default. |
| Existing Rust API | Existing public types, methods and defaults retain their behavior. |
| Same name, different contract | Use a WinUI-specific projection with explicit semantics. |
| `Page` / navigation | Preserve existing page/navigation classes; add WinUI navigation separately. |
| Binding | Preserve source, lifetime and default-mode semantics; `x:Bind` is not a text substitution for `Binding`. |
| Resources | `ThemeResource` must re-evaluate through the framework resource system; static lookup is not equivalent. |
| Styles and visual states | Preserve precedence and state ownership; unsupported state transitions remain explicit. |
| Host services | Route through existing platform abstractions and report actual capabilities. No success-returning stand-ins. |
| Upstream dependencies | Track Windows App SDK, WinRT, community controls and Gallery-local types separately from WinUI XAML. |
| Validation | Compiling, loading, executing, rendering and native comparison are distinct evidence levels. |

## Delivery sequence

- [x] Inspect the framework and pin source revisions.
- [x] Establish the nonbreaking architecture and open a working PR.
- [x] Generate pinned WinUI source-declaration and Gallery requirement inventories.
- [x] Implement the opt-in XAML projection and initial conflict-safe controls.
- [x] Port the complete exported catalog, native shell and a documented interactive subset.
- [x] Add deterministic API/usage maps and automated regression/completeness gates.
- [x] Execute the implemented control, binding, migration and Gallery behavior tests.
- [ ] Complete native rendering and platform comparison acceptance for every original example.
- [ ] Reconcile every remaining requirement and publish a full parity report.

## Related framework work

The existing [WinUI theme](../winui-theme.md), [theme comparison gallery](../winui-theme-gallery.md)
and [WinUI native host](../winui.md) are reusable infrastructure. This port targets the actual
Microsoft Gallery and its API requirements; it does not rename the theme test application.
Active rich-text, repeater, charts and inking work must be accounted for at its actual merged
or branch state. Features in another unmerged PR are dependencies, not implemented mainline APIs.

## Current acceptance state

**Implementation in progress. Full API, behavior, visual and platform parity is not yet
established.** The PR remains draft until the complete requirement map has executable evidence
and all strict acceptance gates pass. Subsequent commits update this record with measured
counts, commands, exact results and limitations.

The pinned source inventory contains **1,545 public-source types, 7,466 public-source
members and 198 separately counted private-source types**. The Gallery has **126 catalog
pages, 350 exported examples and 355 actual ControlExample instances**. All 126 pages are
searchable and navigable. The port records **67 partial pages** (66 markup adaptations plus
the live ScratchPad editor) and **59 pending pages**. No complete page is claimed as ported
or behaviorally verified. The generated [report](data/REPORT.md) is the current ledger.

Execution-backed API evidence currently covers **1 verified type, 26 partial types,
202 verified members and 111 partial members**. The verified type is the complete `Symbol`
enum. Its 197 numeric values account for 197 of the verified members; the remaining five
are narrowly tested icon members. These figures do not imply that 202 independently
implemented control operations or any complete Gallery page have been verified.

| Validation | Current evidence | Boundary |
| --- | --- | --- |
| Existing FerroUI API | 3,170 namespaced types, 28,435 declared member names and 458 source registrations preserved against the starting commit | Source declarations; signature/default/behavior compatibility still needs review and runtime tests |
| Native WinUI controls | 77 tests pass across navigation, NumberBox, RadioButtons, icons, reusable icon sources, AutoSuggestBox and InfoBar | Explicit tested subsets; the control types remain partial |
| WinUI binding lifecycle | 13 tests pass | Root/named-element and typed-template property paths with explicit host initialization; unsupported expressions remain diagnosed |
| XAML migration and runtime | 38 integration tests pass; three deferred-resource error-isolation tests pass | Opt-in policy; existing loading and binding defaults remain unchanged |
| Compiled and interpreted XAML | Five focused tests pass, with one explicitly ignored regeneration task | Real emitted Rust compiles and agrees with interpreted root and typed-template property bindings; standalone DataTemplate document roots are outside the emitter contract |
| Native Gallery | 33 tests pass | Catalog, native adaptations, source interactions, assets and mocked host-service contracts |
| Native rendering and desktop | All 126 light-theme pages and five dark-theme pages render; the desktop application compiles | Real Skia/HarfBuzz rendering and original bitmap decoding; no upstream pixel, OS-input or screen-reader equivalence claim |
| Source comparison | 44 scanner, glyph, namespace-builder and baseline regression tests pass | A source candidate is never automatically promoted to behavioral parity |

The [Gallery validation ledger](../../../samples/WinUIGallery/VALIDATION.md) records exact
commands, source hashes, assets, rendering evidence and platform limits. Durable native
control executions are stored in [the execution records](data/executions/); any supplied
execution record is checked for freshness even when its API entry is only partial.

## Review and run

- [API design and remaining contract obligations](API-DESIGN.md)
- [Native control contracts and execution boundaries](CONTROL-CONTRACTS.md)
- [Native Gallery, run commands and page boundaries](../../../samples/WinUIGallery/README.md)
- [Source scanner, evidence format and regeneration](../../../scripts/winui-gallery/README.md)
- [Reviewed API mappings](data/api-mappings.json) and [generated use-site mappings](data/gallery-xaml-map.json)

```sh
cargo run -p winui-gallery --features desktop -- --page=NumberBox
cargo test -p winui-gallery --lib
cargo test -p ferroui-markup-xaml --test winui_bindings
cargo test -p ferroui-markup-xaml-loader --test winui_migration
cargo test -p ferroui-markup-xaml-loader --features emitter --test winui_compiled
cargo test -p ferroui-controls --features testing \
  --test winui_navigation --test winui_number_box --test winui_icons --test winui_radio_buttons \
  --test winui_auto_suggest_box --test winui_icon_sources --test winui_info_bar
python3 -m unittest discover -s scripts/winui-gallery -p 'test_*.py'
python3 scripts/winui-gallery/scan.py --check
python3 scripts/winui-gallery/existing_api.py --check
python3 scripts/winui-gallery/event_targets.py --check
python3 scripts/winui-gallery/symbol_map.py --check
```

The `WinUI and Gallery parity` workflow runs freshness, existing-API, native contract,
compiled-binding and full-catalog rendering checks. Its
separate `require_complete` workflow-dispatch option runs
`python3 scripts/winui-gallery/scan.py --check --require-complete`; this gate intentionally
fails while API contracts or complete Gallery pages lack current behavioral execution evidence.
