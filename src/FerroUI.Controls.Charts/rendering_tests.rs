//! Rendering regressions exercise the retained drawing commands before a
//! platform backend converts their coordinates to single precision.

use super::*;
use crate::{LineSeries, LinearAxis, Samples};
use ferroui_base::animation::TimeSpan;
use ferroui_controls::testing::{TestServices, UnitTestApplication};

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() <= 1e-9, "{actual} != {expected}");
}

fn point_close(actual: Point, expected: Point) {
    close(actual.x, expected.x);
    close(actual.y, expected.y);
}

fn backend_number(value: f64) {
    assert!(value.is_finite(), "Nonfinite drawing coordinate: {value}");
    assert!(
        (value as f32).is_finite(),
        "Drawing coordinate exceeds backend precision: {value}"
    );
}

fn backend_point(point: Point) {
    backend_number(point.x);
    backend_number(point.y);
}

fn backend_rect(rect: Rect) {
    for value in [
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        rect.right(),
        rect.bottom(),
    ] {
        backend_number(value);
    }
    assert!(
        rect.width >= 0.0 && rect.height >= 0.0,
        "Negative drawing rectangle: {rect:?}"
    );
}

fn backend_pen(pen: &Rc<dyn IPen>) {
    backend_number(pen.thickness());
    assert!(pen.thickness() >= 0.0);
}

fn backend_draw(draw: &Draw) {
    match draw {
        Draw::Line(a, b, pen) => {
            backend_point(*a);
            backend_point(*b);
            backend_pen(pen);
        }
        Draw::Rectangle(rect, _, pen) | Draw::Ellipse(rect, _, pen) => {
            backend_rect(*rect);
            if let Some(pen) = pen {
                backend_pen(pen);
            }
        }
        Draw::Path { points, pen, .. } => {
            for point in points {
                backend_point(*point);
            }
            if let Some(pen) = pen {
                backend_pen(pen);
            }
        }
        Draw::Text(layout, origin, transform) => {
            backend_point(*origin);
            backend_number(layout.width());
            backend_number(layout.height());
            if let Some(matrix) = transform {
                for value in [
                    matrix.m11, matrix.m12, matrix.m13, matrix.m21, matrix.m22, matrix.m23,
                    matrix.m31, matrix.m32, matrix.m33,
                ] {
                    backend_number(value);
                }
            }
        }
    }
}

fn backend_scene(scene: &ChartScene) {
    backend_rect(scene.layout.plot_area);
    for draw in scene
        .background
        .iter()
        .chain(&scene.grid)
        .chain(&scene.series)
        .chain(&scene.annotations)
        .chain(&scene.foreground)
    {
        backend_draw(draw);
    }
    for point in &scene.layout.points {
        backend_point(point.position);
        if let Some(rect) = point.bar_bounds {
            backend_rect(rect);
        }
    }
}

fn unit_point(point: Point) {
    backend_point(point);
    assert!(
        (-1e-12..=1.0 + 1e-12).contains(&point.x),
        "x outside clip: {point:?}"
    );
    assert!(
        (-1e-12..=1.0 + 1e-12).contains(&point.y),
        "y outside clip: {point:?}"
    );
}

fn polygon_area(points: &[Point]) -> f64 {
    signed_polygon_area(points).abs()
}

fn signed_polygon_area(points: &[Point]) -> f64 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .map(|(a, b)| a.x * b.y - b.x * a.y)
        .sum::<f64>()
        * 0.5
}

