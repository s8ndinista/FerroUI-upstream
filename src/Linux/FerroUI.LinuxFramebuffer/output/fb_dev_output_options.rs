//! The options of the framebuffer device output (the port of
//! `FbDevOutputOptions.cs`).

use ferroui_base::platform::PixelFormat;

/// FbDev output options.
#[derive(Clone, Debug, PartialEq)]
pub struct FbDevOutputOptions {
    /// The frame buffer device name. Defaults to the value in the
    /// environment variable FRAMEBUFFER, or /dev/fb0 when FRAMEBUFFER is
    /// not set.
    pub file_name: Option<String>,
    /// The desired pixel format. When not set the format of the device is
    /// used.
    pub pixel_format: Option<PixelFormat>,
    /// Render directly to the mapped memory of the device instead of to a
    /// back buffer that is copied after the vertical sync.
    pub render_directly_to_mapped_memory: bool,
    /// The initial scale factor to use.
    pub scaling: f64,
    /// When true, the wait for the vertical sync and the copy that follows
    /// run on a dedicated thread, saving the rendering thread from a
    /// blocking wait. Only has an effect when
    /// `render_directly_to_mapped_memory` is false.
    pub use_async_front_buffer_blit: Option<bool>,
}

impl Default for FbDevOutputOptions {
    fn default() -> Self {
        Self {
            file_name: None,
            pixel_format: None,
            render_directly_to_mapped_memory: false,
            scaling: 1.0,
            use_async_front_buffer_blit: None,
        }
    }
}
