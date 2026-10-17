//! Where a text block puts the height that layout rounding adds to it, the type of
//! `TextBlock.LayoutRoundingPlacement` (a FerroUI addition, docs/porting/DEVIATIONS.md, Text
//! block; docs/porting/text-rendering.md, section 6).
//!
//! With layout rounding the desired height of a text block is the height of its text and its
//! vertical padding rounded up to whole device pixels. The reference draws the text from the top
//! of the block, so the added height ends up below the text. WinUI's TextBlock puts it above the
//! text: `CTextBlock::MeasureOverride` keeps `m_layoutRoundingHeightAdjustment` (the rounded
//! height minus the height) and `GetContentRenderingOffset` adds it to the top of the text, which
//! lowers the baseline by up to one device pixel before the glyph run is snapped to the pixel
//! grid.

use ferroui_base::ferro_markup_enum;
use ferroui_base::layout::LayoutHelper;

/// Where a text block puts the height layout rounding adds to it: below its text (the
/// reference's placement, the default) or above it (WinUI's).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TextLayoutRoundingPlacement {
    /// The text starts at the top of the block (after its padding); the added height is below
    /// it. The placement of the reference, and of the Fluent and Simple themes.
    #[default]
    Below,
    /// The added height is above the text, as WinUI's TextBlock puts it: the text ends at the
    /// bottom of the rounded box. The placement of the WinUI theme.
    Above,
}

ferro_markup_enum!(TextLayoutRoundingPlacement { Below, Above }, { namespace: "FerroUI.Controls" });

impl TextLayoutRoundingPlacement {
    /// How far down a text block with layout rounding moves its text, in device-independent
    /// pixels: `height` is the height of its content (the text and the vertical padding),
    /// `scale` the layout scale. Zero for [`TextLayoutRoundingPlacement::Below`], and for a
    /// height that is a whole number of device pixels already.
    pub fn offset(self, height: f64, scale: f64) -> f64 {
        match self {
            Self::Below => 0.0,
            Self::Above => {
                if !height.is_finite() || height <= 0.0 || !scale.is_finite() || scale <= 0.0 {
                    return 0.0;
                }
                (LayoutHelper::round_layout_value_up(height, scale) - height).max(0.0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Segoe UI Variable: 2048 units per em, ascent 2210, descent 514, no line gap.
    fn segoe(size: f64) -> (f64, f64) {
        (size * 2210.0 / 2048.0, size * 2724.0 / 2048.0)
    }

    /// The baseline in device pixels from the top of a block on a whole pixel, as Skia and
    /// Direct2D snap it (to the nearest pixel).
    fn baseline(placement: TextLayoutRoundingPlacement, size: f64, scale: f64) -> f64 {
        let (ascent, height) = segoe(size);
        ((ascent + placement.offset(height, scale)) * scale + 0.5).floor()
    }

    #[test]
    fn above_puts_the_baselines_where_winui_draws_them() {
        // Measured on Windows (docs/porting/text-rendering.md, section 6): WinUI's 14 px text is
        // where FerroUI's is at a scale of 1 and one pixel lower at 2; the 28 px title one pixel
        // lower at both.
        let below = TextLayoutRoundingPlacement::Below;
        let above = TextLayoutRoundingPlacement::Above;
        assert_eq!((15.0, 15.0), (baseline(below, 14.0, 1.0), baseline(above, 14.0, 1.0)));
        assert_eq!((30.0, 31.0), (baseline(below, 14.0, 2.0), baseline(above, 14.0, 2.0)));
        assert_eq!((30.0, 31.0), (baseline(below, 28.0, 1.0), baseline(above, 28.0, 1.0)));
        assert_eq!((60.0, 61.0), (baseline(below, 28.0, 2.0), baseline(above, 28.0, 2.0)));
    }

    #[test]
    fn the_offset_is_what_layout_rounding_adds() {
        let above = TextLayoutRoundingPlacement::Above;
        for scale in [1.0, 1.25, 1.5, 2.0] {
            for height in [18.62, 15.96, 37.24, 42.56] {
                let offset = above.offset(height, scale);
                let rounded = ((height * scale).ceil()) / scale;
                assert!((offset - (rounded - height)).abs() < 1e-9, "{height} at {scale}: {offset}");
                assert!((0.0..1.0 / scale).contains(&offset), "{height} at {scale}: {offset}");
                assert_eq!(0.0, TextLayoutRoundingPlacement::Below.offset(height, scale));
            }
        }
        assert!((above.offset(18.62, 1.0) - 0.38).abs() < 1e-9);
        assert!((above.offset(18.62, 2.0) - 0.38).abs() < 1e-9);
        // 27.93 device pixels at 1.5 become 28.
        assert!((above.offset(18.62, 1.5) - (28.0 / 1.5 - 18.62)).abs() < 1e-9);
        // 23.275 device pixels at 1.25 become 24.
        assert!((above.offset(18.62, 1.25) - (24.0 / 1.25 - 18.62)).abs() < 1e-9);
        assert_eq!(0.0, above.offset(19.0, 1.0));
        assert_eq!(0.0, above.offset(0.0, 2.0));
        assert_eq!(0.0, above.offset(f64::INFINITY, 2.0));
        assert_eq!(0.0, above.offset(18.62, 0.0));
    }

    #[test]
    fn below_is_the_default() {
        assert_eq!(TextLayoutRoundingPlacement::Below, TextLayoutRoundingPlacement::default());
    }
}
