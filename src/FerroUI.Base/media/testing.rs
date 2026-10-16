//! The members of the media types the benchmarks of the framework reach:
//! upstream's glyph bounds benchmark reads the bounds of glyphs from the font
//! tables through the internal `GlyphTypeface.TryGetGlyphBounds`, with its
//! access to the internals of the framework.

use super::GlyphTypeface;

/// A glyph's bounding box in font design units, as stored in the `glyf`
/// glyph header (the internal `GlyphBounds`).
pub use super::glyph_bounds::GlyphBounds;

/// `GlyphTypeface.TryGetGlyphBounds`: the glyph-header bounding boxes of a
/// batch of glyphs, read from the `glyf` table; `false` when the font has no
/// such table.
///
/// # Panics
/// Panics when `bounds` is shorter than `glyph_indices`.
pub fn try_get_glyph_bounds(glyph_typeface: &GlyphTypeface, glyph_indices: &[u16], bounds: &mut [GlyphBounds]) -> bool {
    glyph_typeface.try_get_glyph_bounds(glyph_indices, bounds)
}
