//! The protocol version the session negotiates: what `System.Version`
//! holds for RemoteViewing (a major and a minor number). Not a file of
//! RemoteViewing.

use std::fmt;

/// The version of the RFB protocol.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Version {
    /// The major version number.
    pub major: i32,
    /// The minor version number.
    pub minor: i32,
}

impl Version {
    /// A version of the major and minor numbers.
    pub const fn new(major: i32, minor: i32) -> Self {
        Self { major, minor }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}
