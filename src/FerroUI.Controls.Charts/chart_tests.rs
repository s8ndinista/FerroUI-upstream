//! Chart collection, property, lifecycle and automation contracts.
use crate::*;
use ferroui_base::collections::{FerroList, NotifyCollectionChangedAction as Action, NotifyCollectionChangedEventArgs};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Colors, FontFamily, IBrush};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{Ref, Size};
use ferroui_controls::automation::peers::{AutomationControlType, ControlAutomationPeer};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Control, ItemsSource, TextBlock, Window};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn line() -> Ref<LineSeries> {
    let series = LineSeries::new();
    series.set_x_values(Some(Samples::from_values(["Jan", "Feb", "Mar"])));
    series.set_y_values(Some(Samples::from_values([1.0, 2.0, 3.0])));
    series
}

fn populated_chart() -> Ref<Chart> {
    let chart = Chart::new();
    chart.set_width(400.0);
    chart.set_height(300.0);
    chart.series().add(line()).unwrap();
    chart
}

fn show(chart: &Chart) -> Ref<Window> {
    let window = Window::new();
    window.set_content(Some(Control::boxed(chart.to_ref())));
    window.show();
    window.update_layout();
    Dispatcher::ui_thread().run_jobs(None);
    window
}

type Event = (Action, i32, i32);

fn record<T>(sink: &Rc<RefCell<Vec<Event>>>, change: &NotifyCollectionChangedEventArgs<'_, T>) {
    sink.borrow_mut().push((change.action, change.new_starting_index, change.old_starting_index));
}

#[test]
fn chart_defaults_expose_empty_collections_and_no_legend() {
    let chart = Chart::new();
    assert!(!chart.show_legend());
    assert!(chart.legend_title().is_empty());
    assert_eq!(0, chart.series().count());
    assert_eq!(0, chart.axes().count());
    assert_eq!(0, chart.data().count());
    assert!(chart.series() == chart.series());
    assert!(chart.axes() == chart.axes());
    assert!(chart.data() == chart.data());
    let other = Chart::new();
    assert!(chart.series() != other.series());
    assert!(chart.axes() != other.axes());
    assert!(chart.data() != other.data());
}

#[test]
fn chart_measure_uses_available_minimums_and_fallback() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = Chart::new();
    chart.measure(Size::new(80.0, 48.0));
    assert_eq!(Size::new(80.0, 48.0), chart.desired_size());
    chart.measure(Size::default());
    assert_eq!(Size::default(), chart.desired_size());
    chart.measure(Size::new(f64::INFINITY, f64::INFINITY));
    assert_eq!(Size::new(160.0, 96.0), chart.desired_size());
    chart.set_min_width(200.0);
    chart.set_min_height(120.0);
    chart.measure(Size::new(f64::INFINITY, f64::INFINITY));
    assert_eq!(Size::new(200.0, 120.0), chart.desired_size());
}

#[test]
fn chart_can_load_unload_and_reload() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = populated_chart();
    let loaded = Rc::new(Cell::new(0));
    let unloaded = Rc::new(Cell::new(0));
    let count = loaded.clone();
    chart.loaded(move |_, _| count.set(count.get() + 1));
    let count = unloaded.clone();
    chart.unloaded(move |_, _| count.set(count.get() + 1));
    let window = show(&chart);
    assert!(chart.is_loaded());
    window.set_content(None);
    Dispatcher::ui_thread().run_jobs(None);
    assert!(!chart.is_loaded());
    window.set_content(Some(Control::boxed(&chart)));
    window.update_layout();
    Dispatcher::ui_thread().run_jobs(None);
    assert!(chart.is_loaded());
    assert_eq!(2, loaded.get());
    assert_eq!(1, unloaded.get());
    assert_eq!(1, chart.series().count());
    window.close();
}

#[test]
fn chart_resize_while_loaded_updates_actual_size() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = populated_chart();
    let window = show(&chart);
    chart.set_width(640.0);
    chart.set_height(360.0);
    chart.update_layout();
    assert!(chart.is_loaded());
    assert_eq!(Size::new(640.0, 360.0), chart.bounds().size());
    window.close();
}

