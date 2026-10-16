//! Compares obtaining glyph ink bounds via the table-based
//! `GlyphTypeface::try_get_glyph_bounds` against Skia's
//! `Font::get_widths_bounds(.., bounds)`, the path the glyph run of the Skia
//! backend uses today. Both sides compute bounds only (Skia is passed no
//! widths), so the comparison is like-for-like. Two scenario pairs isolate the
//! pure bounds-read cost (font pre-created) from the realistic per-run cost
//! (Skia pays the creation of its font, which the table path avoids
//! entirely).
//!
//! The bounds of the table path are internal to the base crate; its `testing`
//! feature exposes them (`ferroui_base::media::testing`), as upstream's
//! benchmark project reaches them through its access to the internals of the
//! framework. Upstream rents the bounds of `skia_per_run` from the shared
//! array pool and puts those of `table_per_run` on the stack; here the first
//! are a buffer of the benchmark that is taken and given back on every run
//! (what renting from a pool amounts to), the second an array on the stack of
//! the largest glyph count.

use crate::harness::Registry;
use ferroui_base::media::testing::{try_get_glyph_bounds, GlyphBounds};
use ferroui_base::media::{GlyphTypeface, Typeface};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_skia::{FontManagerImpl, PlatformRenderInterface, SkiaTypeface};
use skia_safe::{Font, Rect};
use std::rc::Rc;

const SIZE: f32 = 16.0;

/// The largest glyph count of the parameters: the bounds of `table_per_run`
/// fit on the stack up to it, as upstream's `stackalloc`.
const MAX_STACK_GLYPHS: usize = 256;

pub struct GlyphBoundsBenchmark {
    glyph_typeface: Rc<GlyphTypeface>,
    font: Font,
    glyph_indices: Vec<u16>,
    sk_bounds: Vec<Rect>,
    bounds: Vec<GlyphBounds>,
    /// The buffer `skia_per_run` rents (`ArrayPool<SKRect>.Shared`).
    rented: Vec<Rect>,
    glyph_count: usize,
    /// Declared last: the application ends after the font and the typeface
    /// are released (`Dispose`).
    _app: UnitTestApplicationScope,
}

impl GlyphBoundsBenchmark {
    pub fn new(glyph_count: usize) -> Self {
        let app = UnitTestApplication::start(
            TestServices::mock_platform_render_interface()
                .with_render_interface(Rc::new(PlatformRenderInterface::default()))
                .with_font_manager_impl(Rc::new(FontManagerImpl::new())),
        );

        let glyph_typeface = Typeface::default().glyph_typeface();
        let font = Self::skia_typeface(&glyph_typeface).create_sk_font(SIZE);

        let map = glyph_typeface.character_to_glyph_map();
        let mut pool = Vec::new();
        for c in '!'..='~' {
            if map.contains_glyph(c as i32) {
                pool.push(map.get_glyph(c as i32));
            }
        }

        let glyph_pool = pool;

        // `[GlobalSetup]`.
        let mut glyph_indices = vec![0_u16; glyph_count];

        for (i, glyph_index) in glyph_indices.iter_mut().enumerate() {
            *glyph_index = glyph_pool[i % glyph_pool.len()];
        }

        let sk_bounds = vec![Rect::default(); glyph_count];
        let bounds = vec![GlyphBounds::default(); glyph_count];

        Self { glyph_typeface, font, glyph_indices, sk_bounds, bounds, rented: Vec::new(), glyph_count, _app: app }
    }

    /// `(SkiaTypeface)_glyphTypeface.PlatformTypeface`.
    fn skia_typeface(glyph_typeface: &GlyphTypeface) -> &SkiaTypeface {
        SkiaTypeface::try_get(&**glyph_typeface.platform_typeface()).expect("the typeface of the Skia font manager")
    }

    // ---- Kernel: the Skia font already created; measure only the bounds query. ----

    pub fn skia_bounds_only(&mut self) {
        let count = self.glyph_count;
        self.font.get_widths_bounds(&self.glyph_indices, None, Some(&mut self.sk_bounds[..count]), None);
    }

    pub fn table_bounds_only(&mut self) {
        let count = self.glyph_count;
        try_get_glyph_bounds(&self.glyph_typeface, &self.glyph_indices, &mut self.bounds[..count]);
    }

    // ---- Realistic per-run: Skia pays the creation of its font; the table path doesn't. ----

    pub fn skia_per_run(&mut self) {
        let font = Self::skia_typeface(&self.glyph_typeface).create_sk_font(SIZE);

        let mut bounds = std::mem::take(&mut self.rented);
        bounds.resize(self.glyph_count, Rect::default());
        font.get_widths_bounds(&self.glyph_indices, None, Some(&mut bounds[..self.glyph_count]), None);
        self.rented = bounds;
    }

    pub fn table_per_run(&self) {
        if self.glyph_count <= MAX_STACK_GLYPHS {
            let mut bounds = [GlyphBounds::default(); MAX_STACK_GLYPHS];
            try_get_glyph_bounds(&self.glyph_typeface, &self.glyph_indices, &mut bounds[..self.glyph_count]);
        } else {
            let mut bounds = vec![GlyphBounds::default(); self.glyph_count];
            try_get_glyph_bounds(&self.glyph_typeface, &self.glyph_indices, &mut bounds);
        }
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("text", "GlyphBoundsBenchmark");
    for glyph_count in [1, 16, 256] {
        let parameters = format!("GlyphCount={glyph_count}");
        class
            .benchmark(
                "skia_bounds_only",
                parameters.clone(),
                move || GlyphBoundsBenchmark::new(glyph_count),
                |b| b.skia_bounds_only(),
            )
            .baseline();
        class.benchmark(
            "table_bounds_only",
            parameters.clone(),
            move || GlyphBoundsBenchmark::new(glyph_count),
            |b| b.table_bounds_only(),
        );
        class.benchmark(
            "skia_per_run",
            parameters.clone(),
            move || GlyphBoundsBenchmark::new(glyph_count),
            |b| b.skia_per_run(),
        );
        class.benchmark(
            "table_per_run",
            parameters,
            move || GlyphBoundsBenchmark::new(glyph_count),
            |b| b.table_per_run(),
        );
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn glyph_bounds_benchmark() {
        crate::harness::smoke_class(super::register, "GlyphBoundsBenchmark");
    }
}
