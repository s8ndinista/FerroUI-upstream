use super::*;
use crate::automation::peers::ControlAutomationPeer;
use crate::automation::provider::{
    IExpandCollapseProvider, IRangeValueProvider, ISelectionItemProvider, IValueProvider,
};
use crate::test_support::test_scope;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, IBrush};
use ferroui_base::Point;

#[test]
fn localized_names_update_from_resources_without_overwriting_local_values() {
    use crate::automation::AutomationProperties;
    use ferroui_base::controls::ResourceKey;
    use ferroui_base::utilities::CultureInfo;
    let _scope = test_scope();
    struct RestoreCulture(CultureInfo);
    impl Drop for RestoreCulture {
        fn drop(&mut self) {
            CultureInfo::set_current_ui_culture(self.0.clone());
        }
    }
    let _culture = RestoreCulture(CultureInfo::current_ui_culture());
    CultureInfo::set_current_ui_culture(CultureInfo::get_culture_info("pl-PL"));
    let pen = InkToolbarBallpointPenButton::new();
    assert_eq!(
        AutomationProperties::get_name(&pen).as_deref(),
        Some("Pióro kulkowe")
    );
    pen.resources()
        .add_value("InkToolbarBallpointPenButtonName", "Drawing pen".to_owned());
    assert_eq!(
        AutomationProperties::get_name(&pen).as_deref(),
        Some("Drawing pen")
    );
    pen.resources().set(
        "InkToolbarBallpointPenButtonName",
        Some(Rc::new("Updated pen".to_owned())),
    );
    assert_eq!(
        AutomationProperties::get_name(&pen).as_deref(),
        Some("Updated pen")
    );
    AutomationProperties::set_name(&pen, Some("Application name"));
    pen.resources()
        .remove(&ResourceKey::from("InkToolbarBallpointPenButtonName"));
    assert_eq!(
        AutomationProperties::get_name(&pen).as_deref(),
        Some("Application name")
    );
}

#[test]
fn color_names_quantize_known_families_and_format_unknown_rgb() {
    use super::color_names::{color_name, color_resource_key};
    let _scope = test_scope();
    let control = Control::new();
    control
        .resources()
        .add_value("InkToolbarColorNameRed", "Localized red".to_owned());
    assert_eq!(
        color_resource_key(Color::from_rgb(0xe0, 0x18, 0x1f)),
        Some("InkToolbarColorNameRed")
    );
    assert_eq!(
        color_name(&control, Color::from_rgb(0xe0, 0x18, 0x1f)),
        "Localized red"
    );
    control.resources().add_value(
        "InkToolbarColorRgbFormat",
        "red=%1!u!; green=%2!u!; blue=%3!u!".to_owned(),
    );
    assert_eq!(
        color_name(&control, Color::from_rgb(17, 49, 123)),
        "red=17; green=49; blue=123"
    );
}

#[test]
fn high_contrast_adjustment_obeys_original_system_and_seven_to_one_policy() {
    use super::color_names::{adjust_high_contrast_color, contrast_ratio};
    let black = Color::from_rgb(0, 0, 0);
    let white = Color::from_rgb(255, 255, 255);
    let gray = Color::from_rgb(128, 128, 128);
    assert!((contrast_ratio(black, white) - 21.0).abs() < 1e-12);
    assert!((contrast_ratio(gray, gray) - 1.0).abs() < 1e-12);
    assert_eq!(
        adjust_high_contrast_color(
            gray,
            white,
            black,
            InkHighContrastAdjustment::UseOriginalColors
        ),
        gray
    );
    assert_eq!(
        adjust_high_contrast_color(
            gray,
            white,
            black,
            InkHighContrastAdjustment::UseSystemColors
        ),
        black
    );
    assert_eq!(
        adjust_high_contrast_color(
            gray,
            white,
            black,
            InkHighContrastAdjustment::UseSystemColorsWhenNecessary
        ),
        black
    );
    assert_eq!(
        adjust_high_contrast_color(
            black,
            white,
            gray,
            InkHighContrastAdjustment::UseSystemColorsWhenNecessary
        ),
        black
    );
}

