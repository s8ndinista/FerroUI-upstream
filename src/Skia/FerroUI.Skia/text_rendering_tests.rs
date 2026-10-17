//! Tests of the text rendering of the surfaces of the backend: the text
//! rendering parameters of the platform reach the Skia surfaces and decide
//! the default antialiasing of text (docs/porting/text-rendering.md).

use crate::drawing_context_impl::effective_text_options;
use crate::*;
use ferroui_base::media::{
    BaselinePixelAlignment, Colors, EdgeMode, RenderOptions, TextHintingMode, TextOptions, TextRenderingMode,
};
use ferroui_base::platform::surfaces::{
    FramebufferLockProperties, FuncFramebufferRenderTarget, IFramebufferPlatformSurface, IFramebufferRenderTarget,
    IPlatformRenderSurface, PlatformTextRenderingParameters, SubpixelGeometry,
};
use ferroui_base::platform::{
    AlphaFormat, IDrawingContextImpl, ILockedFramebuffer, IPlatformRenderInterface, PixelFormat,
    RenderTargetSceneInfo,
};
use ferroui_base::rendering::composition::CompositionTransparencyLevel;
use ferroui_base::{PixelSize, Vector};
use skia_safe::{PixelGeometry, SurfaceProps};
use std::any::{Any, TypeId};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

fn with_mode(text_rendering_mode: TextRenderingMode) -> TextOptions {
    TextOptions { text_rendering_mode, ..TextOptions::default() }
}

fn subpixel_surface() -> TextRenderingSettings {
    TextRenderingSettings { default_text_rendering_mode: TextRenderingMode::SubpixelAntialias, ..Default::default() }
}

fn mode(
    text_options: TextOptions,
    render_options: RenderOptions,
    disable_subpixel: bool,
    settings: &TextRenderingSettings,
) -> TextRenderingMode {
    effective_text_options(text_options, render_options, disable_subpixel, settings).text_rendering_mode
}

#[test]
fn unspecified_text_takes_the_default_of_the_surface() {
    let greyscale = TextRenderingSettings::default();
    let subpixel = subpixel_surface();
    let none = RenderOptions::default();

    assert_eq!(TextRenderingMode::Antialias, mode(TextOptions::default(), none, false, &greyscale));
    assert_eq!(TextRenderingMode::SubpixelAntialias, mode(TextOptions::default(), none, false, &subpixel));

    // Aliased edges keep giving aliased text: the glyph run maps the
    // unspecified mode to aliased text.
    let aliased = RenderOptions { edge_mode: EdgeMode::Aliased, ..RenderOptions::default() };
    assert_eq!(TextRenderingMode::Unspecified, mode(TextOptions::default(), aliased, false, &subpixel));
}

#[test]
fn explicit_modes_are_kept() {
    let greyscale = TextRenderingSettings::default();
    let subpixel = subpixel_surface();
    let none = RenderOptions::default();

    // An explicit subpixel mode renders subpixel text on a greyscale
    // surface (the behaviour of the reference stays reachable)...
    assert_eq!(
        TextRenderingMode::SubpixelAntialias,
        mode(with_mode(TextRenderingMode::SubpixelAntialias), none, false, &greyscale)
    );
    // ... and greyscale or aliased text on a subpixel surface.
    assert_eq!(TextRenderingMode::Antialias, mode(with_mode(TextRenderingMode::Antialias), none, false, &subpixel));
    assert_eq!(TextRenderingMode::Alias, mode(with_mode(TextRenderingMode::Alias), none, false, &subpixel));

    // The mode of the render options is used before the default.
    let legacy = RenderOptions { text_rendering_mode: TextRenderingMode::Antialias, ..RenderOptions::default() };
    assert_eq!(TextRenderingMode::Antialias, mode(TextOptions::default(), legacy, false, &subpixel));
    let legacy = RenderOptions { text_rendering_mode: TextRenderingMode::SubpixelAntialias, ..RenderOptions::default() };
    assert_eq!(TextRenderingMode::SubpixelAntialias, mode(TextOptions::default(), legacy, false, &greyscale));

    // The other options pass through.
    let options = TextOptions {
        text_rendering_mode: TextRenderingMode::Unspecified,
        text_hinting_mode: TextHintingMode::Light,
        baseline_pixel_alignment: BaselinePixelAlignment::Unaligned,
    };
    let effective = effective_text_options(options, none, false, &subpixel);
    assert_eq!(TextHintingMode::Light, effective.text_hinting_mode);
    assert_eq!(BaselinePixelAlignment::Unaligned, effective.baseline_pixel_alignment);
}

