//! Not from upstream: the upstream project has no tests. The view models of the properties of a
//! control (name, value, type, priority, group, and the change of a value through the view
//! model), the grouping, sorting and filtering of the property grid (the TreeDataGrid source that
//! replaces upstream's DataGrid), the control details, the layout view model, the pseudo-classes
//! and the thickness editor.

use crate::diagnostics::controls::ThicknessEditor;
use crate::diagnostics::view_models::{
    ClrPropertyViewModel, ControlDetailsViewModel, ControlLayoutViewModel, FerroPropertyViewModel, PropertiesView,
    PropertyViewModelKey, PropertyViewModelRef, PseudoClassViewModel, TreePageViewModel,
};
use ferroui_base::controls::IPseudoClasses;
use ferroui_base::data::model::INotifyPropertyChanged;
use ferroui_base::layout::{HorizontalAlignment, Layoutable};
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::threading::{Dispatcher, UnitTestDispatcherScope};
use ferroui_base::{BoxedValue, Rect, Size, Thickness, Visual};
use ferroui_controls::{Border, Button, Canvas};
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::{Rc, Weak};

fn scope() -> UnitTestDispatcherScope {
    Dispatcher::unit_test_scope()
}

/// The names of the property change notifications of a view model, in order.
fn record(source: &dyn INotifyPropertyChanged) -> Rc<RefCell<Vec<String>>> {
    let raised = Rc::new(RefCell::new(Vec::new()));
    let sink = raised.clone();
    source.property_changed().add(Rc::new(move |name: &str| sink.borrow_mut().push(name.to_string())));
    raised
}

fn unbox<T: Clone + 'static>(value: Option<BoxedValue>) -> Option<T> {
    value.and_then(|value| value.downcast_ref::<T>().cloned())
}

fn ferro_property(border: &Border, property: &'static ferroui_base::FerroProperty) -> PropertyViewModelRef {
    PropertyViewModelRef::Ferro(FerroPropertyViewModel::new(border, property))
}

#[test]
fn a_styled_property_shows_its_name_value_type_and_priority() {
    let _scope = scope();
    let border = Border::new();
    border.set_width(40.0);

    let property = ferro_property(&border, Layoutable::width_property().as_property());

    assert_eq!("Width", property.name());
    assert_eq!(Some(40.0), unbox::<f64>(property.value()));
    assert_eq!("LocalValue", property.priority());
    assert_eq!("Properties", property.group());
    assert_eq!(Some(false), property.is_attached());
    assert!(!property.is_readonly());
    assert!(property.property_type().is::<f64>());
    assert!(property.assigned_type().is::<f64>());
    assert_eq!("Double", property.type_());
    assert_eq!(PropertyViewModelKey::Property(Layoutable::width_property().as_property()), property.key());
    assert!(property.full_name().starts_with("Ferro:"), "{}", property.full_name());
    assert!(property.full_name().ends_with(".Width"), "{}", property.full_name());
}

#[test]
fn an_unset_property_has_the_unset_priority_and_its_default_value() {
    let _scope = scope();
    let border = Border::new();

    let property = ferro_property(&border, Layoutable::horizontal_alignment_property().as_property());

    assert_eq!("Unset", property.priority());
    assert_eq!(Some(HorizontalAlignment::Stretch), unbox::<HorizontalAlignment>(property.value()));
}

#[test]
fn a_direct_property_has_the_direct_priority() {
    let _scope = scope();
    let border = Border::new();

    let property = ferro_property(&border, Visual::bounds_property().as_property());

    assert_eq!("Direct", property.priority());
    assert_eq!("Properties", property.group());
    assert!(property.is_readonly());
}

#[test]
fn an_attached_property_is_named_after_its_owner_and_grouped_apart() {
    let _scope = scope();
    let border = Border::new();
    Canvas::set_left(&border, 5.0);

    let property = ferro_property(&border, Canvas::left_property().as_property());

    assert_eq!("[Canvas.Left]", property.name());
    assert_eq!(Some(true), property.is_attached());
    assert_eq!("Attached Properties", property.group());
    assert_eq!(Some(5.0), unbox::<f64>(property.value()));
}

