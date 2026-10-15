//! OpenGL ES over EAGL: the platform graphics with the one context of the
//! platform, the render surface of a view over its EAGL layer, and the
//! framebuffer of the renderbuffer of the layer.
//!
//! Apple deprecated OpenGL ES in iOS 12, as the reference states on every
//! class of these files, and the bindings are deprecated as a whole.
//!
//! What is drawn with the context needs a render backend that draws with
//! OpenGL ES: the Skia backend built with Ganesh on Apple systems
//! (docs/porting/ios-platform.md, section 7).

mod eagl_display;
mod eagl_layer_surface;
mod layer_fbo;

pub use eagl_display::{EaglPlatformGraphics, GlContext};
pub use eagl_layer_surface::EaglLayerSurface;
pub(crate) use eagl_layer_surface::{eagl_layer_class, SharedEaglLayer};
pub(crate) use layer_fbo::SizeSynchronizedLayerFbo;
