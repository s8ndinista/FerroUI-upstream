//! Port of `Vnc/PointerChangedEventArgs.cs` of RemoteViewing (BSD-2-Clause,
//! Copyright (c) 2013 James F. Bellinger; `NOTICE.md` of this crate).

/// Provides data for the pointer changed event of the server session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PointerChangedEventArgs {
    x: i32,
    y: i32,
    pressed_buttons: i32,
}

impl PointerChangedEventArgs {
    /// Initializes a new instance of the [`PointerChangedEventArgs`] class.
    ///
    /// `x` and `y` are the coordinates of the mouse; `pressed_buttons` is a
    /// bit mask of pressed mouse buttons, in X11 convention: 1 is left, 2
    /// is middle, and 4 is right. Mouse wheel scrolling is treated as a
    /// button event: 8 for up and 16 for down.
    pub fn new(x: i32, y: i32, pressed_buttons: i32) -> Self {
        Self {
            x,
            y,
            pressed_buttons,
        }
    }

    /// Gets the X coordinate of the mouse.
    pub fn x(&self) -> i32 {
        self.x
    }

    /// Gets the Y coordinate of the mouse.
    pub fn y(&self) -> i32 {
        self.y
    }

    /// Gets a bit mask of pressed mouse buttons, in X11 convention: 1 is
    /// left, 2 is middle, and 4 is right. Mouse wheel scrolling is treated
    /// as a button event: 8 for up and 16 for down.
    pub fn pressed_buttons(&self) -> i32 {
        self.pressed_buttons
    }
}
