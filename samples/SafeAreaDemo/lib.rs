//! safe-area-demo
//!
//! Port of the sample application `SafeAreaDemo`: one view that shows the safe area of its
//! top-level (the padding the insets manager of the platform reports), the area the input pane
//! (the software keyboard) leaves free, and the options of the insets manager: whether the
//! window is displayed edge to edge, whether the system bars are hidden, and whether the
//! top-level pads its content by the safe area itself. Three buttons switch the theme variant
//! of the application.
//!
//! This crate is the shared content (`SafeAreaDemo.csproj`); the hosts are separate crates:
//! `samples/SafeAreaDemo.Desktop` (a window), `samples/SafeAreaDemo.Android` (an activity) and
//! `samples/SafeAreaDemo.iOS` (the view of the application delegate). On a desktop the insets
//! manager and the input pane of a window are absent, and the view shows its bounds alone.
//!
//! The sample is markup: one document per class, which populates the class the document
//! names. The documents are converted from the upstream sample by
//! `scripts/convert_catalog_xaml.py` (names only) and compiled by the build of the crate
//! (`build.rs`, docs/porting/xaml.md 9.5.22): a class is populated by the compiled markup of
//! its document.
//!
//! The directories and files mirror the upstream sample: `ViewModels/MainViewModel.cs` is
//! `ViewModels/main_view_model.rs`, `Views/MainView.xaml.cs` is `Views/main_view.rs`. What the
//! framework lacks for the sample is listed in `GAPS.md`.

// The compiled markup of a document names the types of this crate by the name of the crate.
extern crate self as safe_area_demo;

mod app;
pub mod markup;
mod register_types;
// The smoke run of the hosts (not a port).
pub mod smoke;
mod view_locator;

#[path = "ViewModels/mod.rs"]
pub mod view_models;
#[path = "Views/mod.rs"]
pub mod views;

// One module per compiled document and `compiled_markup` (the loader table of the compiled
// markup of the crate and `register()`), which the build script of the crate generates.
ferroui_markup_xaml::include_compiled_xaml!();

pub use app::App;
pub use markup::SAMPLE;
pub use register_types::{register_types, ASSEMBLY};
pub use view_locator::ViewLocator;

#[cfg(test)]
mod tests;
