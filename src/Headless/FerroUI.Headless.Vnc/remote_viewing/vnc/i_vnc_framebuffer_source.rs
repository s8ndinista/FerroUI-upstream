//! Port of `Vnc/IVncFramebufferSource.cs` of RemoteViewing (BSD-2-Clause,
//! Copyright (c) 2013 James F. Bellinger; `NOTICE.md` of this crate).

use super::{ExtendedDesktopSizeStatus, VncFramebuffer};
use std::sync::Arc;

/// Provides a framebuffer.
///
/// A source is called by the threads of the server session, hence `Send`
/// and `Sync`.
pub trait IVncFramebufferSource: Send + Sync {
    /// Gets a value indicating whether the source supports resizing.
    fn supports_resizing(&self) -> bool;

    /// Called when a framebuffer update is needed. You can use this
    /// opportunity to switch framebuffers if desired.
    ///
    /// Returns a framebuffer (`None` stands for the null of the original:
    /// the session keeps the framebuffer it has).
    fn capture(&self) -> Option<Arc<VncFramebuffer>>;

    /// Handles a client request to resize the framebuffer to `width` by
    /// `height`.
    ///
    /// Returns a value indicating how the source handled the update request.
    fn set_desktop_size(&self, width: i32, height: i32) -> ExtendedDesktopSizeStatus;
}
