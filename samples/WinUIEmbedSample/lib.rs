//! winui-embed-sample
//!
//! Port of the sample application `WinUIEmbedSample`: controls of WinUI 3 beside content of the
//! framework in one window. The window of WinUI has two columns: the left one holds controls of
//! WinUI (a button, a slider, a text box, a toggle switch, a drag source, the theme and the
//! diagnostics of the automation peer of the panel), the right one the swap chain panel of the
//! WinUI integration (`ferroui-winui`), whose content is [`EmbeddedView`]: the same kinds of
//! controls of the framework, with the clipboard, drag and drop, cursors, hover and ink.
//!
//! This library is the content: `FerroApp.xaml` (the application of the framework, with the
//! Fluent theme) and `EmbeddedView.xaml` with their classes, converted from the upstream
//! documents by `scripts/convert_catalog_xaml.py` (names only) and compiled by the build of the
//! crate (`build.rs`, docs/porting/xaml.md 9.5.22). It is built and tested on every host. The
//! WinUI application (`App`, `MainWindow` and the generated `Main` of the upstream sample) is the
//! executable of the crate (`main.rs`), which runs on Windows only (docs/porting/winui.md,
//! section 7.6).

// The compiled markup of a document names the types of this crate by the name of the crate.
extern crate self as winui_embed_sample;

mod embedded_view;
mod ferro_app;
pub mod markup;
mod register_types;

// One module per compiled document and `compiled_markup` (the loader table of the compiled
// markup of the crate and `register()`), which the build script of the crate generates.
ferroui_markup_xaml::include_compiled_xaml!();

pub use embedded_view::EmbeddedView;
pub use ferro_app::FerroApp;
pub use markup::SAMPLE;
pub use register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod tests;
