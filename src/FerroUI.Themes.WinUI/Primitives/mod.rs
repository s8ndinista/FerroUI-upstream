//! The primitives of WinUI that the templates of the theme use and FerroUI's control set has no
//! counterpart of (`Microsoft.UI.Xaml.Controls.Primitives` of microsoft-ui-xaml). Markup names them
//! with `xmlns:primitives="using:FerroUI.Themes.WinUI.Primitives"`.

mod info_bar_panel;

pub use info_bar_panel::InfoBarPanel;