#[test]
fn setting_the_value_through_the_view_model_sets_the_property_and_notifies() {
    let _scope = scope();
    let border = Border::new();
    let property = ferro_property(&border, Layoutable::width_property().as_property());
    let raised = record(&property);

    property.set_value(Some(Rc::new(25.0)));

    assert_eq!(25.0, border.width());
    assert_eq!(Some(25.0), unbox::<f64>(property.value()));
    assert_eq!("LocalValue", property.priority());
    assert!(raised.borrow().contains(&"Value".to_string()), "{:?}", raised.borrow());
}

#[test]
fn a_value_the_property_does_not_take_is_ignored() {
    let _scope = scope();
    let border = Border::new();
    border.set_width(10.0);
    let property = ferro_property(&border, Layoutable::width_property().as_property());

    property.set_value(Some(Rc::new(Thickness::uniform(1.0))));
    // The width does not take negative values.
    property.set_value(Some(Rc::new(-5.0)));

    assert_eq!(10.0, border.width());
}

#[test]
fn a_change_of_the_control_is_read_by_update() {
    let _scope = scope();
    let border = Border::new();
    let property = ferro_property(&border, Layoutable::width_property().as_property());

    border.set_width(12.0);
    property.update();

    assert_eq!(Some(12.0), unbox::<f64>(property.value()));
}

#[test]
fn pinning_a_property_moves_it_to_the_pinned_group() {
    let _scope = scope();
    let border = Border::new();
    let property = ferro_property(&border, Layoutable::width_property().as_property());
    let raised = record(&property);

    property.set_is_pinned(true);

    assert!(property.is_pinned());
    assert_eq!("Pinned", property.group());
    assert_eq!(vec!["Group".to_string(), "IsPinned".to_string()], *raised.borrow());
}

#[test]
fn a_plain_property_is_read_and_written_through_the_metadata_of_its_type() {
    let _scope = scope();
    let border = Border::new();
    let layout = ControlLayoutViewModel::new(&border);
    let markup = <ControlLayoutViewModel as MarkupTyped>::MARKUP;
    let target: BoxedValue = Rc::new(layout.clone());

    let margin = ClrPropertyViewModel::new(target.clone(), markup.find_property("MarginThickness").unwrap(), markup);
    let width = ClrPropertyViewModel::new(target, markup.find_property("Width").unwrap(), markup);

    assert_eq!("MarginThickness", margin.name());
    assert_eq!("CLR Properties", margin.group());
    assert_eq!("", margin.priority());
    assert_eq!(None, margin.is_attached());
    assert!(!margin.is_readonly());
    assert!(width.is_readonly());
    assert_eq!(PropertyViewModelKey::Name("MarginThickness".to_string()), margin.key());

    margin.set_value(Some(Rc::new(Thickness::uniform(3.0))));

    assert_eq!(Thickness::uniform(3.0), layout.margin_thickness());
    assert_eq!(Thickness::uniform(3.0), border.margin());
    assert_eq!(Some(Thickness::uniform(3.0)), unbox::<Thickness>(margin.value()));
}

/// The properties of a border: two registered ones, an attached one, a pinned one and a plain one.
fn properties_of(border: &Border, layout: &Rc<ControlLayoutViewModel>) -> Vec<PropertyViewModelRef> {
    Canvas::set_left(border, 1.0);
    let markup = <ControlLayoutViewModel as MarkupTyped>::MARKUP;
    let target: BoxedValue = Rc::new(layout.clone());
    let pinned = ferro_property(border, Layoutable::margin_property().as_property());
    pinned.set_is_pinned(true);

    vec![
        ferro_property(border, Layoutable::width_property().as_property()),
        PropertyViewModelRef::Clr(ClrPropertyViewModel::new(target, markup.find_property("HasBorder").unwrap(), markup)),
        ferro_property(border, Canvas::left_property().as_property()),
        ferro_property(border, Layoutable::height_property().as_property()),
        pinned,
    ]
}

