//! The Linux framebuffer platform of FerroUI: an application without a
//! window system, as one full-screen top-level drawn into a framebuffer
//! device (`/dev/fb0`) or onto a KMS output, with input read from the
//! kernel's input devices.
//!
//! An application starts on it with one of the functions of
//! [`LinuxFramebufferPlatformExtensions`] on its builder, after choosing
//! a renderer:
//!
//! ```ignore
//! let code = AppBuilder::configure::<App>().use_skia().start_linux_fb_dev(&args, None, 1.0, None)?;
//! ```
//!
//! Without an input backend the input is read through `libinput` from
//! every event device (touch, absolute pointers, buttons and the wheel,
//! as the reference: no keys and no relative motion), or, with
//! `FERROUI_USE_EVDEV=1`, from the devices the environment names: one
//! variable `FERROUI_EVDEV_DEVICE_{name}` per device,
//! `(path):type=(touchscreen|multitouch|keyboard|mouse)`, with an optional
//! `:calibration=m11,m12,m21,m22,m31,m32` for the touch screens. The keys
//! of a keyboard are those of a US layout.
//!
//! The design, what is built and what was verified where:
//! `docs/porting/linux-framebuffer-platform.md`. Built: the framebuffer
//! device output with software rendering, the DRM output (a KMS
//! connector drawn with OpenGL through EGL on a GBM surface; `libdrm`,
//! `libgbm` and `libEGL` are opened at run time), the evdev input backend
//! and the `libinput` backend (the default; the library is opened at run
//! time).
//!
//! The parts that are functions of values compile on every Unix system
//! (their tests run on a development machine that is not Linux); what
//! makes Linux system calls is compiled for Linux only. The crate is
//! empty elsewhere.

#![cfg(unix)]
#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

mod drm_connector_type;
mod drm_output_options;
mod epoll_dispatcher_impl;
mod framebuffer_toplevel_impl;
pub mod input;
#[cfg(target_os = "linux")]
mod linux_framebuffer_platform;
mod linux_framebuffer_platform_options;
#[cfg(target_os = "linux")]
mod native_library;
pub mod native_unsafe_methods;
pub mod output;
pub mod raw_event_grouping;
#[cfg(target_os = "linux")]
mod stubs;

pub use drm_connector_type::DrmConnectorType;
pub use drm_output_options::DrmOutputOptions;
#[cfg(target_os = "linux")]
pub use epoll_dispatcher_impl::EpollDispatcherImpl;
pub use framebuffer_toplevel_impl::FramebufferToplevelImpl;
#[cfg(target_os = "linux")]
pub use linux_framebuffer_platform::{
    LinuxFramebufferLifetime, LinuxFramebufferPlatform, LinuxFramebufferPlatformExtensions,
};
pub use linux_framebuffer_platform_options::LinuxFramebufferPlatformOptions;