#[test]
fn contrast_palette_switch_restores_normal_selection_and_tracks_presenter_policy() {
    use crate::ThemeVariantScope;
    use ferroui_base::styling::ThemeVariant;
    let _scope = test_scope();
    let scope = ThemeVariantScope::new();
    let toolbar = InkToolbar::new();
    let presenter = InkPresenter::new();
    toolbar.set_target_ink_presenter(Some(presenter.clone()));
    scope.set_child(&toolbar);
    let pen = toolbar
        .active_tool()
        .unwrap()
        .cast::<InkToolbarPenButton>()
        .unwrap();
    pen.set_selected_brush_index(7);
    let normal_color = pen.create_ink_drawing_attributes().unwrap().color;
    scope
        .resources()
        .add_value("SystemColorWindowColor", Color::from_rgb(0, 0, 0));
    scope
        .resources()
        .add_value("SystemColorWindowTextColor", Color::from_rgb(255, 255, 255));
    scope.set_requested_theme_variant(Some(ThemeVariant::new(
        "HighContrast",
        Some(ThemeVariant::dark()),
    )));
    assert!(pen.effective_palette().len() < pen.palette().count());
    assert!(pen
        .effective_palette()
        .iter()
        .all(|brush| super::color_names::contrast_ratio(
            brush.as_solid_color_brush().unwrap().color(),
            Color::from_rgb(0, 0, 0)
        ) >= 7.0));
    presenter.set_high_contrast_adjustment(InkHighContrastAdjustment::UseSystemColors);
    assert_eq!(pen.effective_palette().len(), 1);
    presenter.set_high_contrast_adjustment(InkHighContrastAdjustment::UseOriginalColors);
    assert_eq!(pen.effective_palette().len(), 30);
    scope.set_requested_theme_variant(Some(ThemeVariant::light()));
    assert_eq!(pen.effective_palette().len(), 30);
    assert_eq!(pen.selected_brush_index(), 7);
    assert_eq!(
        pen.create_ink_drawing_attributes().unwrap().color,
        normal_color
    );
}

#[test]
fn pen_preview_uses_source_path_and_follows_attributes() {
    use ferroui_base::media::PolylineGeometry;
    let _scope = test_scope();
    let pen = InkToolbarBallpointPenButton::new();
    let config = InkToolbarPenConfigurationControl::for_pen(&pen);
    let preview = config.stroke_preview().unwrap();
    let path = preview.data().unwrap().cast::<PolylineGeometry>().unwrap();
    assert_eq!(path.points().len(), 103);
    pen.set_selected_stroke_width(9.0);
    pen.set_selected_brush_index(7);
    assert_eq!(preview.stroke_thickness(), 9.0);
    assert_eq!(
        preview
            .stroke()
            .unwrap()
            .as_solid_color_brush()
            .unwrap()
            .color(),
        pen.create_ink_drawing_attributes().unwrap().color
    );
    assert_eq!(preview.opacity(), 1.0);
    let highlighter = InkToolbarHighlighterButton::new();
    let highlighter_config = InkToolbarPenConfigurationControl::for_pen(&highlighter);
    let preview = highlighter_config.stroke_preview().unwrap();
    assert_eq!(preview.opacity(), 0.5);
    assert_eq!(preview.stroke_thickness(), 16.0 / 3.0);
}

