//! Port of `Vnc/KeyChangedEventArgs.cs` of RemoteViewing (BSD-2-Clause,
//! Copyright (c) 2013 James F. Bellinger; `NOTICE.md` of this crate).

use super::KeySym;

/// Provides data for the key changed event of the server session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyChangedEventArgs {
    keysym: KeySym,
    pressed: bool,
}

impl KeyChangedEventArgs {
    /// Initializes a new instance of the [`KeyChangedEventArgs`] class.
    ///
    /// `keysym` is the X11 keysym of the key (for many keys this is the
    /// ASCII value); `pressed` is true for a key press event, or false for
    /// a key release event.
    pub fn new(keysym: KeySym, pressed: bool) -> Self {
        Self { keysym, pressed }
    }

    /// Gets the X11 keysym of the key. For many keys this is the ASCII
    /// value.
    pub fn keysym(&self) -> KeySym {
        self.keysym
    }

    /// Gets a value indicating whether a key was pressed: true for a key
    /// press event, or false for a key release event.
    pub fn pressed(&self) -> bool {
        self.pressed
    }
}
