//! Acceptance of the original page handlers, separate from control contract tests.

use crate::{examples, tests::start};
use ferroui_base::controls::ResourceKey;
use ferroui_base::media::{Brushes, Color, IBrush};
use ferroui_base::styling::ControlTheme;
use ferroui_base::Ref;
use ferroui_controls::winui::{FontIconSource, InfoBadge, NumberBox, ProgressRing, RatingControl};
use ferroui_controls::{CheckBox, ComboBox, Control, Slider, TextBlock, ToggleSwitch};

fn page(id: &str) -> Ref<Control> {
    examples::create(examples::find(id).unwrap()).unwrap()
}

#[test]
fn progress_ring_page_preserves_both_source_modes_and_active_option() {
    let _application = start();
    let root = page("ProgressRing");
    let spinner = root.get_control::<ProgressRing>("ProgressRing1");
    let determinate = root.get_control::<ProgressRing>("ProgressRing2");
    assert!(spinner.is_active() && spinner.is_indeterminate());
    assert!(!determinate.is_indeterminate());
    let toggle = root.get_control::<ToggleSwitch>("ProgressToggle");
    toggle.set_is_checked(Some(false));
    assert!(!spinner.is_active());
    toggle.set_is_checked(Some(true));
    assert!(spinner.is_active());
    for (option, ring) in [
        ("BackgroundComboBox1", spinner),
        ("BackgroundComboBox2", determinate),
    ] {
        root.get_control::<ComboBox>(option).set_selected_index(1);
        assert_eq!(
            ring.background()
                .unwrap()
                .as_solid_color_brush()
                .unwrap()
                .color(),
            Color::from_rgb(211, 211, 211)
        );
        root.get_control::<ComboBox>(option).set_selected_index(0);
        assert_eq!(
            ring.background()
                .unwrap()
                .as_solid_color_brush()
                .unwrap()
                .color(),
            Brushes::transparent()
                .as_solid_color_brush()
                .unwrap()
                .color()
        );
    }
}

#[test]
fn progress_ring_page_restores_empty_numeric_input_and_updates_the_native_range() {
    let _application = start();
    let root = page("ProgressRing");
    let number = root.get_control::<NumberBox>("ProgressValue");
    let ring = root.get_control::<ProgressRing>("ProgressRing2");
    number.set_numeric_value(67.5);
    assert_eq!(ring.value(), 67.5);
    number.set_numeric_value(f64::NAN);
    assert_eq!(number.value(), 0.0);
    assert_eq!(ring.value(), 0.0);
    number.set_numeric_value(100.0);
    assert_eq!(ring.value(), 100.0);
}

#[test]
fn rating_page_changes_caption_only_for_user_changes_and_obeys_clear_and_readonly() {
    let _application = start();
    let root = page("RatingControl");
    let rating = root.get_control::<RatingControl>("RatingControl1");
    assert_eq!(rating.caption(), "312 ratings");
    assert!(!rating.is_clear_enabled());
    rating.set_rating_value(2.0);
    assert_eq!(rating.caption(), "312 ratings");
    assert_eq!(
        root.get_control::<TextBlock>("RatingOutput")
            .text()
            .as_deref(),
        Some("2")
    );
    rating.select_rating(4.0, true);
    assert_eq!(rating.caption(), "Your rating");
    assert_eq!(
        root.get_control::<TextBlock>("RatingOutput")
            .text()
            .as_deref(),
        Some("4")
    );
    root.get_control::<CheckBox>("ClearEnabled")
        .set_is_checked(Some(true));
    rating.select_rating(4.0, true);
    assert_eq!(rating.value(), -1.0);
    root.get_control::<CheckBox>("ReadOnly")
        .set_is_checked(Some(true));
    rating.select_rating(3.0, true);
    assert_eq!(rating.value(), -1.0);
}

#[test]
fn rating_page_tracks_fractional_placeholder_in_both_directions() {
    let _application = start();
    let root = page("RatingControl");
    let rating = root.get_control::<RatingControl>("RatingControl2");
    let slider = root.get_control::<Slider>("PlaceholderSlider");
    slider.set_range_value(3.5);
    assert_eq!(rating.placeholder_value(), 3.5);
    assert_eq!(rating.value(), -1.0);
    rating.set_placeholder_value(2.5);
    assert_eq!(slider.value(), 2.5);
    rating.select_rating(4.0, true);
    assert_eq!(rating.placeholder_value(), 2.5);
}

#[test]
fn info_badge_page_switches_actual_source_themes_and_keeps_original_icon() {
    let _application = start();
    let root = page("InfoBadge");
    for (index, severity) in ["Attention", "Informational", "Success", "Critical"]
        .iter()
        .enumerate()
    {
        root.get_control::<ComboBox>("InfoBadgeStyleComboBox")
            .set_selected_index(index as i32);
        for (name, kind) in [
            ("StyleIconBadge", "Icon"),
            ("StyleValueBadge", "Value"),
            ("StyleDotBadge", "Dot"),
        ] {
            let value = root
                .try_find_resource(
                    &ResourceKey::from(format!("{severity}{kind}InfoBadgeStyle")),
                    None,
                )
                .flatten()
                .unwrap();
            let expected = value.downcast_ref::<Ref<ControlTheme>>().unwrap();
            assert_eq!(
                root.get_control::<InfoBadge>(name).theme().unwrap(),
                *expected
            );
        }
    }
    assert_eq!(root.get_control::<InfoBadge>("StyleValueBadge").value(), 10);
    let badge = root.get_control::<InfoBadge>("EmbeddedInfoBadge");
    let icon = badge
        .icon_source()
        .unwrap()
        .cast::<FontIconSource>()
        .unwrap();
    assert_eq!(icon.glyph(), "\u{f13c}");
}

