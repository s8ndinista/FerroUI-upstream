//! Port of `Vnc/VncFramebuffer.cs` of RemoteViewing (BSD-2-Clause,
//! Copyright (c) 2013 James F. Bellinger; `NOTICE.md` of this crate).
//!
//! The pixels and the synchronization object of the original are one
//! mutex: [`get_buffer`](VncFramebuffer::get_buffer) locks it, which is what
//! a user of the original does with `SyncRoot` before it reads the buffer.

use super::{ExtendedDesktopSizeStatus, IVncFramebufferSource, VncPixelFormat};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

/// Stores pixel data for a VNC session.
pub struct VncFramebuffer {
    this: Weak<VncFramebuffer>,
    name: String,
    width: i32,
    height: i32,
    stride: i32,
    pixel_format: VncPixelFormat,
    buffer: Mutex<Vec<u8>>,
}

impl VncFramebuffer {
    /// Initializes a new instance of the [`VncFramebuffer`] class.
    ///
    /// `name` is the framebuffer name (many VNC clients set their title bar
    /// to it), `width` and `height` its size, `pixel_format` its pixel
    /// format.
    ///
    /// # Panics
    /// When the width or the height is negative
    /// (`ArgumentOutOfRangeException`).
    pub fn new(
        name: &str,
        width: i32,
        height: i32,
        pixel_format: VncPixelFormat,
    ) -> Arc<VncFramebuffer> {
        assert!(
            width >= 0,
            "Specified argument was out of the range of valid values. (Parameter 'width')"
        );
        assert!(
            height >= 0,
            "Specified argument was out of the range of valid values. (Parameter 'height')"
        );

        let stride = pixel_format.bytes_per_pixel() * width;
        let length =
            (width as usize) * (height as usize) * (pixel_format.bytes_per_pixel() as usize);
        Arc::new_cyclic(|this| VncFramebuffer {
            this: this.clone(),
            name: name.to_owned(),
            width,
            height,
            stride,
            pixel_format,
            buffer: Mutex::new(vec![0; length]),
        })
    }

    /// Gets the framebuffer name. Many VNC clients set their title bar to
    /// this name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Gets the framebuffer width.
    pub fn width(&self) -> i32 {
        self.width
    }

    /// Gets the framebuffer height.
    pub fn height(&self) -> i32 {
        self.height
    }

    /// Gets the framebuffer stride. This is the number of bytes between one
    /// Y coordinate and the next.
    pub fn stride(&self) -> i32 {
        self.stride
    }

    /// Gets the framebuffer pixel format.
    pub fn pixel_format(&self) -> &VncPixelFormat {
        &self.pixel_format
    }

    /// Returns the memory underlying this framebuffer, locked: the lock of
    /// `SyncRoot` of the original, held as long as the guard lives. Hold it
    /// while reading the framebuffer to avoid tearing artifacts.
    pub fn get_buffer(&self) -> MutexGuard<'_, Vec<u8>> {
        self.buffer.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Sets the RGB `color` of a single pixel.
    ///
    /// # Panics
    /// As [`set_pixel_bytes`](Self::set_pixel_bytes).
    pub fn set_pixel(&self, x: i32, y: i32, color: i32) {
        self.set_pixel_bytes(x, y, &color.to_le_bytes());
    }

    /// Sets the color of a single pixel: `color` is the pixel color as a
    /// byte-encoded integer.
    ///
    /// # Panics
    /// When the pixel is outside the framebuffer
    /// (`ArgumentOutOfRangeException`) or the pixel format is not of four
    /// bytes (`NotSupportedException`).
    pub fn set_pixel_bytes(&self, x: i32, y: i32, color: &[u8]) {
        assert!(
            x >= 0 && x < self.width,
            "Specified argument was out of the range of valid values. (Parameter 'x')"
        );
        assert!(
            y >= 0 && y < self.height,
            "Specified argument was out of the range of valid values. (Parameter 'y')"
        );

        let mut buffer = self.get_buffer();
        if self.pixel_format.bytes_per_pixel() == 4 {
            let start = (y * self.stride + x * 4) as usize;
            buffer[start..start + 4].copy_from_slice(&color[..4]);
        } else {
            panic!("Specified method is not supported.");
        }
    }
}

impl IVncFramebufferSource for VncFramebuffer {
    fn supports_resizing(&self) -> bool {
        false
    }

    fn capture(&self) -> Option<Arc<VncFramebuffer>> {
        self.this.upgrade()
    }

    fn set_desktop_size(&self, _width: i32, _height: i32) -> ExtendedDesktopSizeStatus {
        ExtendedDesktopSizeStatus::Prohibited
    }
}

impl std::fmt::Debug for VncFramebuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VncFramebuffer")
            .field("name", &self.name)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("pixel_format", &self.pixel_format)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;

    #[test]
    fn a_framebuffer_has_the_stride_of_its_format_and_is_its_own_source() {
        let framebuffer = VncFramebuffer::new("test", 3, 2, VncPixelFormat::RGB32);
        assert_eq!(12, framebuffer.stride());
        assert_eq!(24, framebuffer.get_buffer().len());

        framebuffer.set_pixel(1, 1, 0x00ff_8040);
        assert_eq!(&[0x40, 0x80, 0xff, 0x00], &framebuffer.get_buffer()[16..20]);

        let source: &dyn IVncFramebufferSource = &*framebuffer;
        assert!(Arc::ptr_eq(&framebuffer, &source.capture().unwrap()));
        assert!(!source.supports_resizing());
        assert_eq!(
            ExtendedDesktopSizeStatus::Prohibited,
            source.set_desktop_size(10, 10)
        );
    }
}
