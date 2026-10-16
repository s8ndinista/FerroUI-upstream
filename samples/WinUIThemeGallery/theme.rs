//! The theme under test, in one place: the gallery names the theme crate here only, so that a
//! rename of the crate or of its class is a change of the `use` line below.
//!
//! The theme has the surface of the Fluent theme of the framework: a `Styles` that is created
//! with `new()`, added to the styles of the application (`as_style()`), with a density style
//! (normal, compact). The accent colour is set as a WinUI application sets it: the resources
//! `SystemAccentColor` and its six shades in the resources of the theme, which take precedence
//! over the colours the theme reads from the platform.

use ferroui_base::media::Color;
use ferroui_base::styling::{IStyle, ThemeVariant};
use ferroui_base::Ref;
use std::rc::Rc;

use ferroui_base::controls::ResourceKey;
use ferroui_themes_winui::accents::SystemAccentColors;
use ferroui_themes_winui::{DensityStyle, WinUITheme as Theme};

/// The theme of the gallery.
pub type GalleryTheme = Theme;

/// Creates the theme.
pub fn create() -> Ref<GalleryTheme> {
    Theme::new()
}

/// The theme as a style of the application.
pub fn as_style(theme: &GalleryTheme) -> Rc<dyn IStyle> {
    theme.as_style()
}

/// The density of the controls.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Density {
    #[default]
    Normal,
    Compact,
}

impl Density {
    /// Every density, in the order of the toolbar and of the capture mode.
    pub const ALL: [Density; 2] = [Density::Normal, Density::Compact];

    /// The name of the density in the toolbar and in the options of the capture mode.
    pub fn name(self) -> &'static str {
        match self {
            Density::Normal => "normal",
            Density::Compact => "compact",
        }
    }

    /// The density with the name `name` (without regard to case).
    pub fn parse(name: &str) -> Option<Density> {
        Density::ALL.into_iter().find(|density| density.name().eq_ignore_ascii_case(name.trim()))
    }
}

/// Sets the density style of the theme.
pub fn set_density(theme: &GalleryTheme, density: Density) {
    theme.set_density_style(match density {
        Density::Normal => DensityStyle::Normal,
        Density::Compact => DensityStyle::Compact,
    });
}

/// Sets the accent colour of the theme; `None` leaves the accent to the platform (the accent
/// colour of the system). The shades are the ones the theme computes from the accent of the
/// platform (`SystemAccentColors::calculate_accent_shades`).
pub fn set_accent(theme: &GalleryTheme, accent: Option<Color>) {
    let keys = [
        SystemAccentColors::ACCENT_KEY,
        SystemAccentColors::ACCENT_DARK1_KEY,
        SystemAccentColors::ACCENT_DARK2_KEY,
        SystemAccentColors::ACCENT_DARK3_KEY,
        SystemAccentColors::ACCENT_LIGHT1_KEY,
        SystemAccentColors::ACCENT_LIGHT2_KEY,
        SystemAccentColors::ACCENT_LIGHT3_KEY,
    ];
    let resources = theme.resources();
    match accent {
        Some(accent) => {
            let (dark1, dark2, dark3, light1, light2, light3) = SystemAccentColors::calculate_accent_shades(accent);
            for (key, color) in keys.into_iter().zip([accent, dark1, dark2, dark3, light1, light2, light3]) {
                resources.set(key, Some(Rc::new(color)));
            }
        }
        None => {
            for key in keys {
                resources.remove(&ResourceKey::from(key));
            }
        }
    }
}

/// The theme variants the gallery offers. `HighContrast` is a variant of its own key, which
/// selects the `HighContrast` theme dictionaries of the theme where the theme has them; where it
/// has none, the resources come from its default dictionaries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variant {
    Light,
    Dark,
    HighContrast,
}

impl Variant {
    /// Every variant, in the order of the toolbar.
    pub const ALL: [Variant; 3] = [Variant::Light, Variant::Dark, Variant::HighContrast];

    /// The name of the variant in the toolbar, in the options of the capture mode and in the
    /// names of the pictures (in lower case there).
    pub fn name(self) -> &'static str {
        match self {
            Variant::Light => "Light",
            Variant::Dark => "Dark",
            Variant::HighContrast => "HighContrast",
        }
    }

    /// The variant with the name `name` (without regard to case).
    pub fn parse(name: &str) -> Option<Variant> {
        Variant::ALL.into_iter().find(|variant| variant.name().eq_ignore_ascii_case(name.trim()))
    }

    /// The theme variant of the framework.
    pub fn theme_variant(self) -> ThemeVariant {
        match self {
            Variant::Light => ThemeVariant::light(),
            Variant::Dark => ThemeVariant::dark(),
            Variant::HighContrast => Theme::high_contrast(),
        }
    }
}
