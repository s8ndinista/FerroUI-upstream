//! A development override of the text rendering of the backend: the text
//! contrast and gamma, and separately the edging, hinting and baseline snap
//! of text that does not ask for its own, read once from environment
//! variables at the first frame and changeable at run time by a calibration
//! host (docs/porting/text-rendering.md, section 10).
//!
//! It is a calibration aid, not a supported API: it exists so that one
//! build can be measured against native text with several settings (the
//! comparison host of the WinUI theme gallery runs a list of them in one
//! process). With none of the variables set the override is empty and
//! [`TextRenderingOverride::apply`] returns the settings unchanged.
//!
//! | Variable | Values | Replaces |
//! |---|---|---|
//! | `FERROUI_TEXT_CONTRAST` | 0 to 1 | the contrast of the surface (the platform's or Skia's) |
//! | `FERROUI_TEXT_GAMMA` | 0 (sRGB) to below 4 | the gamma of the surface |
//! | `FERROUI_TEXT_EDGING` | `alias`, `antialias`, `subpixel` | the default mode of text that asks for none |
//! | `FERROUI_TEXT_HINTING` | `none`, `light`, `strong` | the hinting of text that asks for none (`strong` is the default) |
//! | `FERROUI_TEXT_BASELINE_SNAP` | `on`, `off` | the baseline snap of text that asks for none (`on` is the default) |
//!
//! The override applies to every surface of the backend: the render targets
//! of windows, render target bitmaps and offscreen render targets.

use crate::text_rendering_settings::{clamp_contrast, clamp_gamma, TextRenderingSettings};
use ferroui_base::media::{BaselinePixelAlignment, TextHintingMode, TextRenderingMode};
use std::sync::{OnceLock, RwLock};

/// The prefix of the environment variables of the override.
pub const TEXT_OVERRIDE_ENV_PREFIX: &str = "FERROUI_TEXT_";

/// The keys of the override, in the names of its environment variables
/// (`FERROUI_TEXT_<KEY>`, upper case) and of a calibration specification
/// (`contrast=1,gamma=1.8`).
pub const TEXT_OVERRIDE_KEYS: &[&str] = &["contrast", "gamma", "edging", "hinting", "baseline_snap"];

/// A development override of the text rendering of the backend (see the
/// module): each value that is set replaces the one the surface would have.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextRenderingOverride {
    /// The text contrast of every surface (clamped to 0 to 1).
    pub text_contrast: Option<f32>,
    /// The text gamma of every surface (0 is sRGB; clamped below 4).
    pub text_gamma: Option<f32>,
    /// The mode of text whose options leave it unspecified.
    pub text_rendering_mode: Option<TextRenderingMode>,
    /// The hinting of text whose options leave it unspecified.
    pub text_hinting_mode: Option<TextHintingMode>,
    /// The baseline pixel alignment of text whose options leave it
    /// unspecified.
    pub baseline_pixel_alignment: Option<BaselinePixelAlignment>,
}

impl TextRenderingOverride {
    /// Whether the override changes nothing.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// The override of the environment variables of the process; a value
    /// that does not parse is reported on the error stream and left out.
    pub fn from_env() -> Self {
        let (value, errors) = Self::from_lookup(|name| std::env::var(name).ok());
        for error in errors {
            eprintln!("ferroui-skia: {error} (development override of the text rendering, left out)");
        }
        value
    }

