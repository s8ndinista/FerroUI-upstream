//! The RFB (VNC) server the original is built on: a port of the parts of
//! RemoteViewing (<https://github.com/quamotion/remoteviewing>, version
//! 1.1.211, BSD-2-Clause; `NOTICE.md` of this crate) that the original
//! uses.
//!
//! RemoteViewing is a .NET library with a VNC client and a VNC server. The
//! original creates a server session per connection
//! ([`vnc::server::VncServerSession`]), gives it a framebuffer source
//! ([`vnc::IVncFramebufferSource`]) and handles its key and pointer events;
//! everything the session needs for that is ported, file by file, with the
//! names of the library in Rust form. Not ported: the client, the server
//! that listens by itself (`VncServer`), the zlib encoder the session does
//! not use (`ZlibEncoder`), the hosting extensions and the VMware movie
//! writer. Where the port differs from the library, the file says so; the
//! list is in `docs/porting/headless-vnc.md`.
//!
//! [`logging`] stands for the part of `Microsoft.Extensions.Logging` the
//! library logs through.

pub mod logging;
pub mod utility;
pub mod vnc;
