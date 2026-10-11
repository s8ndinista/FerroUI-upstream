//! What an input backend asks of the screen (the port of
//! `IScreenInfoProvider.cs`).

use ferroui_base::platform::ISurfaceOrientation;
use ferroui_base::Size;

pub trait IScreenInfoProvider {
    fn scaled_size(&self) -> Size;

    /// The screen as one that has an orientation (the reference tests the
    /// type).
    fn as_surface_orientation(&self) -> Option<&dyn ISurfaceOrientation> {
        None
    }
}
