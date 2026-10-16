//! Every themed control gets its template from the theme: one test per
//! control, as the tests of the Fluent theme
//! (`src/FerroUI.Themes.Fluent/tests/control_templates.rs`). The control is
//! created in a `Window` of an application whose theme is the WinUI theme;
//! after the window is shown
//!
//! * the template of the control theme (or of the theme it is based on) has
//!   been applied,
//! * the template parts the class asks for are in the name scope of the
//!   template, with the declared type,
//! * the setters of the control theme and of its bases took effect: the
//!   brushes it sets from resources (`Background`, `BorderBrush`,
//!   `Foreground`) are resolved, and `Padding`, `BorderThickness`,
//!   `MinWidth`, `MinHeight` and `CornerRadius` have the values of their
//!   setters (unless a nested style sets the property again).
//!
//! A batch that ports a control adds its test here and removes the control
//! from `PENDING_CONTROLS` (tests/support.rs).

use super::support::*;
use ferroui_base::controls::{NameScopeRef, ResourceKey};
use ferroui_base::layout::Layoutable;
use ferroui_base::media::IBrush;
use ferroui_base::metadata::{from_markup_value, MarkupAttributeValue};
use ferroui_base::styling::{ControlTheme, Setter, SetterValue, ThemeVariant};
use ferroui_base::{CornerRadius, FerroObject, Ref, Thickness, TypeInfo};
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::{Application, Border, Control, Window};
use std::cell::RefCell;
use std::rc::Rc;

/// The class of the controls or the dialogs assembly named `name`.
fn control_type(name: &str) -> &'static TypeInfo {
    let mut found = TypeInfo::registered_types().into_iter().filter(|type_| {
        type_.name() == name
            && (type_.module_path().starts_with("ferroui_controls") || type_.module_path().starts_with("ferroui_dialogs"))
    });
    let type_ = found.next().unwrap_or_else(|| panic!("{name} is not a class of the controls or the dialogs assembly"));
    assert!(found.next().is_none(), "{name} is ambiguous");
    type_
}

/// The control theme the application has for `type_`.
fn control_theme(type_: &'static TypeInfo) -> Ref<ControlTheme> {
    let application = Application::current().expect("an application");
    let theme = application.try_get_resource(&ResourceKey::Type(type_), None).flatten();
    from_markup_value::<Ref<ControlTheme>>(&theme).unwrap_or_else(|| panic!("the theme has no control theme for {}", type_.name()))
}

/// The parts of `type_` the theme must define.
fn expected_parts(type_: &'static TypeInfo) -> Vec<(&'static str, Option<&'static str>)> {
    requested_parts(type_)
        .into_iter()
        .map(|part| {
            let Some(MarkupAttributeValue::Str(name)) = part.arguments.first() else { panic!("a part without a name") };
            let type_name = match part.arguments.get(1) {
                Some(MarkupAttributeValue::Type(part_type)) => Some(part_type().name()),
                _ => None,
            };
            (*name, type_name)
        })
        .collect()
}

/// The setters of a control theme and of the themes it is based on, the theme's own first: the
/// first setter of a property is the one in effect.
fn setters_of_chain(theme: &Ref<ControlTheme>) -> Vec<(&'static str, SetterValue)> {
    let mut setters = Vec::new();
    let mut current = Some(theme.clone());
    while let Some(theme) = current {
        for setter in theme.setters().snapshot().iter() {
            let Some(setter) = setter.as_any().and_then(|any| any.downcast_ref::<Setter>()) else { continue };
            let (Some(property), Some(value)) = (setter.property(), setter.value()) else { continue };
            if !setters.iter().any(|(name, _): &(&'static str, SetterValue)| *name == property.name()) {
                setters.push((property.name(), value));
            }
        }
        current = theme.based_on();
    }
    setters
}

/// The properties the nested styles of a control theme and of its bases set.
fn set_by_nested_styles(theme: &Ref<ControlTheme>) -> Vec<&'static str> {
    let mut names = Vec::new();
    let mut current = Some(theme.clone());
    while let Some(theme) = current {
        names.extend(
            enumerate_styles(&theme.clone().into())
                .iter()
                .skip(1)
                .flat_map(|style| style.setters().snapshot().iter().cloned().collect::<Vec<_>>())
                .filter_map(|setter| setter.as_any()?.downcast_ref::<Setter>()?.property().map(|property| property.name())),
        );
        current = theme.based_on();
    }
    names
}

