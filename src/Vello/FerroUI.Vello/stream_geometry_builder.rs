use crate::geometry_impl::{try_get_geometry_impl, FillPath, VelloPath};
use crate::simple_geometry_impl::SimpleGeometryImpl;
use crate::vello_extensions::{to_kurbo_point, PATH_TOLERANCE};
use ferroui_base::media::{FillRule, SweepDirection};
use ferroui_base::platform::{IGeometryContext, IGeometryImpl, IStreamGeometryBuilder};
use ferroui_base::{Point, Size};
use kurbo::{BezPath, SvgArc, Vec2};
use std::sync::Arc;

/// A path under construction with the rule it will be filled by.
///
/// The commands are those of the path builder of Skia, with its rules for a
/// command that has no point to start from: it starts at the origin.
#[derive(Clone)]
struct PathBuilder {
    path: BezPath,
    fill_rule: FillRule,
}

impl PathBuilder {
    /// A builder of an empty path, filled by the non-zero rule.
    fn new() -> Self {
        Self { path: BezPath::new(), fill_rule: FillRule::NonZero }
    }

    fn from_path(path: &VelloPath) -> Self {
        Self { path: path.path().clone(), fill_rule: path.fill_rule() }
    }

    fn snapshot(&self) -> VelloPath {
        VelloPath::new(self.path.clone(), self.fill_rule)
    }

    /// The point the next command starts from.
    fn current_point(&mut self) -> kurbo::Point {
        // (A path of kurbo is "empty" until it has a segment: the elements
        // are what tells whether a figure was begun.)
        if self.path.elements().is_empty() {
            self.path.move_to((0.0, 0.0));
        }
        self.path.current_position().unwrap_or_default()
    }

    fn move_to(&mut self, point: kurbo::Point) {
        self.path.move_to(point);
    }

    fn line_to(&mut self, point: kurbo::Point) {
        self.current_point();
        self.path.line_to(point);
    }

    fn quad_to(&mut self, p1: kurbo::Point, p2: kurbo::Point) {
        self.current_point();
        self.path.quad_to(p1, p2);
    }

    fn cubic_to(&mut self, p1: kurbo::Point, p2: kurbo::Point, p3: kurbo::Point) {
        self.current_point();
        self.path.curve_to(p1, p2, p3);
    }

    fn close(&mut self) {
        if !self.path.elements().is_empty() {
            self.path.close_path();
        }
    }

    /// Adds an elliptical arc from the current point to `end`, with the
    /// endpoint parameters of an arc of SVG (which the contract describes):
    /// radii that are too small to reach the end point are scaled up, and
    /// an arc without a radius or without a length is a line to the end
    /// point. The arc is added as cubic Béziers.
    fn arc_to(&mut self, radii: Vec2, rotation_angle: f64, is_large_arc: bool, clockwise: bool, end: kurbo::Point) {
        let from = self.current_point();
        let arc = SvgArc {
            from,
            to: end,
            radii,
            x_rotation: rotation_angle.to_radians(),
            large_arc: is_large_arc,
            // The sweep flag of SVG: the arc is drawn in the direction of
            // growing angles, which is clockwise with the y axis down.
            sweep: clockwise,
        };

        match kurbo::Arc::from_svg_arc(&arc) {
            Some(arc) => {
                let mut curves = Vec::new();
                arc.to_cubic_beziers(PATH_TOLERANCE, |p1, p2, p3| curves.push((p1, p2, p3)));
                // The last Bézier ends at the end of the arc as it was
                // computed; the figure continues from the end point as it
                // was given.
                if let Some(last) = curves.last_mut() {
                    last.2 = end;
                }
                for (p1, p2, p3) in curves {
                    self.path.curve_to(p1, p2, p3);
                }
            }
            None => self.path.line_to(end),
        }
    }
}

/// The fill being built by a [`StreamGeometryBuilder`].
enum FillBuilder {
    /// No fill path exists yet.
    None,
    /// The fill is the stroke.
    SameAsStroke,
    /// The fill is built separately from the stroke.
    Separate(PathBuilder),
}

/// Builds a kurbo geometry from drawing commands.
///
/// The paths of a geometry are immutable, so the builder works on path
/// builders seeded from the paths of its source geometry, and creates the
/// geometry from their snapshots.
pub struct StreamGeometryBuilder {
    stroke: PathBuilder,
    fill: FillBuilder,
    is_filled: bool,
    start_point: Point,
    is_figure_broken: bool,
    built: bool,
}

impl StreamGeometryBuilder {
    /// Creates a builder that starts with the figures of `source`, a geometry
    /// of this backend, or with an empty geometry when `source` is `None`.
    ///
    /// Panics if `source` was not created by the Vello render interface.
    pub fn new(source: Option<&dyn IGeometryImpl>) -> Self {
        let (stroke, fill) = match source {
            None => (PathBuilder::from_path(&VelloPath::empty()), FillBuilder::SameAsStroke),
            Some(source) => {
                let geometry = try_get_geometry_impl(source)
                    .unwrap_or_else(|| panic!("The geometry wasn't created by the Vello render interface."));
                let stroke_path = geometry.stroke_path();
                let fill = match geometry.fill() {
                    FillPath::SameAsStroke => FillBuilder::SameAsStroke,
                    FillPath::Separate(fill_path) => FillBuilder::Separate(PathBuilder::from_path(&fill_path)),
                    // Without a stroke path either, the fill is the (empty) stroke.
                    FillPath::None if stroke_path.is_none() => FillBuilder::SameAsStroke,
                    FillPath::None => FillBuilder::None,
                };
                let stroke = PathBuilder::from_path(&stroke_path.unwrap_or_else(VelloPath::empty));
                (stroke, fill)
            }
        };

        Self { stroke, fill, is_filled: false, start_point: Point::default(), is_figure_broken: false, built: false }
    }

