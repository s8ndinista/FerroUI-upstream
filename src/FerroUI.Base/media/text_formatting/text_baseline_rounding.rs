//! A development override of where a text block puts its text in the
//! height that layout rounding adds to it (docs/porting/text-rendering.md,
//! sections 6 and 10): read once from `FERROUI_TEXT_BASELINE_ROUNDING` and
//! changeable at run time by a calibration host. It is a calibration aid,
//! not a supported API; unset, text blocks behave as they always did.
//!
//! With layout rounding a text block's desired height is its text's height
//! rounded up to whole device pixels. FerroUI (as the reference) draws the
//! text from the top of the block, so the added height ends up below the
//! text. WinUI's TextBlock puts it above the text: `CTextBlock::MeasureOverride`
//! keeps `m_layoutRoundingHeightAdjustment` (the rounded height minus the
//! height) and `GetContentRenderingOffset` adds it to the top of the text,
//! which lowers the baseline by up to one device pixel before the glyph run
//! is snapped to the pixel grid.

use crate::layout::LayoutHelper;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

/// The name of the environment variable of the override.
pub const TEXT_BASELINE_ROUNDING_ENV: &str = "FERROUI_TEXT_BASELINE_ROUNDING";

/// Where a text block puts its text in the height layout rounding adds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextBaselineRounding {
    /// The text starts at the top of the block (after its padding); the
    /// added height is below it. FerroUI's behaviour and the reference's.
    #[default]
    Top,
    /// The added height is above the text, as WinUI's TextBlock puts it:
    /// the text ends at the bottom of the rounded box.
    Above,
}

impl TextBaselineRounding {
    /// Parses a value of the variable: `top` (or `default`, or empty) and
    /// `above` (or `winui`).
    ///
    /// # Errors
    /// The value is none of those.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "" | "default" | "top" => Ok(Self::Top),
            "above" | "winui" => Ok(Self::Above),
            other => Err(format!("{other:?} is not top or above")),
        }
    }

    /// The name of the value, as [`TextBaselineRounding::parse`] reads it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Above => "above",
        }
    }

    /// How far down a text block with layout rounding moves its text, in
    /// device-independent pixels: `height` is the height of its content
    /// (the text and the vertical padding), `scale` the layout scale.
    /// Zero for [`TextBaselineRounding::Top`].
    pub fn offset(self, height: f64, scale: f64) -> f64 {
        match self {
            Self::Top => 0.0,
            Self::Above => {
                if !height.is_finite() || height <= 0.0 {
                    return 0.0;
                }
                (LayoutHelper::round_layout_value_up(height, scale) - height).max(0.0)
            }
        }
    }
}

const UNREAD: u8 = u8::MAX;

static CURRENT: AtomicU8 = AtomicU8::new(UNREAD);

fn encode(value: TextBaselineRounding) -> u8 {
    match value {
        TextBaselineRounding::Top => 0,
        TextBaselineRounding::Above => 1,
    }
}

/// The value of the environment of the process, read once; a value that
/// does not parse is reported on the error stream and left out.
pub fn startup_text_baseline_rounding() -> TextBaselineRounding {
    static STARTUP: OnceLock<TextBaselineRounding> = OnceLock::new();
    *STARTUP.get_or_init(|| {
        let value = std::env::var(TEXT_BASELINE_ROUNDING_ENV).unwrap_or_default();
        TextBaselineRounding::parse(&value).unwrap_or_else(|error| {
            eprintln!("ferroui: {TEXT_BASELINE_ROUNDING_ENV}: {error} (development override, left out)");
            TextBaselineRounding::Top
        })
    })
}

/// The value text blocks use now: the one of the environment unless a
/// calibration host replaced it.
pub fn text_baseline_rounding() -> TextBaselineRounding {
    match CURRENT.load(Ordering::Relaxed) {
        0 => TextBaselineRounding::Top,
        1 => TextBaselineRounding::Above,
        _ => {
            let value = startup_text_baseline_rounding();
            let _ = CURRENT.compare_exchange(UNREAD, encode(value), Ordering::Relaxed, Ordering::Relaxed);
            text_baseline_rounding()
        }
    }
}

/// Replaces the value of the process (a calibration aid, not a supported
/// API): it shows from the next time a text block renders.
pub fn set_text_baseline_rounding(value: TextBaselineRounding) {
    CURRENT.store(encode(value), Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Segoe UI Variable: 2048 units per em, ascent 2210, descent 514, no
    /// line gap.
    fn segoe(size: f64) -> (f64, f64) {
        (size * 2210.0 / 2048.0, size * 2724.0 / 2048.0)
    }

    /// The baseline in device pixels from the top of a block on a whole
    /// pixel, as Skia and Direct2D snap it (to the nearest pixel).
    fn baseline(rounding: TextBaselineRounding, size: f64, scale: f64) -> f64 {
        let (ascent, height) = segoe(size);
        ((ascent + rounding.offset(height, scale)) * scale + 0.5).floor()
    }

    #[test]
    fn above_puts_the_baselines_where_winui_draws_them() {
        // Measured on Windows (docs/porting/text-rendering.md, section 6):
        // WinUI's 14 px text is where FerroUI's is at a scale of 1 and one
        // pixel lower at 2; the 28 px title one pixel lower at both.
        let top = TextBaselineRounding::Top;
        let above = TextBaselineRounding::Above;
        assert_eq!((15.0, 15.0), (baseline(top, 14.0, 1.0), baseline(above, 14.0, 1.0)));
        assert_eq!((30.0, 31.0), (baseline(top, 14.0, 2.0), baseline(above, 14.0, 2.0)));
        assert_eq!((30.0, 31.0), (baseline(top, 28.0, 1.0), baseline(above, 28.0, 1.0)));
        assert_eq!((60.0, 61.0), (baseline(top, 28.0, 2.0), baseline(above, 28.0, 2.0)));
    }

    #[test]
    fn the_offset_is_what_layout_rounding_adds() {
        let above = TextBaselineRounding::Above;
        assert!((above.offset(18.62, 1.0) - 0.38).abs() < 1e-9);
        assert!((above.offset(18.62, 2.0) - 0.38).abs() < 1e-9);
        // 27.93 device pixels at 1.5 become 28.
        assert!((above.offset(18.62, 1.5) - (28.0 / 1.5 - 18.62)).abs() < 1e-9);
        assert_eq!(0.0, above.offset(19.0, 1.0));
        assert_eq!(0.0, above.offset(0.0, 2.0));
        assert_eq!(0.0, above.offset(f64::INFINITY, 2.0));
        assert_eq!(0.0, TextBaselineRounding::Top.offset(18.62, 2.0));
    }

    #[test]
    fn the_values_parse() {
        assert_eq!(Ok(TextBaselineRounding::Top), TextBaselineRounding::parse(""));
        assert_eq!(Ok(TextBaselineRounding::Top), TextBaselineRounding::parse("default"));
        assert_eq!(Ok(TextBaselineRounding::Above), TextBaselineRounding::parse(" WinUI "));
        assert_eq!(Ok(TextBaselineRounding::Above), TextBaselineRounding::parse(TextBaselineRounding::Above.name()));
        assert!(TextBaselineRounding::parse("ceil").is_err());
    }

    #[test]
    fn without_the_variable_text_starts_at_the_top() {
        if std::env::var_os(TEXT_BASELINE_ROUNDING_ENV).is_some() {
            return;
        }
        assert_eq!(TextBaselineRounding::Top, startup_text_baseline_rounding());
        assert_eq!(TextBaselineRounding::Top, text_baseline_rounding());
    }
}
