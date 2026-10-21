# Opt-in control contracts

The controls in this document use `FerroUI.Controls.WinUI` in markup and
`ferroui_controls::winui` in Rust. They compose the framework's existing property,
layout, text, selection, input, automation and template systems. They do not
change the type identities or defaults of the existing controls. A declaration
or successful construction is not a claim that every WinUI contract is complete.

The reference commit is
[`a71c42bd3e352060e5f5ecf3b5e1a98f8a745809`](https://github.com/microsoft/microsoft-ui-xaml/tree/a71c42bd3e352060e5f5ecf3b5e1a98f8a745809).
The implementation is based on its API model, native implementation and the
actual Gallery usages recorded in `data/gallery-usage.json`.

## RadioButtons

`RadioButtons` derives from the native `HeaderedSelectingItemsControl` and uses
existing `RadioButton` containers. It supports inline controls, plain values and
`ItemsSource`; plain values are wrapped in a radio button while supplied radio
buttons remain their own containers. Pointer activation, keyboard activation,
mutual exclusion, effective enabled state and radio accessibility use native
services. Selection synchronizes `SelectedIndex`, `SelectedItem` and the actual
checked state of each realized container.

| WinUI member | FerroUI member | Implemented behavior and boundary |
|---|---|---|
| `Items` | inherited `items()` | Native observable item collection and inline XAML content |
| `ItemsSource` | inherited `items_source()` | Native typed item-source adapter; native collection rules apply |
| `ItemTemplate` | inherited `item_template()` | Native `IDataTemplate` displayed in radio content |
| `SelectedIndex`, `SelectedItem` | inherited properties | Native single selection, selected-item identity and collection updates |
| `SelectionChanged` | inherited routed event | Actual removed/added items; duplicate assignments do not emit changes |
| `ContainerFromIndex` | inherited method | Actual generated or supplied radio container |
| `Header`, `HeaderTemplate` | inherited properties | Live native content presenter and data template |
| `MaxColumns` | `max_columns()` | Validated positive integer; column-major layout constrained by available width |
| `RadioButtonsAutomationPeer` | native selection peer | Group role, class name, selected peers, position/size of each radio set |

The default panel measures the largest item, fits the requested columns into
the available width, and fills columns from top to bottom. Earlier columns get
one extra item when the item count does not divide evenly. Pinned spacing is
7 units between columns and 8 between rows; the header's bottom margin is 8.
The template and item panel remain replaceable native templates.

Down/Up moves through items in collection order, skips unavailable containers
and stops at the ends. Left/Right moves to the adjacent visual column. Keyboard
selection follows focus unless Control is held, and entry by keyboard restores
the selected container. The group consumes directional keys at its boundaries
instead of allowing the underlying radio group to wrap. Native button behavior
handles activation.

### RadioButtons boundaries

- Native `ItemsSource` requires an empty inline `Items` collection before source
  mode is enabled. WinUI keeps its separate inline collection while a source is
  active. The source object and collection API are native Rust adapters.
- Native selection normalizes invalid indices and exposes additional inherited
  selection APIs. Those additional APIs are not WinUI contracts and do not
  establish WinUI multi-selection support.
- `ItemTemplate` accepts `IDataTemplate`, not a WinUI `IElementFactory` or
  `DataTemplateSelector`. A template produces the content of the generated radio
  button. A template that itself creates a radio button requires review.
- WinUI gamepad XY-focus escape, AccessKeyInvoked-specific focus redirection,
  exact Windows UIA event sequences and WinUI theme resources are not certified.
- The supplied native theme controls the radio appearance. The native default
  panel is exercised; custom panels may need their own navigation policy.

The integration target `winui_radio_buttons` exercises container reuse, actual
activation and checked state, selection-event ordering, source replacement,
headers/templates, dynamic collection edits, constrained column layout,
keyboard navigation and automation.

## FontIcon

`FontIcon` derives from the existing `IconElement`. Its default template is a
native `TextBlock` using the framework's text shaping, layout and drawing
services. `Glyph` is a string and supports more than one glyph. The native
typography and foreground properties update the existing presenter, including
changes after the first layout pass. Glyph and font changes invalidate the
icon's measurement as well as the text presenter, including when the icon is
measured as a detached root without a window layout manager.

| WinUI member | FerroUI member | Implemented behavior and boundary |
|---|---|---|
| `Glyph` | `glyph()` / `set_glyph()` | Live string rendered by native text services |
| `FontFamily` | inherited typography property | Default `Segoe Fluent Icons`; applications can supply another native family |
| `FontSize` | inherited typography property | Default 20 units; native text metrics determine measured size |
| `FontStyle`, `FontWeight` | inherited typography properties | Native text formatting and font matching |
| `Foreground` | inherited property | Live inherited/local native brush |
| `MirroredWhenRightToLeft` | same property | Opt-in glyph reflection around its center; public control transform is preserved |
| `IsTextScaleFactorEnabled` | **unsupported** | No OS text-scale service is currently connected; there is no stored no-op property |

Typography inheritance follows the existing native property system. WinUI's
specific inherited-text formatting rules and Windows text-scale integration are
separate contracts; native layout tests do not certify those rules.

The text presenter is excluded from the accessibility content view, so its
private-use font characters are not announced as document text.

## SymbolIcon and Symbol

`SymbolIcon` draws one predefined symbol through the same native text services.
Its default is `Symbol::Emoji`. `SymbolIcon::with_symbol(...)` implements the
constructor taking a symbol. Changing `Symbol` or `Foreground` updates its
existing presenter. Pinned symbol rendering uses Segoe Fluent Icons at 20 units,
normal style and normal weight.

**The Symbol API value is often different from the glyph codepoint.** For
example, `Symbol::Play` retains the public numeric value `0xE102`, while the
native implementation renders `U+E768`. Mapping enum numbers directly to
characters would draw the wrong font entries. All **197** public values and
their modern glyph conversions are included, together with all 197 XAML enum
names. The source table is `data/symbol-glyphs.tsv`.

`scripts/winui-gallery/symbol_map.py --check` verifies the complete enum,
rendering map, markup names and iteration table against a pinned reference.
`data/symbol-sources.lock.json` records SHA-256 values for both the upstream API
model and native `icon.cpp`, as well as the reference-table hash. Refreshing the
reference requires an explicit clean checkout at the main source lock's commit:

```sh
python3 scripts/winui-gallery/symbol_map.py --refresh-source \
    --winui-root ../upstream-winui
```

The `winui_icons` integration target checks every numeric value and glyph, then
passes every symbol through native text layout and records the resulting glyph
draw calls. It also checks live typography/foreground changes, RTL mirroring,
constructor defaults and preservation of existing text defaults. Its fonts are
deterministic test fonts. This proves the native data/rendering path and glyph
selection; it does **not** prove installed-font availability or visual equality
to Windows rendering.

No Microsoft font binary is redistributed. Hosts must provide an appropriately
licensed font with the expected glyphs. `FontIcon` accepts an application-supplied
font family. `SymbolIcon` deliberately uses the pinned family's mapping; a font
with a different private-use encoding cannot be substituted just by using the
same numeric API values.

## Reusable icon sources

`IconSource` is an abstract native object with a `Foreground` property and
`create_icon_element()`. `FontIconSource`, `SymbolIconSource` and `PathIconSource`
create the corresponding real native icons. Reusing one source creates separate
visual elements, so two controls never attempt to share the same visual parent.

| Source | Native element | Live source properties |
|---|---|---|
| `FontIconSource` | opt-in `FontIcon` | Glyph, FontFamily, FontSize, FontStyle, FontWeight, MirroredWhenRightToLeft, Foreground |
| `SymbolIconSource` | opt-in `SymbolIcon` | Symbol, Foreground |
| `PathIconSource` | existing `PathIcon` | Data, Foreground |

The source keeps weak references to the elements it creates. It neither owns
those elements nor subscribes to their changes. Removing an element releases
it; dropping a source leaves surviving elements at their last copied values.
Changing a source property updates only that mapped property, preserving local
values on unrelated element properties. Expired weak references are pruned.
If an element's change handler changes the source again during propagation,
later elements receive the current effective source value. A regression test
covers both same-property redirection and a nested change to a different property;
an older notification cannot overwrite the newer source value on another icon.

An initially unset source foreground leaves a new element's normal inherited
foreground intact. An explicit source foreground propagates to every surviving
element. Changing it back to null writes an explicit null to existing elements;
subsequently created elements again receive no foreground override. This follows
the pinned source's distinction between initial copying and later changes.

Font sources retain the FontIcon limitations above. Bitmap and animated icon
sources, `IsTextScaleFactorEnabled` and Windows font services are separate gaps.
Path sources preserve native geometry identity and rely on the existing native
PathIcon theme and drawing implementation.

## InfoBar

`InfoBar` uses native content, template, property, input, command and resource
services. It derives from `ContentControl` to preserve native content ownership
and data templates. The existing notification controls and theme-specific
InfoBar panel keep their original identities and behavior.

| WinUI API | Native implementation and boundary |
|---|---|
| `IsOpen` | Default false; collapses the template root while preserving the outer control's `IsVisible` |
| `Closing`, `Closed`, `CloseButtonClick` | Real ordered callbacks, typed reasons and cancellable closing |
| `Opened` | Implemented as the pinned **preview** contract, including reopening after cancellation |
| `Title`, `Message`, `Severity` | Live text, severity glyphs, native resources and severity pseudo classes |
| `IsClosable` | Controls close-button visibility; programmatic closing remains allowed |
| `CloseButtonCommand`, parameter | Native button executes the command and observes `CanExecuteChanged` |
| `CloseButtonStyle` | **Explicit adaptation:** a native `ControlTheme`; arbitrary WinUI Style setters are not accepted as equivalent |
| `ActionButton` | Actual native button presented with its existing command/input behavior |
| `Content`, `ContentTemplate` | Native content presenter, ownership and data templates |
| `IconSource`, `IsIconVisible` | Reusable source creates a live icon; source replacement and visibility update native parts |
| `TemplateSettings.IconElement` | Actual current native icon, exposed through a stable template-settings object |
| `InfoBarPanel` | Pinned width/height-sensitive layout with styled padding and attached margins |

On close-button activation, `CloseButtonClick` runs first. The control records
`CloseButton` as the reason and requests `IsOpen = false`. `Closing` can cancel
the operation; cancellation restores `IsOpen = true`, preserves the visible
content and raises the preview `Opened` event. Otherwise the root collapses and
`Closed` runs. Reopening resets the reason to `Programmatic`. Event handlers use
snapshots without held internal borrows, so cancellation, event disposal and
reopening from a handler are supported. Reapplying a template detaches the
previous close button's callback.

The default template contains real native borders, a grid, TextBlocks,
ContentPresenters, a Viewbox, a close button and the responsive InfoBarPanel.
The standard icon uses the pinned `F136` background plus `F13F` informational,
`F13E` success, `F13C` warning or `F13D` error glyph. The close glyph is the exact
`Symbol::Cancel` conversion. The same icon-font availability limitation applies
as for SymbolIcon.

The existing WinUI theme already contains the InfoBar palette and
`InfoBarCloseButtonStyle` as native resources. The control observes native
resource/theme changes and reads the severity brushes, text brushes, icon font,
font sizes, panel padding/margins and close-button theme. Light/dark fallback
severity colors come from the pinned palette. A user background is painted
over the severity background, preserving translucent composition. Explicit
foreground values take precedence over the title/message resource brushes.
Missing localized close-button/icon labels use English fallbacks; hosts can
provide native string resources.

`InfoBarAutomationPeer` reports a native StatusBar role, class name, and a
title/message fallback name, while respecting an explicitly supplied automation
name. Name changes raise native automation property notifications. Severity
sets native polite/assertive live-region metadata. **The Windows
`RaiseNotificationEvent` announcement channel is not implemented.** These
properties do not claim platform-specific announcement or Windows UIA sequence
parity.

Styling remains a reviewed native adaptation. WinUI VisualStateManager execution,
arbitrary WinUI Style setters, Windows localization services, pixel equivalence,
and every inherited WinUI member are outside this implementation's verified
scope. The strict XAML migration layer retains diagnostics for unsupported
syntax. Native templates remain replaceable; replacement templates must supply
the documented parts or bind their own native properties.

## Evidence and completion

`data/api-mappings.json` records explicit namespace/member mappings and their
limits. `data/api-evidence.json` records evidence for exact source signatures;
`data/api-map.json` joins this evidence to declaration candidates. Verified
evidence must point at an actual successful execution record tied to the
implementation/test file hashes. Partial evidence keeps its boundaries and
does not turn a whole type or Gallery page into verified parity.

The first recorded native-control execution is
[`data/executions/native-controls-001.json`](data/executions/native-controls-001.json):
**47 integration tests passed** (`winui_icons`: 7, `winui_radio_buttons`: 9,
`winui_navigation`: 17, `winui_number_box`: 14). The record contains the command,
every passing test name and SHA-256 values for the exact implementation and
test inputs. Native text drawing uses deterministic test fonts; native input,
layout and automation tests use the framework's test services.

The next recorded execution is
[`data/executions/native-controls-002.json`](data/executions/native-controls-002.json):
**30 integration tests passed** (`winui_auto_suggest_box`: 13,
`winui_icon_sources`: 7, `winui_info_bar`: 10). The two records therefore cover
**77 passing native control integration tests**. The second record covers native
suggestion text coalescing and event identity, icon-source ownership and reentrant
updates, and InfoBar lifecycle, commands, resources, layout and automation. Its
hash scope lists the actual implementation/test files; a later shared registration
helper relocation is outside that execution record and is validated separately.

The new controls and their 87 recorded member subsets remain **partial**.
For example, AutoSuggestBox's `QueryIcon` entry records its tested initial null
value without claiming that this test run verified arbitrary live icon content.
Unsupported members have no successful evidence entry, and broader inherited,
host-service and visual obligations remain visible in the comparison.

This evidence verifies the complete `Symbol` enum contract and selected narrow
icon member contracts. The control types, native collection/template adapters,
navigation and number formatting remain marked **partial**. Successful tests
are evidence for their stated scope, not a certification of all inherited
members, platform integrations, visuals or Gallery examples.
