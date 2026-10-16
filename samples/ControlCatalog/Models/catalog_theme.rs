//! Port of `Models/CatalogTheme.cs`, with the WinUI theme of the port as a third theme
//! (`WinUI`, not in the original; docs/porting/winui-theme.md).

use ferroui_base::ferro_markup_enum;

/// The theme of the catalog.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CatalogTheme {
    Fluent = 0,
    Simple = 1,
    /// The WinUI theme (`ferroui-themes-winui`), an addition of the port.
    WinUI = 2,
}

ferro_markup_enum!(CatalogTheme { Fluent, Simple, WinUI });
