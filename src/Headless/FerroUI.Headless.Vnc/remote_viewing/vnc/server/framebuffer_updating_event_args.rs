//! Port of `Vnc/Server/FramebufferUpdatingEventArgs.cs` of RemoteViewing
//! (BSD-2-Clause, Copyright (c) 2013 James F. Bellinger; `NOTICE.md` of
//! this crate).

use std::cell::Cell;

/// Provides data for the framebuffer updating event of the server session.
///
/// The settable properties of the original (`Handled` of its base class
/// among them) are cells: a handler gets the arguments by shared reference.
#[derive(Debug, Default)]
pub struct FramebufferUpdatingEventArgs {
    handled: Cell<bool>,
    sent_changes: Cell<bool>,
}

impl FramebufferUpdatingEventArgs {
    /// Initializes a new instance of the [`FramebufferUpdatingEventArgs`]
    /// class.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets a value indicating whether the event was handled: if a handler
    /// sets it, the session does not find the changes itself.
    pub fn handled(&self) -> bool {
        self.handled.get()
    }

    /// Sets a value indicating whether the event was handled.
    pub fn set_handled(&self, value: bool) {
        self.handled.set(value);
    }

    /// Gets a value indicating whether changes were sent.
    pub fn sent_changes(&self) -> bool {
        self.sent_changes.get()
    }

    /// Sets a value indicating whether changes were sent.
    pub fn set_sent_changes(&self, value: bool) {
        self.sent_changes.set(value);
    }
}
