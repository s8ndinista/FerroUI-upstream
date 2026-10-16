//! winui-theme-gallery
//!
//! The gallery of the WinUI theme (`theme.rs`): the controls of the framework under the theme,
//! one page per family of controls, each control in its states (normal, disabled, checked,
//! selected, indeterminate, with content, with a placeholder, with a validation error, and the
//! pointer-over and pressed states forced through their pseudo-classes). Its purpose is to
//! validate the theme by eye and by pictures: on Windows the comparison host of the crate
//! (`compare/`) shows each page beside the same page built from native WinUI 3 controls
//! (`Pages/*.winui.xml`), and writes both pictures with their difference.
//!
//! The library is the gallery: the application ([`App`], `App.xaml`), its window
//! ([`MainWindow`]: a toolbar with the theme variant, the density, the flow direction and the
//! accent colour, the list of the pages, the page), the host of a page ([`CaptureHost`]: the page
//! in a theme variant scope on the background of the theme, the same for the window and for the
//! pictures), the pages (`Pages/`) and the capture mode ([`capture`]). The documents are compiled
//! by the build of the crate (`build.rs`, docs/porting/xaml.md 9.5.22). It is built and tested on
//! every host; the entry points are `program.rs` (the desktop) and `compare/main.rs` (Windows).
//!
//! docs/porting/winui-theme-gallery.md describes the gallery, its modes and the report.

// The compiled markup of a document names the types of this crate by the name of the crate.
extern crate self as winui_theme_gallery;

mod app;
pub mod capture;
mod capture_host;
pub mod comparison;
mod main_window;
pub mod markup;
mod register_types;
pub mod theme;

#[path = "Pages/mod.rs"]
pub mod pages;

// One module per compiled document and `compiled_markup` (the loader table of the compiled
// markup of the crate and `register()`), which the build script of the crate generates.
ferroui_markup_xaml::include_compiled_xaml!();

pub use app::App;
pub use capture_host::CaptureHost;
pub use main_window::MainWindow;
pub use markup::SAMPLE;
pub use pages::{PageInfo, PAGES};
pub use register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod tests;
