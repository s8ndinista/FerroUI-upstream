//! What the comparison host of Windows (`compare/`) does that does not need Windows: the markup
//! of a native page in a setting (the wrapper around the WinUI XAML of the page), and the
//! comparison of two pictures with the picture of their difference. Both are tested on every
//! host.

use crate::capture::Setting;
use crate::pages::PageInfo;
use crate::theme::{Density, Variant};
use ferroui_controls::TextLayoutRoundingPlacement;

/// The resource dictionary of WinUI that gives the controls their compact sizes (what the
/// compact density of the theme does in the framework).
pub const COMPACT_DICTIONARY: &str = "ms-appx:///Microsoft.UI.Xaml/DensityStyles/Compact.xaml";

/// The requested theme of WinUI (`ElementTheme`) of a variant; `None` for a variant WinUI cannot
/// request (high contrast is a setting of the system, not a theme an element can ask for).
pub fn element_theme(variant: Variant) -> Option<&'static str> {
    match variant {
        Variant::Light => Some("Light"),
        Variant::Dark => Some("Dark"),
        Variant::HighContrast => None,
    }
}

/// The markup the comparison host loads for `page` in `setting`, at `width` by `height`
/// device-independent pixels: the native page inside a grid with the requested theme, the flow
/// direction and, in the compact density, the compact dictionary of WinUI, on the page background
/// of WinUI (the wrapper of a page is the counterpart of the host of a page in the framework,
/// `CaptureHost.xaml`). `None` for a setting WinUI cannot show.
pub fn native_markup(page: &PageInfo, setting: Setting, width: f64, height: f64) -> Option<String> {
    let theme = element_theme(setting.variant)?;
    let flow_direction = if setting.right_to_left { "RightToLeft" } else { "LeftToRight" };
    let resources = if setting.density == Density::Compact {
        format!(
            "\n  <Grid.Resources>\n    <ResourceDictionary>\n      <ResourceDictionary.MergedDictionaries>\n        <ResourceDictionary Source=\"{COMPACT_DICTIONARY}\" />\n      </ResourceDictionary.MergedDictionaries>\n    </ResourceDictionary>\n  </Grid.Resources>"
        )
    } else {
        String::new()
    };
    Some(format!(
        "<Grid xmlns=\"http://schemas.microsoft.com/winfx/2006/xaml/presentation\"\n      xmlns:x=\"http://schemas.microsoft.com/winfx/2006/xaml\"\n      Width=\"{width}\" Height=\"{height}\" HorizontalAlignment=\"Left\" VerticalAlignment=\"Top\"\n      RequestedTheme=\"{theme}\" FlowDirection=\"{flow_direction}\">{resources}\n  <Grid Background=\"{{ThemeResource ApplicationPageBackgroundThemeBrush}}\">\n{native}\n  </Grid>\n</Grid>\n",
        native = page.native.trim()
    ))
}

/// The comparison of two pictures.
#[derive(Debug)]
pub struct Difference {
    /// The size of the comparison: the larger width and the larger height of the two pictures.
    pub width: i32,
    pub height: i32,
    /// The pixels that differ by more than the threshold in a channel, or that only one picture
    /// has.
    pub differing: usize,
    /// The picture of the difference: four bytes a pixel, blue first, opaque. A pixel that
    /// differs is red; the others are the first picture in light grey.
    pub image: Vec<u8>,
}

impl Difference {
    /// The share of the pixels that differ, in percent.
    pub fn percent(&self) -> f64 {
        let total = (self.width.max(0) as usize) * (self.height.max(0) as usize);
        if total == 0 {
            0.0
        } else {
            self.differing as f64 * 100.0 / total as f64
        }
    }
}