#[test]
fn mixed_category_value_order_is_consistent_across_permutations() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = Chart::new();
    let axis = CategoryAxis::new();
    axis.set_sort_key(CategorySortKey::Value);
    chart.axes().add(&axis).unwrap();
    let series = LineSeries::new();
    series.set_x_axis(Some(axis.clone().upcast())).unwrap();
    series.set_y_values(Some(Samples::from_values([1.0, 2.0, 3.0])));
    chart.series().add(&series).unwrap();
    // Numeric 2 < numeric 10 < text "15" forms a comparison cycle if mixed
    // pairs are compared lexically but numeric pairs are compared numerically.
    let values = [
        SampleValue::Number(2.0),
        SampleValue::Number(10.0),
        SampleValue::Category("15".into()),
    ];
    for permutation in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        series.set_x_values(Some(Samples::from_values(
            permutation.map(|i| values[i].clone()),
        )));
        for order in [SortOrder::Ascending, SortOrder::Descending] {
            axis.set_sort_order(order);
            let scene = ChartScene::build(&chart, Size::new(600.0, 400.0));
            assert_eq!(scene.layout.points.len(), 3);
            let mut positions = [0.0; 3];
            for point in &scene.layout.points {
                positions[permutation[point.sample_index]] = point.position.x;
            }
            if order == SortOrder::Ascending {
                assert!(
                    positions[0] < positions[1] && positions[1] < positions[2],
                    "{positions:?}"
                );
            } else {
                assert!(
                    positions[0] > positions[1] && positions[1] > positions[2],
                    "{positions:?}"
                );
            }
            backend_scene(&scene);
        }
    }
}

#[test]
fn normalized_segment_clipping_preserves_huge_crossings_in_either_direction() {
    for magnitude in [1e6, 1e100, 1e307, f64::MAX] {
        let cases = [
            (
                Point::new(-magnitude, 0.5),
                Point::new(magnitude, 0.5),
                Point::new(0.0, 0.5),
                Point::new(1.0, 0.5),
            ),
            (
                Point::new(0.25, -magnitude),
                Point::new(0.25, magnitude),
                Point::new(0.25, 0.0),
                Point::new(0.25, 1.0),
            ),
            (
                Point::new(-magnitude, -magnitude),
                Point::new(magnitude, magnitude),
                Point::new(0.0, 0.0),
                Point::new(1.0, 1.0),
            ),
            (
                Point::new(0.0, -magnitude),
                Point::new(0.0, magnitude),
                Point::new(0.0, 0.0),
                Point::new(0.0, 1.0),
            ),
        ];
        for (a, b, expected_a, expected_b) in cases {
            let (first, last) = clip_segment(a, b).expect("Segment crosses the viewport");
            point_close(first, expected_a);
            point_close(last, expected_b);
            unit_point(first);
            unit_point(last);
            let (first, last) =
                clip_segment(b, a).expect("Reversing a segment preserves its intersection");
            point_close(first, expected_b);
            point_close(last, expected_a);
            unit_point(first);
            unit_point(last);
        }
        assert!(clip_segment(
            Point::new(-magnitude, -magnitude),
            Point::new(-magnitude, magnitude)
        )
        .is_none());
        assert!(clip_segment(
            Point::new(-magnitude, magnitude),
            Point::new(magnitude, magnitude)
        )
        .is_none());
    }
}

#[test]
fn normalized_polyline_clipping_does_not_join_across_exterior_excursions() {
    let points = [
        Point::new(-1.0, 0.25),
        Point::new(0.5, 0.25),
        Point::new(2.0, 0.25),
        Point::new(2.0, 0.75),
        Point::new(0.5, 0.75),
        Point::new(-1.0, 0.75),
    ];
    let clipped = clip_polyline(&points);
    assert_eq!(
        clipped.len(),
        2,
        "The exterior vertical excursion cannot become a visible connector"
    );
    point_close(clipped[0][0], Point::new(0.0, 0.25));
    point_close(*clipped[0].last().unwrap(), Point::new(1.0, 0.25));
    point_close(clipped[1][0], Point::new(1.0, 0.75));
    point_close(*clipped[1].last().unwrap(), Point::new(0.0, 0.75));
    for point in clipped.iter().flatten() {
        unit_point(*point);
    }
}

#[test]
fn normalized_polygon_clipping_preserves_area_with_either_winding() {
    for magnitude in [1e6, 1e100, 1e307, f64::MAX] {
        let square = vec![
            Point::new(-magnitude, -magnitude),
            Point::new(magnitude, -magnitude),
            Point::new(magnitude, magnitude),
            Point::new(-magnitude, magnitude),
        ];
        let triangle = vec![
            Point::new(magnitude, magnitude),
            Point::new(-magnitude, -magnitude),
            Point::new(magnitude, -magnitude),
        ];
        for (polygon, expected_area) in [(square, 1.0), (triangle, 0.5)] {
            for reverse in [false, true] {
                let mut source = polygon.clone();
                if reverse {
                    source.reverse();
                }
                let clipped = clip_polygon(source);
                assert!(
                    clipped.len() >= 3,
                    "A polygon covering visible area must survive clipping"
                );
                for point in &clipped {
                    unit_point(*point);
                }
                close(polygon_area(&clipped), expected_area);
            }
        }
    }
}