fn group_names(view: &PropertiesView) -> Vec<String> {
    view.groups().iter().map(|group| group.name()).collect()
}

fn property_names(view: &PropertiesView) -> Vec<String> {
    view.items().iter().map(|property| property.name()).collect()
}

#[test]
fn the_grid_groups_the_properties_in_the_order_of_the_comparer() {
    let _scope = scope();
    let border = Border::new();
    let layout = ControlLayoutViewModel::new(&border);

    let view = PropertiesView::new(properties_of(&border, &layout), Rc::new(|_: &PropertyViewModelRef| true));

    assert_eq!(vec!["Pinned", "Properties", "Attached Properties", "CLR Properties"], group_names(&view));
    assert_eq!(vec!["Margin", "Height", "Width", "[Canvas.Left]", "HasBorder"], property_names(&view));

    // The groups are the root rows of the source, with their properties below them.
    let groups = view.groups();
    assert!(groups.iter().all(|group| group.is_group() && group.is_expanded()));
    assert_eq!(2, groups[1].item_count());
    assert_eq!(Some("Height".to_string()), groups[1].children()[0].property().map(|property| property.name()));
}

#[test]
fn the_grid_filters_the_properties_and_drops_empty_groups() {
    let _scope = scope();
    let border = Border::new();
    let layout = ControlLayoutViewModel::new(&border);
    let filter_text = Rc::new(RefCell::new(String::new()));
    let filter = filter_text.clone();

    let view = PropertiesView::new(
        properties_of(&border, &layout),
        Rc::new(move |property: &PropertyViewModelRef| property.name().contains(filter.borrow().as_str())),
    );
    *filter_text.borrow_mut() = "eight".to_string();
    view.refresh();

    assert_eq!(vec!["Properties"], group_names(&view));
    assert_eq!(vec!["Height"], property_names(&view));
}

#[test]
fn the_grid_selects_the_row_of_a_property_and_keeps_it_across_a_refresh() {
    let _scope = scope();
    let border = Border::new();
    let layout = ControlLayoutViewModel::new(&border);
    let properties = properties_of(&border, &layout);
    let width = properties[0].clone();
    let selections = Rc::new(RefCell::new(Vec::new()));
    let sink = selections.clone();

    let view = PropertiesView::new(properties, Rc::new(|_: &PropertyViewModelRef| true));
    view.set_selection_changed(Rc::new(move |selected: Option<PropertyViewModelRef>| {
        sink.borrow_mut().push(selected.map(|property| property.name()));
    }));

    view.select(Some(&width));
    assert_eq!(Some(width.clone()), view.selected());

    view.refresh();
    assert_eq!(Some(width.clone()), view.selected());
    assert!(view.row_index_of(&width) > 0);

    view.select(None);
    assert_eq!(None, view.selected());
    // The selections the view model asked for are not reported back to it.
    assert!(selections.borrow().is_empty(), "{:?}", selections.borrow());
}

/// The details of `border` on a tree page without a main view.
fn details_of(border: &Border, pinned: Rc<RefCell<HashSet<String>>>) -> (Rc<TreePageViewModel>, Rc<ControlDetailsViewModel>) {
    let tree_page = TreePageViewModel::with_weak_main_view(Weak::new(), Vec::new(), pinned.clone());
    let details = ControlDetailsViewModel::new(&tree_page, border, pinned);
    (tree_page, details)
}

#[test]
fn the_details_list_the_properties_of_the_control_and_select_one() {
    let _scope = scope();
    // Attached properties register on first access (a static constructor upstream).
    let _ = ferroui_controls::Canvas::left_property();
    let border = Border::new();
    border.set_name(Some("border".to_string()));
    border.set_width(30.0);
    let (_tree_page, details) = details_of(&border, Rc::new(RefCell::new(HashSet::new())));

    assert_eq!(Some("border".to_string()), details.selected_entity_name());
    assert!(details.layout().is_some());
    assert!(details.frames_status().is_some_and(|status| status.starts_with("Value Frames (")));

    let view = details.properties_view().expect("the properties of the control");
    let names = property_names(&view);
    assert!(names.contains(&"Width".to_string()), "{names:?}");
    assert!(names.contains(&"[Canvas.Left]".to_string()), "{names:?}");

    details.select_property(Layoutable::width_property().as_property());

    assert_eq!(Some("Width".to_string()), details.selected_property().map(|property| property.name()));
    assert_eq!(details.selected_property(), view.selected());
}

