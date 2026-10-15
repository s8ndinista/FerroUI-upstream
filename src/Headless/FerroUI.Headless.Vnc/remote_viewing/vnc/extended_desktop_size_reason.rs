//! Port of `Vnc/ExtendedDesktopSizeReason.cs` of RemoteViewing
//! (BSD-2-Clause, Copyright (c) 2013 James F. Bellinger; `NOTICE.md` of
//! this crate).

/// Indicates the reason for a change in screen layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ExtendedDesktopSizeReason {
    /// The screen layout was changed via non-RFB means on the server. For
    /// example the server may have provided means for server-side
    /// applications to manipulate the screen layout. This code is also used
    /// when the client sends a non-incremental FrameBufferUpdateRequest to
    /// learn the server's current state.
    External = 0,

    /// The client receiving this message requested a change of the screen
    /// layout. The change may or may not have happened depending on server
    /// policy or available resources. The status code in the y-position
    /// field must be used to determine which.
    Client = 1,

    /// Another client requested a change of the screen layout and the
    /// server approved it. A rectangle with this code is never sent if the
    /// server denied the request.
    OtherClient = 2,
}