#[test]
fn a_context_without_subpixel_text_maps_subpixel_to_greyscale() {
    let subpixel = subpixel_surface();
    let none = RenderOptions::default();
    assert_eq!(TextRenderingMode::Antialias, mode(TextOptions::default(), none, true, &subpixel));
    assert_eq!(TextRenderingMode::Antialias, mode(with_mode(TextRenderingMode::SubpixelAntialias), none, true, &subpixel));
    assert_eq!(TextRenderingMode::Alias, mode(with_mode(TextRenderingMode::Alias), none, true, &subpixel));
}

#[test]
fn unspecified_hinting_and_baseline_alignment_take_the_defaults_of_the_surface() {
    let none = RenderOptions::default();

    // Without an override the surface leaves them unspecified: strong
    // hinting and snapped baselines when the blob is made.
    let effective = effective_text_options(TextOptions::default(), none, false, &TextRenderingSettings::default());
    assert_eq!(TextHintingMode::Unspecified, effective.text_hinting_mode);
    assert_eq!(BaselinePixelAlignment::Unspecified, effective.baseline_pixel_alignment);

    // The development override sets the defaults of the surface; text that
    // asks for its own keeps it.
    let mut value = TextRenderingOverride::default();
    value.set("hinting", "none").unwrap();
    value.set("baseline_snap", "off").unwrap();
    let settings = value.apply(TextRenderingSettings::default());
    let effective = effective_text_options(TextOptions::default(), none, false, &settings);
    assert_eq!(TextHintingMode::None, effective.text_hinting_mode);
    assert_eq!(BaselinePixelAlignment::Unaligned, effective.baseline_pixel_alignment);

    let explicit = TextOptions {
        text_hinting_mode: TextHintingMode::Strong,
        baseline_pixel_alignment: BaselinePixelAlignment::Aligned,
        ..TextOptions::default()
    };
    let effective = effective_text_options(explicit, none, false, &settings);
    assert_eq!(TextHintingMode::Strong, effective.text_hinting_mode);
    assert_eq!(BaselinePixelAlignment::Aligned, effective.baseline_pixel_alignment);
}

/// A framebuffer surface whose platform reports text rendering parameters
/// that a test changes.
struct TextSurface {
    pixels: Arc<Mutex<Vec<u8>>>,
    size: PixelSize,
    parameters: Mutex<Option<PlatformTextRenderingParameters>>,
}

impl TextSurface {
    fn new(parameters: Option<PlatformTextRenderingParameters>) -> Arc<Self> {
        let size = PixelSize::new(16, 16);
        Arc::new(Self {
            pixels: Arc::new(Mutex::new(vec![0; (size.width * size.height * 4) as usize])),
            size,
            parameters: Mutex::new(parameters),
        })
    }
}

struct Framebuffer {
    pixels: Arc<Mutex<Vec<u8>>>,
    size: PixelSize,
}

impl ILockedFramebuffer for Framebuffer {
    fn address(&self) -> *mut u8 {
        self.pixels.lock().unwrap().as_mut_ptr()
    }
    fn with_data(&self, access: &mut dyn FnMut(&mut [u8])) {
        access(&mut self.pixels.lock().unwrap());
    }
    fn size(&self) -> PixelSize {
        self.size
    }
    fn row_bytes(&self) -> i32 {
        self.size.width * 4
    }
    fn dpi(&self) -> Vector {
        Vector::new(96.0, 96.0)
    }
    fn format(&self) -> PixelFormat {
        PixelFormat::BGRA8888
    }
    fn alpha_format(&self) -> AlphaFormat {
        AlphaFormat::Premul
    }
    fn dispose(&self) {}
}