#[test]
fn area_geometry_precedes_separate_marker_and_label_annotations() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = Chart::new();
    let axis = LinearAxis::new();
    axis.set_minimum(Some(0.0)).unwrap();
    axis.set_maximum(Some(10.0)).unwrap();
    chart.axes().add(&axis).unwrap();
    for values in [[2.0, 4.0, 3.0], [5.0, 7.0, 6.0]] {
        let area = AreaSeries::new();
        area.set_y_axis(Some(axis.clone().upcast())).unwrap();
        area.set_y_values(Some(Samples::from_values(values)));
        area.set_show_data_markers(true);
        area.set_show_data_labels(true);
        area.set_marker_shape(MarkerShape::Circle);
        chart.series().add(&area).unwrap();
    }
    let scene = ChartScene::build(&chart, Size::new(600.0, 400.0));
    assert_eq!(
        scene.series.len(),
        4,
        "Each area has its fill followed by its outline"
    );
    for draws in scene.series.chunks_exact(2) {
        assert!(matches!(
            &draws[0],
            Draw::Path {
                closed: true,
                fill: Some(_),
                pen: None,
                ..
            }
        ));
        assert!(matches!(
            &draws[1],
            Draw::Path {
                closed: false,
                fill: None,
                pen: Some(_),
                ..
            }
        ));
    }
    assert_eq!(scene.annotations.len(), 12);
    for draws in scene.annotations.chunks_exact(2) {
        assert!(matches!(&draws[0], Draw::Ellipse(..)));
        assert!(matches!(&draws[1], Draw::Text(..)));
    }
    assert_eq!(scene.layout.data_labels.len(), 6);
    backend_scene(&scene);
}

#[test]
fn extrema_labels_and_endpoint_markers_survive_automatic_bounds() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = Chart::new();
    let series = LineSeries::new();
    series.set_y_values(Some(Samples::from_values([0.0, 10.0])));
    series.set_show_data_labels(true);
    series.set_show_data_markers(true);
    chart.series().add(&series).unwrap();
    let scene = ChartScene::build(&chart, Size::new(600.0, 400.0));
    let plot = scene.layout.plot_area;
    let mut markers = 0;
    let mut labels = 0;
    for draw in &scene.annotations {
        match draw {
            Draw::Ellipse(rect, ..) => {
                markers += 1;
                assert!(rect.width > 0.0 && rect.height > 0.0);
                // Edge markers remain complete commands, allowed into the
                // margins because annotations use the outer control clip.
                assert!(rect.x < plot.x || rect.right() > plot.right());
            }
            Draw::Text(layout, origin, _) => {
                labels += 1;
                assert!(origin.x >= plot.x - 1e-9 && origin.y >= plot.y - 1e-9);
                assert!(origin.x + layout.width() <= plot.right() + 1e-9);
                assert!(origin.y + layout.height() <= plot.bottom() + 1e-9);
            }
            _ => panic!("Expected only label and circle-marker annotations"),
        }
    }
    assert_eq!((markers, labels), (2, 2));
    backend_scene(&scene);
}