    /// The override of the variables `lookup` gives (by their full names,
    /// `FERROUI_TEXT_CONTRAST`...), with the errors of those that do not
    /// parse. A variable that is empty is unset.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> (Self, Vec<String>) {
        let mut value = Self::default();
        let mut errors = Vec::new();
        for key in TEXT_OVERRIDE_KEYS {
            let name = format!("{TEXT_OVERRIDE_ENV_PREFIX}{}", key.to_ascii_uppercase());
            let Some(text) = lookup(&name).filter(|text| !text.trim().is_empty()) else {
                continue;
            };
            if let Err(error) = value.set(key, &text) {
                errors.push(format!("{name}: {error}"));
            }
        }
        (value, errors)
    }

    /// Sets the value of a key (one of [`TEXT_OVERRIDE_KEYS`], without
    /// regard to case and to `-` or `_`); `default` (or an empty value)
    /// clears it.
    ///
    /// # Errors
    /// The key is not one of the override, or the value is not one of the
    /// key.
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        let key = key.trim().to_ascii_lowercase().replace('-', "_");
        let value = value.trim();
        let clear = value.is_empty() || value.eq_ignore_ascii_case("default");
        let number = |value: &str| {
            value.parse::<f32>().ok().filter(|number| number.is_finite()).ok_or_else(|| format!("{value:?} is not a number"))
        };
        match key.as_str() {
            "contrast" => self.text_contrast = if clear { None } else { Some(clamp_contrast(number(value)?)) },
            "gamma" => {
                self.text_gamma = if clear {
                    None
                } else if value.eq_ignore_ascii_case("srgb") {
                    Some(0.0)
                } else {
                    Some(clamp_gamma(number(value)?))
                }
            }
            "edging" => {
                self.text_rendering_mode = if clear {
                    None
                } else {
                    Some(match value.to_ascii_lowercase().as_str() {
                        "alias" | "aliased" => TextRenderingMode::Alias,
                        "antialias" | "greyscale" | "grayscale" => TextRenderingMode::Antialias,
                        "subpixel" | "subpixelantialias" | "lcd" | "cleartype" => TextRenderingMode::SubpixelAntialias,
                        _ => return Err(format!("{value:?} is not alias, antialias or subpixel")),
                    })
                }
            }
            "hinting" => {
                self.text_hinting_mode = if clear {
                    None
                } else {
                    Some(match value.to_ascii_lowercase().as_str() {
                        "none" => TextHintingMode::None,
                        "light" | "slight" => TextHintingMode::Light,
                        "strong" | "full" | "normal" => TextHintingMode::Strong,
                        _ => return Err(format!("{value:?} is not none, light or strong")),
                    })
                }
            }
            "baseline_snap" => {
                self.baseline_pixel_alignment = if clear {
                    None
                } else {
                    Some(match value.to_ascii_lowercase().as_str() {
                        "on" | "1" | "true" | "aligned" => BaselinePixelAlignment::Aligned,
                        "off" | "0" | "false" | "unaligned" => BaselinePixelAlignment::Unaligned,
                        _ => return Err(format!("{value:?} is not on or off")),
                    })
                }
            }
            _ => return Err(format!("{key:?} is not one of {}", TEXT_OVERRIDE_KEYS.join(", "))),
        }
        Ok(())
    }

    /// The settings of a surface with this override applied.
    pub fn apply(&self, settings: TextRenderingSettings) -> TextRenderingSettings {
        let mut settings = settings;
        if let Some(contrast) = self.text_contrast {
            settings.text_contrast = contrast;
        }
        if let Some(gamma) = self.text_gamma {
            settings.text_gamma = gamma;
        }
        if let Some(mode) = self.text_rendering_mode {
            settings.default_text_rendering_mode = mode;
        }
        if let Some(hinting) = self.text_hinting_mode {
            settings.default_text_hinting_mode = hinting;
        }
        if let Some(alignment) = self.baseline_pixel_alignment {
            settings.default_baseline_pixel_alignment = alignment;
        }
        settings
    }
}

impl std::fmt::Display for TextRenderingOverride {
    /// The override as a calibration specification (`contrast=1,gamma=0`),
    /// `default` when it is empty.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();
        if let Some(contrast) = self.text_contrast {
            parts.push(format!("contrast={contrast}"));
        }
        if let Some(gamma) = self.text_gamma {
            parts.push(format!("gamma={gamma}"));
        }
        if let Some(mode) = self.text_rendering_mode {
            let name = match mode {
                TextRenderingMode::Alias => "alias",
                TextRenderingMode::SubpixelAntialias => "subpixel",
                _ => "antialias",
            };
            parts.push(format!("edging={name}"));
        }
        if let Some(hinting) = self.text_hinting_mode {
            let name = match hinting {
                TextHintingMode::None => "none",
                TextHintingMode::Light => "light",
                _ => "strong",
            };
            parts.push(format!("hinting={name}"));
        }
        if let Some(alignment) = self.baseline_pixel_alignment {
            parts.push(format!("baseline_snap={}", if alignment == BaselinePixelAlignment::Unaligned { "off" } else { "on" }));
        }
        if parts.is_empty() {
            f.write_str("default")
        } else {
            f.write_str(&parts.join(","))
        }
    }
}

fn cell() -> &'static RwLock<TextRenderingOverride> {
    static CELL: OnceLock<RwLock<TextRenderingOverride>> = OnceLock::new();
    CELL.get_or_init(|| RwLock::new(startup_text_rendering_override()))
}

/// The override of the environment of the process, read once.
pub fn startup_text_rendering_override() -> TextRenderingOverride {
    static STARTUP: OnceLock<TextRenderingOverride> = OnceLock::new();
    *STARTUP.get_or_init(TextRenderingOverride::from_env)
}