#[test]
fn toolbar_style_key_tracks_available_theme_and_returns_to_native_fallback() {
    use crate::RadioButton;
    use ferroui_base::controls::ResourceKey;
    use ferroui_base::styling::ControlTheme;
    let _scope = test_scope();
    let pen = InkToolbarBallpointPenButton::new();
    assert!(std::ptr::eq(pen.style_key(), RadioButton::TYPE));
    let inherited = ControlTheme::with_target_type(InkToolbarPenButton::TYPE);
    pen.resources()
        .add_value(ResourceKey::Type(InkToolbarPenButton::TYPE), inherited);
    assert!(std::ptr::eq(pen.style_key(), InkToolbarPenButton::TYPE));
    let specific = ControlTheme::with_target_type(InkToolbarBallpointPenButton::TYPE);
    pen.resources().add_value(
        ResourceKey::Type(InkToolbarBallpointPenButton::TYPE),
        specific,
    );
    assert!(std::ptr::eq(
        pen.style_key(),
        InkToolbarBallpointPenButton::TYPE
    ));
    pen.resources()
        .remove(&ResourceKey::Type(InkToolbarBallpointPenButton::TYPE));
    pen.resources()
        .remove(&ResourceKey::Type(InkToolbarPenButton::TYPE));
    assert!(std::ptr::eq(pen.style_key(), RadioButton::TYPE));
}

