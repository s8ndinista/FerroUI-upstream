//! Public axis contract tests, plus numeric/calendar safety regressions.
use crate::{
    Axis, CartesianAxis, CategoryAxis, CategorySortKey, DateTimeAxis, DateTimeIntervalType,
    GridLines, LinearAxis, NumericRange, SortOrder,
};
use ferroui_base::animation::TimeSpan;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Colors, IBrush};
use ferroui_base::utilities::DateTimeOffset;
use ferroui_base::{Ref, StyledProperty};
use std::rc::Rc;

fn date(year: i32, month: i32, day: i32) -> DateTimeOffset {
    DateTimeOffset::new(year, month, day, 0, 0, 0, TimeSpan::ZERO)
}

fn brush_properties() -> [&'static StyledProperty<Option<Rc<dyn IBrush>>>; 5] {
    [
        CartesianAxis::grid_line_major_brush_property(),
        CartesianAxis::grid_line_minor_brush_property(),
        CartesianAxis::tick_brush_property(),
        CartesianAxis::tick_label_brush_property(),
        CartesianAxis::axis_line_brush_property(),
    ]
}

#[test]
fn linear_axis_defaults_are_automatic() {
    let axis = LinearAxis::new();
    assert_eq!(None, axis.minimum());
    assert_eq!(None, axis.maximum());
    assert_eq!(None, axis.spacing());
}

#[test]
fn linear_axis_rejects_invalid_bounds_and_preserves_previous_values() {
    let axis = LinearAxis::new();
    axis.set_minimum(Some(0.0)).unwrap();
    axis.set_maximum(Some(10.0)).unwrap();
    for value in [11.0, 10.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(axis.set_minimum(Some(value)).is_err());
        assert_eq!(Some(0.0), axis.minimum());
    }
    for value in [-1.0, 0.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(axis.set_maximum(Some(value)).is_err());
        assert_eq!(Some(10.0), axis.maximum());
    }
}

#[test]
fn linear_axis_rejects_invalid_spacing_and_preserves_previous_value() {
    let axis = LinearAxis::new();
    axis.set_minimum(Some(0.0)).unwrap();
    axis.set_maximum(Some(10.0)).unwrap();
    axis.set_spacing(Some(5.0)).unwrap();
    for value in [-1.0, 0.0, f64::NAN, f64::INFINITY, 11.0] {
        assert!(axis.set_spacing(Some(value)).is_err());
        assert_eq!(Some(5.0), axis.spacing());
    }
}

#[test]
fn linear_axis_null_clears_to_automatic() {
    let axis = LinearAxis::new();
    axis.set_minimum(Some(1.0)).unwrap();
    axis.set_maximum(Some(9.0)).unwrap();
    axis.set_spacing(Some(2.0)).unwrap();
    axis.set_minimum(None).unwrap();
    axis.set_maximum(None).unwrap();
    axis.set_spacing(None).unwrap();
    assert_eq!(
        (None, None, None),
        (axis.minimum(), axis.maximum(), axis.spacing())
    );
}

#[test]
fn category_axis_defaults_are_index_ascending() {
    let axis = CategoryAxis::new();
    assert_eq!(CategorySortKey::Index, axis.sort_key());
    assert_eq!(SortOrder::Ascending, axis.sort_order());
}

#[test]
fn category_axis_sort_properties_round_trip() {
    let axis = CategoryAxis::new();
    axis.set_sort_key(CategorySortKey::Value);
    axis.set_sort_order(SortOrder::Descending);
    assert_eq!(CategorySortKey::Value, axis.sort_key());
    assert_eq!(SortOrder::Descending, axis.sort_order());
}

