//! Port of upstream's `Media/StreamGeometryTests.cs`.
//!
//! Upstream's first test renders its two outputs and compares neither with
//! an expected image; so does the port.

use crate::test_base::TestBase;
use ferroui_base::media::{
    Colors, PathGeometry, RotateTransform, SolidColorBrush, Stretch, StreamGeometry, SweepDirection,
    TranslateTransform,
};
use ferroui_base::platform::IGeometryContext;
use ferroui_base::{Point, Size, Thickness};
use ferroui_controls::primitives::UniformGrid;
use ferroui_controls::shapes::Path;

fn base() -> TestBase {
    TestBase::new(r"Media\StreamGeometry")
}

#[test]
fn precise_elliptic_arc_produces_valid_arcs_in_all_directions() {
    let t = base();
    let grid = UniformGrid::new();
    grid.set_columns(2);
    grid.set_rows(4);
    grid.set_width(320.0);
    grid.set_height(400.0);
    for sweep_direction in [SweepDirection::Clockwise, SweepDirection::CounterClockwise] {
        for is_large_arc in [false, true] {
            for is_precise in [false, true] {
                let pt = |x: f64, y: f64| Point::new(x, y);
                let sz = |w: f64, h: f64| Size::new(w, h);
                let stream_geometry = StreamGeometry::new();
                {
                    let mut context = stream_geometry.open();
                    context.begin_figure(pt(20.0, 20.0), true);

                    if is_precise {
                        context.precise_arc_to(pt(40.0, 40.0), sz(20.0, 20.0), 0.0, is_large_arc, sweep_direction);
                    } else {
                        context.arc_to(pt(40.0, 40.0), sz(20.0, 20.0), 0.0, is_large_arc, sweep_direction, true);
                    }
                    context.line_to(pt(40.0, 20.0), true);
                    context.line_to(pt(20.0, 20.0), true);
                    context.end_figure(true);
                }
                let path_shape = Path::new();
                path_shape.set_data(stream_geometry);
                path_shape.set_stroke(Some(SolidColorBrush::with_color(Colors::CORNFLOWER_BLUE).into()));
                path_shape.set_fill(Some(SolidColorBrush::with_color(Colors::GOLD).into()));
                path_shape.set_stroke_thickness(2.0);
                path_shape.set_margin(Thickness::uniform(20.0));
                grid.children().add(path_shape);
            }
        }
    }
    t.render_to_file(&grid, "PreciseEllipticArc_Produces_Valid_Arcs_In_All_Directions");
}

#[test]
fn stretched_path_with_transformed_geometry() {
    let t = base();
    let geometry = StreamGeometry::new();

    {
        let mut context = geometry.open();
        context.begin_figure(Point::new(0.0, 0.0), true);
        context.line_to(Point::new(80.0, 0.0), true);
        context.line_to(Point::new(80.0, 40.0), true);
        context.line_to(Point::new(0.0, 40.0), true);
        context.end_figure(true);
    }

    geometry.set_transform(RotateTransform::with_angle(30.0));

    let target = Path::new();
    target.set_data(geometry);
    target.set_fill(Some(SolidColorBrush::with_color(Colors::CORNFLOWER_BLUE).into()));
    target.set_stroke(Some(SolidColorBrush::with_color(Colors::BLACK).into()));
    target.set_stroke_thickness(2.0);
    target.set_stretch(Stretch::Uniform);
    target.set_width(200.0);
    target.set_height(200.0);

    t.render_to_file(&target, "Stretched_Path_With_Transformed_Geometry");
    t.compare_images("Stretched_Path_With_Transformed_Geometry");
}

#[test]
fn transformed_geometry_can_be_reopened() {
    let t = base();
    let geometry = StreamGeometry::new();
    geometry.set_transform(TranslateTransform::with_offset(60.0, 30.0));

    {
        let mut context = geometry.open();
        context.begin_figure(Point::new(20.0, 20.0), true);
        context.line_to(Point::new(120.0, 20.0), true);
        context.line_to(Point::new(70.0, 100.0), true);
        context.end_figure(true);
    }

    let target = Path::new();
    target.set_data(geometry);
    target.set_fill(Some(SolidColorBrush::with_color(Colors::GOLD).into()));
    target.set_stroke(Some(SolidColorBrush::with_color(Colors::BLACK).into()));
    target.set_stroke_thickness(2.0);
    target.set_width(200.0);
    target.set_height(200.0);

    t.render_to_file(&target, "Transformed_Geometry_Can_Be_Reopened");
    t.compare_images("Transformed_Geometry_Can_Be_Reopened");
}

#[test]
fn path_geometry_can_be_opened_with_a_transform() {
    let t = base();
    let geometry = PathGeometry::new();
    geometry.set_transform(TranslateTransform::with_offset(0.0, 40.0));

    {
        let mut context = geometry.open();
        context.begin_figure(Point::new(20.0, 20.0), false);
        context.line_to(Point::new(100.0, 100.0), true);
        context.line_to(Point::new(180.0, 20.0), true);
        context.end_figure(false);
    }

    let target = Path::new();
    target.set_data(geometry);
    target.set_stroke(Some(SolidColorBrush::with_color(Colors::SEA_GREEN).into()));
    target.set_stroke_thickness(6.0);
    target.set_width(200.0);
    target.set_height(200.0);

    t.render_to_file(&target, "PathGeometry_Can_Be_Opened_With_A_Transform");
    t.compare_images("PathGeometry_Can_Be_Opened_With_A_Transform");
}

#[test]
fn path_geometry_can_be_opened() {
    let t = base();
    let geometry = PathGeometry::new();

    {
        let mut context = geometry.open();
        context.begin_figure(Point::new(20.0, 20.0), false);
        context.line_to(Point::new(100.0, 100.0), true);
        context.line_to(Point::new(180.0, 20.0), true);
        context.end_figure(false);
    }

    let target = Path::new();
    target.set_data(geometry);
    target.set_stroke(Some(SolidColorBrush::with_color(Colors::CRIMSON).into()));
    target.set_stroke_thickness(6.0);
    target.set_width(200.0);
    target.set_height(200.0);

    t.render_to_file(&target, "PathGeometry_Can_Be_Opened");
    t.compare_images("PathGeometry_Can_Be_Opened");
}

#[test]
fn stretched_path_with_path_geometry_data() {
    let t = base();
    let target = Path::new();
    target.set_data(PathGeometry::parse("M 3,2 L 17,2 L 17,7 L 10,18 L 3,7 Z").unwrap());
    target.set_fill(Some(SolidColorBrush::with_color(Colors::MEDIUM_SEA_GREEN).into()));
    target.set_stretch(Stretch::Uniform);
    target.set_width(200.0);
    target.set_height(200.0);

    t.render_to_file(&target, "Stretched_Path_With_PathGeometry_Data");
    t.compare_images("Stretched_Path_With_PathGeometry_Data");
}