#[test]
fn the_details_follow_a_change_of_the_control() {
    let _scope = scope();
    let border = Border::new();
    let (_tree_page, details) = details_of(&border, Rc::new(RefCell::new(HashSet::new())));
    details.select_property(Layoutable::width_property().as_property());

    border.set_width(55.0);

    let width = details.selected_property().expect("the width");
    assert_eq!(Some(55.0), unbox::<f64>(width.value()));
}

#[test]
fn toggling_a_pin_remembers_the_property_and_regroups_the_grid() {
    let _scope = scope();
    let border = Border::new();
    let pinned = Rc::new(RefCell::new(HashSet::new()));
    let (_tree_page, details) = details_of(&border, pinned.clone());
    details.select_property(Layoutable::width_property().as_property());
    let width = details.selected_property().expect("the width");

    details.toggle_pinned_property(Some(width.boxed()));

    assert!(width.is_pinned());
    assert!(pinned.borrow().contains(&width.full_name()));
    let view = details.properties_view().unwrap();
    assert_eq!("Pinned", view.groups()[0].name());
    assert_eq!(Some(width.clone()), view.selected());

    details.toggle_pinned_property(Some(width.boxed()));

    assert!(!width.is_pinned());
    assert!(pinned.borrow().is_empty());
    assert_ne!("Pinned", view.groups()[0].name());
}

#[test]
fn the_properties_filter_of_the_tree_page_filters_the_grid() {
    let _scope = scope();
    let border = Border::new();
    let (tree_page, details) = details_of(&border, Rc::new(RefCell::new(HashSet::new())));

    tree_page.properties_filter().set_filter_string("MinWidth".to_string());
    details.properties_view().unwrap().refresh();

    assert_eq!(vec!["MinWidth"], property_names(&details.properties_view().unwrap()));
}

#[test]
fn navigating_into_a_property_and_back() {
    let _scope = scope();
    let border = Border::new();
    border.set_name(Some("border".to_string()));
    border.set_child(Button::new());
    let (_tree_page, details) = details_of(&border, Rc::new(RefCell::new(HashSet::new())));
    details.select_property(ferroui_controls::Decorator::child_property().as_property());

    details.navigate_to_selected_property();

    assert!(details.can_navigate_to_parent_property());
    assert_eq!(Some("border.Child".to_string()), details.selected_entity_name());

    details.navigate_to_parent_property();

    assert!(!details.can_navigate_to_parent_property());
    assert_eq!(Some("border".to_string()), details.selected_entity_name());
}

#[test]
fn the_layout_view_model_reads_and_edits_the_margin_border_and_padding() {
    let _scope = scope();
    let border = Border::new();
    border.set_margin(Thickness::uniform(1.0));
    border.set_border_thickness(Thickness::uniform(2.0));
    border.set_padding(Thickness::new(3.0, 4.0, 5.0, 6.0));

    let layout = ControlLayoutViewModel::new(&border);

    assert!(layout.has_border());
    assert!(layout.has_padding());
    assert_eq!(Thickness::uniform(1.0), layout.margin_thickness());
    assert_eq!(Thickness::uniform(2.0), layout.border_thickness());
    assert_eq!(Thickness::new(3.0, 4.0, 5.0, 6.0), layout.padding_thickness());
    assert_eq!(HorizontalAlignment::Stretch, layout.horizontal_alignment());

    layout.set_margin_thickness(Thickness::uniform(7.0));
    layout.set_padding_thickness(Thickness::uniform(8.0));
    layout.set_border_thickness(Thickness::uniform(9.0));
    layout.set_horizontal_alignment(HorizontalAlignment::Left);

    assert_eq!(Thickness::uniform(7.0), border.margin());
    assert_eq!(Thickness::uniform(8.0), border.padding());
    assert_eq!(Thickness::uniform(9.0), border.border_thickness());
    assert_eq!(HorizontalAlignment::Left, border.horizontal_alignment());
}