#[test]
fn category_axis_sort_dependency_properties_update_and_clear_values() {
    let axis = CategoryAxis::new();
    axis.set_value(CategoryAxis::sort_key_property(), CategorySortKey::Value);
    axis.set_value(CategoryAxis::sort_order_property(), SortOrder::Descending);
    assert_eq!(CategorySortKey::Value, axis.sort_key());
    assert_eq!(SortOrder::Descending, axis.sort_order());
    assert_eq!(
        CategorySortKey::Value,
        axis.get_value(CategoryAxis::sort_key_property())
    );
    assert_eq!(
        SortOrder::Descending,
        axis.get_value(CategoryAxis::sort_order_property())
    );
    axis.clear_value(CategoryAxis::sort_key_property());
    axis.clear_value(CategoryAxis::sort_order_property());
    assert_eq!(CategorySortKey::Index, axis.sort_key());
    assert_eq!(SortOrder::Ascending, axis.sort_order());
}

#[test]
fn axis_base_defaults_and_round_trips() {
    let axis = LinearAxis::new();
    assert!(axis.is_visible());
    assert!(axis.label().is_empty());
    axis.set_is_visible(false);
    axis.set_label("Revenue");
    assert!(!axis.is_visible());
    assert_eq!("Revenue", axis.label());
    axis.set_is_visible(true);
    axis.set_optional_label(None);
    assert!(axis.is_visible());
    assert!(axis.label().is_empty());
}

#[test]
fn axis_base_dependency_properties_update_and_clear_values() {
    let axes: [Ref<Axis>; 3] = [
        LinearAxis::new().upcast(),
        CategoryAxis::new().upcast(),
        DateTimeAxis::new().upcast(),
    ];
    for axis in axes {
        axis.set_value(Axis::is_visible_property(), false);
        axis.set_value(Axis::label_property(), "Revenue".to_owned());
        assert!(!axis.is_visible());
        assert_eq!("Revenue", axis.label());
        assert!(!axis.get_value(Axis::is_visible_property()));
        assert_eq!("Revenue", axis.get_value(Axis::label_property()));
        axis.clear_value(Axis::is_visible_property());
        axis.clear_value(Axis::label_property());
        assert!(axis.is_visible());
        assert!(axis.label().is_empty());
    }
}

#[test]
fn cartesian_axis_presentation_defaults_are_expected() {
    let axis = LinearAxis::new();
    assert!(axis.show_tick_labels());
    assert!(!axis.show_tick_marks());
    assert_eq!(GridLines::None, axis.grid_lines());
}

#[test]
fn cartesian_axis_presentation_properties_round_trip() {
    let axis = LinearAxis::new();
    axis.set_show_tick_labels(false);
    axis.set_show_tick_marks(true);
    axis.set_grid_lines(GridLines::Major);
    assert!(!axis.show_tick_labels());
    assert!(axis.show_tick_marks());
    assert_eq!(GridLines::Major, axis.grid_lines());
    axis.set_grid_lines(GridLines::Minor);
    assert_eq!(GridLines::Minor, axis.grid_lines());
}

#[test]
fn cartesian_axis_presentation_dependency_properties_update_and_clear_values() {
    let axis = LinearAxis::new();
    axis.set_value(CartesianAxis::show_tick_labels_property(), false);
    axis.set_value(CartesianAxis::show_tick_marks_property(), true);
    axis.set_value(CartesianAxis::grid_lines_property(), GridLines::Major);
    assert!(!axis.show_tick_labels());
    assert!(axis.show_tick_marks());
    assert_eq!(GridLines::Major, axis.grid_lines());
    assert!(!axis.get_value(CartesianAxis::show_tick_labels_property()));
    assert!(axis.get_value(CartesianAxis::show_tick_marks_property()));
    assert_eq!(
        GridLines::Major,
        axis.get_value(CartesianAxis::grid_lines_property())
    );
    axis.clear_value(CartesianAxis::show_tick_labels_property());
    axis.clear_value(CartesianAxis::show_tick_marks_property());
    axis.clear_value(CartesianAxis::grid_lines_property());
    assert!(axis.show_tick_labels());
    assert!(!axis.show_tick_marks());
    assert_eq!(GridLines::None, axis.grid_lines());
}

