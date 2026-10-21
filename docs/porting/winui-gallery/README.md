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
- [ ] Generate complete WinUI declaration and Gallery requirement inventories.
- [ ] Implement the opt-in XAML projection and conflict-safe controls.
- [ ] Port the Gallery catalog, shell and interactive samples.
- [ ] Add deterministic API/usage maps and automated regression/completeness gates.
- [ ] Validate runtime loading, interaction, rendering and platform behavior.
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
