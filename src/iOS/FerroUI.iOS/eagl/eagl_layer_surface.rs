//! The render surface of a view over its EAGL layer, with its render
//! target and its rendering session.

use super::{EaglPlatformGraphics, GlContext, SizeSynchronizedLayerFbo};
use crate::display_link_timer::DisplayLinkTimer;
use crate::metal::SurfaceShared;
use ferroui_base::platform::surfaces::{IPlatformRenderSurface, IPlatformRenderSurfaceRenderTarget};
use ferroui_base::platform::{PlatformRenderTargetState, RenderTargetSceneInfo};
use ferroui_base::reactive::IDisposable;
use ferroui_base::PixelSize;
use ferroui_opengl::surfaces::{IGlPlatformSurface, IGlPlatformSurfaceRenderTarget, IGlPlatformSurfaceRenderingSession};
use ferroui_opengl::IGlContext;
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, ProtocolObject};
use objc2_foundation::NSObjectProtocol;
use objc2_open_gl_es::EAGLDrawable;
use objc2_quartz_core::CALayer;
use std::any::{Any, TypeId};
use std::rc::Rc;
use std::sync::Arc;

/// The class of the layer of OpenGL ES (`CAEAGLLayer`), which the bindings
/// of Core Animation do not declare: it is found by its name.
///
/// # Panics
/// Panics when the system has no such class (every iOS has it).
pub(crate) fn eagl_layer_class() -> &'static AnyClass {
    match AnyClass::get(c"CAEAGLLayer") {
        Some(class) => class,
        None => panic!("The class CAEAGLLayer is not available."),
    }
}

/// The EAGL layer of a view, for the thread that renders.
pub(crate) struct SharedEaglLayer(Retained<CALayer>);

// SAFETY: the layer is created by UIKit for a view of the main thread and
// handed to the thread that renders, as the reference does. Through this
// type that thread reads `bounds` and `contentsScale`, gives the layer to
// `renderbufferStorage:fromDrawable:` as the drawable of a renderbuffer and
// presents that renderbuffer, and nothing else. An EAGL layer is made to be
// drawn to from a thread that is not the main thread (its renderbuffer
// belongs to the context, which is used on one thread at a time), and the
// properties of a Core Animation layer may be read from any thread. The
// main thread sets the drawable properties before the surface exists and
// does not touch them afterwards. Retaining and releasing an Objective-C
// object is thread-safe.
unsafe impl Send for SharedEaglLayer {}
// SAFETY: see `Send`.
unsafe impl Sync for SharedEaglLayer {}

impl SharedEaglLayer {
    /// The layer, when it is an EAGL layer.
    pub(crate) fn new(layer: Retained<CALayer>) -> Option<SharedEaglLayer> {
        layer.isKindOfClass(eagl_layer_class()).then_some(SharedEaglLayer(layer))
    }

    /// The layer.
    pub(crate) fn layer(&self) -> &CALayer {
        &self.0
    }

    /// The layer as the drawable of a renderbuffer.
    pub(crate) fn drawable(&self) -> &ProtocolObject<dyn EAGLDrawable> {
        let layer: *const CALayer = &*self.0;
        // SAFETY: the layer is a `CAEAGLLayer` (checked by `new`), which
        // conforms to `EAGLDrawable`, and a protocol object is the object
        // itself (a transparent wrapper of `AnyObject`), so the reference
        // is one to the same object for as long as the layer is borrowed.
        unsafe { &*layer.cast::<ProtocolObject<dyn EAGLDrawable>>() }
    }
}

/// The render surface of a view over its EAGL layer.
///
/// The surface is shared between the threads; all it holds is shared: the
/// layer, the graphics and the render timer of the platform (which the
/// reference reads from the platform, a static class, on the thread that
/// renders; the state of the port's platform is the main thread's), and
/// what the view shares with its surfaces. The render target it creates
/// belongs to the thread that renders, with the context.
pub struct EaglLayerSurface {
    layer: Arc<SharedEaglLayer>,
    graphics: Arc<EaglPlatformGraphics>,
    timer: Arc<DisplayLinkTimer>,
    /// The frames and their capture, for diagnostics (an addition of the
    /// port, as for the Metal layer).
    shared: Arc<SurfaceShared>,
}

impl EaglLayerSurface {
    pub(crate) fn new(
        layer: Arc<SharedEaglLayer>,
        graphics: Arc<EaglPlatformGraphics>,
        timer: Arc<DisplayLinkTimer>,
        shared: Arc<SurfaceShared>,
    ) -> Arc<EaglLayerSurface> {
        Arc::new(EaglLayerSurface { layer, graphics, timer, shared })
    }
}

/// One frame drawn to the framebuffer of the layer; disposing the session
/// presents the frame.
struct RenderSession {
    ctx: Rc<GlContext>,
    restore_context: Rc<dyn IDisposable>,
    fbo: Rc<SizeSynchronizedLayerFbo>,
    size: PixelSize,
    scaling: f64,
    context: Rc<dyn IGlContext>,
    is_y_flipped: bool,
    shared: Arc<SurfaceShared>,
}

