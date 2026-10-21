# Additive WinUI contracts and the Gallery port

## Objective and compatibility boundary

The target is a native FerroUI application that can keep the structure, properties,
events and behavior of the WinUI Gallery's XAML. A port should principally change
namespaces and explicitly mapped types. Its C# code-behind still needs a Rust
implementation and registered application metadata; neither an XML rewrite nor a
source browser executes C#.

FerroUI's existing Avalonia-compatible object model, APIs, markup and default
behavior remain the foundation. The opt-in API is `ferroui_controls::winui` in
Rust and `FerroUI.Controls.WinUI` in XAML. It is deliberately absent from the
default presentation namespace: both the existing `Page` hierarchy and the new
WinUI `Page` must remain addressable without changing an old document's meaning.

The source contracts are pinned in [sources.lock.json](data/sources.lock.json).
The complete declaration and use-site inventory, rather than the examples in
this document, defines the remaining work. [REPORT.md](data/REPORT.md) states
the measured port state. This design includes required future contracts and
must not be read as a claim that all of them are implemented.

## Projection rules

| Situation | Required implementation |
|---|---|
| Same API and semantics | Reuse the existing native type and registered member. |
| Different spelling, same audited meaning | Add a per-document namespace, type or member mapping; retain a source-range edit record. |
| Additional non-conflicting capability | Add a normal native property, event, method or service with existing conventions. |
| Conflicting default, value domain or lifetime | Add a type in `Controls.WinUI`; keep the original type and its defaults. |
| Platform-dependent capability | Implement the corresponding platform service and expose the actual result/capability. |
| Unsupported requirement | Keep its declaration/use site in the parity ledger and produce an actionable error. |

Every projected control uses the same visual tree, object/property registry,
layout, input, dispatcher, text system, renderer, resource system and automation
infrastructure as other FerroUI controls. There is no second rendering engine
or separate emulation tree. Native templates are replaceable through the
existing template APIs.

The code generator resolves scoped identities before deriving builders and
property types. Existing `PageProps`, `ContentPage` and their inheritance remain
unchanged. Opt-in builders have explicit aliases such as `WinUIPage`,
`WinUINumberBox`, `win_ui_page()` and `win_ui_number_box()`.

## Implemented control foundations

### Page and Frame

`Page` derives from native `ContentControl` and adds a weak `Frame` link,
`NavigationCacheMode` and virtual `OnNavigatingFrom`, `OnNavigatedFrom` and
`OnNavigatedTo` callbacks. `Frame` is a native content host. A page type can be
created through its registered constructor or an explicit `PageFactory`; the
factory must return a page compatible with the requested type.

`Frame` exposes `Navigate`, `NavigateToType`, `GoBack`, `GoForward`,
`CanGoBack`, `CanGoForward`, `BackStackDepth`, `SourcePageType`,
`CurrentSourcePageType`, `CacheSize`, `IsNavigationStackEnabled`, live
`BackStack`/`ForwardStack`, and navigation events. Rust operations that can fail
return `Result`. `PageStackEntry` carries the source type and parameter.

The journal is a controlled observable native collection. Public mutation
methods validate ownership and reject changes while navigation is pending.
Notification delivery observes committed state; event handlers can inspect the
content and stacks without borrowing partially updated storage. Page-to-frame
links do not keep frames alive.

The native event sequence is part of the contract: `Frame.Navigating`, the old
page's cancellation callback, then either `NavigationStopped` or a committed
content/history change followed by `Frame.Navigated`, old-page departure and
new-page arrival. Cancellation returns accepted-request success; a reentrant
navigation attempt returns false. A handled construction failure returns false;
an unhandled failure retains the error. These outcomes are not interchangeable.

Caching follows the pinned implementation, including its less obvious details:
default capacity 10, an LRU for Enabled entries, a persistent bucket for
Required entries, zero-capacity admission rules, and explicit invalidation when
a cached page changes to Disabled. Existing `NavigationPage` behavior remains
independent. Navigation transition effects and serialized journals are separate
requirements, not successful empty methods.

