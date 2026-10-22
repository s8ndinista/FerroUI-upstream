//! ferroui-diagnostics
//!
//! The developer tools of FerroUI, a port of the last open-source version of
//! upstream's developer tools (docs/porting/diagnostics.md): a window that
//! shows the logical and visual trees of a running application, the
//! properties of the selected control with their value sources and
//! priorities, the styles and setters that apply to it, its layout, the
//! routed events as they travel, and takes screenshots. It opens on F12 in
//! a window or an application it is attached to ([`DevToolsExtensions`]).
//!
//! ```ignore
//! window.attach_dev_tools();
//! ```
//!
//! The property grid of the control details is built on the TreeDataGrid of
//! the port, where upstream compiled a copy of DataGrid into its project.

// The compiled markup of a document names the types of this crate by the name of the crate.
extern crate self as ferroui_diagnostics;

mod dev_tools_extensions;
pub mod diagnostics;
mod register_types;
mod rust_paths;

// One module per compiled document and `compiled_markup` (the embedded assets, the loader table
// of the crate and `register()`), which the build script of the crate generates
// (docs/porting/xaml.md, 9.5.13 and 9.6).
ferroui_markup_xaml::include_compiled_xaml!();

pub use dev_tools_extensions::*;
pub use register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod tests;
