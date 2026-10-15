//! Port of `Vnc/AuthenticationMethod.cs` of RemoteViewing (BSD-2-Clause,
//! Copyright (c) 2013 James F. Bellinger; `NOTICE.md` of this crate).

/// Supported authentication methods.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum AuthenticationMethod {
    /// No authentication is performed.
    #[default]
    None = 1,

    /// A password is used.
    Password = 2,
}

impl AuthenticationMethod {
    /// The method of a security type number, if it is one of these.
    pub fn from_value(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::None),
            2 => Some(Self::Password),
            _ => None,
        }
    }
}

impl std::fmt::Display for AuthenticationMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::None => "None",
            Self::Password => "Password",
        })
    }
}