#[test]
fn cartesian_axis_brush_defaults_are_null() {
    let axis = LinearAxis::new();
    for property in brush_properties() {
        assert!(axis.get_value(property).is_none());
    }
}

#[test]
fn cartesian_axis_brushes_round_trip_and_clear() {
    let axis = LinearAxis::new();
    let brush: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(Colors::RED));
    axis.set_grid_line_major_brush(Some(brush.clone()));
    axis.set_grid_line_minor_brush(Some(brush.clone()));
    axis.set_tick_brush(Some(brush.clone()));
    axis.set_tick_label_brush(Some(brush.clone()));
    axis.set_axis_line_brush(Some(brush.clone()));
    for value in [
        axis.grid_line_major_brush(),
        axis.grid_line_minor_brush(),
        axis.tick_brush(),
        axis.tick_label_brush(),
        axis.axis_line_brush(),
    ] {
        assert!(Rc::ptr_eq(&brush, &value.unwrap()));
    }
    axis.set_grid_line_major_brush(None);
    axis.set_grid_line_minor_brush(None);
    axis.set_tick_brush(None);
    axis.set_tick_label_brush(None);
    axis.set_axis_line_brush(None);
    for property in brush_properties() {
        assert!(axis.get_value(property).is_none());
    }
}

#[test]
fn cartesian_axis_brush_dependency_properties_update_and_clear_values() {
    let axis = LinearAxis::new();
    let brush: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(Colors::BLUE));
    for property in brush_properties() {
        axis.set_value(property, Some(brush.clone()));
        assert!(Rc::ptr_eq(&brush, &axis.get_value(property).unwrap()));
        axis.clear_value(property);
        assert!(axis.get_value(property).is_none());
    }
}

#[test]
fn date_time_axis_defaults_are_automatic() {
    let axis = DateTimeAxis::new();
    assert_eq!(None, axis.minimum());
    assert_eq!(None, axis.maximum());
    assert_eq!(DateTimeIntervalType::Auto, axis.interval_type());
    assert!(axis.label_format().is_empty());
}

#[test]
fn date_time_axis_interval_type_round_trips_through_property_and_dependency_property() {
    let axis = DateTimeAxis::new();
    for value in [
        DateTimeIntervalType::Auto,
        DateTimeIntervalType::Day,
        DateTimeIntervalType::Week,
        DateTimeIntervalType::Month,
        DateTimeIntervalType::Year,
    ] {
        axis.set_interval_type(value);
        assert_eq!(value, axis.interval_type());
        axis.set_value(DateTimeAxis::interval_type_property(), value);
        assert_eq!(
            value,
            axis.get_value(DateTimeAxis::interval_type_property())
        );
    }
    axis.clear_value(DateTimeAxis::interval_type_property());
    assert_eq!(DateTimeIntervalType::Auto, axis.interval_type());
}

#[test]
fn date_time_axis_label_format_round_trips_through_property_and_dependency_property() {
    let axis = DateTimeAxis::new();
    axis.set_label_format("month year");
    assert_eq!("month year", axis.label_format());
    axis.set_value(
        DateTimeAxis::label_format_property(),
        "shortdate".to_owned(),
    );
    assert_eq!("shortdate", axis.label_format());
    assert_eq!(
        "shortdate",
        axis.get_value(DateTimeAxis::label_format_property())
    );
    axis.clear_value(DateTimeAxis::label_format_property());
    assert!(axis.label_format().is_empty());
}

#[test]
fn date_time_axis_bounds_round_trip_and_null_clear() {
    let axis = DateTimeAxis::new();
    axis.set_minimum(Some(date(2020, 1, 1))).unwrap();
    axis.set_maximum(Some(date(2023, 12, 31))).unwrap();
    assert_eq!(Some(date(2020, 1, 1)), axis.minimum());
    assert_eq!(Some(date(2023, 12, 31)), axis.maximum());
    axis.set_minimum(None).unwrap();
    axis.set_maximum(None).unwrap();
    assert_eq!((None, None), (axis.minimum(), axis.maximum()));
}