#[test]
fn parent_requested_theme_can_change_while_chart_is_loaded() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = populated_chart();
    let window = show(&chart);
    for theme in [ThemeVariant::dark(), ThemeVariant::light()] {
        window.set_requested_theme_variant(Some(theme.clone()));
        chart.update_layout();
        assert!(chart.is_loaded());
        assert_eq!(Size::new(400.0, 300.0), chart.bounds().size());
        assert_eq!(Some(theme), chart.actual_theme_variant());
    }
    window.close();
}

#[test]
fn legend_properties_round_trip_before_and_after_load() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = populated_chart();
    chart.set_show_legend(true);
    chart.set_legend_title("Quarterly revenue");
    let window = show(&chart);
    assert!(chart.is_loaded());
    assert!(chart.show_legend());
    assert_eq!("Quarterly revenue", chart.legend_title());
    for title in ["Annual revenue", ""] {
        chart.set_legend_title(title);
        chart.update_layout();
        assert_eq!(title, chart.legend_title());
        assert_eq!(Size::new(400.0, 300.0), chart.bounds().size());
    }
    chart.set_show_legend(false);
    chart.set_legend_title("Hidden title");
    chart.update_layout();
    assert!(!chart.show_legend());
    assert_eq!("Hidden title", chart.legend_title());
    window.close();
}

#[test]
fn chart_legend_dependency_properties_update_and_clear_values() {
    let chart = Chart::new();
    chart.set_value(Chart::show_legend_property(), true);
    chart.set_value(Chart::legend_title_property(), "Revenue".to_owned());
    assert!(chart.show_legend());
    assert_eq!("Revenue", chart.legend_title());
    assert!(chart.get_value(Chart::show_legend_property()));
    assert_eq!("Revenue", chart.get_value(Chart::legend_title_property()));
    chart.clear_value(Chart::show_legend_property());
    chart.clear_value(Chart::legend_title_property());
    assert!(!chart.show_legend());
    assert!(chart.legend_title().is_empty());
}

#[test]
fn chart_formatting_properties_round_trip_before_and_after_load() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = populated_chart();
    let red: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(Colors::RED));
    let blue: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(Colors::BLUE));
    chart.set_background(Some(red.clone()));
    chart.set_foreground(Some(blue.clone()));
    chart.set_font_family(FontFamily::new("Consolas"));
    chart.set_font_size(18.0);
    let window = show(&chart);
    assert!(Rc::ptr_eq(&red, &chart.background().unwrap()));
    assert!(Rc::ptr_eq(&blue, &chart.foreground().unwrap()));
    assert_eq!(FontFamily::new("Consolas"), chart.font_family());
    assert_eq!(18.0, chart.font_size());
    chart.set_background(Some(blue.clone()));
    chart.set_foreground(Some(red.clone()));
    chart.set_font_family(FontFamily::new("Segoe UI"));
    chart.set_font_size(20.0);
    chart.update_layout();
    assert!(chart.is_loaded());
    assert!(Rc::ptr_eq(&blue, &chart.background().unwrap()));
    assert!(Rc::ptr_eq(&red, &chart.foreground().unwrap()));
    assert_eq!(FontFamily::new("Segoe UI"), chart.font_family());
    assert_eq!(20.0, chart.font_size());
    assert_eq!(Size::new(400.0, 300.0), chart.bounds().size());
    window.close();
}

#[test]
fn line_series_without_explicit_axes_keeps_axis_properties_null_after_load() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = populated_chart();
    let window = show(&chart);
    let series = chart.series().get(0).unwrap();
    let _layout = chart.layout_snapshot(Size::new(400.0, 300.0));
    assert!(chart.is_loaded());
    assert!(series.x_axis().is_none());
    assert!(series.y_axis().is_none());
    assert_eq!(0, chart.axes().count());
    window.close();
}

