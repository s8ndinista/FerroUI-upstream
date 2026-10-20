//! The frame of a window drawn by the GPU modes: what a window holds
//! between its frames, and what the texture a window presents gets a copy
//! of. Shared by the windows of the macOS platform (`metal.rs`) and those
//! drawn through a surface of `wgpu` (`window_surface.rs`).

use crate::drawing_context_impl::DrawingContextImpl;
use crate::gpu::i_vello_gpu::window_rendering_mode;
use crate::gpu::{DeviceSurfaceRenderTarget, VelloWgpuDevice};
use crate::i_drawable_bitmap_impl::IDrawableBitmapImpl;
use crate::scene::VelloScenePixelRect;
use crate::surface_render_target::{SurfaceChange, SurfaceRenderTarget, SurfaceRenderTargetCreateInfo};
use crate::vello_options::VelloRenderingMode;
use ferroui_base::platform::{IBitmapImpl, IDrawingContextLayerImpl};
use ferroui_base::PixelSize;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

/// What a window holds between its frames: the surface its scenes are
/// rendered into. The texture of each drawable is a copy of it.
pub(crate) enum WindowFrame {
    /// A texture of the device: the hybrid and the GPU mode.
    Device(DeviceSurfaceRenderTarget),
    /// Pixels in memory: the CPU mode, with a texture of the device that
    /// holds the same pixels once a frame was copied, and into which only
    /// what the scenes changed is uploaded.
    Memory(SurfaceRenderTarget, RefCell<Option<wgpu::Texture>>),
}

impl WindowFrame {
    pub(crate) fn pixel_size(&self) -> PixelSize {
        match self {
            WindowFrame::Device(surface) => surface.pixel_size(),
            WindowFrame::Memory(surface, _) => surface.pixel_size(),
        }
    }

    pub(crate) fn dpi(&self) -> ferroui_base::Vector {
        match self {
            WindowFrame::Device(surface) => surface.dpi(),
            WindowFrame::Memory(surface, _) => surface.dpi(),
        }
    }

    pub(crate) fn has_content(&self) -> bool {
        match self {
            WindowFrame::Device(surface) => surface.has_content(),
            WindowFrame::Memory(surface, _) => surface.has_content(),
        }
    }

    pub(crate) fn is_corrupted(&self) -> bool {
        match self {
            WindowFrame::Device(surface) => surface.is_corrupted(),
            WindowFrame::Memory(surface, _) => surface.is_corrupted(),
        }
    }

    pub(crate) fn create_drawing_context_with(&self, disposables: Vec<Box<dyn FnOnce()>>) -> DrawingContextImpl {
        match self {
            WindowFrame::Device(surface) => surface.create_drawing_context_with(disposables),
            WindowFrame::Memory(surface, _) => surface.create_drawing_context_with(disposables),
        }
    }

    /// Copies what the frame holds into the texture of a drawable.
    pub(crate) fn copy_to(&self, device: &VelloWgpuDevice, target: &wgpu::Texture) {
        match self {
            WindowFrame::Device(surface) => {
                if let Some(texture) = surface.texture() {
                    let premultiply = texture.alpha() == crate::gpu::VelloTextureAlpha::Straight;
                    device.copy_texture_to_texture(texture.view(), target, premultiply);
                }
            }
            WindowFrame::Memory(surface, mirror) => {
                let Some(image) = IDrawableBitmapImpl::image(surface) else {
                    return;
                };
                let (width, height) = (image.width, image.height);
                let mut mirror = mirror.borrow_mut();
                let changed = surface.take_changed();
                let fits = mirror.as_ref().is_some_and(|texture| texture.width() == width && texture.height() == height);
                let changed = match fits {
                    true => changed,
                    false => {
                        *mirror = Some(device.create_pixel_texture(width, height));
                        Some(SurfaceChange::All)
                    }
                };
                let mirror = mirror.as_ref().expect("the texture of the frame was just made");
                let rect = match changed {
                    None => None,
                    Some(SurfaceChange::All) => {
                        Some(VelloScenePixelRect { x0: 0, y0: 0, x1: width as u16, y1: height as u16 })
                    }
                    Some(SurfaceChange::Rect(rect)) => Some(rect),
                };
                if let Some(rect) = rect {
                    device.write_pixels_to_texture(image.data.data(), width, rect, mirror);
                }
                let view = mirror.create_view(&wgpu::TextureViewDescriptor::default());
                device.copy_texture_to_texture(&view, target, false);
            }
        }
    }

    pub(crate) fn dispose(&self) {
        match self {
            WindowFrame::Device(surface) => IBitmapImpl::dispose(surface),
            WindowFrame::Memory(surface, _) => IBitmapImpl::dispose(surface),
        }
    }
}

/// The frame of a window from frame to frame.
pub(crate) struct WindowFrames {
    device: Arc<VelloWgpuDevice>,
    rendering_modes: Vec<VelloRenderingMode>,
    /// The frame of the window, once a frame was drawn: of the size and
    /// the scaling of the last frame.
    frame: RefCell<Option<Rc<WindowFrame>>>,
}

impl WindowFrames {
    pub(crate) fn new(device: Arc<VelloWgpuDevice>, rendering_modes: Vec<VelloRenderingMode>) -> Self {
        Self { device, rendering_modes, frame: RefCell::new(None) }
    }

    /// The frame of the window for a texture of `pixel_size` at `dpi`: the
    /// one of the frame before when it still fits, a new one otherwise (the
    /// window was resized, its scaling changed, the device was lost).
    pub(crate) fn frame_for(&self, pixel_size: PixelSize, dpi: ferroui_base::Vector) -> Rc<WindowFrame> {
        let mut frame = self.frame.borrow_mut();

        let fits = frame
            .as_ref()
            .is_some_and(|frame| frame.pixel_size() == pixel_size && frame.dpi() == dpi && !frame.is_corrupted());
        if !fits {
            if let Some(old) = frame.take() {
                old.dispose();
            }

            let device = &self.device;
            let mode = window_rendering_mode(device, &self.rendering_modes);
            *frame = Some(Rc::new(match DeviceSurfaceRenderTarget::is_available(mode, device) {
                true => WindowFrame::Device(DeviceSurfaceRenderTarget::new(
                    device.clone(),
                    mode,
                    pixel_size,
                    dpi,
                    self.rendering_modes.clone(),
                    false,
                )),
                false => WindowFrame::Memory(
                    SurfaceRenderTarget::new(SurfaceRenderTargetCreateInfo {
                        width: pixel_size.width,
                        height: pixel_size.height,
                        dpi,
                        rendering_modes: vec![VelloRenderingMode::Cpu],
                        use_scaled_drawing: false,
                    }),
                    RefCell::new(None),
                ),
            }));
        }

        frame.clone().unwrap_or_else(|| panic!("The frame of the window was just created"))
    }

    /// Ends the frame of the window.
    pub(crate) fn dispose(&self) {
        if let Some(frame) = self.frame.borrow_mut().take() {
            frame.dispose();
        }
    }
}