/// The override the surfaces of the backend apply now: the one of the
/// environment unless a calibration host replaced it.
pub fn text_rendering_override() -> TextRenderingOverride {
    *cell().read().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Replaces the override of the process (a calibration aid, not a
/// supported API): it shows from the next frame, render target bitmap or
/// layer that is created.
pub fn set_text_rendering_override(value: TextRenderingOverride) {
    *cell().write().unwrap_or_else(|poisoned| poisoned.into_inner()) = value;
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::platform::surfaces::{PlatformTextRenderingParameters, SubpixelGeometry};
    use ferroui_base::rendering::composition::CompositionTransparencyLevel;
    use skia_safe::SurfaceProps;
    use std::collections::HashMap;

    fn lookup(variables: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let variables: HashMap<String, String> =
            variables.iter().map(|(name, value)| (name.to_string(), value.to_string())).collect();
        move |name| variables.get(name).cloned()
    }

    fn windows_scene() -> TextRenderingSettings {
        TextRenderingSettings::for_scene(
            Some(PlatformTextRenderingParameters {
                subpixel_antialiasing: true,
                subpixel_geometry: SubpixelGeometry::RgbHorizontal,
                composited: true,
                text_contrast: Some(1.0),
                text_gamma: Some(0.0),
            }),
            2.0,
            CompositionTransparencyLevel::None,
        )
    }

    #[test]
    fn without_variables_the_override_is_empty_and_changes_nothing() {
        let (value, errors) = TextRenderingOverride::from_lookup(lookup(&[]));
        assert!(errors.is_empty());
        assert!(value.is_empty());
        assert_eq!("default", value.to_string());
        for settings in [TextRenderingSettings::default(), windows_scene()] {
            assert_eq!(settings, value.apply(settings));
        }

        // Empty variables are unset.
        let (value, errors) = TextRenderingOverride::from_lookup(lookup(&[("FERROUI_TEXT_CONTRAST", " ")]));
        assert!(errors.is_empty() && value.is_empty());
    }

    #[test]
    fn the_variables_replace_contrast_gamma_edging_hinting_and_snap() {
        let (value, errors) = TextRenderingOverride::from_lookup(lookup(&[
            ("FERROUI_TEXT_CONTRAST", "0.75"),
            ("FERROUI_TEXT_GAMMA", "1.8"),
            ("FERROUI_TEXT_EDGING", "subpixel"),
            ("FERROUI_TEXT_HINTING", "none"),
            ("FERROUI_TEXT_BASELINE_SNAP", "off"),
        ]));
        assert!(errors.is_empty(), "{errors:?}");
        let settings = value.apply(windows_scene());
        assert_eq!(0.75, settings.text_contrast);
        assert_eq!(1.8, settings.text_gamma);
        assert_eq!(TextRenderingMode::SubpixelAntialias, settings.default_text_rendering_mode);
        assert_eq!(TextHintingMode::None, settings.default_text_hinting_mode);
        assert_eq!(BaselinePixelAlignment::Unaligned, settings.default_baseline_pixel_alignment);
        let props = settings.surface_props();
        assert_eq!((0.75, 1.8), (props.text_contrast(), props.text_gamma()));
        assert_eq!("contrast=0.75,gamma=1.8,edging=subpixel,hinting=none,baseline_snap=off", value.to_string());

        // Only what is set is replaced.
        let (value, _) = TextRenderingOverride::from_lookup(lookup(&[("FERROUI_TEXT_GAMMA", "srgb")]));
        let defaults = SurfaceProps::default();
        let settings = value.apply(TextRenderingSettings::default());
        assert_eq!(defaults.text_contrast(), settings.text_contrast);
        assert_eq!(0.0, settings.text_gamma);
        assert_eq!(TextRenderingMode::Antialias, settings.default_text_rendering_mode);
    }

    #[test]
    fn values_are_clamped_and_wrong_ones_reported() {
        let (value, errors) = TextRenderingOverride::from_lookup(lookup(&[
            ("FERROUI_TEXT_CONTRAST", "3"),
            ("FERROUI_TEXT_GAMMA", "much"),
            ("FERROUI_TEXT_EDGING", "blurry"),
        ]));
        assert_eq!(Some(1.0), value.text_contrast);
        assert_eq!(None, value.text_gamma);
        assert_eq!(None, value.text_rendering_mode);
        assert_eq!(2, errors.len(), "{errors:?}");
        assert!(errors[0].starts_with("FERROUI_TEXT_GAMMA"));

        let mut value = TextRenderingOverride::default();
        assert!(value.set("Baseline-Snap", "on").is_ok());
        assert_eq!(Some(BaselinePixelAlignment::Aligned), value.baseline_pixel_alignment);
        assert!(value.set("baseline_snap", "default").is_ok());
        assert!(value.is_empty());
        assert!(value.set("weight", "1").is_err());
    }

    #[test]
    fn the_process_override_starts_from_the_environment() {
        // The test process has none of the variables (the build of the
        // tests does not set them): the override starts empty. It is not
        // replaced here: the render tests of the crate run in parallel and
        // read it.
        if TEXT_OVERRIDE_KEYS
            .iter()
            .any(|key| std::env::var_os(format!("{TEXT_OVERRIDE_ENV_PREFIX}{}", key.to_ascii_uppercase())).is_some())
        {
            return;
        }
        assert!(startup_text_rendering_override().is_empty());
        assert_eq!(startup_text_rendering_override(), text_rendering_override());
    }
}
