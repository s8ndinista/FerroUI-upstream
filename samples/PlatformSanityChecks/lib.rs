//! platform-sanity-checks
//!
//! Port of the sample application `PlatformSanityChecks`: a program without a window that
//! initializes the X11 platform and checks its threading: a signal posted from another thread
//! runs on the UI thread, two timers fire in the order of their intervals, and a timer ticks at
//! its rate. The entry point is `program.rs`; it runs on Linux, as the reference does.
//!
//! The application class (`App.xaml.cs`) and its document (`App.xaml`, converted from the
//! upstream sample by `scripts/convert_catalog_xaml.py`, names only) are part of the sample,
//! though the program creates the application without initializing it.

// The compiled markup of a document names the types of this crate by the name of the crate.
extern crate self as platform_sanity_checks;

mod app;
pub mod markup;
mod register_types;

// One module per compiled document and `compiled_markup` (the loader table of the compiled
// markup of the crate and `register()`), which the build script of the crate generates.
ferroui_markup_xaml::include_compiled_xaml!();

pub use app::App;
pub use markup::SAMPLE;
pub use register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod tests;
