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
searchable and navigable. The port records **71 partial pages**, including the live
ScratchPad editor, and **55 pending pages**. There are **35 explicit partial mappings to
original exported examples**. No complete page is claimed as ported or behaviorally
verified. The generated [report](data/REPORT.md) and [summary](data/summary.json) are the
current ledger.

Source comparison identifies **297 type candidates and 1,413 member candidates**.
These are lexical declarations connected through explicit namespace/member maps;
they do not establish signature, default, behavioral or WinMD ABI equivalence.

Execution-backed API evidence currently covers **1 verified type, 36 partial types,
202 verified members and 172 partial members**. The verified type is the complete `Symbol`
enum. Its 197 numeric values account for 197 of the verified members; the remaining five
are narrowly tested icon members. These figures do not imply that 202 independently
implemented control operations or any complete Gallery page have been verified.

The current increment adds native `InfoBadge`, `ProgressRing` and `RatingControl`, their
supporting settings/item descriptors and the two applicable automation peers. The Gallery
adapts both original ProgressRing examples, both RatingControl examples and three of the
four InfoBadge examples. It also preserves the CompactSizing example's navigation between
distinct standard/compact pages, all four text/password fields and its nullable date. The
compact resource dictionary belongs to the compact page; application density stays under
the application's control.

These remain bounded native adaptations. InfoBadge's NavigationView composition is pending;
ProgressRing's custom animated-visual sources are absent; RatingControl's Windows number
formatting, text-scale/template-resource details and raw platform gamepad mapping remain
unsupported or unverified. RatingControl rejects NaN for `Value` and `PlaceholderValue` as
an explicit native validation exception. CompactSizing retains explicit TextBox/composition
adapters for PasswordBox and Header. Installed fonts, exact pixels and Windows accessibility
acceptance remain separate from the passing native tests.

The separate [theme resource/style ledger](../winui-theme/TRACKING.md) records
**6,912 ported, 1,991 waived and 70 pending entries**. The pending entries belong to
InfoBadge (27), ProgressRing (8) and RatingControl (35). These counts concern scoped
source definitions, with the theme's own reference revision; resource lookup support
does not supply a missing source dictionary or establish visual parity. The current
theme suite passes after the reference-fixture storage update, with both compiler
outputs still compared against the complete reference bytes.

| Validation | Current evidence | Boundary |
| --- | --- | --- |
| Existing FerroUI API | 3,170 namespaced types, 28,435 declared member names and 458 source registrations preserved against the starting commit; 82 additive types | Current source-declaration gate passes; full signature/default/behavior compatibility is a separate requirement |
| Native WinUI controls | 127 tests pass: 77 existing tests plus 13 InfoBadge, 16 ProgressRing and 21 RatingControl tests | Native properties, templates, input, clock lifetime and automation; every new control type remains partial |
| Base property/framework regression | 4,383 tests pass on the current source | Includes the additive local-value query; existing clearing and priority semantics are preserved |
| WinUI binding lifecycle | 13 tests pass on the current source | Root/named-element and typed-template property paths with explicit host initialization; unsupported expressions remain diagnosed |
| XAML migration and runtime | 41 integration tests and three deferred-resource error-isolation tests pass on the current source | Includes native feedback-control construction, audited InfoBadge style mapping, nested error scopes and unchanged ordinary panic behavior |
| Compiled and interpreted XAML | Five focused tests pass, with one explicitly ignored regeneration task | Real emitted Rust compiles and agrees with interpreted root and typed-template property bindings; standalone DataTemplate document roots are outside the emitter contract |
| Native view builders | `cargo check -p ferroui-view` passes | Generated typed builders compile; this does not establish every builder's runtime behavior |
| Native Gallery | 42 tests pass | Catalog, original-example interactions, local compact resources, assets, lifetime and mocked host-service contracts |
| WinUI theme | 294 tests pass, with two explicitly ignored regeneration/timing tasks | Includes exact joined-reference comparisons and four reference-storage tests; source-resource and visual gaps remain explicit |
| Native rendering and desktop | Current checkpoint: all 126 light-theme pages and nine dark-theme pages render; desktop compilation passes | Every frame decodes as nonblank 1280 × 900 PNG; 126/9 distinct sweep hashes. Eight new retained frames were reviewed. Missing Segoe glyphs remain visible; interactive desktop, upstream pixels and Windows accessibility are unverified |
| Source comparison | 44 scanner/comparison regressions and two theme path-case tests pass; API, event, symbol, builder, theme, catalog and asset freshness gates pass | A source candidate is never automatically promoted to behavioral parity; the strict completeness gate remains closed |

The [Gallery validation ledger](../../../samples/WinUIGallery/VALIDATION.md) records exact
commands, source hashes, assets, rendering evidence and platform limits. Durable native
control executions are stored in [the execution records](data/executions/). The current
[third record](data/executions/native-controls-003.json) contains the 50 new focused test
names and source hashes from the passing 127-test run. Any supplied execution record is
checked for freshness even when its API entry is only partial.

Current rendered evidence uses native source checkpoint
`3c7ceb81dcc4948f374c05c4757c6cb72ed5de5d`. The
[capture manifest](captures/capture-manifest.json) keeps the four dedicated light
captures separate from the 126-page light sweep and nine-page dark sweep. The eight
new reviewed frames show InfoBadge, ProgressRing, RatingControl and CompactSizing in
both themes; CompactSizing is captured in its initial Standard state. Five retained
`d008845` frames remain explicitly historical.

## Review and run

- [API design and remaining contract obligations](API-DESIGN.md)
- [Native control contracts and execution boundaries](CONTROL-CONTRACTS.md)
- [Native Gallery, run commands and page boundaries](../../../samples/WinUIGallery/README.md)
- [Source scanner, evidence format and regeneration](../../../scripts/winui-gallery/README.md)
- [Reviewed API mappings](data/api-mappings.json) and [generated use-site mappings](data/gallery-xaml-map.json)

```sh
cargo run -p winui-gallery --features desktop -- --page=NumberBox
cargo test --locked --offline -p winui-gallery --lib
cargo test --locked --offline -p ferroui-base \
  --features testing,compiler-metadata,markup-functions --lib
cargo test -p ferroui-markup-xaml --test winui_bindings
cargo check -p ferroui-view
cargo test --locked --offline -p ferroui-markup-xaml-loader \
  --features emitter,ferroui-base/markup-functions --test winui_migration --test winui_compiled
cargo test --locked --offline -p ferroui-themes-winui --lib
cargo check --locked --offline -p winui-gallery --features desktop
cargo test --locked --offline -p ferroui-controls \
  --features testing,ferroui-base/compiler-metadata,ferroui-base/markup-functions \
  --test winui_navigation --test winui_number_box --test winui_icons --test winui_radio_buttons \
  --test winui_auto_suggest_box --test winui_icon_sources --test winui_info_bar \
  --test winui_info_badge --test winui_progress_ring --test winui_rating_control --no-fail-fast
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
The current strict run reports 1,544 types, 7,264 members and all 126 Gallery pages
without complete behavioral execution evidence; partial contract tests do not satisfy
that full-completion requirement.