fn check_control(name: &str) {
    let _app = start_themed_application();
    let type_ = control_type(name);
    let theme = control_theme(type_);

    let instance = type_.create_instance().unwrap_or_else(|| panic!("{name} has no default constructor"));
    let control = instance.cast::<Control>().unwrap_or_else(|| panic!("{name} is not a control"));
    let templated = control.cast::<TemplatedControl>();

    let name_scope: Rc<RefCell<Option<NameScopeRef>>> = Rc::new(RefCell::new(None));
    if let Some(templated) = &templated {
        let name_scope = name_scope.clone();
        let _ = templated.template_applied(move |_, e| *name_scope.borrow_mut() = Some(e.name_scope().clone()));
    }

    let window = Window::new();
    window.set_content(Some(Control::boxed(&control)));
    window.show();

    // The control theme of the theme is the one the control uses.
    assert!(theme.target_type().is_some_and(|target| std::ptr::eq(target, type_)));

    let setters = setters_of_chain(&theme);
    let setter = |property: &str| setters.iter().find(|(name, _)| *name == property).map(|(_, value)| value.clone());

    // The template of the theme has been applied (for ScrollBar, the template of the nested style
    // of its orientation).
    if setter("Template").is_some() || name == "ScrollBar" {
        let templated = templated.as_ref().unwrap_or_else(|| panic!("{name} has a template but is not templated"));
        assert!(templated.template().is_some(), "{name}: no template");
        assert_eq!(1, control.visual_children_count(), "{name}: the template was not applied");
        assert!(name_scope.borrow().is_some(), "{name}: TemplateApplied was not raised");
    }

    // The template parts.
    for (part, part_type) in expected_parts(type_) {
        let name_scope = name_scope.borrow();
        let name_scope = name_scope.as_ref().unwrap_or_else(|| panic!("{name}: no template name scope"));
        let found = name_scope.find(part).unwrap_or_else(|| panic!("{name}: the part {part} is not in the template"));
        if let Some(part_type) = part_type {
            assert!(
                is_instance_of(&found, part_type),
                "{name}: the part {part} is a {}, not a {part_type}",
                found.get_type().name()
            );
        }
    }

    // The setters of the theme.
    let Some(templated) = templated else { return };
    let brushes: [(&str, Option<Rc<dyn IBrush>>); 3] = [
        ("Background", templated.background()),
        ("BorderBrush", templated.border_brush()),
        ("Foreground", templated.foreground()),
    ];
    for (property, brush) in brushes {
        if matches!(setter(property), Some(SetterValue::BindingBase(_))) {
            assert!(brush.is_some(), "{name}: {property} is set by the theme from a resource but is not resolved");
        }
    }
    let nested = set_by_nested_styles(&theme);
    let value_of = |property: &str| match setter(property) {
        Some(SetterValue::Value(value)) if !nested.contains(&property) => Some(Some(value)),
        _ => None,
    };
    if let Some(expected) = value_of("Padding").and_then(|value| from_markup_value::<Thickness>(&value)) {
        assert_eq!(expected, templated.padding(), "{name}: Padding");
    }
    if let Some(expected) = value_of("BorderThickness").and_then(|value| from_markup_value::<Thickness>(&value)) {
        assert_eq!(expected, templated.border_thickness(), "{name}: BorderThickness");
    }
    if let Some(expected) = value_of("CornerRadius").and_then(|value| from_markup_value::<CornerRadius>(&value)) {
        assert_eq!(expected, templated.corner_radius(), "{name}: CornerRadius");
    }
    let layoutable: &Layoutable = &templated;
    if let Some(expected) = value_of("MinWidth").and_then(|value| from_markup_value::<f64>(&value)) {
        assert_eq!(expected, layoutable.min_width(), "{name}: MinWidth");
    }
    if let Some(expected) = value_of("MinHeight").and_then(|value| from_markup_value::<f64>(&value)) {
        assert_eq!(expected, layoutable.min_height(), "{name}: MinHeight");
    }
}

macro_rules! control_tests {
    ($($test:ident => $name:literal;)*) => {
        $(
            #[test]
            fn $test() {
                check_control($name);
            }
        )*

        /// The controls with a test.
        const TESTED: &[&str] = &[$($name),*];
    };
}

