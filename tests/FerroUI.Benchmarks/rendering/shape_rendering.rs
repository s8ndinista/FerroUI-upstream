//! Rendering a line into a drawing context that draws nothing: without
//! brushes, with a stroke, with a fill, and with both.
//!
//! The drawing context stub and the render interface of the headless
//! platform are internal to the headless crate; its `testing` feature
//! exposes them (`ferroui_headless::testing`), as upstream's benchmark
//! project reaches them through its access to the internals of the platform.

use crate::harness::Registry;
use ferroui_base::media::{Brush, DrawingContext, IBrush, PlatformDrawingContext, SolidColorBrush};
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::{FerroLocator, Ref};
use ferroui_controls::shapes::Line;
use ferroui_headless::testing::{headless_drawing_context_stub, headless_platform_render_interface};
use std::rc::Rc;

pub struct ShapeRendering {
    drawing_context: DrawingContext<'static>,
    line_fill: Ref<Line>,
    line_fill_and_stroke: Ref<Line>,
    line_no_brushes: Ref<Line>,
    line_stroke: Ref<Line>,
}

/// `new SolidColorBrush()`.
fn solid_color_brush() -> Option<Rc<dyn IBrush>> {
    let brush: Ref<Brush> = SolidColorBrush::new().upcast();
    Some(brush.into())
}

impl ShapeRendering {
    pub fn new() -> Self {
        let line_no_brushes = Line::new();
        let line_stroke = Line::new();
        line_stroke.set_stroke(solid_color_brush());
        let line_fill = Line::new();
        line_fill.set_fill(solid_color_brush());
        let line_fill_and_stroke = Line::new();
        line_fill_and_stroke.set_stroke(solid_color_brush());
        line_fill_and_stroke.set_fill(solid_color_brush());

        let drawing_context =
            DrawingContext::owned(Box::new(PlatformDrawingContext::new(headless_drawing_context_stub())));

        FerroLocator::current_mutable()
            .bind::<dyn IPlatformRenderInterface>()
            .to_constant(headless_platform_render_interface());

        Self { drawing_context, line_fill, line_fill_and_stroke, line_no_brushes, line_stroke }
    }

    pub fn render_line_no_brushes(&mut self) {
        self.line_no_brushes.render(&mut self.drawing_context);
    }

    pub fn render_line_with_stroke(&mut self) {
        self.line_stroke.render(&mut self.drawing_context);
    }

    pub fn render_line_with_fill(&mut self) {
        self.line_fill.render(&mut self.drawing_context);
    }

    pub fn render_line_with_fill_and_stroke(&mut self) {
        self.line_fill_and_stroke.render(&mut self.drawing_context);
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("rendering", "ShapeRendering");
    class.benchmark("render_line_no_brushes", "", ShapeRendering::new, |b| b.render_line_no_brushes());
    class.benchmark("render_line_with_stroke", "", ShapeRendering::new, |b| b.render_line_with_stroke());
    class.benchmark("render_line_with_fill", "", ShapeRendering::new, |b| b.render_line_with_fill());
    class.benchmark("render_line_with_fill_and_stroke", "", ShapeRendering::new, |b| {
        b.render_line_with_fill_and_stroke()
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn shape_rendering() {
        crate::harness::smoke_class(super::register, "ShapeRendering");
    }
}
