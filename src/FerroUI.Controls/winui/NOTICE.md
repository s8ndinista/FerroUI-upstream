# WinUI compatibility controls

These opt-in controls and their behavior are derived from the Microsoft WinUI sources:
https://github.com/microsoft/microsoft-ui-xaml

Reference revision: `a71c42bd3e352060e5f5ecf3b5e1a98f8a745809`.

The implementations use FerroUI's existing control, property, template, event and collection
systems. They do not replace the existing Avalonia-compatible API. Source correspondence:

| FerroUI file | WinUI reference |
|---|---|
| `page.rs` | `dxaml/xcp/dxaml/lib/Page_Partial.cpp`; `dxaml/xcp/tools/XCPTypesAutoGen/Modules/Controls/Page.cs` |
| `frame.rs` | `dxaml/xcp/dxaml/lib/Frame_Partial.cpp`; `NavigationCache.cpp`; `NavigationHistory.cpp`; `dxaml/xcp/tools/XCPTypesAutoGen/XamlOM/Model/Microsoft.UI.Xaml.Controls.cs` |
| `navigation.rs` | `dxaml/xcp/tools/XCPTypesAutoGen/XamlOM/Model/Microsoft.UI.Xaml.Navigation.cs`; `dxaml/xcp/dxaml/lib/PageStackEntry_Partial.cpp`; `NavigationHistory.cpp` |
| `number_box.rs`, `number_box_expression.rs`, `number_box_automation_peer.rs` | `controls/dev/NumberBox/NumberBox.cpp`, `NumberBoxParser.cpp`, `NumberBox.idl`, `NumberBoxAutomationPeer.cpp` |
| `radio_buttons.rs` | `controls/dev/RadioButtons/RadioButtons.cpp`, `RadioButtons.idl`, `RadioButtons.xaml`, `RadioButtons_themeresources.xaml`, `ColumnMajorUniformToLargestGridLayout.cpp` |
| `font_icon.rs`, `symbol_icon.rs` | `dxaml/xcp/tools/XCPTypesAutoGen/XamlOM/Model/Microsoft.UI.Xaml.Controls.cs`; `dxaml/xcp/core/core/elements/icon.cpp` |
| `auto_suggest_box.rs` | `dxaml/xcp/tools/XCPTypesAutoGen/Modules/AutoSuggestBox/AutoSuggestBox.cs`; `dxaml/xcp/dxaml/lib/AutoSuggestBox_Partial.cpp`, `AutoSuggestBoxTextChangedEventArgs_Partial.h`, `AutoSuggestBoxAutomationPeer_Partial.cpp`; `dxaml/xcp/components/DependencyObject/DependencyProperty.cpp` |
| `info_bar.rs`, `info_bar_panel.rs` | `controls/dev/InfoBar/InfoBar.idl`, `InfoBar.cpp`, `InfoBarPanel.cpp`, `InfoBar.xaml`, `InfoBar_themeresources.xaml` |
| IconSource projections | `dxaml/xcp/dxaml/lib/IconSource_Partial.cpp`, `FontIconSource_Partial.cpp`, `SymbolIconSource_Partial.cpp`, `PathIconSource_Partial.cpp`, `DependencyProperty.cpp`; light/dark resource fallbacks follow the existing WinUI theme and `Common_themeresources_any.xaml` |

## API and behavior boundaries

Navigation transitions, frame journal serialization, Windows application bars and Windows
layout-bounds integration are not implemented and are not exposed as successful placeholders.
Navigation errors are Rust `Result` values. Registered property-system setters retain the
framework's throwing convention; application code can call the fallible setters directly.
The navigation stacks wrap a live framework collection to reject edits during pending
navigation and prevent an entry from being owned by two frames. Page-to-frame links are weak.
Frame construction failures are reported through `NavigationFailed` and a Rust error; a handled
failure returns `false` from `navigate`. Page callbacks are virtual and synchronous.

The cache follows the pinned native implementation: one instance per source type, Required
entries in a permanent cache, Enabled entries in a transient LRU cache, and default capacity 10.
Admission requires nonzero capacity even for Required; already cached Required pages survive
capacity shrinking. Changing a cached page to Disabled removes its type from the cache.
Changing between Enabled and Required does not move an existing entry between cache buckets.
Canceled requests return `true` from Navigate because the native operation accepted the request;
reentrant requests return `false`. Disabling the journal changes content without committing
source-type or history state. These details have focused regression tests.

## Microsoft license

MIT License

Copyright (c) Microsoft Corporation. All rights reserved.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
