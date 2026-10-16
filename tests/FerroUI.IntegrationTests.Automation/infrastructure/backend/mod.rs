//! The automation backend of the platform: one type `Backend` with the same
//! methods on every platform, over UI Automation on Windows and the
//! accessibility API on macOS (docs/porting/integration-tests.md, section 2).

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::{Backend, NativeElement};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::{Backend, NativeElement};

#[cfg(not(any(windows, target_os = "macos")))]
mod unsupported;
#[cfg(not(any(windows, target_os = "macos")))]
pub use unsupported::{Backend, NativeElement};