/// Compares two pictures of four bytes a pixel (blue, green, red, alpha), rows from the top
/// without padding: a pixel differs when a colour channel differs by more than `threshold`
/// (anti-aliasing and the rounding of colours differ by a few steps), or when it lies outside
/// one of the pictures.
pub fn compare(first: &[u8], first_size: (i32, i32), second: &[u8], second_size: (i32, i32), threshold: u8) -> Difference {
    let width = first_size.0.max(second_size.0).max(0);
    let height = first_size.1.max(second_size.1).max(0);
    let pixel = |pixels: &[u8], size: (i32, i32), x: i32, y: i32| -> Option<[u8; 3]> {
        if x >= size.0 || y >= size.1 {
            return None;
        }
        let at = ((y * size.0 + x) * 4) as usize;
        pixels.get(at..at + 3).map(|bgr| [bgr[0], bgr[1], bgr[2]])
    };
    let mut image = Vec::with_capacity((width as usize) * (height as usize) * 4);
    let mut differing = 0;
    for y in 0..height {
        for x in 0..width {
            let a = pixel(first, first_size, x, y);
            let b = pixel(second, second_size, x, y);
            let differs = match (a, b) {
                (Some(a), Some(b)) => (0..3).any(|channel| a[channel].abs_diff(b[channel]) > threshold),
                _ => true,
            };
            if differs {
                differing += 1;
                image.extend_from_slice(&[0x20, 0x20, 0xE0, 0xFF]);
            } else {
                let [blue, green, red] = a.unwrap_or([0xFF, 0xFF, 0xFF]);
                let luminance = (u32::from(red) * 299 + u32::from(green) * 587 + u32::from(blue) * 114) / 1000;
                // A light grey: the page is recognisable, and the red stands out.
                let grey = (0xFF - (0xFF - luminance) / 4) as u8;
                image.extend_from_slice(&[grey, grey, grey, 0xFF]);
            }
        }
    }
    Difference { width, height, differing, image }
}

/// Crops a picture of four bytes a pixel to the rectangle at `x`, `y` of `width` by `height`
/// (clipped to the picture).
pub fn crop(pixels: &[u8], size: (i32, i32), x: i32, y: i32, width: i32, height: i32) -> (Vec<u8>, (i32, i32)) {
    let x0 = x.clamp(0, size.0);
    let y0 = y.clamp(0, size.1);
    let x1 = (x + width).clamp(x0, size.0);
    let y1 = (y + height).clamp(y0, size.1);
    let mut cropped = Vec::with_capacity(((x1 - x0) * (y1 - y0) * 4).max(0) as usize);
    for row in y0..y1 {
        let start = ((row * size.0 + x0) * 4) as usize;
        let end = ((row * size.0 + x1) * 4) as usize;
        cropped.extend_from_slice(&pixels[start..end]);
    }
    (cropped, (x1 - x0, y1 - y0))
}

/// The environment variable of the comparison host with the list of text settings to compare
/// with (docs/porting/text-rendering.md, section 10).
pub const TEXT_VARIANTS_ENV: &str = "FERROUI_WINUI_COMPARE_TEXT_VARIANTS";

/// The keys of a text setting: those of the development override of the text rendering of the
/// Skia backend (`ferroui_skia::TEXT_OVERRIDE_KEYS`) and `baseline`, where the text blocks of the
/// page put the height layout rounding adds to them (`TextBlock.LayoutRoundingPlacement`, set on
/// the page; [`parse_baseline`]).
pub const TEXT_VARIANT_KEYS: &[&str] = &["contrast", "gamma", "edging", "hinting", "baseline_snap", "baseline"];

/// Parses the value of the key `baseline` of a text setting: `below` (or `top`) and `above` (or
/// `winui`) set `TextBlock.LayoutRoundingPlacement` on the page of the framework, `theme` (or
/// `default`, or empty) leaves it to the theme (the WinUI theme sets `Above` on its top-levels).
///
/// # Errors
/// The value is none of those.
pub fn parse_baseline(value: &str) -> Result<Option<TextLayoutRoundingPlacement>, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "" | "default" | "theme" => Ok(None),
        "below" | "top" => Ok(Some(TextLayoutRoundingPlacement::Below)),
        "above" | "winui" => Ok(Some(TextLayoutRoundingPlacement::Above)),
        other => Err(format!("the baseline {other:?} is not below, above or theme")),
    }
}