#[test]
fn series_collection_supports_indexing_copy_and_enumeration() {
    let chart = Chart::new();
    let first: Ref<CartesianSeries> = line().upcast();
    let second: Ref<CartesianSeries> = AreaSeries::new().upcast();
    chart.series().add(first.clone()).unwrap();
    chart.series().add(second.clone()).unwrap();
    assert_eq!(2, chart.series().count());
    assert!(chart.series().get(0).unwrap() == first);
    assert!(chart.series().get(1).unwrap() == second);
    assert!(chart.series().contains(first.clone()));
    assert_eq!(Some(1), chart.series().index_of(second.clone()));
    let copied = chart.series().snapshot().as_ref().clone();
    assert!(copied == vec![first.clone(), second.clone()]);
    let enumerated: Vec<_> = chart.series().snapshot().iter().cloned().collect();
    assert!(enumerated == vec![first, second]);
}

#[test]
fn series_collection_mutation_preserves_existing_snapshot() {
    // The Rust adaptation uses FerroList snapshots instead of fail-fast CLR iterators.
    let chart = Chart::new();
    let first: Ref<CartesianSeries> = line().upcast();
    chart.series().add(first.clone()).unwrap();
    let snapshot = chart.series().snapshot();
    chart.series().add(AreaSeries::new()).unwrap();
    assert_eq!(1, snapshot.len());
    assert!(snapshot[0] == first);
    assert_eq!(2, chart.series().count());
}

#[test]
fn series_collection_raises_exact_vector_events() {
    let chart = Chart::new();
    let events = Rc::new(RefCell::new(Vec::new()));
    let sink = events.clone();
    chart.series().add_collection_changed(Rc::new(move |change| record(&sink, change)));
    let first = line();
    let second = AreaSeries::new();
    let third = BarSeries::new();
    chart.series().add(first).unwrap();
    chart.series().insert(0, second).unwrap();
    chart.series().set(1, third.clone()).unwrap();
    chart.series().set(1, third).unwrap();
    chart.series().remove_at(0).unwrap();
    chart.series().clear();
    chart.series().clear();
    assert_eq!(vec![(Action::Add, 0, -1), (Action::Add, 0, -1), (Action::Replace, 1, 1), (Action::Remove, -1, 0), (Action::Reset, -1, -1)], *events.borrow());
}

#[test]
fn series_collection_rejects_out_of_range_and_null() {
    // Ref<T> excludes null statically; runtime index errors remain fallible.
    let chart = Chart::new();
    assert!(chart.series().get(0).is_err());
    assert!(chart.series().set(0, line()).is_err());
    assert!(chart.series().insert(1, line()).is_err());
    assert!(chart.series().remove_at(0).is_err());
    assert_eq!(0, chart.series().count());
}

#[test]
fn series_can_be_added_again_after_removal() {
    let first = Chart::new();
    let second = Chart::new();
    let series = line();
    first.series().add(series.clone()).unwrap();
    first.series().remove_at(0).unwrap();
    second.series().add(series.clone()).unwrap();
    assert_eq!(0, first.series().count());
    assert_eq!(1, second.series().count());
    assert!(second.series().get(0).unwrap() == series);
}

#[test]
fn failed_series_mutations_do_not_change_collection_or_raise_events() {
    let chart = Chart::new();
    let other = Chart::new();
    let original = line();
    chart.series().add(original.clone()).unwrap();
    let events = Rc::new(Cell::new(0));
    let count = events.clone();
    chart.series().add_collection_changed(Rc::new(move |_| count.set(count.get() + 1)));
    assert!(chart.series().add(original.clone()).is_err());
    assert!(chart.series().insert(0, original.clone()).is_err());
    assert!(other.series().add(original.clone()).is_err());
    chart.series().set(0, original.clone()).unwrap();
    assert_eq!(0, events.get());
    assert_eq!(1, chart.series().count());
    assert_eq!(0, other.series().count());
    assert!(chart.series().get(0).unwrap() == original);
}

