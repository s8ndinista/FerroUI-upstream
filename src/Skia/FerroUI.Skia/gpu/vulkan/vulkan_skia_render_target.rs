use super::sk_image_info;
use crate::gpu::ganesh::GaneshGrContext;
use crate::gpu::{ISkiaGpuRenderSession, ISkiaGpuRenderTarget, ISkiaGrContext, SkiaSurfaceOrigin};
use ferroui_base::platform::{PlatformRenderTargetState, RenderTargetSceneInfo};
use ferroui_vulkan::{IVulkanPlatformGraphicsContext, IVulkanRenderSession, IVulkanRenderTarget};
use skia_safe::gpu::{backend_render_targets, surfaces, SurfaceOrigin};
use skia_safe::{ColorSpace, ColorType, Surface};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The Skia render target of a Vulkan render target: a frame is drawn into
/// the image of a session of the target.
pub struct VulkanSkiaRenderTarget {
    gr_context: Rc<GaneshGrContext>,
    vulkan: Rc<dyn IVulkanPlatformGraphicsContext>,
    target: Rc<dyn IVulkanRenderTarget>,
}

impl VulkanSkiaRenderTarget {
    /// The render target of `target` for the GPU that renders with `vulkan`
    /// through `gr_context`.
    // The original holds its `VulkanSkiaGpu`; the two things it reads of the
    // GPU are held here instead.
    pub fn new(
        gr_context: Rc<GaneshGrContext>,
        vulkan: Rc<dyn IVulkanPlatformGraphicsContext>,
        target: Rc<dyn IVulkanRenderTarget>,
    ) -> Self {
        Self { gr_context, vulkan, target }
    }
}

/// Disposes a Vulkan session unless the Skia session took it over; what the
/// `finally` block of a managed implementation does.
struct SessionGuard {
    session: Rc<dyn IVulkanRenderSession>,
    success: Cell<bool>,
}

impl Drop for SessionGuard {
    fn drop(&mut self) {
        if !self.success.get() {
            self.session.dispose();
        }
    }
}

impl ISkiaGpuRenderTarget for VulkanSkiaRenderTarget {
    fn dispose(&self) {
        self.target.dispose();
    }

    fn begin_rendering_session(&self, _scene_info: &RenderTargetSceneInfo) -> Rc<dyn ISkiaGpuRenderSession> {
        // TODO: use expectedPixelSize
        let session = self.target.begin_draw();
        let guard = SessionGuard { session: session.clone(), success: Cell::new(false) };

        let size = session.size();
        let scaling = session.scaling();
        if size.width <= 0 || size.height <= 0 || scaling < 0.0 {
            panic!("Can't create drawing context for surface with {size} size and {scaling} scaling");
        }

        self.gr_context.reset_context();
        let session_image_info = session.image_info();
        let image_info = sk_image_info(&session_image_info, self.vulkan.device().graphics_queue_family_index());
        let render_target = backend_render_targets::make_vk((size.width, size.height), &image_info);
        let surface = self.gr_context.with_context(|context| {
            surfaces::wrap_backend_render_target(
                context,
                &render_target,
                if session.is_y_flipped() { SurfaceOrigin::TopLeft } else { SurfaceOrigin::BottomLeft },
                if session.is_rgba() { ColorType::RGBA8888 } else { ColorType::BGRA8888 },
                ColorSpace::new_srgb(),
                None,
            )
        });
        drop(render_target);

        let surface = surface.unwrap_or_else(|| panic!("Surface can't be created with the provided render target"));

        guard.success.set(true);

        Rc::new(VulkanSkiaRenderSession::new(self.gr_context.clone(), surface, session))
    }

    fn state(&self) -> PlatformRenderTargetState {
        self.target.state()
    }
}

/// One frame rendered with Skia to the image of a Vulkan session.
pub(crate) struct VulkanSkiaRenderSession {
    gr_context: Rc<GaneshGrContext>,
    sk_surface: RefCell<Option<Surface>>,
    vulkan_session: Rc<dyn IVulkanRenderSession>,
    surface_origin: SkiaSurfaceOrigin,
    disposed: Cell<bool>,
}

impl VulkanSkiaRenderSession {
    pub fn new(gr_context: Rc<GaneshGrContext>, surface: Surface, vulkan_session: Rc<dyn IVulkanRenderSession>) -> Self {
        let surface_origin =
            if vulkan_session.is_y_flipped() { SkiaSurfaceOrigin::TopLeft } else { SkiaSurfaceOrigin::BottomLeft };
        Self {
            gr_context,
            sk_surface: RefCell::new(Some(surface)),
            vulkan_session,
            surface_origin,
            disposed: Cell::new(false),
        }
    }
}

impl ISkiaGpuRenderSession for VulkanSkiaRenderSession {
    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }

        if let Some(mut surface) = self.sk_surface.borrow_mut().take() {
            if !self.gr_context.is_disposed() {
                self.gr_context.with_context(|context| {
                    context.flush_and_submit_surface(&mut surface, None);
                });
            }
        }
        self.gr_context.flush();
        self.vulkan_session.dispose();
    }

    fn gr_context(&self) -> Rc<dyn ISkiaGrContext> {
        self.gr_context.clone()
    }

    fn sk_surface(&self) -> Surface {
        self.sk_surface.borrow().clone().expect("the Vulkan render session has been disposed")
    }

    fn scale_factor(&self) -> f64 {
        self.vulkan_session.scaling()
    }

    fn surface_origin(&self) -> SkiaSurfaceOrigin {
        self.surface_origin
    }
}
