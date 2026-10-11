//! The options of the `libinput` backend (the port of
//! `LibInputBackendOptions.cs`).

/// LibInputBackend Options.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LibInputBackendOptions {
    /// List Events of events handler to monitoring eg: /dev/eventX.
    pub events: Option<Vec<String>>,
}