#[test]
fn date_time_axis_rejects_invalid_bounds_and_preserves_previous_values() {
    let axis = DateTimeAxis::new();
    axis.set_minimum(Some(date(2020, 1, 1))).unwrap();
    axis.set_maximum(Some(date(2021, 1, 1))).unwrap();
    for value in [date(2022, 1, 1), date(2021, 1, 1)] {
        assert!(axis.set_minimum(Some(value)).is_err());
        assert_eq!(Some(date(2020, 1, 1)), axis.minimum());
    }
    for value in [date(2019, 1, 1), date(2020, 1, 1)] {
        assert!(axis.set_maximum(Some(value)).is_err());
        assert_eq!(Some(date(2021, 1, 1)), axis.maximum());
    }
}

#[test]
fn date_time_axis_accepts_pre_epoch_dates() {
    let axis = DateTimeAxis::new();
    for value in [date(1899, 12, 30), date(1601, 1, 1)] {
        axis.set_minimum(Some(value)).unwrap();
        assert_eq!(Some(value), axis.minimum());
    }
}

#[test]
fn invalid_raw_dependency_property_writes_preserve_numeric_invariants() {
    let axis = LinearAxis::new();
    axis.set_minimum(Some(0.0)).unwrap();
    axis.set_maximum(Some(10.0)).unwrap();
    axis.set_spacing(Some(5.0)).unwrap();
    axis.set_value(LinearAxis::minimum_property(), Some(11.0));
    axis.set_value(LinearAxis::maximum_property(), Some(f64::NAN));
    axis.set_value(LinearAxis::spacing_property(), Some(f64::NEG_INFINITY));
    assert_eq!(
        (Some(0.0), Some(10.0), Some(5.0)),
        (axis.minimum(), axis.maximum(), axis.spacing())
    );
    assert!(axis.set_maximum(Some(4.0)).is_err());
    axis.set_value(LinearAxis::maximum_property(), Some(4.0));
    assert_eq!(Some(10.0), axis.maximum());
}

#[test]
fn invalid_raw_date_bounds_preserve_instants_and_compare_offsets() {
    let axis = DateTimeAxis::new();
    let minimum = date(2020, 1, 1);
    let maximum = date(2021, 1, 1);
    axis.set_minimum(Some(minimum)).unwrap();
    axis.set_maximum(Some(maximum)).unwrap();
    axis.set_value(DateTimeAxis::minimum_property(), Some(maximum));
    axis.set_value(DateTimeAxis::maximum_property(), Some(minimum));
    assert_eq!(
        (Some(minimum), Some(maximum)),
        (axis.minimum(), axis.maximum())
    );
    let same_instant = DateTimeOffset::new(2020, 1, 1, 2, 0, 0, TimeSpan::from_hours(2.0));
    assert!(axis.set_maximum(Some(same_instant)).is_err());
}

#[test]
fn category_value_sort_is_stable_and_never_changes_source_indices() {
    let axis = CategoryAxis::new();
    let values = ["b", "a", "a", "c"];
    axis.set_sort_key(CategorySortKey::Value);
    assert_eq!(
        vec![1, 2, 0, 3],
        axis.sorted_indices_by(values.len(), |a, b| values[a].cmp(values[b]))
    );
    axis.set_sort_order(SortOrder::Descending);
    assert_eq!(
        vec![3, 0, 1, 2],
        axis.sorted_indices_by(values.len(), |a, b| values[a].cmp(values[b]))
    );
    axis.set_sort_key(CategorySortKey::Index);
    assert_eq!(
        vec![3, 2, 1, 0],
        axis.sorted_indices_by(values.len(), |_, _| panic!(
            "Index ordering does not compare values"
        ))
    );
}

