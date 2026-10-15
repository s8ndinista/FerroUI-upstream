//! Port of `Vnc/Server/FramebufferUpdateRequest.cs` of RemoteViewing
//! (BSD-2-Clause, Copyright (c) 2013 James F. Bellinger; `NOTICE.md` of
//! this crate).

use crate::remote_viewing::vnc::VncRectangle;

/// Stores information about an update request from the client.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FramebufferUpdateRequest {
    incremental: bool,
    region: VncRectangle,
}

impl FramebufferUpdateRequest {
    /// Initializes a new instance of the [`FramebufferUpdateRequest`]
    /// class: `incremental` says whether an incremental update is desired,
    /// `region` is the region to update.
    pub fn new(incremental: bool, region: VncRectangle) -> Self {
        Self {
            incremental,
            region,
        }
    }

    /// Gets a value indicating whether an incremental update is desired.
    pub fn incremental(&self) -> bool {
        self.incremental
    }

    /// Gets the region to update.
    pub fn region(&self) -> VncRectangle {
        self.region
    }
}
