use crate::media::precise_elliptic_arc_helper::PreciseEllipticArcHelper;
use crate::media::{FillRule, SweepDirection};
#[allow(deprecated)]
use crate::platform::IStreamGeometryContextImpl;
use crate::platform::IGeometryContext;
use crate::{Point, Size};

/// Describes a geometry using drawing commands.
///
/// This type is used to describe a stream geometry: obtain one by calling
/// [`StreamGeometry::open`](crate::media::StreamGeometry::open). The context
/// is completed when it is disposed or dropped.
pub struct StreamGeometryContext {
    context: Box<dyn IGeometryContext>,
    current_point: Point,
    is_disposed: bool,
}

impl StreamGeometryContext {
    /// Creates a context over a platform implementation.
    #[deprecated(note = "Use StreamGeometry::open instead.")]
    #[allow(deprecated)]
    pub fn new(impl_: Box<dyn IStreamGeometryContextImpl>) -> Self {
        Self::from_context(impl_)
    }

    /// Creates a context that forwards the drawing commands to `context`.
    pub(crate) fn from_context(context: Box<dyn IGeometryContext>) -> Self {
        Self { context, current_point: Point::default(), is_disposed: false }
    }

    /// Draws an arc to the specified point using polylines, quadratic or cubic Bezier curves.
    /// This is significantly more precise when drawing elliptic arcs with extreme width:height ratios.
    ///
    /// The parameters are those of [`IGeometryContext::arc_to`]:
    ///
    /// * `point` - The destination point where the arc ends.
    /// * `size` - The radii of an ellipse whose path is used to draw the arc.
    /// * `rotation_angle` - The rotation angle (in degrees) of the ellipse that specifies the path of
    ///   the arc; positive values are clockwise. This will rotate the entire arc relative to the
    ///   X-axis and is not commonly used.
    /// * `is_large_arc` - Specifies whether the arc should follow the longer path around an ellipse
    ///   rather than the shorter one. `true` to draw the arc greater than 180 degrees; otherwise,
    ///   `false`.
    /// * `sweep_direction` - A value that indicates whether the arc is drawn in the Clockwise or
    ///   Counterclockwise direction.
    pub fn precise_arc_to(
        &mut self,
        point: Point,
        size: Size,
        rotation_angle: f64,
        is_large_arc: bool,
        sweep_direction: SweepDirection,
    ) {
        let current_point = self.current_point;
        PreciseEllipticArcHelper::arc_to(self, current_point, point, size, rotation_angle, is_large_arc, sweep_direction);
    }
}

impl IGeometryContext for StreamGeometryContext {
    fn set_fill_rule(&mut self, fill_rule: FillRule) {
        self.context.set_fill_rule(fill_rule);
    }

    fn arc_to(
        &mut self,
        point: Point,
        size: Size,
        rotation_angle: f64,
        is_large_arc: bool,
        sweep_direction: SweepDirection,
        is_stroked: bool,
    ) {
        self.context.arc_to(point, size, rotation_angle, is_large_arc, sweep_direction, is_stroked);
        self.current_point = point;
    }

    fn begin_figure(&mut self, start_point: Point, is_filled: bool) {
        self.context.begin_figure(start_point, is_filled);
        self.current_point = start_point;
    }

    fn cubic_bezier_to(&mut self, control_point1: Point, control_point2: Point, end_point: Point, is_stroked: bool) {
        self.context.cubic_bezier_to(control_point1, control_point2, end_point, is_stroked);
        self.current_point = end_point;
    }

    fn quadratic_bezier_to(&mut self, control_point: Point, end_point: Point, is_stroked: bool) {
        self.context.quadratic_bezier_to(control_point, end_point, is_stroked);
        self.current_point = end_point;
    }

    fn line_to(&mut self, point: Point, is_stroked: bool) {
        self.context.line_to(point, is_stroked);
        self.current_point = point;
    }

    fn end_figure(&mut self, is_closed: bool) {
        self.context.end_figure(is_closed);
    }

    /// Finishes the drawing session.
    fn dispose(&mut self) {
        if !self.is_disposed {
            self.is_disposed = true;
            self.context.dispose();
        }
    }
}

impl Drop for StreamGeometryContext {
    fn drop(&mut self) {
        self.dispose();
    }
}