#[test]
fn info_badge_page_preserves_source_integer_guard_for_dynamic_values() {
    let _application = start();
    let root = page("InfoBadge");
    let badge = root.get_control::<InfoBadge>("DynamicInfoBadge");
    let value = root.get_control::<NumberBox>("ValueNumberBox");
    assert_eq!(badge.value(), 1);
    value.set_numeric_value(27.9);
    assert_eq!(badge.value(), 27);
    value.set_numeric_value(f64::NAN);
    assert_eq!(badge.value(), 27);
    value.set_numeric_value(i32::MAX as f64 + 1.0);
    assert_eq!(badge.value(), 27);
    value.set_numeric_value(-1.0);
    assert_eq!(badge.value(), -1);
    value.set_numeric_value(0.0);
    assert_eq!(badge.value(), 0);
}

#[test]
fn feedback_pages_and_callbacks_release_their_native_controls() {
    let _application = start();
    for id in ["InfoBadge", "ProgressRing", "RatingControl"] {
        let root = page(id);
        let weak = root.downgrade();
        drop(root);
        assert!(weak.upgrade().is_none(), "{id} retained its page");
    }
}

#[test]
fn compact_sizing_copies_every_field_across_distinct_page_types() {
    use ferroui_base::animation::TimeSpan;
    use ferroui_base::utilities::DateTimeOffset;
    use ferroui_controls::winui::{Frame, RadioButtons};
    use ferroui_controls::{DatePicker, TextBox};
    let _application = start();
    let root = page("CompactSizing");
    let frame = root.get_control::<Frame>("SizingFrame");
    let options = root.get_control::<RadioButtons>("SizingMode");
    let initial_type = frame.current_source_page_type();
    let initial = crate::sizing_sample::form(&frame).unwrap();
    for (name, value) in [
        ("FirstName", "Ada"),
        ("LastName", "Lovelace"),
        ("Password", "example only"),
        ("ConfirmPassword", "example only"),
    ] {
        initial.get_control::<TextBox>(name).set_text(Some(value));
    }
    let selected = DateTimeOffset::new(2026, 10, 22, 0, 0, 0, TimeSpan::ZERO);
    initial
        .get_control::<DatePicker>("ChosenDate")
        .set_selected_date(Some(selected));
    options.set_selected_index(1);
    assert_ne!(frame.current_source_page_type(), initial_type);
    let compact = crate::sizing_sample::form(&frame).unwrap();
    for name in ["FirstName", "LastName", "Password", "ConfirmPassword"] {
        assert_eq!(
            compact.get_control::<TextBox>(name).text(),
            initial.get_control::<TextBox>(name).text()
        );
    }
    assert_eq!(
        compact
            .get_control::<DatePicker>("ChosenDate")
            .selected_date(),
        Some(selected)
    );
    compact
        .get_control::<TextBox>("LastName")
        .set_text(Some("Byron"));
    compact
        .get_control::<DatePicker>("ChosenDate")
        .set_selected_date(None);
    options.set_selected_index(0);
    assert_eq!(frame.current_source_page_type(), initial_type);
    let standard = crate::sizing_sample::form(&frame).unwrap();
    assert_eq!(
        standard
            .get_control::<TextBox>("LastName")
            .text()
            .as_deref(),
        Some("Byron")
    );
    assert_eq!(
        standard
            .get_control::<DatePicker>("ChosenDate")
            .selected_date(),
        None
    );
    assert!(!root.get_control::<TextBlock>("SizingError").is_visible());
}

#[test]
fn compact_sizing_resources_remain_page_scoped_and_release_with_the_gallery_page() {
    use ferroui_controls::winui::{Frame, RadioButtons};
    use ferroui_controls::Application;
    let _application = start();
    let key = ResourceKey::from("TextControlThemeMinHeight");
    let app = Application::current().unwrap();
    assert!(!app.resources().contains_key(&key));
    let root = page("CompactSizing");
    let frame = root.get_control::<Frame>("SizingFrame");
    assert!(!frame.current_page().unwrap().resources().contains_key(&key));
    root.get_control::<RadioButtons>("SizingMode")
        .set_selected_index(1);
    let compact = frame.current_page().unwrap();
    let height = compact.resources().try_get_value(&key).flatten().unwrap();
    assert_eq!(height.downcast_ref::<f64>(), Some(&24.0));
    assert!(!app.resources().contains_key(&key));
    root.get_control::<RadioButtons>("SizingMode")
        .set_selected_index(0);
    assert!(!frame.current_page().unwrap().resources().contains_key(&key));
    let weak_root = root.downgrade();
    let weak_frame = frame.downgrade();
    let weak_compact = compact.downgrade();
    drop(compact);
    drop(frame);
    drop(root);
    assert!(weak_root.upgrade().is_none());
    assert!(weak_frame.upgrade().is_none());
    assert!(weak_compact.upgrade().is_none());
}