/// The name of a value of the key `baseline`, as [`parse_baseline`] reads it.
pub fn baseline_name(placement: Option<TextLayoutRoundingPlacement>) -> &'static str {
    match placement {
        None => "theme",
        Some(TextLayoutRoundingPlacement::Below) => "below",
        Some(TextLayoutRoundingPlacement::Above) => "above",
    }
}

/// One text setting of a calibration run: a name (the directory of its pictures) and the values
/// of the development overrides it sets; the others keep the values of the environment of the
/// process (`FERROUI_TEXT_*`), and the placement of the baseline that of the theme.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextVariant {
    pub name: String,
    /// `(key, value)`, keys in lower case with `_` (one of [`TEXT_VARIANT_KEYS`]).
    pub values: Vec<(String, String)>,
}

impl TextVariant {
    /// The value of `key`, if the setting sets it.
    pub fn value(&self, key: &str) -> Option<&str> {
        self.values.iter().rev().find(|(name, _)| name == key).map(|(_, value)| value.as_str())
    }

    /// The setting as it is written in the list (`name:key=value,...`).
    pub fn specification(&self) -> String {
        if self.values.is_empty() {
            return format!("{}:default", self.name);
        }
        let values: Vec<String> = self.values.iter().map(|(key, value)| format!("{key}={value}")).collect();
        format!("{}:{}", self.name, values.join(","))
    }
}

