//! Updating a composition target over a pyramid of 87380 draw list visuals
//! after the offset of its root changed: the commit of the change, the frame
//! of the server compositor and, without rendering, the update of the server
//! target (`CompositionTargetUpdateOnly`); with rendering
//! (`CompositionTargetUpdateWithRender`) the target is enabled and the frame
//! renders it into a framebuffer of one pixel.
//!
//! The draw list visuals have no visual of the visual tree; the `testing`
//! feature of the base crate creates them
//! (`ferroui_base::rendering::composition::testing`), as upstream's benchmark
//! project does through its access to the internals of the framework. The
//! framebuffer is the locked framebuffer of the Skia backend over memory of
//! the benchmark (the class `LockedFramebuffer` of upstream, which the base
//! crate does not hold). Upstream creates its compositor with the batch pools
//! reclaiming their buffers at once; the compositor of the port keeps no
//! pools of batch buffers, so there is no such choice.

use crate::harness::Registry;
use ferroui_base::media::{Brushes, DrawingContext, IBrush};
use ferroui_base::platform::surfaces::{
    FuncFramebufferRenderTarget, IFramebufferPlatformSurface, IFramebufferRenderTarget, IPlatformRenderSurface,
};
use ferroui_base::platform::{AlphaFormat, ILockedFramebuffer, PixelFormat};
use ferroui_base::rendering::composition::drawing::RenderDataDrawingContext;
use ferroui_base::rendering::composition::server::ServerCompositionTarget;
use ferroui_base::rendering::composition::testing::create_draw_list_visual;
use ferroui_base::rendering::composition::{
    CompositionContainerVisual, CompositionTarget, CompositionVisual, Compositor, ICompositorScheduler, RenderSurfaces,
};
use ferroui_base::rendering::{IRenderTimer, RenderLoop, RenderTimerTick};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{PixelSize, Rect, Size, Vector, Vector3D};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_skia::LockedFramebuffer;
use std::any::Any;
use std::rc::Rc;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

/// A render timer that never ticks.
#[derive(Default)]
struct Timer {
    tick: Mutex<Option<RenderTimerTick>>,
}

impl IRenderTimer for Timer {
    fn tick(&self) -> Option<RenderTimerTick> {
        self.tick.lock().expect("the tick of the timer").clone()
    }

    fn set_tick(&self, value: Option<RenderTimerTick>) {
        *self.tick.lock().expect("the tick of the timer") = value;
    }

    fn runs_in_background(&self) -> bool {
        false
    }
}

/// A scheduler that does nothing when a commit is requested: the benchmark
/// commits itself.
struct ManualScheduler;

impl ICompositorScheduler for ManualScheduler {
    fn commit_requested(&self, _compositor: &Rc<Compositor>) {}
}

/// The memory of the framebuffer: four bytes, allocated once for the process
/// (`Marshal.AllocHGlobal(4)` in a static field).
fn buffer() -> *mut u8 {
    static BUFFER: OnceLock<usize> = OnceLock::new();
    *BUFFER.get_or_init(|| Box::leak(Box::new([0_u8; 4])).as_mut_ptr() as usize) as *mut u8
}

/// A framebuffer surface of one pixel.
struct NullFramebuffer;

impl IPlatformRenderSurface for NullFramebuffer {
    fn as_framebuffer_surface(&self) -> Option<&dyn IFramebufferPlatformSurface> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IFramebufferPlatformSurface for NullFramebuffer {
    fn create_framebuffer_render_target(&self) -> Rc<dyn IFramebufferRenderTarget> {
        Rc::new(FuncFramebufferRenderTarget::new(|| -> Rc<dyn ILockedFramebuffer> {
            Rc::new(LockedFramebuffer::new(
                buffer(),
                PixelSize::new(1, 1),
                4,
                Vector::new(96.0, 96.0),
                PixelFormat::RGBA8888,
                AlphaFormat::Premul,
                None,
            ))
        }))
    }
}

pub struct CompositionTargetUpdateOnly {
    include_render: bool,
    compositor: Rc<Compositor>,
    target: Rc<CompositionTarget>,
    /// The compositor holds its scheduler weakly.
    _scheduler: Rc<dyn ICompositorScheduler>,
    /// Declared last: the application ends after the compositor is released
    /// (`Dispose`).
    _app: UnitTestApplicationScope,
}

impl CompositionTargetUpdateOnly {
    pub fn new() -> Self {
        Self::with_render(false)
    }

