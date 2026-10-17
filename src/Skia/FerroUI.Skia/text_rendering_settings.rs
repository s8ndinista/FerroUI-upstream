//! How text is rendered on a surface of the backend: the pixel geometry,
//! contrast and gamma of its Skia surface properties and the antialiasing
//! text gets when nothing asks for a mode (docs/porting/text-rendering.md).
//!
//! The reference renders text with subpixel antialiasing and assumes an
//! RGB display on every surface (`SKPixelGeometry.RgbHorizontal`, Skia's
//! default contrast and gamma). FerroUI decides per surface from what the
//! platform reports (`IPlatformRenderSurface::text_rendering_parameters`),
//! the render scaling and the transparency of the scene: a FerroUI
//! deviation (docs/porting/DEVIATIONS.md, area `Text rendering`).

use crate::text_rendering_override::text_rendering_override;
use ferroui_base::media::{BaselinePixelAlignment, TextHintingMode, TextRenderingMode};
use ferroui_base::platform::surfaces::{IPlatformRenderSurface, PlatformTextRenderingParameters, SubpixelGeometry};
use ferroui_base::rendering::composition::CompositionTransparencyLevel;
use skia_safe::{PixelGeometry, SurfaceProps, SurfacePropsFlags};
use std::sync::Arc;

/// From this render scaling on, text is antialiased in greyscale by
/// default: the pixels are small enough that subpixel antialiasing gains
/// nothing visible, and its colour fringes remain (Chromium and the
/// platforms switch to greyscale on high density displays as well).
pub const GREYSCALE_TEXT_SCALING: f64 = 1.5;

/// The text rendering of a surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextRenderingSettings {
    /// The pixel geometry of the surface: the one of the display when the
    /// platform knows it, else horizontal RGB (the assumption of the
    /// reference), so that an explicit
    /// [`TextRenderingMode::SubpixelAntialias`] still renders subpixel text.
    pub pixel_geometry: PixelGeometry,

    /// The contrast Skia adds to the coverage of text (0 to 1).
    pub text_contrast: f32,

    /// The gamma Skia corrects the coverage of text for (0 is sRGB).
    pub text_gamma: f32,

    /// The antialiasing of text whose text options and render options leave
    /// the mode unspecified: [`TextRenderingMode::Antialias`] or
    /// [`TextRenderingMode::SubpixelAntialias`].
    pub default_text_rendering_mode: TextRenderingMode,

    /// The hinting of text whose text options leave it unspecified:
    /// [`TextHintingMode::Unspecified`] (strong hinting) unless the
    /// development override sets one (`text_rendering_override.rs`).
    pub default_text_hinting_mode: TextHintingMode,

    /// The baseline pixel alignment of text whose text options leave it
    /// unspecified: [`BaselinePixelAlignment::Unspecified`] (snapped to the
    /// pixel grid) unless the development override sets one.
    pub default_baseline_pixel_alignment: BaselinePixelAlignment,
}

impl Default for TextRenderingSettings {
    /// The settings of a surface the platform says nothing about (an
    /// offscreen surface, a render target bitmap): greyscale text by
    /// default, Skia's own contrast and gamma, horizontal RGB for explicit
    /// subpixel text.
    fn default() -> Self {
        let defaults = SurfaceProps::default();
        Self {
            pixel_geometry: PixelGeometry::RGBH,
            text_contrast: defaults.text_contrast(),
            text_gamma: defaults.text_gamma(),
            default_text_rendering_mode: TextRenderingMode::Antialias,
            default_text_hinting_mode: TextHintingMode::Unspecified,
            default_baseline_pixel_alignment: BaselinePixelAlignment::Unspecified,
        }
    }
}

