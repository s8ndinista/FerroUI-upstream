//! single-project-sandbox
//!
//! Port of the sample application `SingleProjectSandbox`: one project with the application and
//! the entry point of each platform it targets. The upstream project is a .NET project with
//! several target frameworks (desktop, Android, browser, and iOS and macOS when the build asks
//! for them), each of which compiles the application and the files of its platform
//! (`Platforms/<platform>/`). The counterpart in Rust is one package: the application is its
//! library, the entry points are its binaries and its shared library, and Cargo builds the one
//! the target asks for:
//!
//! - `Platforms/Desktop/program.rs`: the binary `single-project-sandbox` (Windows, Linux, macOS);
//! - `Platforms/macOS/program.rs`: the binary `single-project-sandbox-macos` (the native platform
//!   of macOS with Skia);
//! - `Platforms/Android/application.rs`: the shared library of the package, built for an Android
//!   target (`scripts/android/apk.sh sandbox`);
//! - `Platforms/iOS/app_delegate.rs`: the binary `single-project-sandbox-ios`, built for an iOS
//!   target (`scripts/ios/bundle.sh`).
//!
//! The application is markup: `App.xaml`, `MainView.xaml` and `MainWindow.xaml`, converted from
//! the upstream documents by `scripts/convert_catalog_xaml.py` (names only) and compiled by the
//! build of the crate (`build.rs`, docs/porting/xaml.md 9.5.22). What the port does differently
//! is listed in `GAPS.md`.

// The compiled markup of a document names the types of this crate by the name of the crate.
extern crate self as single_project_sandbox;

mod app;
mod main_view;
mod main_window;
pub mod markup;
mod register_types;

// The Android entry point: the shared library of the package.
#[cfg(target_os = "android")]
#[path = "Platforms/Android/application.rs"]
mod android_application;

// One module per compiled document and `compiled_markup` (the loader table of the compiled
// markup of the crate and `register()`), which the build script of the crate generates.
ferroui_markup_xaml::include_compiled_xaml!();

pub use app::App;
pub use main_view::MainView;
pub use main_window::MainWindow;
pub use markup::SAMPLE;
pub use register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod tests;
