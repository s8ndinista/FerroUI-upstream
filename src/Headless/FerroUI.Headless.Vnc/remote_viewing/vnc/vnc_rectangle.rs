//! Port of `Vnc/VncRectangle.cs` of RemoteViewing (BSD-2-Clause, Copyright
//! (c) 2013 James F. Bellinger; `NOTICE.md` of this crate).

use std::fmt;

/// Bounds the changed pixels of a framebuffer.
///
/// The fields are the settable properties of the original.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VncRectangle {
    /// The X coordinate of the leftmost changed pixel.
    pub x: i32,
    /// The Y coordinate of the topmost changed pixel.
    pub y: i32,
    /// The width of the changed region.
    pub width: i32,
    /// The height of the changed region.
    pub height: i32,
}

impl VncRectangle {
    /// Initializes a new instance of the [`VncRectangle`] structure.
    ///
    /// # Panics
    /// When a value is negative (`ArgumentOutOfRangeException`).
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        assert!(
            x >= 0,
            "Specified argument was out of the range of valid values. (Parameter 'x')"
        );
        assert!(
            y >= 0,
            "Specified argument was out of the range of valid values. (Parameter 'y')"
        );
        assert!(
            width >= 0,
            "Specified argument was out of the range of valid values. (Parameter 'width')"
        );
        assert!(
            height >= 0,
            "Specified argument was out of the range of valid values. (Parameter 'height')"
        );

        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Gets the number of pixels.
    pub fn area(&self) -> i32 {
        self.width * self.height
    }

    /// Gets a value indicating whether the region is empty: true if the
    /// region contains no pixels.
    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// Intersects two rectangles.
    pub fn intersect(rect1: VncRectangle, rect2: VncRectangle) -> VncRectangle {
        if rect1.is_empty() {
            return rect1;
        } else if rect2.is_empty() {
            return rect2;
        }

        let x = rect1.x.max(rect2.x);
        let y = rect1.y.max(rect2.y);
        let w = (rect1.x + rect1.width).min(rect2.x + rect2.width) - x;
        let h = (rect1.y + rect1.height).min(rect2.y + rect2.height) - y;
        if w > 0 && h > 0 {
            VncRectangle::new(x, y, w, h)
        } else {
            VncRectangle::default()
        }
    }

    /// Finds a region that contains both rectangles.
    pub fn union(rect1: VncRectangle, rect2: VncRectangle) -> VncRectangle {
        if rect1.is_empty() {
            return rect2;
        } else if rect2.is_empty() {
            return rect1;
        }

        let x = rect1.x.min(rect2.x);
        let y = rect1.y.min(rect2.y);
        let w = (rect1.x + rect1.width).max(rect2.x + rect2.width) - x;
        let h = (rect1.y + rect1.height).max(rect2.y + rect2.height) - y;
        VncRectangle::new(x, y, w, h)
    }
}

impl std::hash::Hash for VncRectangle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (self.x | self.y << 16).hash(state);
    }
}

impl fmt::Display for VncRectangle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}x{} at {}, {}",
            self.width, self.height, self.x, self.y
        )
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;

    #[test]
    fn rectangles_intersect_and_unite() {
        let a = VncRectangle::new(0, 0, 10, 10);
        let b = VncRectangle::new(5, 5, 10, 10);
        assert_eq!(VncRectangle::new(5, 5, 5, 5), VncRectangle::intersect(a, b));
        assert_eq!(VncRectangle::new(0, 0, 15, 15), VncRectangle::union(a, b));
        assert!(VncRectangle::intersect(a, VncRectangle::new(20, 20, 1, 1)).is_empty());
        assert_eq!(100, a.area());
        assert_eq!("10x10 at 0, 0", a.to_string());
    }

    #[test]
    #[should_panic(expected = "Parameter 'width'")]
    fn a_negative_size_is_rejected() {
        VncRectangle::new(0, 0, -1, 1);
    }
}
