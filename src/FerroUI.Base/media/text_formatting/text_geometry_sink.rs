use std::rc::Rc;

use super::ITextDrawingSink;
use crate::media::{
    FillRule, Geometry, GeometryGroup, GlyphRun, IBrush, IPen, LineGeometry, MatrixTransform,
    RectangleGeometry,
};
use crate::{Matrix, Point, Rect, Ref};

/// Collects the contours emitted by the already shaped lines of a text layout.
/// Keeping this at the drawing-sink boundary preserves line alignment, bidi run
/// placement, fallback fonts and the actual decoration pen geometry.
pub(super) struct TextGeometrySink {
    group: Ref<GeometryGroup>,
    transform: Matrix,
    transforms: Vec<Matrix>,
}

impl TextGeometrySink {
    pub(super) fn new() -> Self {
        let group = GeometryGroup::new();
        group.set_fill_rule(FillRule::NonZero);
        Self { group, transform: Matrix::IDENTITY, transforms: Vec::new() }
    }

    pub(super) fn into_geometry(self) -> Ref<Geometry> {
        self.group.upcast()
    }

    fn append(&self, geometry: Ref<Geometry>) {
        geometry.set_transform(MatrixTransform::with_matrix(self.transform));
        self.group.children().add(geometry);
    }

    fn append_stroke(&self, geometry: &Ref<Geometry>, pen: &dyn IPen) {
        // A widened geometry has a fixed immutable implementation. Carry the
        // text-line transform on a mutable group instead of setting a property
        // that cannot invalidate that implementation.
        let group = GeometryGroup::new();
        group.set_fill_rule(FillRule::NonZero);
        group.children().add(geometry.get_widened_geometry(pen));
        self.append(group.upcast());
    }
}

impl ITextDrawingSink for TextGeometrySink {
    fn draw_glyph_run(&mut self, _foreground: Option<&Rc<dyn IBrush>>, glyph_run: &Rc<GlyphRun>) {
        self.append(glyph_run.build_geometry());
    }

    fn draw_rectangle(&mut self, brush: Option<&Rc<dyn IBrush>>, pen: Option<&Rc<dyn IPen>>, rect: Rect) {
        let geometry: Ref<Geometry> = RectangleGeometry::with_rect(rect).upcast();
        if let Some(pen) = pen {
            self.append_stroke(&geometry, pen.as_ref());
        }
        if brush.is_some() {
            self.append(geometry);
        }
    }

    fn draw_line(&mut self, pen: &Rc<dyn IPen>, p1: Point, p2: Point) {
        self.append_stroke(&LineGeometry::with_points(p1, p2).upcast(), pen.as_ref());
    }

    fn push_transform(&mut self, matrix: Matrix) {
        self.transforms.push(self.transform);
        // Row-vector composition: each new local transform precedes its parent.
        self.transform = matrix * self.transform;
    }

    fn pop_transform(&mut self) {
        self.transform = self.transforms.pop().expect("A text drawing transform was pushed");
    }
}