impl TextRenderingSettings {
    /// The settings of a scene rendered to a surface with the given
    /// parameters of the platform (`None`: the platform reports none) at a
    /// render scaling and a transparency level.
    ///
    /// Text is subpixel antialiased by default only when the platform
    /// renders subpixel text on the display, the pixel geometry is known,
    /// the surface is not composed by a system compositor, the scene is
    /// opaque and the render scaling is below
    /// [`GREYSCALE_TEXT_SCALING`]; it is greyscale otherwise, which is what
    /// WinUI 3, macOS, iOS, Android and the browsers do.
    pub fn for_scene(
        parameters: Option<PlatformTextRenderingParameters>,
        scaling: f64,
        transparency_level: CompositionTransparencyLevel,
    ) -> Self {
        let mut settings = Self::default();
        let Some(parameters) = parameters else {
            return settings;
        };

        let geometry = to_pixel_geometry(parameters.subpixel_geometry);
        if geometry != PixelGeometry::Unknown {
            settings.pixel_geometry = geometry;
        }
        if let Some(contrast) = parameters.text_contrast {
            settings.text_contrast = clamp_contrast(contrast);
        }
        if let Some(gamma) = parameters.text_gamma {
            settings.text_gamma = clamp_gamma(gamma);
        }

        let subpixel = parameters.subpixel_antialiasing
            && geometry != PixelGeometry::Unknown
            && !parameters.composited
            && transparency_level == CompositionTransparencyLevel::None
            && scaling < GREYSCALE_TEXT_SCALING;
        if subpixel {
            settings.default_text_rendering_mode = TextRenderingMode::SubpixelAntialias;
        }

        settings
    }

    /// The Skia surface properties of a surface with these settings.
    pub fn surface_props(&self) -> SurfaceProps {
        SurfaceProps::new_with_text_properties(
            SurfacePropsFlags::default(),
            self.pixel_geometry,
            self.text_contrast,
            self.text_gamma,
        )
    }
}

/// Where a render target reads the text rendering parameters of the
/// platform from: the surfaces of the top-level it renders, asked every
/// frame.
#[derive(Clone, Default)]
pub struct TextRenderingSource {
    surfaces: Vec<Arc<dyn IPlatformRenderSurface>>,
}

impl TextRenderingSource {
    /// The source over the surfaces a render target is created for: those
    /// that report parameters.
    pub fn new(surfaces: &[Arc<dyn IPlatformRenderSurface>]) -> Self {
        Self {
            surfaces: surfaces
                .iter()
                .filter(|surface| surface.text_rendering_parameters().is_some())
                .cloned()
                .collect(),
        }
    }

    /// The parameters the platform reports now: those of the first surface
    /// that reports any.
    pub fn parameters(&self) -> Option<PlatformTextRenderingParameters> {
        self.surfaces.iter().find_map(|surface| surface.text_rendering_parameters())
    }

    /// The settings of a frame at a render scaling and a transparency level,
    /// with the development override of the process applied (none unless
    /// its environment variables are set; `text_rendering_override.rs`).
    pub fn settings(&self, scaling: f64, transparency_level: CompositionTransparencyLevel) -> TextRenderingSettings {
        text_rendering_override().apply(TextRenderingSettings::for_scene(self.parameters(), scaling, transparency_level))
    }
}

fn to_pixel_geometry(geometry: SubpixelGeometry) -> PixelGeometry {
    match geometry {
        SubpixelGeometry::Unknown => PixelGeometry::Unknown,
        SubpixelGeometry::RgbHorizontal => PixelGeometry::RGBH,
        SubpixelGeometry::BgrHorizontal => PixelGeometry::BGRH,
        SubpixelGeometry::RgbVertical => PixelGeometry::RGBV,
        SubpixelGeometry::BgrVertical => PixelGeometry::BGRV,
    }
}

/// The contrast Skia accepts: 0 to 1 (`SkSurfaceProps::kMinContrastInclusive`
/// and `kMaxContrastInclusive`), as Chromium clamps it.
pub(crate) fn clamp_contrast(contrast: f32) -> f32 {
    if contrast.is_nan() {
        return SurfaceProps::default().text_contrast();
    }
    contrast.clamp(0.0, 1.0)
}

