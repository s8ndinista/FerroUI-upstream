//! A real CPU backend and real glyph shaping, with a mock window host only.
//! Images are evidence of native rendering, not upstream pixel comparisons.

use crate::ChartsDemo;
use ferroui_base::media::imaging::{PngBitmapEncoderOptions, RenderTargetBitmap};
use ferroui_base::platform::ITextShaperImpl;
use ferroui_base::{FerroLocator, PixelRect, PixelSize, Rect, Size};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Control, UserControl, Window};
use ferroui_harfbuzz::HarfBuzzTextShaper;
use ferroui_themes_winui::WinUITheme;
use ferroui_vello::{VelloOptions, VelloPlatform, VelloRenderingMode};
use std::path::Path;
use std::rc::Rc;

fn render(view: &UserControl, path: &Path) -> Vec<u8> {
    let size = Size::new(1120.0, 880.0);
    view.measure(size);
    view.arrange(Rect::from_size(size));
    let bitmap = RenderTargetBitmap::new(PixelSize::new(1120, 880));
    bitmap.render(&view.to_ref().upcast());
    let mut pixels = vec![0; 1120 * 880 * 4];
    bitmap.copy_pixels(PixelRect::new(0, 0, 1120, 880), &mut pixels, 1120 * 4);
    let colored = pixels
        .chunks_exact(4)
        .filter(|p| p[0].abs_diff(p[1]) > 24 || p[1].abs_diff(p[2]) > 24)
        .count();
    assert!(colored > 1500, "The actual backend image must contain the colored chart paths, fills and markers ({colored} pixels)");
    bitmap
        .save_to_file(
            path.to_str().unwrap(),
            &PngBitmapEncoderOptions::default().into(),
        )
        .unwrap();
    bitmap.dispose();
    println!("Rendered {}", path.display());
    pixels
}

#[test]
fn charts_demo_renders_native_light_dark_and_live_mutations() {
    let locator = FerroLocator::enter_scope();
    VelloPlatform::initialize_with_options(VelloOptions::with_rendering_mode(
        VelloRenderingMode::Cpu,
    ));
    FerroLocator::current_mutable()
        .bind::<dyn ITextShaperImpl>()
        .to_constant(Rc::new(HarfBuzzTextShaper::new()));
    let mut services = TestServices::styled_window();
    // Preserve the real renderer/font manager/shaper installed above.
    services.render_interface = None;
    services.font_manager_impl = None;
    services.text_shaper_impl = None;
    services.theme = Some(Rc::new(|| WinUITheme::new().as_style()));
    let _app = UnitTestApplication::start(services);
    let demo = ChartsDemo::new();
    let view = demo.view();
    let window = Window::new();
    window.set_width(1120.0);
    window.set_height(880.0);
    window.set_content(Some(Control::boxed(&view)));
    window.show();
    window.update_layout();

    let directory = std::env::var_os("FERROUI_CHARTS_RENDER_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("ferroui-charts-render"));
    std::fs::create_dir_all(&directory).unwrap();
    let light = render(&view, &directory.join("charts-light.png"));
    demo.toggle_theme();
    let dark = render(&view, &directory.join("charts-dark.png"));
    assert_ne!(
        light, dark,
        "A native theme change must alter rendered pixels"
    );
    demo.add_sample();
    demo.toggle_bars();
    demo.toggle_legend();
    let changed = render(&view, &directory.join("charts-live-mutation.png"));
    assert_ne!(
        dark, changed,
        "Live samples, orientation and legends must alter rendered pixels"
    );
    assert_eq!(demo.sample_count(), 7);
    window.close();
    drop(window);
    drop(view);
    drop(demo);
    drop(_app);
    locator.dispose();
}
