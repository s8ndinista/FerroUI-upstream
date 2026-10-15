//! Port of `Vnc/ExtendedDesktopSizeStatus.cs` of RemoteViewing
//! (BSD-2-Clause, Copyright (c) 2013 James F. Bellinger; `NOTICE.md` of
//! this crate).

/// Indicates the status code for a change requested by a client.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ExtendedDesktopSizeStatus {
    /// No error.
    Success = 0,

    /// Resize is administratively prohibited.
    Prohibited = 1,

    /// Out of resources.
    OutOfResources = 2,

    /// Invalid screen layout.
    InvalidScreenLayout = 3,
}