#[test]
fn nonfinite_samples_and_extreme_finite_values_never_reach_draw_geometry() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    for series in [
        LineSeries::new().upcast::<CartesianSeries>(),
        AreaSeries::new().upcast::<CartesianSeries>(),
        BarSeries::new().upcast::<CartesianSeries>(),
    ] {
        let chart = Chart::new();
        let axis = LinearAxis::new();
        axis.set_minimum(Some(-1.0)).unwrap();
        axis.set_maximum(Some(1.0)).unwrap();
        chart.axes().add(&axis).unwrap();
        series.set_y_axis(Some(axis.upcast())).unwrap();
        series.set_y_values(Some(Samples::from_values([
            f64::MAX,
            -f64::MAX,
            0.0,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            0.5,
        ])));
        series.set_show_data_labels(true);
        series.set_show_data_markers(true);
        chart.series().add(&series).unwrap();
        for size in [
            Size::new(600.0, 400.0),
            Size::new(1.0, 1.0),
            Size::new(f64::MAX, f64::MAX),
        ] {
            let scene = ChartScene::build(&chart, size);
            let indices: Vec<_> = scene
                .layout
                .points
                .iter()
                .map(|point| point.sample_index)
                .collect();
            assert_eq!(
                indices,
                [0, 1, 2, 6],
                "Only nonfinite samples are discarded"
            );
            assert!(
                !scene.series.is_empty(),
                "Visible crossings and finite points remain drawable"
            );
            backend_scene(&scene);
        }
    }
}

#[test]
fn unmatched_and_invalid_x_pairs_cannot_distort_automatic_y_range() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = Chart::new();
    let series = LineSeries::new();
    series.set_x_values(Some(Samples::from_values([0.0, 1.0])));
    series.set_y_values(Some(Samples::from_values([10.0, 20.0])));
    chart.series().add(&series).unwrap();
    let size = Size::new(600.0, 400.0);
    let original = ChartScene::build(&chart, size);
    let expected: Vec<_> = original
        .layout
        .points
        .iter()
        .map(|point| point.position)
        .collect();
    assert_eq!(expected.len(), 2);

    series.set_y_values(Some(Samples::from_values([10.0, 20.0, f64::MAX])));
    let unmatched = ChartScene::build(&chart, size);
    assert_eq!(unmatched.layout.plot_area, original.layout.plot_area);
    assert_eq!(unmatched.layout.axis_labels, original.layout.axis_labels);
    assert_eq!(
        unmatched
            .layout
            .points
            .iter()
            .map(|point| point.position)
            .collect::<Vec<_>>(),
        expected
    );

    series.set_x_values(Some(Samples::from_values([0.0, f64::NAN, 1.0])));
    series.set_y_values(Some(Samples::from_values([10.0, f64::MAX, 20.0])));
    let invalid_x = ChartScene::build(&chart, size);
    assert_eq!(invalid_x.layout.plot_area, original.layout.plot_area);
    assert_eq!(invalid_x.layout.axis_labels, original.layout.axis_labels);
    assert_eq!(
        invalid_x
            .layout
            .points
            .iter()
            .map(|point| point.position)
            .collect::<Vec<_>>(),
        expected
    );
    assert_eq!(
        invalid_x
            .layout
            .points
            .iter()
            .map(|point| point.sample_index)
            .collect::<Vec<_>>(),
        [0, 2]
    );
    backend_scene(&unmatched);
    backend_scene(&invalid_x);
}

