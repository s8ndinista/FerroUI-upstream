//! The class of the theme and its resources as a whole (not from upstream): the accent colours,
//! the three theme variants, the density style, and the theme in markup.

use super::support::*;
use crate::accents::SystemAccentColors;
use crate::{DensityStyle, WinUITheme};
use ferroui_base::controls::ResourceKey;
use ferroui_base::media::Color;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{Ref, Thickness};
use ferroui_controls::{Application, Border, Button, ContentControl, Control, Window};

fn color_of(value: Option<Option<ferroui_base::BoxedValue>>) -> Option<Color> {
    from_markup_value::<Color>(&value?)
}

fn key(text: &str) -> ResourceKey {
    ResourceKey::from(text)
}

#[test]
fn system_accent_colors_provide_the_default_accent_without_an_owner() {
    let _app = start_application();
    let colors = SystemAccentColors::new();
    assert!(colors.has_resources());
    assert_eq!(Some(Color::from_rgb(0, 120, 215)), color_of(colors.try_get_resource(&key("SystemAccentColor"), None)));
    for shade in ["Dark1", "Dark2", "Dark3", "Light1", "Light2", "Light3"] {
        assert!(color_of(colors.try_get_resource(&key(&format!("SystemAccentColor{shade}")), None)).is_some());
    }
    assert!(colors.try_get_resource(&ResourceKey::Type(Button::TYPE), None).is_none());
}

#[test]
fn high_contrast_is_a_variant_of_its_own_that_inherits_the_dark_one() {
    let high_contrast = WinUITheme::high_contrast();
    assert_eq!(high_contrast, WinUITheme::high_contrast());
    assert_ne!(high_contrast, ThemeVariant::dark());
    assert_ne!(high_contrast, ThemeVariant::default());
    assert_eq!(Some(ThemeVariant::dark()), high_contrast.inherit_variant());
}

#[test]
fn theme_loads_and_is_added_to_the_styles_of_an_application() {
    let _app = start_application();
    let application = Application::current().expect("an application");

    let theme = WinUITheme::new();
    application.styles().add(theme.as_style());

    assert_eq!(1, application.styles().count());
    assert_eq!(1, theme.count());
    assert_eq!(DensityStyle::Normal, theme.density_style());
    // The compact resources are taken out of the resources by the constructor.
    assert!(!theme.resources().contains_key(&key("CompactStyles")));
    // The system accent colours stay a merged dictionary; the resource documents are merged into
    // the dictionary of the theme.
    assert_eq!(1, theme.resources().merged_dictionaries().count());
    let button_theme = application.try_get_resource(&ResourceKey::Type(Button::TYPE), None);
    assert!(button_theme.flatten().is_some());
    assert_eq!(
        Some(Color::from_rgb(0, 120, 215)),
        color_of(application.try_get_resource(&key("SystemAccentColor"), None))
    );
}

#[test]
fn the_three_variants_resolve_their_resources() {
    let _app = start_themed_application();
    let application = Application::current().expect("an application");

    let resource = |name: &str, variant: ThemeVariant| -> Color {
        color_of(application.try_get_resource(&key(name), Some(&variant)))
            .unwrap_or_else(|| panic!("{name} is not a colour of {variant:?}"))
    };
    // Common_themeresources_any.xaml: Default is the dark theme of WinUI.
    assert_eq!(Color::parse("#E4000000").unwrap(), resource("TextFillColorPrimary", ThemeVariant::light()));
    assert_eq!(Color::parse("#FFFFFFFF").unwrap(), resource("TextFillColorPrimary", ThemeVariant::dark()));
    assert_eq!(Color::parse("#F3F3F3").unwrap(), resource("SolidBackgroundFillColorBase", ThemeVariant::light()));
    assert_eq!(Color::parse("#202020").unwrap(), resource("SolidBackgroundFillColorBase", ThemeVariant::dark()));
    // Resources/SystemResources.xaml: the "High Contrast Black" colours.
    assert_eq!(Color::parse("#FF000000").unwrap(), resource("SystemColorWindowColor", WinUITheme::high_contrast()));
    assert_eq!(Color::parse("#FFFFFFFF").unwrap(), resource("SystemColorWindowColor", ThemeVariant::light()));

    // A brush of the high contrast dictionary takes the system colour of that variant: the
    // background of a window (SolidBackgroundFillColorBaseBrush, SystemColorWindowColor).
    let window = Window::new();
    window.set_content(Some(Control::boxed(&Border::new())));
    window.show();
    window.set_requested_theme_variant(Some(WinUITheme::high_contrast()));
    let background = window.background().and_then(|brush| brush.as_solid_color_brush().map(|solid| solid.color()));
    assert_eq!(Some(Color::parse("#FF000000").unwrap()), background);
}

#[test]
fn compact_density_style_is_checked_first() {
    let _app = start_application();
    let application = Application::current().expect("an application");
    let theme = WinUITheme::new();
    application.styles().add(theme.as_style());

    let padding = || {
        let value = application.try_get_resource(&key("TextControlThemePadding"), Some(&ThemeVariant::light()));
        from_markup_value::<Thickness>(&value.expect("TextControlThemePadding is defined")).expect("a thickness")
    };
    assert_eq!(Thickness::new(10.0, 5.0, 6.0, 6.0), padding());

    theme.set_density_style(DensityStyle::Compact);
    assert_eq!(DensityStyle::Compact, theme.get_direct_value(WinUITheme::density_style_property()));
    assert_eq!(Thickness::new(2.0, 2.0, 6.0, 1.0), padding());

    theme.set_density_style(DensityStyle::Normal);
    assert_eq!(Thickness::new(10.0, 5.0, 6.0, 6.0), padding());
}

#[test]
fn theme_and_high_contrast_are_written_in_markup() {
    let _app = start_application();
    let control = load_text(
        "
<ContentControl xmlns='https://github.com/ferroui'
                xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
                xmlns:winui='using:FerroUI.Themes.WinUI'
                RequestedThemeVariant='{x:Static winui:WinUITheme.HighContrast}'>
    <ContentControl.Styles>
        <winui:WinUITheme DensityStyle='Compact' />
    </ContentControl.Styles>
</ContentControl>",
    );
    let control = from_markup_value::<Ref<ContentControl>>(&Some(control)).expect("a content control");
    let style = control.styles().get(0);
    let theme = style.as_object().and_then(|object| object.downcast_ref::<WinUITheme>()).expect("the theme").to_ref();

    assert_eq!(DensityStyle::Compact, theme.density_style());
    assert_eq!(Some(WinUITheme::high_contrast()), control.get_value(ThemeVariant::requested_theme_variant_property()));
    assert_eq!(
        Some(Color::parse("#FF1AEBFF").unwrap()),
        color_of(style.try_get_resource(&key("SystemColorHighlightColor"), Some(&WinUITheme::high_contrast())))
    );
}
