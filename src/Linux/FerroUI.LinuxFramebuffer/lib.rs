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
//! Without an input backend the devices are those the environment names
//! when `FERROUI_USE_EVDEV=1`: one variable `FERROUI_EVDEV_DEVICE_{name}`
//! per device, `(path):type=(touchscreen|multitouch|keyboard|mouse)`,
//! with an optional `:calibration=m11,m12,m21,m22,m31,m32` for the touch
//! screens. The keys of a keyboard are those of a US layout.
//!
//! The design, what is built and what was verified where:
//! `docs/porting/linux-framebuffer-platform.md`. Built so far: the
//! framebuffer device output with software rendering and the evdev input
//! backend. The DRM output and the `libinput` backend are stage 2; their
//! entry points fail with a message that says so.
//!
//! The parts that are functions of values compile on every Unix system
//! (their tests run on a development machine that is not Linux); what
//! makes Linux system calls is compiled for Linux only. The crate is
//! empty elsewhere.

#![cfg(unix)]
#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

mod epoll_dispatcher_impl;
mod framebuffer_toplevel_impl;
pub mod input;
#[cfg(target_os = "linux")]
mod linux_framebuffer_platform;
mod linux_framebuffer_platform_options;
pub mod native_unsafe_methods;
pub mod output;
pub mod raw_event_grouping;
#[cfg(target_os = "linux")]
mod stubs;

#[cfg(target_os = "linux")]
pub use epoll_dispatcher_impl::EpollDispatcherImpl;
pub use framebuffer_toplevel_impl::FramebufferToplevelImpl;
#[cfg(target_os = "linux")]
pub use linux_framebuffer_platform::{
    LinuxFramebufferLifetime, LinuxFramebufferPlatform, LinuxFramebufferPlatformExtensions,
};
pub use linux_framebuffer_platform_options::LinuxFramebufferPlatformOptions;
