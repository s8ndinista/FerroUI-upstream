//! The VNC server of the headless platform.
//!
//! An application runs on the headless platform (`ferroui-headless`),
//! renders its windows with a rendering backend (Skia by default), and a
//! VNC (RFB) server shows its main window to any VNC viewer and passes the
//! keyboard and the mouse of the viewer to it:
//!
//! ```no_run
//! use ferroui_controls::{AppBuilder, Application, ShutdownMode};
//! use ferroui_headless_vnc::HeadlessVncPlatformExtensions;
//!
//! fn main() {
//!     let args: Vec<String> = std::env::args().skip(1).collect();
//!     // The builder of the application, with its rendering backend (for
//!     // example `use_skia` of `ferroui-skia`).
//!     let builder = AppBuilder::configure::<Application>();
//!     let exit_code = builder.start_with_headless_vnc_platform(None, 5901, &args, ShutdownMode::OnMainWindowClose);
//!     std::process::exit(exit_code);
//! }
//! ```
//!
//! [`HeadlessVncPlatformExtensions`] starts the application with the
//! headless platform, binds the server (on the loopback address unless a
//! host is given, with an optional password) and runs the lifetime of the
//! application. Each client gets a session ([`remote_viewing::vnc::server::VncServerSession`])
//! whose framebuffer is the last frame of the main window
//! ([`HeadlessVncFramebufferSource`]).
//!
//! The RFB server is [`remote_viewing`], a port of the parts of the .NET
//! library RemoteViewing that the original builds on (`NOTICE.md` of this
//! crate; `docs/porting/headless-vnc.md` of the repository says why it was
//! ported and what differs).

mod ferro_vnc_logger;
mod headless_vnc_framebuffer_source;
mod headless_vnc_platform_extensions;
pub mod remote_viewing;

pub use headless_vnc_framebuffer_source::HeadlessVncFramebufferSource;
pub use headless_vnc_platform_extensions::HeadlessVncPlatformExtensions;