#[test]
fn date_bars_outside_explicit_bounds_do_not_emit_outside_geometry_or_annotations() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let first = DateTimeOffset::new(2020, 1, 1, 0, 0, 0, TimeSpan::ZERO);
    let middle = DateTimeOffset::new(2020, 1, 2, 0, 0, 0, TimeSpan::ZERO);
    let last = DateTimeOffset::new(2020, 1, 3, 0, 0, 0, TimeSpan::ZERO);
    for orientation in [BarOrientation::Vertical, BarOrientation::Horizontal] {
        let chart = Chart::new();
        let x = DateTimeAxis::new();
        x.set_minimum(Some(first)).unwrap();
        x.set_maximum(Some(last)).unwrap();
        let y = LinearAxis::new();
        y.set_minimum(Some(0.0)).unwrap();
        y.set_maximum(Some(10.0)).unwrap();
        chart.axes().add(&x).unwrap();
        chart.axes().add(&y).unwrap();
        let series = BarSeries::new();
        series.set_x_axis(Some(x.upcast())).unwrap();
        series.set_y_axis(Some(y.upcast())).unwrap();
        series.set_orientation(orientation);
        series.set_x_values(Some(Samples::from_values([
            DateTimeOffset::MIN_VALUE,
            first,
            middle,
            last,
            DateTimeOffset::MAX_VALUE,
        ])));
        series.set_y_values(Some(Samples::from_values([2.0, 3.0, 4.0, 5.0, 6.0])));
        series.set_show_data_labels(true);
        series.set_show_data_markers(true);
        chart.series().add(&series).unwrap();
        let scene = ChartScene::build(&chart, Size::new(600.0, 400.0));
        let plot = scene.layout.plot_area;
        assert_eq!(
            scene.layout.points.len(),
            5,
            "Diagnostics retain clipped source indices"
        );
        assert_eq!(
            scene.series.len(),
            3,
            "Only three bars intersect the explicit date range"
        );
        for draw in &scene.series {
            let Draw::Rectangle(rect, ..) = draw else {
                panic!("Bar geometry must be rectangular")
            };
            assert!(rect.width > 0.0 && rect.height > 0.0);
            assert!(rect.x >= plot.x - 1e-9 && rect.y >= plot.y - 1e-9);
            assert!(rect.right() <= plot.right() + 1e-9 && rect.bottom() <= plot.bottom() + 1e-9);
        }
        assert_eq!(
            scene
                .annotations
                .iter()
                .filter(|draw| matches!(draw, Draw::Ellipse(..)))
                .count(),
            3
        );
        assert_eq!(
            scene
                .annotations
                .iter()
                .filter(|draw| matches!(draw, Draw::Text(..)))
                .count(),
            3
        );
        for point in &scene.layout.points {
            let bounds = point.bar_bounds.unwrap();
            if point.sample_index == 0 || point.sample_index == 4 {
                assert!(bounds.width == 0.0 || bounds.height == 0.0);
            }
        }
        backend_scene(&scene);
    }
}

#[test]
fn dense_area_fill_reduces_vertices_without_losing_extrema_or_winding() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = Chart::new();
    let x = CategoryAxis::new();
    let y = LinearAxis::new();
    y.set_minimum(Some(0.0)).unwrap();
    y.set_maximum(Some(100.0)).unwrap();
    chart.axes().add(&x).unwrap();
    chart.axes().add(&y).unwrap();
    let series = AreaSeries::new();
    series.set_x_axis(Some(x.clone().upcast())).unwrap();
    series.set_y_axis(Some(y.upcast())).unwrap();
    let count = 20_000;
    let peak = 10_001;
    let values: Vec<_> = (0..count)
        .map(|index| {
            if index == peak {
                90.0
            } else {
                10.0 + (index % 13) as f64
            }
        })
        .collect();
    series.set_y_values(Some(Samples::from_values(values)));
    chart.series().add(&series).unwrap();
    for order in [SortOrder::Ascending, SortOrder::Descending] {
        x.set_sort_order(order);
        let scene = ChartScene::build(&chart, Size::new(600.0, 400.0));
        assert_eq!(
            scene.layout.points.len(),
            count,
            "Diagnostic points keep every source sample"
        );
        let peak_position = scene
            .layout
            .points
            .iter()
            .find(|point| point.sample_index == peak)
            .unwrap()
            .position;
        let plot = scene.layout.plot_area;
        let budget = 4 * (plot.width.ceil() as usize + 1) + 16;
        let mut filled = 0;
        for draw in &scene.series {
            let Draw::Path {
                points,
                fill: Some(_),
                closed,
                ..
            } = draw
            else {
                continue;
            };
            filled += 1;
            assert!(*closed);
            assert!(
                points.len() <= budget,
                "{} area vertices exceed the pixel envelope budget {budget}",
                points.len()
            );
            assert!(
                points
                    .iter()
                    .any(|point| (point.x - peak_position.x).abs() < 1e-9
                        && (point.y - peak_position.y).abs() < 1e-9),
                "The isolated peak must survive area reduction"
            );
            assert!(
                polygon_area(points) > 0.0,
                "Reduction must retain a nondegenerate filled contour"
            );
            assert_eq!(
                signed_polygon_area(points).is_sign_positive(),
                order == SortOrder::Ascending,
                "Reduction preserves the source contour's winding after category reversal"
            );
            assert!(points.iter().all(|point| point.x >= plot.x - 1e-9
                && point.x <= plot.right() + 1e-9
                && point.y >= plot.y - 1e-9
                && point.y <= plot.bottom() + 1e-9));
        }
        assert_eq!(filled, 1);
        backend_scene(&scene);
    }
}