#[test]
fn default_controls_and_selection_match_upstream() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    assert_eq!(toolbar.children().count(), 5);
    assert_eq!(
        toolbar.active_tool().unwrap().tool_kind(),
        InkToolbarTool::BallpointPen
    );
    assert_eq!(toolbar.orientation(), Orientation::Horizontal);
    assert!(toolbar
        .get_menu_button(InkToolbarMenuKind::Stencil)
        .is_some());
}
#[test]
fn changing_initial_controls_preserves_application_children() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    let custom = InkToolbarCustomToolButton::new();
    toolbar.children().add(&custom);
    toolbar.set_initial_controls(InkToolbarInitialControls::PensOnly);
    assert_eq!(toolbar.children().count(), 4);
    toolbar.set_initial_controls(InkToolbarInitialControls::None);
    assert_eq!(toolbar.children().count(), 1);
    assert_eq!(toolbar.children().get(0), custom.upcast::<Control>());
}
#[test]
fn presenter_target_wins_then_falls_back_to_canvas() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    let canvas = InkCanvas::new();
    let explicit = InkPresenter::new();
    toolbar.set_target_ink_canvas(Some(canvas.clone()));
    toolbar.set_target_ink_presenter(Some(explicit.clone()));
    assert!(Rc::ptr_eq(
        &toolbar.effective_ink_presenter().unwrap(),
        &explicit
    ));
    let highlighter = toolbar
        .get_tool_button(InkToolbarTool::Highlighter)
        .unwrap();
    toolbar.set_active_tool(Some(highlighter));
    assert!(
        explicit
            .copy_default_drawing_attributes()
            .draw_as_highlighter
    );
    assert!(
        !canvas
            .ink_presenter()
            .copy_default_drawing_attributes()
            .draw_as_highlighter
    );
    toolbar.set_target_ink_presenter(None);
    assert!(Rc::ptr_eq(
        &toolbar.effective_ink_presenter().unwrap(),
        &canvas.ink_presenter()
    ));
    assert!(
        canvas
            .ink_presenter()
            .copy_default_drawing_attributes()
            .draw_as_highlighter
    );
}
#[test]
fn highlighter_uses_exact_palette_and_chisel_nib() {
    let _scope = test_scope();
    let highlighter = InkToolbarHighlighterButton::new();
    let attrs = highlighter.create_ink_drawing_attributes().unwrap();
    assert_eq!(highlighter.palette().count(), 6);
    assert_eq!(attrs.color, Color::from_uint32(0xffffe600));
    assert_eq!(attrs.size, ferroui_base::Size::new(16.0 / 3.0, 16.0));
    assert_eq!(attrs.pen_tip, PenTipShape::Rectangle);
    assert!(attrs.draw_as_highlighter);
}
#[test]
fn pen_palettes_and_widths_are_independent() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    let a = toolbar
        .get_tool_button(InkToolbarTool::BallpointPen)
        .unwrap()
        .cast::<InkToolbarPenButton>()
        .unwrap();
    let b = toolbar
        .get_tool_button(InkToolbarTool::Pencil)
        .unwrap()
        .cast::<InkToolbarPenButton>()
        .unwrap();
    a.set_selected_stroke_width(8.0);
    a.palette().clear();
    assert_eq!(b.palette().count(), 30);
    assert_eq!(b.selected_stroke_width(), 2.0);
}
#[test]
fn editing_palette_updates_active_presenter_without_reselecting() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    let presenter = InkPresenter::new();
    toolbar.set_target_ink_presenter(Some(presenter.clone()));
    let pen = toolbar
        .active_tool()
        .unwrap()
        .cast::<InkToolbarPenButton>()
        .unwrap();
    let red = Color::from_uint32(0xffe61b1b);
    pen.palette().set(
        0,
        Rc::new(ImmutableSolidColorBrush::new(red)) as Rc<dyn IBrush>,
    );
    assert_eq!(presenter.copy_default_drawing_attributes().color, red);
}
#[test]
fn invalid_widths_are_coerced_to_valid_bounds() {
    let _scope = test_scope();
    let pen = InkToolbarBallpointPenButton::new();
    pen.set_selected_stroke_width(f64::NAN);
    assert!(pen.selected_stroke_width().is_finite());
    pen.set_min_stroke_width(5.0);
    pen.set_max_stroke_width(10.0);
    pen.set_selected_stroke_width(100.0);
    assert_eq!(pen.selected_stroke_width(), 10.0);
    pen.set_selected_stroke_width(-1.0);
    assert_eq!(pen.selected_stroke_width(), 5.0);
}
#[test]
fn tool_selection_changes_processing_mode() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    let presenter = InkPresenter::new();
    toolbar.set_target_ink_presenter(Some(presenter.clone()));
    toolbar.set_active_tool(toolbar.get_tool_button(InkToolbarTool::Eraser));
    assert_eq!(
        presenter.input_processing_configuration().mode(),
        InkInputProcessingMode::Erasing
    );
    let custom = InkToolbarCustomToolButton::new();
    toolbar.children().add(&custom);
    toolbar.set_active_tool(Some(custom.upcast()));
    assert_eq!(
        presenter.input_processing_configuration().mode(),
        InkInputProcessingMode::None
    );
    toolbar.set_active_tool(toolbar.get_tool_button(InkToolbarTool::BallpointPen));
    assert_eq!(
        presenter.input_processing_configuration().mode(),
        InkInputProcessingMode::Inking
    );
}
#[test]
fn erase_all_event_precedes_clear_and_restores_previous_tool() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    let presenter = InkPresenter::new();
    toolbar.set_target_ink_presenter(Some(presenter.clone()));
    presenter
        .stroke_container()
        .add_stroke(
            InkStrokeBuilder::new()
                .create_stroke([Point::new(2.0, 3.0)])
                .unwrap(),
        )
        .unwrap();
    toolbar.set_active_tool(toolbar.get_tool_button(InkToolbarTool::Pencil));
    toolbar.set_active_tool(toolbar.get_tool_button(InkToolbarTool::Eraser));
    let seen = Rc::new(Cell::new(0));
    let seen_copy = seen.clone();
    toolbar.erase_all_clicked(move |toolbar| {
        seen_copy.set(
            toolbar
                .effective_ink_presenter()
                .unwrap()
                .stroke_container()
                .len(),
        )
    });
    toolbar.erase_all();
    assert_eq!(seen.get(), 1);
    assert!(presenter.stroke_container().is_empty());
    assert_eq!(
        toolbar.active_tool().unwrap().tool_kind(),
        InkToolbarTool::Pencil
    );
}
#[test]
fn stencil_switch_is_exclusive_and_target_change_hides_old_stencil() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    let a = InkPresenter::new();
    let b = InkPresenter::new();
    toolbar.set_target_ink_presenter(Some(a.clone()));
    toolbar.set_is_stencil_button_checked(true);
    assert!(a.ruler_visible());
    let stencil = toolbar
        .get_menu_button(InkToolbarMenuKind::Stencil)
        .unwrap()
        .cast::<InkToolbarStencilButton>()
        .unwrap();
    stencil.set_selected_stencil(InkToolbarStencilKind::Protractor);
    assert!(!a.ruler_visible());
    assert!(a.protractor_visible());
    toolbar.set_target_ink_presenter(Some(b.clone()));
    assert!(!a.protractor_visible());
    assert!(b.protractor_visible());
    assert!(Rc::ptr_eq(&stencil.protractor().unwrap(), &b.protractor()));
}
#[test]
fn configuration_slider_and_palette_stay_synchronized() {
    let _scope = test_scope();
    let pen = InkToolbarBallpointPenButton::new();
    let config = InkToolbarPenConfigurationControl::for_pen(&pen);
    let slider = config.stroke_width_slider().unwrap();
    slider.set_range_value(7.0);
    assert_eq!(pen.selected_stroke_width(), 7.0);
    pen.set_selected_stroke_width(11.0);
    assert_eq!(slider.value(), 11.0);
    pen.set_selected_brush_index(7);
    let palette = config
        .children()
        .snapshot()
        .iter()
        .find_map(|child| child.cast::<crate::WrapPanel>())
        .unwrap();
    assert_eq!(
        palette
            .children()
            .get(7)
            .cast::<InkToolbarFlyoutItem>()
            .unwrap()
            .is_checked(),
        Some(true)
    );
}
#[test]
fn default_pen_configuration_is_created_lazily() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    let pen = toolbar.active_tool().unwrap();
    let flyout = pen.flyout().unwrap().cast::<Flyout>().unwrap();
    assert!(flyout.content().is_none());
}
#[test]
fn removed_tool_no_longer_updates_presenter() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    let presenter = InkPresenter::new();
    toolbar.set_target_ink_presenter(Some(presenter.clone()));
    let pen = toolbar
        .active_tool()
        .unwrap()
        .cast::<InkToolbarPenButton>()
        .unwrap();
    toolbar.children().remove(&pen);
    let previous = presenter.copy_default_drawing_attributes();
    pen.set_selected_stroke_width(15.0);
    assert_eq!(presenter.copy_default_drawing_attributes(), previous);
}
#[test]
fn event_subscriptions_can_be_disposed_and_handlers_reenter() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    let count = Rc::new(Cell::new(0));
    let copy = count.clone();
    let subscription = toolbar.active_tool_changed(move |toolbar| {
        let _ = toolbar.children().snapshot();
        copy.set(copy.get() + 1);
    });
    toolbar.set_active_tool(toolbar.get_tool_button(InkToolbarTool::Pencil));
    assert_eq!(count.get(), 1);
    subscription.dispose();
    toolbar.set_active_tool(toolbar.get_tool_button(InkToolbarTool::Highlighter));
    assert_eq!(count.get(), 1);
}
#[test]
fn named_automation_peers_expose_expected_patterns() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    let tool = toolbar.active_tool().unwrap();
    let peer = ControlAutomationPeer::create_peer_for_element(&tool);
    assert!(peer.get_provider::<dyn ISelectionItemProvider>().is_some());
    assert!(peer.get_provider::<dyn IExpandCollapseProvider>().is_some());
    let slider = InkToolbarStrokeWidthSlider::new();
    let peer = ControlAutomationPeer::create_peer_for_element(&slider);
    assert!(peer.get_provider::<dyn IRangeValueProvider>().is_some());
    assert!(peer.get_provider::<dyn IValueProvider>().is_some());
    assert_eq!(
        ControlAutomationPeer::create_peer_for_element(&toolbar).get_automation_control_type(),
        AutomationControlType::ToolBar
    );
}
#[test]
fn slider_value_automation_validates_text_and_disabled_state() {
    let _scope = test_scope();
    let slider = InkToolbarStrokeWidthSlider::new();
    slider.set_minimum(1.0);
    slider.set_maximum(20.0);
    let peer = ControlAutomationPeer::create_peer_for_element(&slider);
    let provider = peer.get_provider::<dyn IValueProvider>().unwrap();
    provider.set_value(Some("8.5")).unwrap();
    assert_eq!(slider.value(), 8.5);
    assert!(provider.set_value(Some("NaN")).is_err());
    provider.set_value(Some("40")).unwrap();
    assert_eq!(slider.value(), 20.0);
    slider.set_is_enabled(false);
    assert!(provider.set_value(Some("2")).is_err());
}