/// The gamma Skia accepts: from 0 up to 4 exclusive
/// (`SkSurfaceProps::kMinGammaInclusive` and `kMaxGammaExclusive`), as
/// Chromium clamps it.
pub(crate) fn clamp_gamma(gamma: f32) -> f32 {
    if gamma.is_nan() {
        return SurfaceProps::default().text_gamma();
    }
    gamma.clamp(0.0, 4.0 - 4.0 * f32::EPSILON)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clear_type() -> PlatformTextRenderingParameters {
        PlatformTextRenderingParameters {
            subpixel_antialiasing: true,
            subpixel_geometry: SubpixelGeometry::RgbHorizontal,
            composited: false,
            text_contrast: Some(1.0),
            text_gamma: Some(0.0),
        }
    }

    const OPAQUE: CompositionTransparencyLevel = CompositionTransparencyLevel::None;

    #[test]
    fn without_parameters_text_is_greyscale_with_the_defaults_of_skia() {
        let settings = TextRenderingSettings::for_scene(None, 1.0, OPAQUE);
        assert_eq!(TextRenderingMode::Antialias, settings.default_text_rendering_mode);
        assert_eq!(PixelGeometry::RGBH, settings.pixel_geometry, "explicit subpixel text stays reachable");
        let defaults = SurfaceProps::default();
        assert_eq!(defaults.text_contrast(), settings.text_contrast);
        assert_eq!(defaults.text_gamma(), settings.text_gamma);
        assert_eq!(settings, TextRenderingSettings::default());
    }

    #[test]
    fn subpixel_text_is_the_default_on_an_opaque_low_density_clear_type_surface() {
        let settings = TextRenderingSettings::for_scene(Some(clear_type()), 1.0, OPAQUE);
        assert_eq!(TextRenderingMode::SubpixelAntialias, settings.default_text_rendering_mode);
        assert_eq!(PixelGeometry::RGBH, settings.pixel_geometry);
        assert_eq!(1.0, settings.text_contrast);
        assert_eq!(0.0, settings.text_gamma);

        let settings = TextRenderingSettings::for_scene(Some(clear_type()), 1.25, OPAQUE);
        assert_eq!(TextRenderingMode::SubpixelAntialias, settings.default_text_rendering_mode);
    }

    #[test]
    fn high_density_composited_and_transparent_surfaces_get_greyscale_text() {
        let greyscale = |parameters, scaling, level| {
            TextRenderingSettings::for_scene(Some(parameters), scaling, level).default_text_rendering_mode
                == TextRenderingMode::Antialias
        };

        assert!(greyscale(clear_type(), 1.5, OPAQUE));
        assert!(greyscale(clear_type(), 2.0, OPAQUE));
        assert!(greyscale(PlatformTextRenderingParameters { composited: true, ..clear_type() }, 1.0, OPAQUE));
        assert!(greyscale(clear_type(), 1.0, CompositionTransparencyLevel::Transparent));
        assert!(greyscale(clear_type(), 1.0, CompositionTransparencyLevel::Mica));
        assert!(greyscale(PlatformTextRenderingParameters { subpixel_antialiasing: false, ..clear_type() }, 1.0, OPAQUE));
        assert!(greyscale(
            PlatformTextRenderingParameters { subpixel_geometry: SubpixelGeometry::Unknown, ..clear_type() },
            1.0,
            OPAQUE
        ));
    }

    #[test]
    fn the_geometry_of_the_display_reaches_the_surface_even_when_text_is_greyscale() {
        // An explicit SubpixelAntialias renders for the real display.
        let parameters = PlatformTextRenderingParameters { subpixel_geometry: SubpixelGeometry::BgrHorizontal, ..clear_type() };
        let settings = TextRenderingSettings::for_scene(Some(parameters), 2.0, OPAQUE);
        assert_eq!(PixelGeometry::BGRH, settings.pixel_geometry);
        assert_eq!(TextRenderingMode::Antialias, settings.default_text_rendering_mode);

        for (geometry, expected) in [
            (SubpixelGeometry::RgbVertical, PixelGeometry::RGBV),
            (SubpixelGeometry::BgrVertical, PixelGeometry::BGRV),
            (SubpixelGeometry::Unknown, PixelGeometry::RGBH),
        ] {
            let parameters = PlatformTextRenderingParameters { subpixel_geometry: geometry, ..clear_type() };
            assert_eq!(expected, TextRenderingSettings::for_scene(Some(parameters), 1.0, OPAQUE).pixel_geometry);
        }
    }

    #[test]
    fn contrast_and_gamma_are_clamped_to_what_skia_accepts() {
        let settings = |contrast, gamma| {
            TextRenderingSettings::for_scene(
                Some(PlatformTextRenderingParameters {
                    text_contrast: Some(contrast),
                    text_gamma: Some(gamma),
                    ..clear_type()
                }),
                1.0,
                OPAQUE,
            )
        };
        let clamped = settings(3.0, 9.0);
        assert_eq!(1.0, clamped.text_contrast);
        assert!(clamped.text_gamma < 4.0 && clamped.text_gamma > 3.99);
        let clamped = settings(-1.0, -2.0);
        assert_eq!(0.0, clamped.text_contrast);
        assert_eq!(0.0, clamped.text_gamma);
        let defaults = SurfaceProps::default();
        let unknown = settings(f32::NAN, f32::NAN);
        assert_eq!(defaults.text_contrast(), unknown.text_contrast);
        assert_eq!(defaults.text_gamma(), unknown.text_gamma);

        // Values the platform leaves unset keep Skia's defaults.
        let unset = TextRenderingSettings::for_scene(
            Some(PlatformTextRenderingParameters { text_contrast: None, text_gamma: None, ..clear_type() }),
            1.0,
            OPAQUE,
        );
        assert_eq!(defaults.text_contrast(), unset.text_contrast);
        assert_eq!(defaults.text_gamma(), unset.text_gamma);
    }

    #[test]
    fn the_surface_props_carry_the_settings() {
        let settings = TextRenderingSettings::for_scene(
            Some(PlatformTextRenderingParameters {
                subpixel_geometry: SubpixelGeometry::BgrHorizontal,
                text_contrast: Some(0.75),
                text_gamma: Some(1.8),
                ..clear_type()
            }),
            1.0,
            OPAQUE,
        );
        let props = settings.surface_props();
        assert_eq!(PixelGeometry::BGRH, props.pixel_geometry());
        assert_eq!(0.75, props.text_contrast());
        assert_eq!(1.8, props.text_gamma());
        assert_eq!(SurfacePropsFlags::default(), props.flags());
    }

    struct Surface(Option<PlatformTextRenderingParameters>, std::sync::Mutex<Option<PlatformTextRenderingParameters>>);

    impl IPlatformRenderSurface for Surface {
        fn text_rendering_parameters(&self) -> Option<PlatformTextRenderingParameters> {
            *self.1.lock().unwrap()
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    fn surface(parameters: Option<PlatformTextRenderingParameters>) -> Arc<Surface> {
        Arc::new(Surface(parameters, std::sync::Mutex::new(parameters)))
    }

    #[test]
    fn the_source_asks_the_surfaces_that_report_parameters_every_time() {
        let silent = surface(None);
        let reporting = surface(Some(clear_type()));
        let surfaces: Vec<Arc<dyn IPlatformRenderSurface>> = vec![silent.clone(), reporting.clone()];
        let source = TextRenderingSource::new(&surfaces);

        assert_eq!(Some(clear_type()), source.parameters());
        assert_eq!(TextRenderingMode::SubpixelAntialias, source.settings(1.0, OPAQUE).default_text_rendering_mode);

        // The system turned ClearType off: the next frame follows.
        *reporting.1.lock().unwrap() = Some(PlatformTextRenderingParameters { subpixel_antialiasing: false, ..clear_type() });
        assert_eq!(TextRenderingMode::Antialias, source.settings(1.0, OPAQUE).default_text_rendering_mode);

        let none = TextRenderingSource::new(&[silent.clone() as Arc<dyn IPlatformRenderSurface>]);
        assert_eq!(None, none.parameters());
        assert_eq!(TextRenderingSettings::default(), none.settings(1.0, OPAQUE));
        assert!(reporting.0.is_some());
    }
}
