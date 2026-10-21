use crate::{catalog, examples, navigation::NavigationHistory, GalleryView, CATALOG};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Button, CheckBox, ComboBox, Slider, TextBlock, TextBox};
use std::collections::HashSet;

fn start() -> ferroui_controls::testing::UnitTestApplicationScope {
    ferroui_markup_xaml::register_types();
    UnitTestApplication::start(TestServices::styled_window())
}

#[test]
fn complete_pinned_catalog_has_unique_ids_and_all_source_samples() {
    assert_eq!(CATALOG.len(), 126);
    assert_eq!(
        CATALOG.iter().map(|page| page.samples.len()).sum::<usize>(),
        350
    );
    let mut ids = HashSet::new();
    for entry in CATALOG {
        assert!(ids.insert(entry.id));
        assert!(!entry.source.is_empty());
        assert!(!entry.description.is_empty());
        assert!(entry.source_url().contains(crate::SOURCE_COMMIT));
        for sample in entry.samples {
            assert_eq!(sample.source_sha256.len(), 64);
            assert!(!sample.source.is_empty());
        }
    }
}

#[test]
fn search_uses_words_categories_and_stable_ids() {
    assert_eq!(
        catalog::search("CALENDARDATEPICKER", None)[0].id,
        "CalendarDatePicker"
    );
    let category = catalog::search("", Some("Date & time"));
    assert_eq!(category.len(), 4);
    assert!(category.iter().all(|entry| entry.category == "Date & time"));
    assert!(catalog::search("a term absent from this complete catalog", None).is_empty());
    assert!(catalog::search("button basic", None)
        .iter()
        .all(|entry| entry.matches("button basic")));
}

#[test]
fn navigation_rejects_unknown_pages_and_keeps_valid_forward_history() {
    let mut history = NavigationHistory::default();
    assert!(!history.navigate("NoSuchPage"));
    assert!(history.navigate("Button"));
    assert!(history.navigate("Slider"));
    assert!(history.go_back());
    assert_eq!(history.current(), Some("Button"));
    assert!(!history.navigate("BUTTON"));
    assert!(history.can_go_forward());
    assert!(!history.navigate("NoSuchPage"));
    assert!(history.can_go_forward());
    assert!(history.go_forward());
    assert_eq!(history.current(), Some("Slider"));
    assert!(history.go_back());
    assert!(history.navigate("TextBox"));
    assert!(!history.can_go_forward());
}

