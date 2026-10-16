//! The render interface of the platform and its drawing context stub, for
//! the benchmarks of the framework: upstream's shape rendering benchmark
//! creates both (`new HeadlessPlatformRenderInterface()`,
//! `new HeadlessPlatformRenderInterface.HeadlessDrawingContextStub()`)
//! through its access to the internals of the platform.

use crate::headless_platform_render_interface::{HeadlessDrawingContextStub, HeadlessPlatformRenderInterface};
use ferroui_base::platform::{IDrawingContextImpl, IPlatformRenderInterface};
use std::rc::Rc;

/// `new HeadlessPlatformRenderInterface()`: the render interface of the
/// headless drawing, in which geometries behave like their bounds and
/// bitmaps have a size and no content.
pub fn headless_platform_render_interface() -> Rc<dyn IPlatformRenderInterface> {
    HeadlessPlatformRenderInterface::create()
}

/// `new HeadlessPlatformRenderInterface.HeadlessDrawingContextStub()`: a
/// drawing context that draws nothing.
pub fn headless_drawing_context_stub() -> Box<dyn IDrawingContextImpl> {
    Box::new(HeadlessDrawingContextStub::new())
}