/// Parses the list of text settings of a calibration run: settings separated by `;`, each
/// `[name:]key=value,key=value` or `[name:]default` (no override), for example
/// `default;c05:contrast=0.5;c1g18:contrast=1,gamma=1.8;winui:baseline=above`. A setting without
/// a name is named after its values (`contrast-1_gamma-1.8`). An empty list gives no setting.
///
/// # Errors
/// A key that is not one of [`TEXT_VARIANT_KEYS`], a setting without `=` that is not `default`,
/// a name that is not letters, digits, `-`, `_` and `.`, or two settings of the same name.
pub fn parse_text_variants(list: &str) -> Result<Vec<TextVariant>, String> {
    let mut variants: Vec<TextVariant> = Vec::new();
    for item in list.split(';').map(str::trim).filter(|item| !item.is_empty()) {
        let (name, body) = match item.split_once(':') {
            Some((name, body)) => (Some(name.trim()), body.trim()),
            None => (None, item),
        };
        let mut values = Vec::new();
        if !body.eq_ignore_ascii_case("default") {
            for assignment in body.split(',').map(str::trim).filter(|assignment| !assignment.is_empty()) {
                let Some((key, value)) = assignment.split_once('=') else {
                    return Err(format!("{item:?}: {assignment:?} is not key=value"));
                };
                let key = key.trim().to_ascii_lowercase().replace('-', "_");
                if !TEXT_VARIANT_KEYS.contains(&key.as_str()) {
                    return Err(format!("{item:?}: {key:?} is not one of {}", TEXT_VARIANT_KEYS.join(", ")));
                }
                values.push((key, value.trim().to_string()));
            }
        }
        let name = match name {
            Some(name) if !name.is_empty() => name.to_string(),
            _ if values.is_empty() => "default".to_string(),
            _ => values.iter().map(|(key, value)| format!("{key}-{value}")).collect::<Vec<_>>().join("_"),
        };
        if !name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')) {
            return Err(format!("{item:?}: the name {name:?} is not letters, digits, '-', '_' and '.' (it names a directory)"));
        }
        if variants.iter().any(|variant| variant.name == name) {
            return Err(format!("{item:?}: a setting named {name:?} is in the list twice"));
        }
        variants.push(TextVariant { name, values });
    }
    Ok(variants)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PAGES;

    #[test]
    fn the_text_variants_of_a_calibration_run_parse() {
        assert_eq!(Ok(Vec::new()), parse_text_variants(" ; "));
        let variants =
            parse_text_variants("default; c1g18: contrast=1, gamma=1.8 ;Baseline=above;hinting=none,edging=antialias")
                .expect("a list");
        let names: Vec<&str> = variants.iter().map(|variant| variant.name.as_str()).collect();
        assert_eq!(names, ["default", "c1g18", "baseline-above", "hinting-none_edging-antialias"]);
        assert!(variants[0].values.is_empty());
        assert_eq!(variants[1].value("gamma"), Some("1.8"));
        assert_eq!(variants[1].value("edging"), None);
        assert_eq!(variants[2].value("baseline"), Some("above"));
        assert_eq!(variants[0].specification(), "default:default");
        assert_eq!(variants[1].specification(), "c1g18:contrast=1,gamma=1.8");

        assert!(parse_text_variants("weight=1").is_err());
        assert!(parse_text_variants("contrast").is_err());
        assert!(parse_text_variants("a b:contrast=1").is_err());
        assert!(parse_text_variants("x:contrast=1;x:gamma=0").is_err());
    }

    #[test]
    fn the_baseline_of_a_text_setting_parses() {
        assert_eq!(Ok(None), parse_baseline(""));
        assert_eq!(Ok(None), parse_baseline(" Theme "));
        assert_eq!(Ok(Some(TextLayoutRoundingPlacement::Below)), parse_baseline("top"));
        assert_eq!(Ok(Some(TextLayoutRoundingPlacement::Above)), parse_baseline("WinUI"));
        for placement in [None, Some(TextLayoutRoundingPlacement::Below), Some(TextLayoutRoundingPlacement::Above)] {
            assert_eq!(Ok(placement), parse_baseline(baseline_name(placement)));
        }
        assert!(parse_baseline("ceil").is_err());
    }

    #[test]
    fn equal_pictures_do_not_differ() {
        let picture = [10u8, 20, 30, 255].repeat(6);
        let difference = compare(&picture, (3, 2), &picture, (3, 2), 0);
        assert_eq!(difference.differing, 0);
        assert_eq!(difference.percent(), 0.0);
        assert_eq!(difference.image.len(), 3 * 2 * 4);
    }

    #[test]
    fn a_pixel_differs_beyond_the_threshold_only() {
        let first = [100u8, 100, 100, 255, 100, 100, 100, 255];
        let second = [110u8, 100, 100, 255, 100, 100, 140, 255];
        let difference = compare(&first, (2, 1), &second, (2, 1), 16);
        assert_eq!(difference.differing, 1);
        assert_eq!(difference.percent(), 50.0);
        assert_eq!(&difference.image[4..8], &[0x20, 0x20, 0xE0, 0xFF]);
    }

    #[test]
    fn what_only_one_picture_has_differs() {
        let first = [0u8, 0, 0, 255].repeat(4);
        let second = [0u8, 0, 0, 255].repeat(2);
        let difference = compare(&first, (2, 2), &second, (2, 1), 0);
        assert_eq!((difference.width, difference.height), (2, 2));
        assert_eq!(difference.differing, 2);
    }

    #[test]
    fn a_crop_is_clipped_to_the_picture() {
        let pixels: Vec<u8> = (0..4 * 4 * 4).map(|value| value as u8).collect();
        let (cropped, size) = crop(&pixels, (4, 4), 2, 3, 4, 4);
        assert_eq!(size, (2, 1));
        assert_eq!(cropped, pixels[(3 * 4 + 2) * 4..(3 * 4 + 4) * 4].to_vec());
    }

    #[test]
    fn the_markup_of_a_native_page_is_a_document_of_winui_xaml() {
        let setting = Setting { variant: Variant::Dark, density: Density::Compact, right_to_left: true };
        for page in PAGES {
            let markup = native_markup(page, setting, 720.0, 960.0).expect("the dark variant");
            let document = roxmltree::Document::parse(&markup)
                .unwrap_or_else(|error| panic!("{}: the wrapped page is not XML: {error}", page.id));
            let root = document.root_element();
            assert_eq!(root.attribute("RequestedTheme"), Some("Dark"));
            assert_eq!(root.attribute("FlowDirection"), Some("RightToLeft"));
            assert_eq!(root.attribute("Width"), Some("720"));
            assert!(markup.contains(COMPACT_DICTIONARY));
        }
    }

    #[test]
    fn high_contrast_has_no_native_page() {
        let setting = Setting { variant: Variant::HighContrast, density: Density::Normal, right_to_left: false };
        assert!(native_markup(&PAGES[0], setting, 720.0, 960.0).is_none());
    }
}
