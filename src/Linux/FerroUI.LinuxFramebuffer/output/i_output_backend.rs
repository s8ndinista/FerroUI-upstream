//! The output a top-level is drawn onto (the port of `IOutputBackend.cs`).

use super::IGlOutputBackend;
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::ISurfaceOrientation;
use ferroui_base::PixelSize;
use std::sync::Arc;

pub trait IOutputBackend: IPlatformRenderSurface {
    fn pixel_size(&self) -> PixelSize;

    fn scaling(&self) -> f64;

    fn set_scaling(&self, value: f64);

    /// The output as the render surface of the top-level.
    fn as_render_surface(self: Arc<Self>) -> Arc<dyn IPlatformRenderSurface>;

    /// The output as one that renders with OpenGL (the reference tests
    /// the type).
    fn as_gl_output_backend(&self) -> Option<&dyn IGlOutputBackend> {
        None
    }

    /// The output as one that has an orientation (the reference tests the
    /// type).
    fn as_surface_orientation(&self) -> Option<&dyn ISurfaceOrientation> {
        None
    }
}