#[test]
fn invalid_palette_index_preserves_previous_selection() {
    let _scope = test_scope();
    let pen = InkToolbarBallpointPenButton::new();
    let previous = pen.selected_brush();
    pen.set_selected_brush_index(-1);
    assert!(pen.selected_brush() == previous);
    pen.set_selected_brush_index(999);
    assert!(pen.selected_brush() == previous);
}
#[test]
fn readonly_contract_properties_are_registered_readonly() {
    assert!(InkToolbar::children_property().is_read_only());
    assert!(InkToolbar::ink_drawing_attributes_property().is_read_only());
    assert!(InkToolbarPenButton::selected_brush_property().is_read_only());
    assert!(InkToolbarStencilButton::ruler_property().is_read_only());
    assert!(InkToolbarStencilButton::protractor_property().is_read_only());
    assert!(InkToolbarPenConfigurationControl::pen_button_property().is_read_only());
}
#[test]
fn target_changes_do_not_raise_active_tool_changed() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    let count = Rc::new(Cell::new(0));
    let copy = count.clone();
    toolbar.active_tool_changed(move |_| copy.set(copy.get() + 1));
    toolbar.set_target_ink_presenter(Some(InkPresenter::new()));
    toolbar.set_target_ink_presenter(Some(InkPresenter::new()));
    assert_eq!(count.get(), 0);
}
#[test]
fn eraser_retains_last_pen_drawing_attributes() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    toolbar.set_active_tool(toolbar.get_tool_button(InkToolbarTool::Highlighter));
    let attrs = toolbar.ink_drawing_attributes();
    toolbar.set_active_tool(toolbar.get_tool_button(InkToolbarTool::Eraser));
    assert_eq!(toolbar.ink_drawing_attributes(), attrs);
}