#[test]
fn axis_collection_supports_add_remove_and_vector_events() {
    let chart = Chart::new();
    let events = Rc::new(RefCell::new(Vec::new()));
    let sink = events.clone();
    chart.axes().add_collection_changed(Rc::new(move |change| record(&sink, change)));
    let first = LinearAxis::new();
    let second = CategoryAxis::new();
    chart.axes().add(first.clone()).unwrap();
    chart.axes().insert(0, second.clone()).unwrap();
    assert_eq!(2, chart.axes().count());
    assert!(chart.axes().get(0).unwrap() == second);
    assert!(chart.axes().get(1).unwrap() == first);
    chart.axes().remove_at(1).unwrap();
    chart.axes().clear();
    assert_eq!(0, chart.axes().count());
    assert_eq!(vec![(Action::Add, 0, -1), (Action::Add, 0, -1), (Action::Remove, -1, 1), (Action::Reset, -1, -1)], *events.borrow());
}

#[test]
fn axis_collection_rejects_duplicate_cross_chart_null_and_out_of_range() {
    let first = Chart::new();
    let second = Chart::new();
    let axis = LinearAxis::new();
    first.axes().add(axis.clone()).unwrap();
    assert!(first.axes().add(axis.clone()).is_err());
    assert!(second.axes().add(axis).is_err());
    // A nullable axis cannot be passed to this typed collection.
    assert!(first.axes().set(1, LinearAxis::new()).is_err());
    assert!(first.axes().insert(3, LinearAxis::new()).is_err());
    assert!(second.axes().remove_at(0).is_err());
    assert_eq!(1, first.axes().count());
    assert_eq!(0, second.axes().count());
}

#[test]
fn explicit_axes_can_be_assigned_to_series_in_same_chart() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = Chart::new();
    let x: Ref<Axis> = CategoryAxis::new().upcast();
    let y: Ref<Axis> = LinearAxis::new().upcast();
    chart.axes().add(x.clone()).unwrap();
    chart.axes().add(y.clone()).unwrap();
    let series = line();
    series.set_x_axis(Some(x.clone())).unwrap();
    series.set_y_axis(Some(y.clone())).unwrap();
    chart.series().add(series.clone()).unwrap();
    let window = show(&chart);
    assert!(series.x_axis() == Some(x));
    assert!(series.y_axis() == Some(y));
    assert_eq!(2, chart.axes().count());
    window.close();
}

#[test]
fn series_rejects_axis_not_present_in_same_chart() {
    let chart = Chart::new();
    let series = line();
    series.set_y_axis(Some(LinearAxis::new().upcast())).unwrap();
    assert!(chart.series().add(series).is_err());
    assert_eq!(0, chart.series().count());
}

#[test]
fn series_rejects_axis_from_another_chart() {
    let first = Chart::new();
    let second = Chart::new();
    let axis: Ref<Axis> = LinearAxis::new().upcast();
    first.axes().add(axis.clone()).unwrap();
    let series = line();
    second.series().add(series.clone()).unwrap();
    assert!(series.set_y_axis(Some(axis)).is_err());
    assert!(series.y_axis().is_none());
}

#[test]
fn data_collection_supports_basic_list_and_vector_semantics() {
    let chart = Chart::new();
    let events = Rc::new(RefCell::new(Vec::new()));
    let sink = events.clone();
    chart.data().add_collection_changed(Rc::new(move |change| record(&sink, change)));
    let first = Samples::from_values([1.0, 2.0]);
    let second = Samples::from_values([3.0, 4.0]);
    let replacement = Samples::from_values([5.0, 6.0]);
    chart.data().add(first.clone()).unwrap();
    chart.data().insert(0, second.clone()).unwrap();
    assert!(chart.data().get(0).unwrap() == second);
    assert!(chart.data().get(1).unwrap() == first);
    chart.data().set(1, replacement.clone()).unwrap();
    assert!(chart.data().get(1).unwrap() == replacement);
    chart.data().remove_at(0).unwrap();
    chart.data().clear();
    assert_eq!(vec![(Action::Add, 0, -1), (Action::Add, 0, -1), (Action::Replace, 1, 1), (Action::Remove, -1, 0), (Action::Reset, -1, -1)], *events.borrow());
}

