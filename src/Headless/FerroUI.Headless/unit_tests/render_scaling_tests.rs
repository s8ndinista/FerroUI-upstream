//! Tests of the port (not in upstream): a border with a box shadow and a
//! child, drawn at a render scaling of 2, by the window (the compositor)
//! and into a render target bitmap of 192 DPI (the immediate renderer).
//!
//! The child must be where layout put it, in both. The navigation items
//! of the catalog are such borders (a rounded border with a shadow when
//! selected), and a picture of the catalog at a scaling of 2 showed the
//! content of the selected item twice as large and away from its place.
//! See `docs/porting/decisions/render-target-bitmap-scaled-box-shadow.md`.

use super::test_application::ferro_fact;
use crate::HeadlessWindowExtensions;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::imaging::RenderTargetBitmap;
use ferroui_base::media::{BoxShadows, Brushes};
use ferroui_base::platform::PixelFormat;
use ferroui_base::{CornerRadius, PixelRect, PixelSize, Ref, Thickness, Vector, Visual};
use ferroui_controls::{Border, Control, Window};

/// Where the red probe is in the window, in units of the window: its left,
/// top, width and height.
const PROBE: (f64, f64, f64, f64) = (40.0, 30.0, 20.0, 10.0);

/// A window of 200 by 100 with a rounded border with a box shadow at 10,
/// 10, and in it a red border of 20 by 10 at 30, 20 of the shadowed
/// border: at `PROBE` of the window.
fn window_with_shadowed_border() -> Ref<Window> {
    let probe = Border::new();
    probe.set_width(PROBE.2);
    probe.set_height(PROBE.3);
    probe.set_horizontal_alignment(HorizontalAlignment::Left);
    probe.set_vertical_alignment(VerticalAlignment::Top);
    probe.set_margin(Thickness::new(PROBE.0 - 10.0, PROBE.1 - 10.0, 0.0, 0.0));
    probe.set_background(Some(Brushes::red()));

    let shadowed = Border::new();
    shadowed.set_margin(Thickness::uniform(10.0));
    shadowed.set_background(Some(Brushes::white()));
    shadowed.set_corner_radius(CornerRadius::new(8.0, 8.0, 8.0, 8.0));
    shadowed.set_box_shadow(BoxShadows::parse("0 1 4 0 #40000000").unwrap());
    shadowed.set_child(probe);

    let window = Window::new();
    window.set_width(200.0);
    window.set_height(100.0);
    window.set_background(Some(Brushes::white()));
    window.set_content(Some(Control::boxed(shadowed)));
    window
}

/// The bounds, in pixels, of the red pixels of `pixels` (four bytes each,
/// in `format`, `size.width * 4` bytes a row): left, top, right and bottom
/// (exclusive), or `None` when there are none.
fn red_bounds(pixels: &[u8], size: PixelSize, format: PixelFormat) -> Option<(i32, i32, i32, i32)> {
    let (red, blue) = if format == PixelFormat::BGRA8888 { (2, 0) } else { (0, 2) };
    let mut bounds: Option<(i32, i32, i32, i32)> = None;
    for y in 0..size.height {
        for x in 0..size.width {
            let pixel = &pixels[((y * size.width + x) * 4) as usize..][..4];
            if pixel[red] > 200 && pixel[1] < 60 && pixel[blue] < 60 {
                bounds = Some(match bounds {
                    None => (x, y, x + 1, y + 1),
                    Some((l, t, r, b)) => (l.min(x), t.min(y), r.max(x + 1), b.max(y + 1)),
                });
            }
        }
    }
    bounds
}

/// The bounds the probe has at a scaling of 2.
fn expected_bounds() -> (i32, i32, i32, i32) {
    let (x, y, width, height) = PROBE;
    ((x * 2.0) as i32, (y * 2.0) as i32, ((x + width) * 2.0) as i32, ((y + height) * 2.0) as i32)
}

/// The frame the window renders at a scaling of 2 has the probe where
/// layout put it, at its size.
fn shadowed_border_child_is_in_place_in_the_frame_at_scaling_2() {
    let window = window_with_shadowed_border();
    window.show();
    window.set_render_scaling(2.0);

    let frame = window.capture_rendered_frame().expect("a rendered frame");
    let size = frame.pixel_size();
    assert_eq!(PixelSize::new(400, 200), size);
    let mut pixels = vec![0u8; (size.width * size.height * 4) as usize];
    frame.copy_pixels(PixelRect::default(), &mut pixels, size.width * 4);

    let format = frame.format().expect("the format of the frame");
    assert_eq!(Some(expected_bounds()), red_bounds(&pixels, size, format));
}
ferro_fact!(shadowed_border_child_is_in_place_in_the_frame_at_scaling_2);

/// The bounds of the red pixels of the window rendered into a render
/// target bitmap of 192 DPI (400 by 200 pixels).
fn red_bounds_in_a_render_target_bitmap_of_192_dpi() -> Option<(i32, i32, i32, i32)> {
    let window = window_with_shadowed_border();
    window.show();
    window.capture_rendered_frame();

    let size = PixelSize::new(400, 200);
    let bitmap = RenderTargetBitmap::with_dpi(size, Vector::new(192.0, 192.0));
    bitmap.render(&window.clone().upcast::<Visual>());
    let mut pixels = vec![0u8; (size.width * size.height * 4) as usize];
    bitmap.copy_pixels(PixelRect::default(), &mut pixels, size.width * 4);

    let format = bitmap.format().expect("the format of the bitmap");
    red_bounds(&pixels, size, format)
}

/// What a render target bitmap of 192 DPI has, as upstream has it: the
/// probe twice as large and twice as far from the origin of the bitmap.
/// The drawing context of the bitmap scales what it draws to its DPI (a
/// transform after the transform of the context), and after the box shadow
/// it reads the transform of the context back from the canvas, scale
/// included: the transforms pushed for the children of the border are then
/// scaled twice.
fn shadowed_border_child_is_scaled_twice_in_a_render_target_bitmap_of_192_dpi_known_difference() {
    let (left, top, right, bottom) = expected_bounds();
    assert_eq!(Some((left * 2, top * 2, right * 2, bottom * 2)), red_bounds_in_a_render_target_bitmap_of_192_dpi());
}
ferro_fact!(shadowed_border_child_is_scaled_twice_in_a_render_target_bitmap_of_192_dpi_known_difference);

/// A render target bitmap of 192 DPI has the probe where the frame of the
/// window has it: the acceptance test of the fix the decision document
/// describes.
fn shadowed_border_child_is_in_place_in_a_render_target_bitmap_of_192_dpi() {
    assert_eq!(Some(expected_bounds()), red_bounds_in_a_render_target_bitmap_of_192_dpi());
}
ferro_fact!(
    #[ignore = "upstream behaviour, a decision of the owner: docs/porting/decisions/render-target-bitmap-scaled-box-shadow.md"]
    shadowed_border_child_is_in_place_in_a_render_target_bitmap_of_192_dpi
);
