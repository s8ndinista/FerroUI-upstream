//! What the worker needs of the system (the directory `Server/Interop` of
//! the reference): the connection, the wake-up descriptor, the calls of
//! the C library, and the declarations of `libgbm` and `libdrm` the dmabuf
//! swapchain calls.
//!
//! `WaylandEglNativeMethods.cs` and `XkbCommonNativeMethods.cs` are not
//! ported: their declarations are those of the crates `wayland-egl` and
//! `xkbcommon-dl`.

pub mod drm_gbm_unsafe_native_methods;
pub mod pipe2_stream;
pub mod unsafe_native_methods;
pub mod wakeup_fd;
pub mod wayland_connection;