    pub(crate) fn with_render(include_render: bool) -> Self {
        let app = UnitTestApplication::start(TestServices::styled_window());
        let scheduler: Rc<dyn ICompositorScheduler> = Rc::new(ManualScheduler);
        let compositor = Compositor::with_scheduler(
            RenderLoop::from_timer(Arc::new(Timer::default())),
            None,
            true,
            &scheduler,
            Dispatcher::ui_thread(),
            None,
            None,
        );
        let surfaces: RenderSurfaces =
            Arc::new(|| vec![Arc::new(NullFramebuffer) as Arc<dyn IPlatformRenderSurface>]);
        let target = compositor.create_composition_target(surfaces);
        target.set_size(Size::new(1000.0, 1000.0));
        target.set_scaling(1.0);
        let root = compositor.create_container_visual();
        root.set_size(Vector::new(1000.0, 1000.0));
        target.set_root(Some(root.clone()));
        if include_render {
            target.set_is_enabled(true);
        }
        Self::create_pyramid(&root, 7);
        compositor.commit();

        Self { include_render, compositor, target, _scheduler: scheduler, _app: app }
    }

    fn create_pyramid(visual: &CompositionContainerVisual, depth: i32) {
        for c in 0..4 {
            let child = create_draw_list_visual(visual.compositor());
            let right = if c == 1 || c == 3 { 1.0 } else { 0.0 };
            let bottom = if c > 1 { 1.0 } else { 0.0 };

            let size = visual.size();
            let rect = Rect::new(size.x / 2.0 * right, size.y / 2.0 * bottom, size.x / 2.0, size.y / 2.0);
            child.set_offset(Vector3D::new(rect.x, rect.y, 0.0));
            child.set_size(Vector::new(rect.width, rect.height));

            let mut core = RenderDataDrawingContext::new(Some(child.compositor().clone()));
            {
                let mut ctx = DrawingContext::new(&mut core);
                let aqua: Rc<dyn IBrush> = Brushes::aqua();
                ctx.draw_rectangle(Some(&aqua), None, Rect::from_size(rect.size()), 0.0, 0.0, &Default::default());
            }
            child.set_draw_list(core.get_render_results());
            child.set_visible(true);
            let child_visual: Rc<CompositionVisual> = (*child).clone();
            visual.children().add(child_visual.clone());
            if depth > 0 {
                Self::create_pyramid(&child_visual, depth - 1);
            }
        }
    }

    pub fn target_update(&self) {
        let root = self.target.root().expect("the root of the target");
        root.set_offset(Vector3D::new(if root.offset().x == 0.0 { 1.0 } else { 0.0 }, 0.0, 0.0));
        self.compositor.commit();
        self.compositor.server().render();
        if !self.include_render {
            let server_target = self
                .compositor
                .server()
                .get::<ServerCompositionTarget>(self.target.server())
                .expect("the server target of the target");
            server_target.update(Duration::ZERO);
        }
    }
}

/// `CompositionTargetUpdateWithRender`: the benchmark of
/// [`CompositionTargetUpdateOnly`] with the target enabled.
pub struct CompositionTargetUpdateWithRender;

impl CompositionTargetUpdateWithRender {
    pub fn new() -> CompositionTargetUpdateOnly {
        CompositionTargetUpdateOnly::with_render(true)
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("compositor", "CompositionTargetUpdateOnly");
    class.benchmark("target_update", "", CompositionTargetUpdateOnly::new, |b| b.target_update());

    let mut class = registry.class("compositor", "CompositionTargetUpdateWithRender");
    class.benchmark("target_update", "", CompositionTargetUpdateWithRender::new, |b| b.target_update());
}

#[cfg(test)]
mod tests {
    #[test]
    fn composition_target_update_only() {
        crate::harness::smoke_class(super::register, "CompositionTargetUpdateOnly");
    }

    #[test]
    fn composition_target_update_with_render() {
        crate::harness::smoke_class(super::register, "CompositionTargetUpdateWithRender");
    }
}