#[test]
fn all_reviewed_native_adaptations_load_real_controls() {
    let _application = start();
    let mut errors = Vec::new();
    for adaptation in examples::ADAPTATIONS {
        assert!(catalog::find(adaptation.page_id).is_some());
        if let Err(error) = examples::create(adaptation) {
            errors.push(format!("{}: {error}", adaptation.page_id));
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

#[test]
fn button_click_and_disabled_option_update_native_controls() {
    let _application = start();
    let root = examples::create(examples::find("Button").unwrap()).unwrap();
    let primary = root.get_control::<Button>("Primary");
    primary.raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
    primary.raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
    assert_eq!(
        root.get_control::<TextBlock>("Output").text().as_deref(),
        Some("Clicked 2 times")
    );
    root.get_control::<CheckBox>("Disable")
        .set_is_checked(Some(true));
    assert!(!primary.is_enabled());
    root.get_control::<CheckBox>("Disable")
        .set_is_checked(Some(false));
    assert!(primary.is_enabled());
}

#[test]
fn three_state_checkbox_preserves_null_and_reports_each_state() {
    let _application = start();
    let root = examples::create(examples::find("CheckBox").unwrap()).unwrap();
    let primary = root.get_control::<ToggleButton>("Primary");
    let output = root.get_control::<TextBlock>("Output");
    primary.set_is_checked(Some(true));
    assert_eq!(output.text().as_deref(), Some("Checked"));
    primary.set_is_checked(None);
    assert_eq!(output.text().as_deref(), Some("Indeterminate"));
    primary.set_is_checked(Some(false));
    assert_eq!(output.text().as_deref(), Some("Unchecked"));
}

#[test]
fn textbox_counts_unicode_scalar_values_without_revealing_passwords() {
    let _application = start();
    for page in ["TextBox", "PasswordBox"] {
        let root = examples::create(examples::find(page).unwrap()).unwrap();
        root.get_control::<TextBox>("Primary")
            .set_text(Some("A🦀é"));
        assert_eq!(
            root.get_control::<TextBlock>("Output").text().as_deref(),
            Some("3 characters")
        );
    }
}

#[test]
fn slider_and_combobox_outputs_observe_shared_properties() {
    let _application = start();
    let root = examples::create(examples::find("Slider").unwrap()).unwrap();
    root.get_control::<Slider>("Primary").set_range_value(900.0);
    assert_eq!(
        root.get_control::<TextBlock>("Output").text().as_deref(),
        Some("Value: 900")
    );
    let root = examples::create(examples::find("ComboBox").unwrap()).unwrap();
    root.get_control::<ComboBox>("Primary")
        .set_selected_index(2);
    assert_eq!(
        root.get_control::<TextBlock>("Output").text().as_deref(),
        Some("Selected index: 2")
    );
}

#[test]
fn native_gallery_filters_navigates_and_retains_history() {
    let _application = start();
    let gallery = GalleryView::new().unwrap();
    assert_eq!(gallery.visible_page_count(), 126);
    assert_eq!(gallery.current_page(), Some("Button"));
    gallery.set_search("CalendarDatePicker");
    assert_eq!(gallery.visible_page_count(), 1);
    assert!(gallery.navigate("CalendarDatePicker"));
    assert_eq!(gallery.current_page(), Some("CalendarDatePicker"));
    assert!(gallery.go_back());
    assert_eq!(gallery.current_page(), Some("Button"));
    assert!(gallery.go_forward());
    assert_eq!(gallery.current_page(), Some("CalendarDatePicker"));
    assert!(!gallery.navigate("NoSuchPage"));
    gallery.set_search("");
    assert_eq!(gallery.visible_page_count(), 126);
}

#[test]
fn every_catalog_page_opens_in_the_native_shell() {
    let _application = start();
    let gallery = GalleryView::new().unwrap();
    for entry in CATALOG {
        gallery.navigate(entry.id);
        assert_eq!(gallery.current_page(), Some(entry.id));
    }
}

#[test]
fn source_preview_preserves_unported_bindings_and_substitution_tokens() {
    let button = catalog::find("Button").unwrap();
    let sample = &button.samples[0];
    assert!(sample.xaml.contains("$(IsEnabled)"));
    let wrapped = crate::gallery::wrap_winui_sample(sample);
    assert!(wrapped.contains("$(IsEnabled)"));
    assert!(wrapped.contains("Click=\"Button_Click\""));
    assert!(wrapped.contains("http://schemas.microsoft.com/winfx/2006/xaml/presentation"));
}

#[test]
fn winui_preview_uses_opt_in_public_loader_and_rejects_unported_bindings() {
    let _application = start();
    let root = crate::gallery::load_winui_preview("<TextBlock xmlns=\"http://schemas.microsoft.com/winfx/2006/xaml/presentation\" Text=\"Preview\"/>").unwrap();
    assert_eq!(
        root.cast::<TextBlock>().unwrap().text().as_deref(),
        Some("Preview")
    );
    assert!(crate::gallery::load_winui_preview("<TextBlock xmlns=\"http://schemas.microsoft.com/winfx/2006/xaml/presentation\" xmlns:x=\"http://schemas.microsoft.com/winfx/2006/xaml\" Text=\"{x:Bind Text}\"/>").is_err());
}

#[test]
fn winui_preview_reports_missing_resource_without_unwinding() {
    let _application = start();
    assert!(crate::gallery::load_winui_preview(r#"<StackPanel xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><StackPanel.Resources><ControlTheme x:Key="Example" TargetType="Button" BasedOn="{StaticResource {x:Type Button}}"/></StackPanel.Resources><Button Theme="{StaticResource Example}"/></StackPanel>"#).is_err());
}

#[test]
fn contrast_calculation_matches_original_endpoints_symmetry_and_thresholds() {
    use crate::contrast::{contrast_ratio, parse_rgb, relative_luminance, ContrastResult};
    let black = parse_rgb("#000000").unwrap();
    let white = parse_rgb("FFFFFF").unwrap();
    assert_eq!(relative_luminance(black), 0.0);
    assert_eq!(relative_luminance(white), 1.0);
    assert_eq!(contrast_ratio(black, white), 21.0);
    assert_eq!(contrast_ratio(white, black), 21.0);
    assert_eq!(contrast_ratio(white, white), 1.0);
    let marginal = ContrastResult::calculate(parse_rgb("#777777").unwrap(), white);
    assert!((marginal.ratio - 4.47809).abs() < 0.0001);
    assert!(!marginal.normal_text);
    assert!(marginal.large_text);
    assert!(marginal.components);
    for invalid in ["#fff", "#FFFFFFFF", "FFGG00", "", "🦀0000"] {
        assert!(parse_rgb(invalid).is_none());
    }
}

#[test]
fn contrast_inputs_update_ratio_and_report_invalid_values() {
    let _application = start();
    let root = examples::create(examples::find("AccessibilityColorContrast").unwrap()).unwrap();
    let ratio = root.get_control::<TextBlock>("ContrastRatio");
    assert_eq!(ratio.text().as_deref(), Some("21.00:1"));
    let input = root.get_control::<TextBox>("TextColor");
    input.set_text(Some("#777777"));
    assert_eq!(ratio.text().as_deref(), Some("4.48:1"));
    assert!(root
        .get_control::<TextBlock>("ContrastChecks")
        .text()
        .unwrap()
        .starts_with("Regular text (4.5:1): Fail"));
    input.set_text(Some("invalid"));
    assert_eq!(
        ratio.text().as_deref(),
        Some("Enter two six-digit RGB colors.")
    );
    input.set_text(Some("#FFFFFF"));
    assert_eq!(ratio.text().as_deref(), Some("1.00:1"));
}

#[test]
fn winui_numberbox_example_keeps_expression_nan_and_valuechanged_behavior() {
    use ferroui_controls::winui::NumberBox;
    let _application = start();
    let root = examples::create(examples::find("NumberBox").unwrap()).unwrap();
    let expression = root.get_control::<NumberBox>("Expression");
    assert!(expression.value().is_nan());
    expression.set_text("1 + 2^2");
    assert!(expression.commit_input());
    assert_eq!(expression.value(), 5.0);
    expression.set_text("");
    assert!(expression.commit_input());
    assert!(expression.value().is_nan());
    root.get_control::<NumberBox>("Number")
        .set_numeric_value(31.0);
    assert_eq!(
        root.get_control::<TextBlock>("Output").text().as_deref(),
        Some("Value: 31")
    );
}

#[test]
fn page_transition_sample_uses_winui_frame_journal_and_source_page_types() {
    use crate::frame_sample::{FirstSamplePage, SecondSamplePage};
    use ferroui_controls::winui::Frame;
    let _application = start();
    let root = examples::create(examples::find("PageTransition").unwrap()).unwrap();
    let frame = root.get_control::<Frame>("ContentFrame");
    assert_eq!(
        frame.current_source_page_type(),
        Some(FirstSamplePage::TYPE)
    );
    assert!(!frame.can_go_back());
    root.get_control::<Button>("NavigationNext")
        .raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
    assert_eq!(
        frame.current_source_page_type(),
        Some(SecondSamplePage::TYPE)
    );
    assert_eq!(frame.back_stack_depth(), 1);
    root.get_control::<Button>("NavigationBack")
        .raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
    assert_eq!(
        frame.current_source_page_type(),
        Some(FirstSamplePage::TYPE)
    );
    assert!(frame.can_go_forward());
    root.get_control::<Button>("NavigationForward")
        .raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
    assert_eq!(
        frame.current_source_page_type(),
        Some(SecondSamplePage::TYPE)
    );
    assert!(root
        .get_control::<TextBlock>("NavigationStatus")
        .text()
        .unwrap()
        .starts_with("Second page · Forward"));
}

#[test]
fn command_bar_examples_keep_click_and_three_state_handlers() {
    let _application = start();
    for page in ["AppBarButton", "CommandBar"] {
        let root = examples::create(examples::find(page).unwrap()).unwrap();
        root.get_control::<Button>("Primary")
            .raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
        assert_eq!(
            root.get_control::<TextBlock>("Output").text().as_deref(),
            Some("Clicked 1 times")
        );
    }
    let root = examples::create(examples::find("AppBarToggleButton").unwrap()).unwrap();
    root.get_control::<ToggleButton>("Primary")
        .set_is_checked(None);
    assert_eq!(
        root.get_control::<TextBlock>("Output").text().as_deref(),
        Some("Indeterminate")
    );
}

#[test]
fn radio_groups_port_original_selection_and_color_handlers() {
    use ferroui_base::media::Color;
    use ferroui_controls::winui::RadioButtons;
    use ferroui_controls::Border;
    let _application = start();
    let root = examples::create(examples::find("RadioButton").unwrap()).unwrap();
    root.get_control::<ToggleButton>("Option2")
        .set_is_checked(Some(true));
    assert_eq!(
        root.get_control::<TextBlock>("Control1Output")
            .text()
            .as_deref(),
        Some("You selected Option 2")
    );
    let background = root.get_control::<RadioButtons>("BackgroundRadioButtons");
    let border = root.get_control::<RadioButtons>("BorderRadioButtons");
    assert_eq!(background.max_columns(), 3);
    background.set_selected_index(0);
    border.set_selected_index(0);
    let output = root.get_control::<Border>("ControlOutput");
    assert_eq!(
        output
            .background()
            .unwrap()
            .as_solid_color_brush()
            .unwrap()
            .color(),
        Color::from_rgb(0, 128, 0)
    );
    assert_eq!(
        output
            .border_brush()
            .unwrap()
            .as_solid_color_brush()
            .unwrap()
            .color(),
        Color::from_rgb(0, 100, 0)
    );
    background.set_selected_index(1);
    border.set_selected_index(1);
    assert_eq!(
        output
            .background()
            .unwrap()
            .as_solid_color_brush()
            .unwrap()
            .color(),
        Color::from_rgb(255, 255, 0)
    );
    assert_eq!(
        output
            .border_brush()
            .unwrap()
            .as_solid_color_brush()
            .unwrap()
            .color(),
        Color::from_rgb(255, 215, 0)
    );
}

#[test]
fn icon_examples_preserve_source_symbol_and_font_glyph() {
    use ferroui_controls::winui::{FontIcon, Symbol, SymbolIcon};
    let _application = start();
    let root = examples::create(examples::find("IconElement").unwrap()).unwrap();
    assert_eq!(
        root.get_control::<SymbolIcon>("AcceptIcon").symbol(),
        Symbol::Accept
    );
    assert_eq!(
        root.get_control::<FontIcon>("FontGlyph").glyph(),
        "\u{e790}"
    );
}

#[test]
fn dropping_native_example_releases_handler_targets() {
    let _application = start();
    let root = examples::create(examples::find("Button").unwrap()).unwrap();
    let primary = root.get_control::<Button>("Primary").downgrade();
    let output = root.get_control::<TextBlock>("Output").downgrade();
    drop(root);
    assert!(primary.upgrade().is_none());
    assert!(output.upgrade().is_none());
}