#[test]
fn automatic_numeric_ranges_are_finite_for_empty_missing_constant_and_extreme_data() {
    let axis = LinearAxis::new();
    for values in [
        &[][..],
        &[f64::NAN, f64::INFINITY],
        &[0.0, 0.0],
        &[-20.0],
        &[f64::MAX],
        &[-f64::MAX],
        &[-f64::MAX, f64::MAX],
    ] {
        let range = axis.resolve_range(values, 5);
        assert!(
            range.minimum.is_finite() && range.maximum.is_finite() && range.minimum < range.maximum,
            "{range:?}"
        );
        assert!(
            range.spacing.is_finite() && range.spacing > 0.0,
            "{range:?}"
        );
        assert_eq!(0.0, range.normalize(range.minimum));
        assert_eq!(1.0, range.normalize(range.maximum));
    }
}

#[test]
fn numeric_ticks_are_aligned_unique_and_bounded_even_with_tiny_spacing() {
    let range = NumericRange {
        minimum: -2.0,
        maximum: 2.0,
        spacing: 0.5,
    };
    assert_eq!(
        vec![-2.0, -1.5, -1.0, -0.5, 0.0, 0.5, 1.0, 1.5, 2.0],
        range.ticks()
    );
    let tiny = NumericRange {
        minimum: -1.0,
        maximum: 1.0,
        spacing: 1.0e-12,
    }
    .ticks();
    assert!(tiny.len() <= NumericRange::MAX_TICKS);
    assert!(tiny.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(tiny
        .iter()
        .all(|value| value.is_finite() && *value >= -1.0 && *value <= 1.0));
}

#[test]
fn numeric_tick_index_stride_does_not_overflow_across_extreme_bounds() {
    let ticks = NumericRange {
        minimum: -f64::MAX,
        maximum: f64::MAX,
        spacing: 1.0,
    }
    .ticks();
    assert!(ticks.len() > 2);
    assert!(ticks.len() <= NumericRange::MAX_TICKS);
    assert!(ticks.iter().all(|value| value.is_finite()));
    assert!(ticks.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(ticks[0] < 0.0);
    assert!(*ticks.last().unwrap() > 0.0);
}

#[test]
fn calendar_ticks_keep_real_month_boundaries_and_leap_days() {
    let axis = DateTimeAxis::new();
    axis.set_interval_type(DateTimeIntervalType::Month);
    assert_eq!(
        vec![date(2020, 2, 1), date(2020, 3, 1), date(2020, 4, 1)],
        axis.ticks(date(2020, 1, 15), date(2020, 4, 1), 8)
    );
    axis.set_interval_type(DateTimeIntervalType::Day);
    assert_eq!(
        vec![date(2020, 2, 28), date(2020, 2, 29), date(2020, 3, 1)],
        axis.ticks(date(2020, 2, 28), date(2020, 3, 1), 8)
    );
    axis.set_interval_type(DateTimeIntervalType::Year);
    assert_eq!(
        vec![date(1601, 1, 1), date(1602, 1, 1)],
        axis.ticks(date(1600, 6, 1), date(1602, 1, 1), 8)
    );
}

#[test]
fn calendar_ticks_and_auto_ranges_handle_datetime_extrema_without_panics() {
    let axis = DateTimeAxis::new();
    for value in [DateTimeOffset::MIN_VALUE, DateTimeOffset::MAX_VALUE] {
        let (minimum, maximum) = axis.resolve_range(&[value]);
        assert!(minimum < maximum);
        assert!(axis.ticks(minimum, maximum, 5).len() <= 4096);
    }
    axis.set_interval_type(DateTimeIntervalType::Day);
    let ticks = axis.ticks(DateTimeOffset::MIN_VALUE, DateTimeOffset::MAX_VALUE, 10);
    assert!(ticks.len() <= 4096);
    assert!(ticks.windows(2).all(|pair| pair[0] < pair[1]));
}