impl RenderSession {
    fn new(
        ctx: Rc<GlContext>,
        restore_context: Rc<dyn IDisposable>,
        fbo: Rc<SizeSynchronizedLayerFbo>,
        shared: Arc<SurfaceShared>,
    ) -> Self {
        let size = PixelSize::new(fbo.width(), fbo.height());
        let scaling = fbo.scaling();
        let context: Rc<dyn IGlContext> = ctx.clone();
        Self { ctx, restore_context, fbo, size, scaling, context, is_y_flipped: false, shared }
    }
}

impl IGlPlatformSurfaceRenderingSession for RenderSession {
    fn context(&self) -> Rc<dyn IGlContext> {
        self.context.clone()
    }

    fn size(&self) -> PixelSize {
        self.size
    }

    fn scaling(&self) -> f64 {
        self.scaling
    }

    fn is_y_flipped(&self) -> bool {
        self.is_y_flipped
    }

    fn dispose(&self) {
        self.ctx.gl_interface().finish();
        // The pixels of the frame, when they were asked for: read before
        // the frame is presented (an addition of the port).
        if let Some(callback) = self.shared.take_capture() {
            match self.fbo.capture() {
                Some(capture) => callback(capture),
                // The request stays for a later frame.
                None => self.shared.request_capture(callback),
            }
        }
        self.fbo.present();
        self.restore_context.dispose();
        self.shared.frame_presented();
    }
}

/// The render target of the layer: its framebuffer, with the one context
/// of the graphics.
struct RenderTarget {
    ctx: Rc<GlContext>,
    fbo: Rc<SizeSynchronizedLayerFbo>,
    timer: Arc<DisplayLinkTimer>,
    shared: Arc<SurfaceShared>,
}

impl RenderTarget {
    fn new(
        ctx: Rc<GlContext>,
        fbo: SizeSynchronizedLayerFbo,
        timer: Arc<DisplayLinkTimer>,
        shared: Arc<SurfaceShared>,
    ) -> Self {
        Self { ctx, fbo: Rc::new(fbo), timer, shared }
    }
}

impl IPlatformRenderSurfaceRenderTarget for RenderTarget {
    /// Ready: the target is never corrupted (`IsCorrupted` of the
    /// reference is false).
    fn state(&self) -> PlatformRenderTargetState {
        PlatformRenderTargetState::READY
    }
}

impl IGlPlatformSurfaceRenderTarget for RenderTarget {
    /// # Panics
    /// Panics on another thread than the one of the display link, and when
    /// the framebuffer cannot be created for the layer (the exceptions of
    /// the reference).
    fn begin_draw(&self, _scene_info: &RenderTargetSceneInfo) -> Rc<dyn IGlPlatformSurfaceRenderingSession> {
        check_thread(&self.timer);
        let restore_context = IGlContext::make_current(&*self.ctx);
        self.fbo.bind();
        self.shared.frame_begun();
        Rc::new(RenderSession::new(self.ctx.clone(), restore_context, self.fbo.clone(), self.shared.clone()))
    }

    /// # Panics
    /// Panics on another thread than the one of the display link (the
    /// exception of the reference).
    fn dispose(&self) {
        check_thread(&self.timer);
        let current = IGlContext::make_current(&*self.ctx);
        self.fbo.dispose();
        current.dispose();
    }
}

/// Fails on another thread than the one of the display link: the thread
/// the context of the graphics is current on.
///
/// # Panics
/// Panics on another thread (the exception of the reference).
fn check_thread(timer: &DisplayLinkTimer) {
    if timer.timer_thread().id() != std::thread::current().id() {
        panic!("Invalid thread, go away");
    }
}

impl IGlPlatformSurface for EaglLayerSurface {
    /// # Panics
    /// Panics on another thread than the one of the display link, for
    /// another context than the one of the graphics, and when no
    /// framebuffer can be created for the layer (the exceptions of the
    /// reference).
    fn create_gl_render_target(&self, context: &Rc<dyn IGlContext>) -> Rc<dyn IGlPlatformSurfaceRenderTarget> {
        check_thread(&self.timer);
        let ctx = self.graphics.context();
        if !std::ptr::addr_eq(Rc::as_ptr(&ctx), Rc::as_ptr(context)) {
            panic!("Platform surface is only usable with tha main context");
        }
        let current = IGlContext::make_current(&*ctx);
        // Made current above, so not disposed.
        let Some(eagl_context) = ctx.context() else {
            panic!("Object reference not set to an instance of an object.");
        };
        let fbo = SizeSynchronizedLayerFbo::new(eagl_context, ctx.gl_interface(), self.layer.clone());
        if !fbo.sync() {
            current.dispose();
            panic!("Unable to create render target");
        }
        current.dispose();
        Rc::new(RenderTarget::new(ctx, fbo, self.timer.clone(), self.shared.clone()))
    }
}

impl IPlatformRenderSurface for EaglLayerSurface {
    fn try_get_surface_kind(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
        if kind == TypeId::of::<dyn IGlPlatformSurface>() {
            // The view is a handle of the calling thread, the surface a
            // handle shared between the threads: the view is a surface of
            // its own over the same layer, which is all the state of a
            // surface.
            let this: Rc<dyn IGlPlatformSurface> = Rc::new(EaglLayerSurface {
                layer: self.layer.clone(),
                graphics: self.graphics.clone(),
                timer: self.timer.clone(),
                shared: self.shared.clone(),
            });
            return Some(Rc::new(this));
        }
        None
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
