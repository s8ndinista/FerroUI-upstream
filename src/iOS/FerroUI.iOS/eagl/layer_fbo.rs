//! The framebuffer of the renderbuffer of an EAGL layer, and the one that
//! follows the size of the layer.

use super::eagl_display::is_current;
use super::SharedEaglLayer;
use crate::metal::FrameCapture;
use ferroui_opengl::gl_consts::{
    GL_COLOR_ATTACHMENT0, GL_DEPTH_ATTACHMENT, GL_FRAMEBUFFER, GL_FRAMEBUFFER_COMPLETE, GL_RENDERBUFFER,
    GL_RENDERBUFFER_HEIGHT, GL_RENDERBUFFER_WIDTH, GL_RGBA, GL_UNSIGNED_BYTE,
};
use ferroui_opengl::GlInterface;
use objc2::rc::Retained;
use objc2_foundation::NSUInteger;
use objc2_open_gl_es::EAGLContext;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

/// A framebuffer whose colour attachment is a renderbuffer with the storage
/// of an EAGL layer.
pub(crate) struct LayerFbo {
    context: Retained<EAGLContext>,
    gl: Rc<GlInterface>,
    /// The layer whose storage the renderbuffer has, kept as long as the
    /// framebuffer, as the reference keeps it.
    _layer: Arc<SharedEaglLayer>,
    framebuffer: i32,
    renderbuffer: i32,
    depth_buffer: i32,
    disposed: Cell<bool>,
    width: i32,
    height: i32,
}

impl LayerFbo {
    /// The framebuffer of the layer, created with `context`, which has to be
    /// current; none when it is not, or when the framebuffer is not
    /// complete.
    pub(crate) fn try_create(
        context: &Retained<EAGLContext>,
        gl: &Rc<GlInterface>,
        layer: &Arc<SharedEaglLayer>,
    ) -> Option<LayerFbo> {
        if !is_current(context) {
            return None;
        }

        let rb = gl.gen_renderbuffer();
        gl.bind_renderbuffer(GL_RENDERBUFFER, rb);
        // SAFETY: the context is current on this thread and the renderbuffer
        // just created is bound to its target; the drawable is the EAGL
        // layer of a view (`SharedEaglLayer`), whose storage the
        // renderbuffer gets. The result is not checked, as in the
        // reference: a renderbuffer without storage makes the framebuffer
        // incomplete below.
        unsafe { context.renderbufferStorage_fromDrawable(GL_RENDERBUFFER as NSUInteger, Some(layer.drawable())) };

        let fb = gl.gen_framebuffer();
        gl.bind_framebuffer(GL_FRAMEBUFFER, fb);
        gl.framebuffer_renderbuffer(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_RENDERBUFFER, rb);

        let w = gl.get_renderbuffer_parameteriv(GL_RENDERBUFFER, GL_RENDERBUFFER_WIDTH);
        let h = gl.get_renderbuffer_parameteriv(GL_RENDERBUFFER, GL_RENDERBUFFER_HEIGHT);

        let db = gl.gen_renderbuffer();

        //GL.BindRenderbuffer(RenderbufferTarget.Renderbuffer, depthBuffer);
        //GL.RenderbufferStorage(RenderbufferTarget.Renderbuffer, RenderbufferInternalFormat.DepthComponent16, w, h);
        gl.framebuffer_renderbuffer(GL_FRAMEBUFFER, GL_DEPTH_ATTACHMENT, GL_RENDERBUFFER, db);

        let frame_buffer_error = gl.check_framebuffer_status(GL_FRAMEBUFFER);
        if frame_buffer_error != GL_FRAMEBUFFER_COMPLETE {
            gl.delete_framebuffer(fb);
            gl.delete_renderbuffer(db);
            gl.delete_renderbuffer(rb);
            return None;
        }

        Some(LayerFbo {
            context: context.clone(),
            gl: gl.clone(),
            _layer: layer.clone(),
            framebuffer: fb,
            renderbuffer: rb,
            depth_buffer: db,
            disposed: Cell::new(false),
            width: w,
            height: h,
        })
    }

    /// The width of the renderbuffer in pixels.
    pub(crate) fn width(&self) -> i32 {
        self.width
    }

    /// The height of the renderbuffer in pixels.
    pub(crate) fn height(&self) -> i32 {
        self.height
    }

    /// Binds the framebuffer.
    pub(crate) fn bind(&self) {
        self.gl.bind_framebuffer(GL_FRAMEBUFFER, self.framebuffer);
    }

    /// Presents the renderbuffer on the layer.
    pub(crate) fn present(&self) {
        self.bind();
        // SAFETY: the context is current on this thread (the render
        // session made it current) and its renderbuffer has the storage of
        // the layer. The result is not checked, as in the reference.
        unsafe { self.context.presentRenderbuffer(GL_RENDERBUFFER as NSUInteger) };
    }

    /// Deletes the framebuffer and its renderbuffers.
    ///
    /// # Panics
    /// Panics when the context of the framebuffer is not current (after
    /// the objects were deleted, as in the reference).
    pub(crate) fn dispose(&self) {
        if self.disposed.get() {
            return;
        }
        self.disposed.set(true);
        self.gl.delete_framebuffer(self.framebuffer);
        self.gl.delete_renderbuffer(self.depth_buffer);
        self.gl.delete_renderbuffer(self.renderbuffer);
        if !is_current(&self.context) {
            panic!("Associated EAGLContext is not current");
        }
    }