#[test]
fn the_layout_view_model_follows_the_size_and_its_constraints() {
    let _scope = scope();
    let border = Border::new();
    let layout = ControlLayoutViewModel::new(&border);
    assert_eq!(None, layout.width_constraint());
    let raised = record(&*layout);

    border.set_use_layout_rounding(false);
    border.set_min_width(10.0);
    let _ = border.property_changed({
        let layout = layout.clone();
        move |e| layout.control_property_changed(e)
    });
    border.set_max_width(20.123);
    border.measure(Size::new(100.0, 100.0));
    border.arrange(Rect::new(0.0, 0.0, 15.554, 12.0));

    assert_eq!(Some("Min: 10\nMax: 20.12".to_string()), layout.width_constraint());
    assert_eq!(15.55, layout.width());
    assert_eq!(12.0, layout.height());
    assert!(raised.borrow().contains(&"Width".to_string()), "{:?}", raised.borrow());
}

#[test]
fn a_change_from_the_control_does_not_write_back() {
    let _scope = scope();
    let border = Border::new();
    let layout = ControlLayoutViewModel::new(&border);
    let _ = border.property_changed({
        let layout = layout.clone();
        move |e| layout.control_property_changed(e)
    });

    border.set_margin(Thickness::uniform(4.0));

    assert_eq!(Thickness::uniform(4.0), layout.margin_thickness());
    // The control keeps its local value (no second write through the view model).
    assert_eq!(Thickness::uniform(4.0), border.margin());
}

#[test]
fn a_pseudo_class_view_model_reads_and_sets_the_pseudo_class() {
    let _scope = scope();
    let button = Button::new();

    let pressed = PseudoClassViewModel::new(":custom", &button);
    assert!(!pressed.is_active());

    pressed.set_is_active(true);
    assert!(button.classes().contains(":custom"));

    button.pseudo_classes().set(":custom", false);
    pressed.update();
    assert!(!pressed.is_active());
    assert!(!button.classes().contains(":custom"));

    let classes: &dyn IPseudoClasses = &button.classes();
    classes.set(":custom", true);
    pressed.update();
    assert!(pressed.is_active());
}

#[test]
fn the_thickness_editor_splits_the_thickness_into_its_sides() {
    let _scope = scope();
    let editor = ThicknessEditor::new();

    editor.set_thickness(Thickness::new(1.0, 2.0, 3.0, 4.0));

    assert_eq!((1.0, 2.0, 3.0, 4.0), (editor.left(), editor.top(), editor.right(), editor.bottom()));
}

#[test]
fn the_thickness_editor_joins_its_sides_into_the_thickness() {
    let _scope = scope();
    let editor = ThicknessEditor::new();
    editor.set_thickness(Thickness::uniform(1.0));

    editor.set_left(5.0);
    editor.set_bottom(6.0);

    assert_eq!(Thickness::new(5.0, 1.0, 1.0, 6.0), editor.thickness());
    assert!(editor.is_present());
}

#[test]
fn the_thickness_editor_takes_a_thickness_parsed_from_text() {
    let _scope = scope();
    let editor = ThicknessEditor::new();

    editor.set_thickness(Thickness::parse("1,2,3,4").unwrap());
    assert_eq!((1.0, 2.0, 3.0, 4.0), (editor.left(), editor.top(), editor.right(), editor.bottom()));

    editor.set_thickness(Thickness::parse("7").unwrap());
    assert_eq!((7.0, 7.0, 7.0, 7.0), (editor.left(), editor.top(), editor.right(), editor.bottom()));

    editor.set_thickness(Thickness::parse("1 2").unwrap());
    assert_eq!(Thickness::new(1.0, 2.0, 1.0, 2.0), editor.thickness());
    assert!(Thickness::parse("1,2,3").is_err());
}