### NumberBox

`NumberBox` derives from native `TemplatedControl`. Its `Value` is `f64` with
NaN representing an empty value; its initial bounds are the finite double
extremes. It cannot alias `NumericUpDown`, whose decimal value model must remain
unchanged. The new control has native text, repeat-button and popup parts and
an actual range-value automation peer.

The numeric contract includes small/large steps, minimum/maximum coercion,
wrapping, invalid-input policies, explicit edit commitment, escape restoration,
keyboard and wheel input, header/description presentation, inline/compact/hidden
spinners, and the old/new `ValueChanged` event. Reentrant event changes retain
the value chosen by the handler without issuing an extra outer event. Detached
template parts lose their subscriptions.

Expression evaluation uses the pinned grammar and precedence, including
left-associative exponentiation and NaN for division by zero. It is iterative
and bounded. Numeric tokens use the caller's number parser; formatters implement
both parsing and formatting. Display rounds to ten significant digits with
ties toward positive infinity while preserving the full stored value. The
portable default formatter is invariant. Windows regional number services and
remaining text-input-specific properties still need host adapters.

The rounding policy comes from the pinned `NumberBox.cpp` and the documented
[SignificantDigitsNumberRounder default](https://learn.microsoft.com/en-us/uwp/api/windows.globalization.numberformatting.significantdigitsnumberrounder.roundingalgorithm)
and [rounding algorithms](https://learn.microsoft.com/en-us/uwp/api/windows.globalization.numberformatting.roundingalgorithm).
It does not establish a bit-for-bit comparison against Windows globalization.

The automation peer reports a single coerced value after application handlers.
Its writable pattern follows the pinned native peer, including programmatic
writes when interactive input is disabled. Explicit accessibility names take
precedence over the header fallback. Step-property identifiers also map to the
Windows UIA property identifiers.

### Selection and icons

`RadioButtons` reuses native selection and actual radio containers, with a
column-major layout and directional navigation. `FontIcon` and `SymbolIcon`
render through native text services. The `Symbol` public numeric values and
rendered glyph values have separate, pinned maps; equating them is incorrect.
See [CONTROL-CONTRACTS.md](CONTROL-CONTRACTS.md) for exact supported members,
remaining differences, font requirements and test boundaries.

## XAML API

`migrate_winui_xaml(source, options)` returns a reviewable document, exact
non-overlapping source edits and diagnostics with original byte ranges and
one-based line/column positions. `into_xaml()` rejects unresolved errors. The
optional runtime entry points `parse_winui`, `parse_winui_as` and `load_winui`
use that check before the existing loader validates real type/member metadata.

Options are per document. Applications can provide audited namespace mappings,
exact type aliases, exact member renames, theme policy, known style-resource
keys and an asset URI base. Duplicate/conflicting mappings are errors. A map
does not prove the target implements the source contract.

| Construct | Contract |
|---|---|
| Presentation and `using:` namespaces | Resolve XML scopes and prefix shadowing; migrate only the intended identity. |
| Text, comments, escaped values and handlers | Preserve their contents and original source locations. |
| Ordinary WinUI `Binding` | Make the OneWay default explicit; preserve explicitly selected modes and the existing binding engine. |
| `ThemeResource` | Require explicit dynamic-resource policy and installed ported dictionaries/theme scopes. |
| Named WinUI `Style` resource | Map to `ControlTheme` only for an audited key; arbitrary setter/state semantics require a real projection. |
| `ms-appx:///` | Convert resource-bearing URI attributes only when the host supplies an absolute asset base. |
| Application `x:Class` | Apply explicit namespace identity mappings; require the application's actual Rust type and code-behind. |
| `mc:Ignorable` | Respect namespace scopes; ignored designer trees do not create runtime compatibility claims. |
| Unsupported directives or semantics | Preserve the source requirement and reject loading; never silently remove it. |

Deferred resource construction may cross a framework interface that cannot
return `Result`. Fresh-root WinUI loading has a narrow typed-error bridge for
the framework's own XAML exceptions. It preserves inner errors and resumes
unrelated application panics. It does not promise rollback for an existing root
and requires an unwinding Rust build. Ordinary loader behavior remains unchanged.

## Additional contracts required for complete Gallery portability

The following are implementation obligations, not declarations of completed
features. A supported subset must retain diagnostics for the rest.

| Area | Required native contract and acceptance boundary |
|---|---|
| `x:Bind` | Owner/named-element sources, item-root template scope, OneTime default, explicit OneWay/TwoWay, generated Initialize/Update/StopTracking lifetime and change tracking. Functions, casts, indexers and event bindings need actual compiled expressions. |
| Deferred markup | `x:Load` and deferred strategies must manage construction, name scopes, subscriptions, focus and disposal; visibility alone is insufficient. |
| Visual states | State groups, triggers, setters and transitions must own temporary property values and restore prior bindings/styles without overwriting local state. |
| Template selection and virtualization | DataTemplateSelector, element factories, recycling, ownership, index changes and layout algorithms must use the native item generator/layout services. |
| NavigationView | Hierarchical item containers, selection/invocation distinctions, adaptive left/top modes, overflow, pane lifecycle, settings/footer items, back request, focus and automation. Existing drawer composition alone does not provide this API. |
| AutoSuggestBox | Text-change reasons, suggestion choice, query submission, item/text mapping, popup lifetime, keyboard selection and asynchronous result ownership. Existing filtering alone is insufficient. |
| InfoBar and icon sources | Cancellable close lifecycle and reasons, severity, live title/message/action/content, native accessibility notifications, and live IconSource-to-IconElement projections. |
| ListView, GridView and TabView | Item-click semantics, selection, container lifecycle, drag/drop, tab closing/adding/tear-out, incremental loading and virtualization. Same-shaped basic lists/tabs remain partial adapters. |
| Menus and command bars | Text/icon/accelerator mapping, command execution, radio/toggle state, submenus, flyout ownership, placement, overflow and accessible input. |
| Rich text, ink, charts and specialized controls | Actual merged implementations and their platform/rendering contracts; another unmerged PR is a dependency rather than parity evidence. |
| Composition and animation | Shared compositor primitives, clocks, cancellation, expression inputs, theme transitions and connected-animation ownership, with native/render comparison tests. |
| Desktop/WinRT integration | Pickers, clipboard, storage, launchers, app windows/title bars, notifications, globalization, accessibility settings and WebView adapters through real host services. |

The scanner keeps imported Windows/WinRT/community namespaces distinct from
Microsoft.UI.Xaml. A portable host must report capabilities and fallible
results, preserve cancellation and asynchronous completion semantics, and run
its native integration tests. An in-memory substitute is not evidence for an
OS service.

## Acceptance and evidence

The ledger separates source presence, executable adaptations and behavioral
verification. It retains exact upstream namespaces, signatures, inheritance,
locations, file hashes, Gallery use sites and sample IDs. Overloads cannot
share a single completion flag. Private source contracts are counted separately.

Evidence for a reviewed contract names the implementation, actual tests and a
successful execution record tied to current source hashes. Tests must exercise
values, defaults, ordering, cancellation, ownership and failure behavior as
appropriate. Native construction, interaction, drawing, Windows comparison and
pixel comparison are distinct evidence levels. Font-test draw calls do not prove
that a machine has the required licensed icon font installed.

Every exported Gallery example and every actual `ControlExample` instance must
be accounted for before its page is complete. The exported catalog currently
omits five actual instances, so both inventories participate in the strict gate.
Source availability, navigation to a page and one working example cannot promote
an entire page to verified status.

Run the ordinary offline comparison and tests on every change. Run
`scan.py --check --require-complete` as the separate full acceptance gate. A
fresh report may pass while completeness fails; CI exposes those as distinct
operations. The source scanner is lexical and does not claim WinMD ABI or
Roslyn semantic binding completeness. Those are additional verification layers
required before claiming a complete SDK-compatible projection.
