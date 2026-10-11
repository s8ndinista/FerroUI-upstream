//! The input backend over the `libinput` library.

mod lib_input_backend;
mod lib_input_backend_options;
mod lib_input_backend_pointer;
mod lib_input_backend_touch;
pub mod lib_input_native_unsafe_methods;

#[cfg(target_os = "linux")]
pub use lib_input_backend::LibInputBackend;
pub use lib_input_backend::{LibInputEvent, LibInputState};
pub use lib_input_backend_options::LibInputBackendOptions;
