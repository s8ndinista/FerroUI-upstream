//! Port of `Vnc/Server/CreatingDesktopEventArgs.cs` of RemoteViewing
//! (BSD-2-Clause, Copyright (c) 2013 James F. Bellinger; `NOTICE.md` of
//! this crate).

/// Provides data for the creating desktop event of the server session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CreatingDesktopEventArgs {
    share_desktop: bool,
}

impl CreatingDesktopEventArgs {
    /// Initializes a new instance of the [`CreatingDesktopEventArgs`] class:
    /// `share_desktop` is true if the client is willing to share the
    /// desktop with other clients.
    pub fn new(share_desktop: bool) -> Self {
        Self { share_desktop }
    }

    /// Gets a value indicating whether the client is willing to share the
    /// desktop with other clients (shared access) or wants exclusive access.
    pub fn share_desktop(&self) -> bool {
        self.share_desktop
    }
}