control_tests! {
    button => "Button";
    check_box => "CheckBox";
    hyperlink_button => "HyperlinkButton";
    radio_button => "RadioButton";
    repeat_button => "RepeatButton";
    scroll_bar => "ScrollBar";
    scroll_viewer => "ScrollViewer";
    text_box => "TextBox";
    theme_variant_scope => "ThemeVariantScope";
    thumb => "Thumb";
    toggle_button => "ToggleButton";
    // B1: primitives and simple content
    data_validation_errors => "DataValidationErrors";
    grid_splitter => "GridSplitter";
    group_box => "GroupBox";
    headered_content_control => "HeaderedContentControl";
    items_control => "ItemsControl";
    label => "Label";
    path_icon => "PathIcon";
    progress_bar => "ProgressBar";
    selectable_text_block => "SelectableTextBlock";
    separator => "Separator";
    text_selection_handle => "TextSelectionHandle";
    tool_tip => "ToolTip";
    transitioning_content_control => "TransitioningContentControl";
}

/// The window itself is themed: its template is applied when it is shown
/// and its brushes come from the theme.
#[test]
fn window() {
    let _app = start_themed_application();
    let name_scope: Rc<RefCell<Option<NameScopeRef>>> = Rc::new(RefCell::new(None));
    let window = Window::new();
    let _ = window.template_applied({
        let name_scope = name_scope.clone();
        move |_, e| *name_scope.borrow_mut() = Some(e.name_scope().clone())
    });
    let content = Border::new();
    window.set_content(Some(Control::boxed(&content)));
    window.show();

    assert!(window.template().is_some());
    assert_eq!(1, window.visual_children_count());
    assert!(window.background().is_some());
    assert!(window.foreground().is_some());
    let name_scope = name_scope.borrow();
    let name_scope = name_scope.as_ref().expect("TemplateApplied was raised");
    for (part, _) in expected_parts(Window::TYPE) {
        assert!(name_scope.find(part).is_some(), "Window: the part {part} is not in the template");
    }
    let presenter = name_scope.find("PART_ContentPresenter").expect("the content presenter");
    assert!(content.get_visual_ancestors().any(|ancestor| ancestor.upcast::<FerroObject>().ptr_eq(&presenter)));
}

/// The window follows the theme variants: the background of a window without its backdrop
/// (SolidBackgroundFillColorBaseBrush) differs in the light, the dark and the high contrast
/// variant.
#[test]
fn window_follows_the_theme_variants() {
    let _app = start_themed_application();
    let window = Window::new();
    window.set_content(Some(Control::boxed(&Border::new())));
    window.show();

    let background = || window.background().and_then(|brush| brush.as_solid_color_brush().map(|solid| solid.color()));
    window.set_requested_theme_variant(Some(ThemeVariant::light()));
    let light = background().expect("a solid background");
    window.set_requested_theme_variant(Some(ThemeVariant::dark()));
    let dark = background().expect("a solid background");
    window.set_requested_theme_variant(Some(crate::WinUITheme::high_contrast()));
    let high_contrast = background().expect("a solid background");
    assert_ne!(light, dark);
    assert_ne!(dark, high_contrast);
}

/// Every control with a default control theme in the theme (a type-keyed
/// control theme whose target can be created and put in a window) has a
/// test above.
#[test]
fn every_themed_control_has_a_test() {
    let _app = start_application();
    let theme = create_attached_theme();
    // Top levels, layers and window decorations are created by the framework, not put in a window
    // as content.
    let not_content =
        ["Window", "EmbeddableControlRoot", "PopupRoot", "OverlayPopupHost", "AdornerLayer", "WindowDrawnDecorations"];
    let mut themed: Vec<&str> = enumerate_resources(&theme.as_style())
        .into_iter()
        .filter(|entry| entry.control_theme().is_some())
        .filter_map(|entry| entry.type_key().map(|type_| type_.name()))
        .filter(|name| !not_content.contains(name))
        .collect();
    themed.sort_unstable();
    for name in &themed {
        assert!(TESTED.contains(name), "{name} has a control theme but no test");
    }
    for name in TESTED {
        assert!(themed.contains(name), "{name} has a test but no control theme");
    }
}