    /// Reads the pixels of the renderbuffer back. An addition of the port,
    /// for the checks of an application that tests itself
    /// (`FerroView::capture_next_frame`): the rows are put from the top and
    /// the channels in the order of `BGRA8Unorm`, which is how a frame of
    /// the Metal layer is handed over.
    pub(crate) fn capture(&self) -> FrameCapture {
        self.bind();
        let (width, height) = (self.width.max(0) as usize, self.height.max(0) as usize);
        let bytes_per_row = width * 4;
        let mut rgba = vec![0u8; bytes_per_row * height];
        // SAFETY: the framebuffer is bound and complete; the buffer has room
        // for `width * height` pixels of four bytes each, which is what
        // `GL_RGBA` and `GL_UNSIGNED_BYTE` write with the default pack
        // alignment of four.
        unsafe {
            self.gl.read_pixels(
                0,
                0,
                self.width,
                self.height,
                GL_RGBA,
                GL_UNSIGNED_BYTE,
                rgba.as_mut_ptr().cast(),
            );
        }

        // OpenGL counts the rows from the bottom.
        let mut pixels = vec![0u8; bytes_per_row * height];
        for y in 0..height {
            let source = &rgba[(height - 1 - y) * bytes_per_row..][..bytes_per_row];
            let target = &mut pixels[y * bytes_per_row..][..bytes_per_row];
            for (source, target) in source.chunks_exact(4).zip(target.chunks_exact_mut(4)) {
                target.copy_from_slice(&[source[2], source[1], source[0], source[3]]);
            }
        }

        // 80 is `MTLPixelFormatBGRA8Unorm`.
        FrameCapture { width, height, bytes_per_row, pixel_format: 80, pixels }
    }
}

/// The framebuffer of an EAGL layer, created again when the size or the
/// scale of the layer changed.
pub(crate) struct SizeSynchronizedLayerFbo {
    context: Retained<EAGLContext>,
    gl: Rc<GlInterface>,
    layer: Arc<SharedEaglLayer>,
    fbo: RefCell<Option<LayerFbo>>,
    old_layer_width: Cell<f64>,
    old_layer_height: Cell<f64>,
    old_layer_scale: Cell<f64>,
}

impl SizeSynchronizedLayerFbo {
    pub(crate) fn new(context: Retained<EAGLContext>, gl: Rc<GlInterface>, layer: Arc<SharedEaglLayer>) -> Self {
        Self {
            context,
            gl,
            layer,
            fbo: RefCell::new(None),
            old_layer_width: Cell::new(0.0),
            old_layer_height: Cell::new(0.0),
            old_layer_scale: Cell::new(0.0),
        }
    }

    /// Creates the framebuffer again when there is none or the layer
    /// changed since; whether there is one afterwards.
    ///
    /// # Panics
    /// Panics when the framebuffer it replaces cannot be disposed (its
    /// context is not current).
    pub(crate) fn sync(&self) -> bool {
        let layer = self.layer.layer();
        let bounds = layer.bounds();
        if self.fbo.borrow().is_some()
            && self.old_layer_width.get() == bounds.size.width
            && self.old_layer_height.get() == bounds.size.height
            && self.old_layer_scale.get() == layer.contentsScale()
        {
            return true;
        }
        let old = self.fbo.borrow_mut().take();
        if let Some(old) = old {
            old.dispose();
        }
        let fbo = LayerFbo::try_create(&self.context, &self.gl, &self.layer);
        let created = fbo.is_some();
        *self.fbo.borrow_mut() = fbo;
        let bounds = layer.bounds();
        self.old_layer_width.set(bounds.size.width);
        self.old_layer_height.set(bounds.size.height);
        self.old_layer_scale.set(layer.contentsScale());
        created
    }

    /// Deletes the framebuffer.
    ///
    /// # Panics
    /// Panics when the context is not current.
    pub(crate) fn dispose(&self) {
        if !is_current(&self.context) {
            panic!("Associated EAGLContext is not current");
        }
        let fbo = self.fbo.borrow_mut().take();
        if let Some(fbo) = fbo {
            fbo.dispose();
        }
    }

    /// Binds the framebuffer, created again first when the layer changed.
    ///
    /// # Panics
    /// Panics when there is no framebuffer for the layer.
    pub(crate) fn bind(&self) {
        if !self.sync() {
            panic!("Unable to create a render target");
        }
        self.with_fbo(LayerFbo::bind);
    }

    /// Presents the renderbuffer on the layer.
    ///
    /// # Panics
    /// Panics when there is no framebuffer (the null reference of the
    /// reference).
    pub(crate) fn present(&self) {
        self.with_fbo(LayerFbo::present);
    }

    /// The width of the framebuffer in pixels; 0 without one.
    pub(crate) fn width(&self) -> i32 {
        self.fbo.borrow().as_ref().map_or(0, LayerFbo::width)
    }

    /// The height of the framebuffer in pixels; 0 without one.
    pub(crate) fn height(&self) -> i32 {
        self.fbo.borrow().as_ref().map_or(0, LayerFbo::height)
    }

    /// The scale of the layer when the framebuffer was created.
    pub(crate) fn scaling(&self) -> f64 {
        self.old_layer_scale.get()
    }

    /// Reads the pixels of the framebuffer back; none without one. An
    /// addition of the port (`LayerFbo::capture`).
    pub(crate) fn capture(&self) -> Option<FrameCapture> {
        self.fbo.borrow().as_ref().map(LayerFbo::capture)
    }

    fn with_fbo(&self, action: impl FnOnce(&LayerFbo)) {
        match self.fbo.borrow().as_ref() {
            Some(fbo) => action(fbo),
            None => panic!("Object reference not set to an instance of an object."),
        }
    }
}