#[repr(C)]
struct CustomNibPen {
    base: InkToolbarCustomPen,
}
ferroui_base::ferro_class!(CustomNibPen:InkToolbarCustomPen);
ferroui_base::ferro_impl_classes!(CustomNibPen:FerroObjectImpl);
impl InkToolbarCustomPenImpl for CustomNibPen {
    fn create_ink_drawing_attributes_core(
        _this: &Self,
        _brush: Rc<dyn IBrush>,
        stroke_width: f64,
    ) -> Result<InkDrawingAttributes, InkError> {
        Ok(InkDrawingAttributes {
            color: Color::from_uint32(0xff123456),
            size: ferroui_base::Size::new(stroke_width, stroke_width * 2.0),
            pen_tip: PenTipShape::Rectangle,
            ..InkDrawingAttributes::default()
        })
    }
}
#[test]
fn custom_pen_factory_is_dispatched_virtually_by_toolbar() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    let presenter = InkPresenter::new();
    toolbar.set_target_ink_presenter(Some(presenter.clone()));
    let factory = ferroui_base::instantiate(CustomNibPen {
        base: InkToolbarCustomPen::construct(),
    });
    let button = InkToolbarCustomPenButton::new();
    button.set_custom_pen(Some(factory.upcast()));
    button.set_selected_stroke_width(5.0);
    toolbar.children().add(&button);
    toolbar.set_active_tool(Some(button.upcast()));
    let attrs = presenter.copy_default_drawing_attributes();
    assert_eq!(attrs.color, Color::from_uint32(0xff123456));
    assert_eq!(attrs.size, ferroui_base::Size::new(5.0, 10.0));
}
