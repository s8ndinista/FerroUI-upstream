//! Tests of the fragments of the GPU mode: the content of a visual drawn
//! again from what its scene kept gives the pixels its commands give, at
//! the place it was made at moved by whole pixels, and is refused anywhere
//! else.
//!
//! Skipped with a message on a machine without an adapter that runs the
//! compute shaders of the mode.

use crate::drawing_context_impl::{CreateInfo, DrawingContextImpl};
use crate::gpu::VelloWgpuDevice;
use crate::vello_options::VelloRenderingMode;
use ferroui_base::media::immutable::{ImmutablePen, ImmutableSolidColorBrush};
use ferroui_base::media::{BoxShadows, Color, PenLineCap, PenLineJoin};
use ferroui_base::platform::{IDrawingContextImpl, IDrawingContextImplWithFragments, IDrawingFragment};
use ferroui_base::{Matrix, Rect, RoundedRect, Vector};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

const SIZE: u16 = 96;

fn gpu_device(test: &str) -> Option<Arc<VelloWgpuDevice>> {
    if !cfg!(feature = "gpu") {
        return None;
    }
    match VelloWgpuDevice::headless() {
        Ok(device) if device.supports_compute() => Some(device),
        Ok(_) => {
            println!("skipped: the adapter has no compute shaders ({test})");
            None
        }
        Err(error) => {
            println!("skipped: no adapter ({test}): {error}");
            None
        }
    }
}

/// A drawing context of the GPU mode whose scene is rendered into `pixels`
/// when it is disposed.
fn gpu_context(device: &Arc<VelloWgpuDevice>, pixels: &Rc<RefCell<Vec<u8>>>) -> DrawingContextImpl {
    let sink = crate::gpu::create_window_scene_sink(
        device,
        &[VelloRenderingMode::Gpu],
        SIZE,
        SIZE,
        wgpu::TextureFormat::Rgba8Unorm,
    );
    let pixels = pixels.clone();
    DrawingContextImpl::new(
        CreateInfo {
            sink,
            backdrop: None,
            on_finished: Box::new(move |sink| sink.render_to_pixels(&mut pixels.borrow_mut())),
            scale_drawing_to_dpi: false,
            dpi: Vector::new(96.0, 96.0),
            rendering_modes: vec![VelloRenderingMode::Gpu],
        },
        Vec::new(),
    )
}

/// The content of a visual: a rounded rectangle with a border, an ellipse
/// and a line, with curves and strokes the sink flattens and outlines.
fn content(context: &mut dyn IDrawingContextImpl) {
    let fill = ImmutableSolidColorBrush::new(Color::from_argb(255, 30, 120, 200));
    let border = ImmutableSolidColorBrush::new(Color::from_argb(200, 200, 40, 40));
    let pen = ImmutablePen::new(Some(Rc::new(border)), 3.0, None, PenLineCap::Round, PenLineJoin::Miter, 10.0);
    context.draw_rectangle(
        Some(&fill),
        Some(&pen),
        RoundedRect::from_radius(Rect::new(4.3, 5.7, 40.0, 30.0), 7.5),
        &BoxShadows::default(),
    );
    context.draw_ellipse(Some(&fill), Some(&pen), Rect::new(20.25, 30.5, 30.0, 20.0));
    context.draw_line(Some(&pen), ferroui_base::Point::new(2.0, 50.0), ferroui_base::Point::new(45.5, 12.25));
}

fn frame(device: &Arc<VelloWgpuDevice>, draw: impl FnOnce(&mut DrawingContextImpl)) -> Vec<u8> {
    let pixels = Rc::new(RefCell::new(vec![0u8; usize::from(SIZE) * usize::from(SIZE) * 4]));
    let mut context = gpu_context(device, &pixels);
    draw(&mut context);
    context.dispose();
    let pixels = pixels.borrow().clone();
    pixels
}

