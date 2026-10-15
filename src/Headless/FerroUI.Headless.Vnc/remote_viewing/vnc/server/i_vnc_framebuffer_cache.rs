//! Port of `Vnc/Server/IVncFramebufferCache.cs` of RemoteViewing
//! (BSD-2-Clause, Copyright (c) 2013 James F. Bellinger; `NOTICE.md` of
//! this crate).

use super::IVncServerSession;
use crate::remote_viewing::vnc::VncFramebuffer;
use std::sync::Arc;

/// Caches the framebuffer pixel data and updates them as new framebuffer
/// commands are received.
pub trait IVncFramebufferCache: Send + Sync {
    /// Gets an up-to-date and complete framebuffer.
    fn framebuffer(&self) -> Arc<VncFramebuffer>;

    /// Responds to an update request of `session`.
    ///
    /// Returns true if the operation completed successfully; otherwise,
    /// false.
    fn respond_to_update_request(&self, session: &dyn IVncServerSession) -> bool;
}