impl IPlatformRenderSurface for TextSurface {
    fn as_framebuffer_surface(&self) -> Option<&dyn IFramebufferPlatformSurface> {
        Some(self)
    }

    fn text_rendering_parameters(&self) -> Option<PlatformTextRenderingParameters> {
        *self.parameters.lock().unwrap()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IFramebufferPlatformSurface for TextSurface {
    fn create_framebuffer_render_target(&self) -> Rc<dyn IFramebufferRenderTarget> {
        let (pixels, size) = (self.pixels.clone(), self.size);
        Rc::new(FuncFramebufferRenderTarget::with_scene_info(
            move |_| {
                let framebuffer: Rc<dyn ILockedFramebuffer> = Rc::new(Framebuffer { pixels: pixels.clone(), size });
                (framebuffer, FramebufferLockProperties { previous_frame_is_retained: true })
            },
            true,
        ))
    }
}

/// The surface properties of the Skia surface a drawing context draws to.
fn surface_props(context: &mut dyn IDrawingContextImpl) -> SurfaceProps {
    let feature = context
        .get_feature(TypeId::of::<dyn ISkiaApiLeaseFeature>())
        .and_then(|feature| feature.downcast_ref::<Rc<dyn ISkiaApiLeaseFeature>>().cloned())
        .expect("surface backed contexts provide the lease feature");
    let lease = feature.lease();
    let props = *lease.sk_surface().expect("a surface").props();
    lease.dispose();
    props
}

fn clear_type() -> PlatformTextRenderingParameters {
    PlatformTextRenderingParameters {
        subpixel_antialiasing: true,
        subpixel_geometry: SubpixelGeometry::BgrHorizontal,
        composited: false,
        text_contrast: Some(1.0),
        text_gamma: Some(1.8),
    }
}

#[test]
fn the_parameters_of_the_platform_reach_the_surface_of_every_frame() {
    let render_interface = crate::tests::render_interface();
    let backend = render_interface.create_backend_context(None);

    let surface = TextSurface::new(Some(clear_type()));
    let surfaces: Vec<Arc<dyn IPlatformRenderSurface>> = vec![surface.clone()];
    let render_target = backend.create_render_target(&surfaces);

    let frame = |scaling| {
        let scene_info = RenderTargetSceneInfo::new(surface.size, scaling, CompositionTransparencyLevel::None);
        let (mut context, _) = render_target.create_drawing_context(&scene_info);
        context.clear(Colors::WHITE);
        let props = surface_props(&mut *context);
        context.dispose();
        props
    };

    let props = frame(1.0);
    assert_eq!(PixelGeometry::BGRH, props.pixel_geometry());
    assert_eq!(1.0, props.text_contrast());
    assert_eq!(1.8, props.text_gamma());

    // The user tuned the text: the next frame has the new values.
    *surface.parameters.lock().unwrap() = Some(PlatformTextRenderingParameters { text_contrast: Some(0.25), ..clear_type() });
    assert_eq!(0.25, frame(2.0).text_contrast());

    render_target.dispose();
    backend.dispose();
}

#[test]
fn a_surface_without_parameters_keeps_the_geometry_of_the_reference_and_the_defaults_of_skia() {
    let render_interface = crate::tests::render_interface();
    let backend = render_interface.create_backend_context(None);

    let surface = TextSurface::new(None);
    let surfaces: Vec<Arc<dyn IPlatformRenderSurface>> = vec![surface.clone()];
    let render_target = backend.create_render_target(&surfaces);
    let scene_info = RenderTargetSceneInfo::new(surface.size, 1.0, CompositionTransparencyLevel::None);
    let (mut context, _) = render_target.create_drawing_context(&scene_info);

    let props = surface_props(&mut *context);
    assert_eq!(PixelGeometry::RGBH, props.pixel_geometry());
    assert_eq!(SurfaceProps::default().text_contrast(), props.text_contrast());
    assert_eq!(SurfaceProps::default().text_gamma(), props.text_gamma());

    // Layers of the context have the surface properties of the context.
    let layer = context.create_layer(PixelSize::new(4, 4));
    let mut layer_context = layer.create_drawing_context();
    assert_eq!(props, surface_props(&mut *layer_context));
    layer_context.dispose();
    layer.dispose();

    context.dispose();
    render_target.dispose();
    backend.dispose();
}

#[test]
fn layers_take_the_text_rendering_of_their_surface() {
    let render_interface = crate::tests::render_interface();
    let backend = render_interface.create_backend_context(None);

    let surface = TextSurface::new(Some(clear_type()));
    let surfaces: Vec<Arc<dyn IPlatformRenderSurface>> = vec![surface.clone()];
    let render_target = backend.create_render_target(&surfaces);
    let scene_info = RenderTargetSceneInfo::new(surface.size, 1.0, CompositionTransparencyLevel::None);
    let (mut context, _) = render_target.create_drawing_context(&scene_info);

    let layer = context.create_layer(PixelSize::new(4, 4));
    let mut layer_context = layer.create_drawing_context();
    let props = surface_props(&mut *layer_context);
    assert_eq!(PixelGeometry::BGRH, props.pixel_geometry());
    assert_eq!(1.0, props.text_contrast());
    assert_eq!(1.8, props.text_gamma());
    layer_context.dispose();
    layer.dispose();

    context.dispose();
    render_target.dispose();
    backend.dispose();
}

/// The composition target keeps the layer of a window between frames and creates it again only
/// when it is corrupted (`ServerCompositionTarget::render`): a layer whose surface properties are
/// not those of the window's frame any more reports itself corrupted, so that a change of the
/// parameters of the platform (or of the development override of a calibration host) reaches the
/// text of the window from the next frame.
#[test]
fn a_layer_of_a_window_is_stale_once_the_text_rendering_of_the_frames_changes() {
    let render_interface = crate::tests::render_interface();
    let backend = render_interface.create_backend_context(None);

    let surface = TextSurface::new(Some(clear_type()));
    let surfaces: Vec<Arc<dyn IPlatformRenderSurface>> = vec![surface.clone()];
    let render_target = backend.create_render_target(&surfaces);
    let scene_info = RenderTargetSceneInfo::new(surface.size, 1.0, CompositionTransparencyLevel::None);

    let (mut context, _) = render_target.create_drawing_context(&scene_info);
    let layer = context.create_layer(PixelSize::new(4, 4));
    // A layer of a layer follows the frames of the window as well.
    let mut layer_context = layer.create_drawing_context();
    let inner = layer_context.create_layer(PixelSize::new(2, 2));
    layer_context.dispose();
    context.dispose();
    assert!(!layer.is_corrupted() && !inner.is_corrupted());

    // A frame with the same parameters keeps the layer.
    let (mut context, _) = render_target.create_drawing_context(&scene_info);
    context.dispose();
    assert!(!layer.is_corrupted());

    // The user tuned the text: the layer is stale from the next frame on.
    *surface.parameters.lock().unwrap() = Some(PlatformTextRenderingParameters { text_contrast: Some(0.25), ..clear_type() });
    let (mut context, _) = render_target.create_drawing_context(&scene_info);
    assert!(layer.is_corrupted() && inner.is_corrupted());
    // The layer created again has the new properties and is not stale.
    let fresh = context.create_layer(PixelSize::new(4, 4));
    let mut fresh_context = fresh.create_drawing_context();
    assert_eq!(0.25, surface_props(&mut *fresh_context).text_contrast());
    fresh_context.dispose();
    context.dispose();
    assert!(!fresh.is_corrupted());

    // An offscreen layer (no window) is never stale for its text.
    let offscreen = backend.create_offscreen_render_target(PixelSize::new(4, 4), Vector::new(1.0, 1.0), true);
    assert!(!offscreen.is_corrupted());

    for layer in [layer, inner, fresh, offscreen] {
        layer.dispose();
    }
    render_target.dispose();
    backend.dispose();
}
