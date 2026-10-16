//! What the platform says about how text is rendered on a surface: the
//! settings of the system a render backend needs to render text the way
//! the native text of the platform looks (docs/porting/text-rendering.md).
//!
//! This is FerroUI's own addition (docs/porting/DEVIATIONS.md, area
//! `Text rendering`): the reference renders text the same way on every
//! surface.

/// The arrangement of the colour stripes of the pixels of a display, which
/// subpixel (LCD) text antialiasing is rendered for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SubpixelGeometry {
    /// Not known: subpixel text cannot be rendered for the display.
    #[default]
    Unknown,
    /// Red, green and blue from left to right.
    RgbHorizontal,
    /// Blue, green and red from left to right.
    BgrHorizontal,
    /// Red, green and blue from top to bottom.
    RgbVertical,
    /// Blue, green and red from top to bottom.
    BgrVertical,
}

/// The text rendering settings of the platform for a render surface.
///
/// A surface reports them through
/// [`IPlatformRenderSurface::text_rendering_parameters`](super::IPlatformRenderSurface::text_rendering_parameters)
/// when the platform knows them; a surface that reports nothing gets the
/// defaults of the backend (greyscale antialiasing).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlatformTextRenderingParameters {
    /// Whether the system renders text with subpixel antialiasing on the
    /// display of the surface: ClearType on Windows (`SPI_GETFONTSMOOTHING`
    /// and `SPI_GETFONTSMOOTHINGTYPE`), `Xft.rgba` on X11.
    pub subpixel_antialiasing: bool,

    /// The arrangement of the subpixels of the display, when it is known
    /// (`SPI_GETFONTSMOOTHINGORIENTATION` on Windows).
    pub subpixel_geometry: SubpixelGeometry,

    /// Whether the surface is composed with other content by a system
    /// compositor (Windows.UI.Composition, DirectComposition, a swap chain
    /// panel of a WinUI host). Native frameworks render greyscale text on
    /// such surfaces (WinUI 3 does), so subpixel text is not the default
    /// there.
    pub composited: bool,

    /// The contrast added to the coverage of text, from 0 to 1, or `None`
    /// for the default of the backend.
    pub text_contrast: Option<f32>,

    /// The gamma the coverage of text is corrected for, from 0 (exclusive
    /// of 4), where 0 means sRGB, or `None` for the default of the backend.
    pub text_gamma: Option<f32>,
}