#[test]
fn labeled_by_supplies_automation_name() {
    let chart = Chart::new();
    let label = TextBlock::new();
    label.set_text(Some("CPU history"));
    AutomationProperties::set_labeled_by(&chart, label.clone());
    let peer = ControlAutomationPeer::create_peer_for_element(&chart);
    assert_eq!("CPU history", peer.get_name());
}

#[test]
fn explicit_automation_name_wins_over_labeled_by() {
    for name_first in [true, false] {
        let chart = Chart::new();
        let label = TextBlock::new();
        label.set_text(Some("CPU history"));
        if name_first {
            AutomationProperties::set_name(&chart, Some("Processor utilization"));
            AutomationProperties::set_labeled_by(&chart, label.clone());
        } else {
            AutomationProperties::set_labeled_by(&chart, label.clone());
            AutomationProperties::set_name(&chart, Some("Processor utilization"));
        }
        let peer = ControlAutomationPeer::create_peer_for_element(&chart);
        assert_eq!("Processor utilization", peer.get_name());
    }
}

#[test]
fn app_full_description_wins() {
    // WinRT FullDescription is represented by the framework's existing HelpText.
    let chart = Chart::new();
    AutomationProperties::set_name(&chart, Some("CPU history"));
    AutomationProperties::set_help_text(&chart, Some("Processor utilization over the last minute."));
    let peer = ControlAutomationPeer::create_peer_for_element(&chart);
    assert_eq!("Processor utilization over the last minute.", peer.get_help_text());
}

#[test]
fn peer_queries_do_not_lay_out_the_chart() {
    let chart = populated_chart();
    AutomationProperties::set_name(&chart, Some("Quarterly revenue"));
    let peer = ControlAutomationPeer::create_peer_for_element(&chart);
    assert_eq!("Quarterly revenue", peer.get_name());
    assert_eq!(AutomationControlType::Custom, peer.get_automation_control_type());
    assert_eq!("Chart", peer.get_class_name());
    let _localized = peer.get_localized_control_type();
    assert_eq!(0.0, chart.bounds().width);
    assert_eq!(0.0, chart.desired_size().width);
    assert_eq!(0, chart.scene_build_count());
}

#[test]
fn dropping_chart_releases_series_and_axes_even_when_collections_are_retained() {
    let series = line();
    let axis = LinearAxis::new();
    let chart = Chart::new();
    chart.series().add(series.clone()).unwrap();
    chart.axes().add(axis.clone()).unwrap();
    let retained = chart.series();
    let weak = chart.downgrade();
    drop(chart);
    assert!(weak.upgrade().is_none());
    assert_eq!(1, retained.count());
    let replacement = Chart::new();
    replacement.series().add(series).unwrap();
    replacement.axes().add(axis).unwrap();
}

#[test]
fn collection_notifications_commit_ownership_before_reentrant_edits() {
    let first = Chart::new();
    let second = Chart::new();
    let series = line();
    let other = second.clone();
    let moved = series.clone();
    first.series().add_collection_changed(Rc::new(move |event| {
        if event.action == Action::Remove {
            other.series().add(moved.clone()).unwrap();
        }
    }));
    first.series().add(series.clone()).unwrap();
    first.series().remove_at(0).unwrap();
    assert!(first.series().is_empty());
    assert!(second.series().get(0).unwrap() == series);
}

#[test]
fn shared_samples_invalidate_every_chart_and_old_sources_disconnect() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let values = Rc::new(FerroList::from_items([1.0, 2.0, 3.0]));
    let samples = Samples::from_source(ItemsSource::new(values.clone()));
    let first = Chart::new();
    let second = Chart::new();
    for chart in [&first, &second] {
        let series = line();
        series.set_y_values(Some(samples.clone()));
        chart.series().add(series).unwrap();
    }
    let first_revision = first.revision();
    let second_revision = second.revision();
    values.set(1, 9.0);
    assert!(first.revision() > first_revision);
    assert!(second.revision() > second_revision);
    samples.set_source(Some(ItemsSource::from_values([4.0, 5.0, 6.0])));
    let first_revision = first.revision();
    let second_revision = second.revision();
    values.set(1, 20.0);
    assert_eq!(first_revision, first.revision());
    assert_eq!(second_revision, second.revision());
}
