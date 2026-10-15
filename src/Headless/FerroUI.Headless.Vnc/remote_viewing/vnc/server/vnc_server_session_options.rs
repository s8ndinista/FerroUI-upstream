//! Port of `Vnc/Server/VncServerSessionOptions.cs` of RemoteViewing
//! (BSD-2-Clause, Copyright (c) 2013 James F. Bellinger; `NOTICE.md` of
//! this crate).

use crate::remote_viewing::vnc::AuthenticationMethod;

/// Specifies options for interacting with a VNC client.
///
/// The settable property of the original is a field.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VncServerSessionOptions {
    /// The desired authentication method.
    pub authentication_method: AuthenticationMethod,
}
