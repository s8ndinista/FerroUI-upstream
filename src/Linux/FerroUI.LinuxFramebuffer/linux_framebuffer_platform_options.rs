//! The options of the platform (the port of
//! `LinuxFramebufferPlatformOptions.cs`).

/// Platform-specific options which apply to the Linux framebuffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxFramebufferPlatformOptions {
    /// Gets or sets the number of frames per second at which the renderer
    /// should run. Default 60.
    pub fps: i32,

    /// Render directly on the UI thread instead of using a dedicated
    /// render thread. This can be usable if your device don't have
    /// multiple cores to begin with. This setting is false by default.
    pub should_render_on_ui_thread: bool,
}

impl Default for LinuxFramebufferPlatformOptions {
    fn default() -> Self {
        Self { fps: 60, should_render_on_ui_thread: false }
    }
}