fn fragments(context: &mut DrawingContextImpl) -> &mut dyn IDrawingContextImplWithFragments {
    context.as_drawing_context_impl_with_fragments().expect("the GPU mode keeps fragments")
}

fn largest_difference(a: &[u8], b: &[u8]) -> u8 {
    a.iter().zip(b).map(|(a, b)| a.abs_diff(*b)).max().unwrap_or(0)
}

/// Makes the fragment of the content at `transform`.
fn make_fragment(device: &Arc<VelloWgpuDevice>, transform: Matrix) -> (Rc<dyn IDrawingFragment>, Vec<u8>) {
    let mut kept = None;
    let pixels = frame(device, |context| {
        context.set_transform(transform);
        fragments(context).begin_fragment();
        content(context);
        kept = fragments(context).end_fragment();
    });
    (kept.expect("the content is kept as a fragment"), pixels)
}

#[test]
fn a_fragment_gives_the_pixels_of_its_commands() {
    let Some(device) = gpu_device("a_fragment_gives_the_pixels_of_its_commands") else { return };
    let transform = Matrix::create_translation(3.0, 2.0);

    let direct = frame(&device, |context| {
        context.set_transform(transform);
        content(context);
    });
    let (fragment, recorded) = make_fragment(&device, transform);
    let drawn_again = frame(&device, |context| {
        context.set_transform(transform);
        assert!(fragments(context).draw_fragment(&fragment));
    });

    assert!(direct.iter().any(|value| *value != 0), "the content is drawn");
    assert_eq!(largest_difference(&direct, &recorded), 0, "making a fragment draws what the commands draw");
    assert_eq!(largest_difference(&direct, &drawn_again), 0, "a fragment drawn again draws the same pixels");
}

#[test]
fn a_fragment_moved_by_whole_pixels_gives_the_pixels_of_its_commands_there() {
    let Some(device) = gpu_device("a_fragment_moved_by_whole_pixels_gives_the_pixels_of_its_commands_there") else {
        return;
    };
    // Twice the scale, as on a display of two pixels a unit: a move of
    // 13.5 by -3.5 units is one of 27 by -7 pixels.
    let made_at = Matrix::create_translation(1.0, 0.5) * Matrix::create_scale(2.0, 2.0);
    let moved_to = Matrix::create_translation(14.5, -3.0) * Matrix::create_scale(2.0, 2.0);

    let (fragment, _) = make_fragment(&device, made_at);
    let direct = frame(&device, |context| {
        context.set_transform(moved_to);
        content(context);
    });
    let drawn_again = frame(&device, |context| {
        context.set_transform(moved_to);
        assert!(fragments(context).draw_fragment(&fragment));
    });

    assert!(largest_difference(&direct, &drawn_again) <= 1, "the moved fragment draws what the commands draw there");
}

#[test]
fn a_fragment_is_refused_where_its_pixels_would_differ() {
    let Some(device) = gpu_device("a_fragment_is_refused_where_its_pixels_would_differ") else { return };
    let (fragment, _) = make_fragment(&device, Matrix::create_translation(3.0, 2.0));

    let empty = vec![0u8; usize::from(SIZE) * usize::from(SIZE) * 4];
    for (case, refused) in [
        ("a move by part of a pixel", Box::new(|context: &mut DrawingContextImpl| {
            context.set_transform(Matrix::create_translation(3.5, 2.0));
        }) as Box<dyn Fn(&mut DrawingContextImpl)>),
        ("another scale", Box::new(|context: &mut DrawingContextImpl| {
            context.set_transform(Matrix::create_scale(1.5, 1.5));
        })),
        ("another opacity", Box::new(|context: &mut DrawingContextImpl| {
            context.set_transform(Matrix::create_translation(3.0, 2.0));
            context.push_opacity(0.5, None);
        })),
    ] {
        let pixels = frame(&device, |context| {
            refused(context);
            assert!(!fragments(context).draw_fragment(&fragment), "{case}: refused");
        });
        assert_eq!(pixels, empty, "{case}: nothing is drawn");
    }
}