    /// The fill builder, created on first use.
    fn fill(&mut self) -> &mut PathBuilder {
        if matches!(self.fill, FillBuilder::None) {
            self.fill = FillBuilder::Separate(PathBuilder::new());
        }

        match &mut self.fill {
            FillBuilder::SameAsStroke => &mut self.stroke,
            FillBuilder::Separate(fill) => fill,
            FillBuilder::None => unreachable!("the fill builder was just created"),
        }
    }

    /// Whether commands have to be repeated on a separate fill path.
    fn duplicate(&self) -> bool {
        self.is_filled && !matches!(self.fill, FillBuilder::SameAsStroke)
    }

    fn ensure_separate_fill_path(&mut self) {
        if matches!(self.fill, FillBuilder::SameAsStroke) {
            self.fill = FillBuilder::Separate(self.stroke.clone());
        }
    }

    fn break_figure(&mut self) {
        if !self.is_figure_broken {
            self.is_figure_broken = true;
            self.ensure_separate_fill_path();
        }
    }
}

impl IStreamGeometryBuilder for StreamGeometryBuilder {
    fn to_geometry(&mut self) -> Arc<dyn IGeometryImpl> {
        if self.built {
            panic!("The geometry has already been built.");
        }

        self.built = true;

        let stroke = self.stroke.snapshot();
        let fill = match &self.fill {
            FillBuilder::None => FillPath::None,
            FillBuilder::SameAsStroke => FillPath::SameAsStroke,
            FillBuilder::Separate(fill) => FillPath::Separate(fill.snapshot()),
        };

        SimpleGeometryImpl::new(stroke, fill, None)
    }
}

impl IGeometryContext for StreamGeometryBuilder {
    fn arc_to(
        &mut self,
        point: Point,
        size: Size,
        rotation_angle: f64,
        is_large_arc: bool,
        sweep_direction: SweepDirection,
        is_stroked: bool,
    ) {
        let clockwise = sweep_direction == SweepDirection::Clockwise;
        let radii = Vec2::new(size.width, size.height);
        let end = to_kurbo_point(point);

        if is_stroked {
            self.stroke.arc_to(radii, rotation_angle, is_large_arc, clockwise, end);
        } else {
            self.break_figure();
            self.stroke.move_to(end);
        }

        if self.duplicate() {
            self.fill().arc_to(radii, rotation_angle, is_large_arc, clockwise, end);
        }
    }

    fn begin_figure(&mut self, start_point: Point, is_filled: bool) {
        if !is_filled {
            self.ensure_separate_fill_path();
        }

        self.is_filled = is_filled;
        self.start_point = start_point;
        self.is_figure_broken = false;
        self.stroke.move_to(to_kurbo_point(start_point));

        if self.duplicate() {
            self.fill().move_to(to_kurbo_point(start_point));
        }
    }

    fn cubic_bezier_to(&mut self, control_point1: Point, control_point2: Point, end_point: Point, is_stroked: bool) {
        let (p1, p2, p3) =
            (to_kurbo_point(control_point1), to_kurbo_point(control_point2), to_kurbo_point(end_point));

        if is_stroked {
            self.stroke.cubic_to(p1, p2, p3);
        } else {
            self.break_figure();
            self.stroke.move_to(p3);
        }

        if self.duplicate() {
            self.fill().cubic_to(p1, p2, p3);
        }
    }

    fn quadratic_bezier_to(&mut self, control_point: Point, end_point: Point, is_stroked: bool) {
        let (p1, p2) = (to_kurbo_point(control_point), to_kurbo_point(end_point));

        if is_stroked {
            self.stroke.quad_to(p1, p2);
        } else {
            self.break_figure();
            self.stroke.move_to(p2);
        }

        if self.duplicate() {
            self.fill().quad_to(p1, p2);
        }
    }

    fn line_to(&mut self, point: Point, is_stroked: bool) {
        let point = to_kurbo_point(point);

        if is_stroked {
            self.stroke.line_to(point);
        } else {
            self.break_figure();
            self.stroke.move_to(point);
        }

        if self.duplicate() {
            self.fill().line_to(point);
        }
    }

    fn end_figure(&mut self, is_closed: bool) {
        if is_closed {
            if self.is_figure_broken {
                self.stroke.line_to(to_kurbo_point(self.start_point));
                self.is_figure_broken = false;
            } else {
                self.stroke.close();
            }

            if self.duplicate() {
                self.fill().close();
            }
        }
    }

    fn set_fill_rule(&mut self, fill_rule: FillRule) {
        self.fill().fill_rule = fill_rule;
    }

    fn dispose(&mut self) {}
}
