//! Port of `DrawingSurfaceDemoBase.cs`: the base class of the demos. It shows a drawing surface
//! of the compositor as the child visual of the control, asks the compositor for its GPU
//! interop, has the demo create its graphics resources for it, and has the demo draw a frame
//! whenever the size or the values of the sliders change (every frame while the demo is in its
//! disco mode).
//!
//! The initialization is an asynchronous method, as upstream: it awaits the interop of the
//! compositor (`TryGetCompositionGpuInterop`), whose answer comes from the render thread.

use crate::gpu_demo::{GpuDemo, IGpuDemo};
use crate::task_support::start;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::logical_tree::LogicalTreeAttachmentEventArgs;
use ferroui_base::rendering::composition::{
    CompositionDrawingSurface, CompositionSurfaceVisual, Compositor, ElementComposition, ICompositionGpuInterop,
};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, FerroObject, FerroObjectImpl, FerroObjectImplExt,
    FerroPropertyChangedEventArgs, ObjectType, PixelSize, Ref, StyledElementImpl, StyledElementImplExt, Upcast, Vector,
    Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs, WeakRef,
};
use ferroui_controls::{Control, ControlImpl};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[repr(C)]
pub struct DrawingSurfaceDemoBase {
    base: Control,
    visual: RefCell<Option<CompositionSurfaceVisual>>,
    compositor: RefCell<Option<Rc<Compositor>>>,
    info: RefCell<String>,
    update_queued: Cell<bool>,
    initialized: Cell<bool>,
    surface: RefCell<Option<CompositionDrawingSurface>>,
    /// The demo control that shows this demo. Upstream holds it as it is; here the demo control
    /// holds the demo, so the demo holds it weakly.
    parent_control: RefCell<Option<WeakRef<GpuDemo>>>,
    disco: Cell<f32>,
    roll: Cell<f32>,
    pitch: Cell<f32>,
    yaw: Cell<f32>,
}

ferro_class! {
    DrawingSurfaceDemoBase: Control, virtuals DrawingSurfaceDemoBaseImpl: ControlImpl {
        /// Creates the graphics resources of the demo for the compositor and its GPU interop:
        /// whether that succeeded, and the text the demo shows.
        fn initialize_graphics_resources(
            this,
            compositor: &Rc<Compositor>,
            composition_drawing_surface: &CompositionDrawingSurface,
            gpu_interop: &Rc<dyn ICompositionGpuInterop>
        ) -> (bool, String);

        /// Releases the graphics resources of the demo.
        fn free_graphics_resources(this);

        /// Draws a frame of `pixel_size` and presents it to the surface.
        fn render_frame(this, pixel_size: PixelSize);

        /// Whether the demo draws a frame after a frame while its disco slider is above zero.
        fn supports_disco(this) -> bool;
    }
}
ferro_class_info!(DrawingSurfaceDemoBase { interfaces: [Rc<dyn IGpuDemo> => DrawingSurfaceDemoBase::as_gpu_demo] });

ferro_impl_classes!(DrawingSurfaceDemoBase: LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl DrawingSurfaceDemoBaseImpl for DrawingSurfaceDemoBase {
    fn initialize_graphics_resources(
        _this: &Self,
        _compositor: &Rc<Compositor>,
        _composition_drawing_surface: &CompositionDrawingSurface,
        _gpu_interop: &Rc<dyn ICompositionGpuInterop>,
    ) -> (bool, String) {
        panic!("DrawingSurfaceDemoBase is abstract: 'initialize_graphics_resources' must be implemented by the deriving class")
    }

    fn free_graphics_resources(_this: &Self) {
        panic!("DrawingSurfaceDemoBase is abstract: 'free_graphics_resources' must be implemented by the deriving class")
    }

    fn render_frame(_this: &Self, _pixel_size: PixelSize) {
        panic!("DrawingSurfaceDemoBase is abstract: 'render_frame' must be implemented by the deriving class")
    }

    fn supports_disco(_this: &Self) -> bool {
        false
    }
}

impl VisualImpl for DrawingSurfaceDemoBase {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.initialize();
    }
}

impl StyledElementImpl for DrawingSurfaceDemoBase {
    fn on_detached_from_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        if this.initialized.get() {
            if let Some(surface) = this.surface.borrow().as_ref() {
                surface.dispose();
            }
            this.free_graphics_resources();
        }

        this.initialized.set(false);
        Self::parent_on_detached_from_logical_tree(this, e);
    }
}

impl FerroObjectImpl for DrawingSurfaceDemoBase {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        if change.property() == Visual::bounds_property().as_property() {
            this.queue_next_frame();
        }
        Self::parent_on_property_changed(this, change);
    }
}

/// The handle of the contract a demo implements (`IGpuDemo`).
struct GpuDemoRef(Ref<DrawingSurfaceDemoBase>);

impl IGpuDemo for GpuDemoRef {
    fn update(&self, parent: Option<&Ref<GpuDemo>>, yaw: f32, pitch: f32, roll: f32, disco: f32) {
        self.0.update(parent, yaw, pitch, roll, disco)
    }

    fn as_object(&self) -> Option<Ref<FerroObject>> {
        Some(self.0.clone().upcast())
    }
}

impl DrawingSurfaceDemoBase {
    /// The demo as the contract it implements: what the demo control holds (`Demo`).
    pub fn as_gpu_demo<T: ObjectType + Upcast<DrawingSurfaceDemoBase>>(this: Ref<T>) -> Rc<dyn IGpuDemo> {
        Rc::new(GpuDemoRef(this.upcast()))
    }

    /// `Update` of `IGpuDemo`.
    pub fn update(&self, parent: Option<&Ref<GpuDemo>>, yaw: f32, pitch: f32, roll: f32, disco: f32) {
        *self.parent_control.borrow_mut() = parent.map(Ref::downgrade);
        if let Some(parent_control) = self.parent_control() {
            parent_control.set_info(self.info.borrow().clone());
            parent_control.set_disco_visible(true);
        }

        self.yaw.set(yaw);
        self.pitch.set(pitch);
        self.roll.set(roll);
        self.disco.set(disco);
        self.queue_next_frame();
    }
}

impl DrawingSurfaceDemoBase {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            visual: RefCell::new(None),
            compositor: RefCell::new(None),
            info: RefCell::new(String::new()),
            update_queued: Cell::new(false),
            initialized: Cell::new(false),
            surface: RefCell::new(None),
            parent_control: RefCell::new(None),
            disco: Cell::new(0.0),
            roll: Cell::new(0.0),
            pitch: Cell::new(0.0),
            yaw: Cell::new(0.0),
        }
    }

    /// The drawing surface the demo presents its frames to, once the control is initialized.
    pub fn surface(&self) -> Option<CompositionDrawingSurface> {
        self.surface.borrow().clone()
    }

    /// The demo control that shows this demo, while there is one.
    pub fn parent_control(&self) -> Option<Ref<GpuDemo>> {
        self.parent_control.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    /// Whether the graphics resources were created (`_initialized`).
    pub fn is_initialized(&self) -> bool {
        self.initialized.get()
    }

    /// The text the demo shows: the device it draws with, or why it does not draw.
    pub fn info(&self) -> String {
        self.info.borrow().clone()
    }

    pub fn disco(&self) -> f32 {
        self.disco.get()
    }

    pub fn roll(&self) -> f32 {
        self.roll.get()
    }

    pub fn pitch(&self) -> f32 {
        self.pitch.get()
    }

    pub fn yaw(&self) -> f32 {
        self.yaw.get()
    }

    /// Shows `info` in the demo control, as the `catch` of `Initialize` upstream does with the
    /// text of the exception.
    fn show_error(&self, info: String) {
        if let Some(parent_control) = self.parent_control() {
            parent_control.set_info(info);
        }
    }

    /// `async void Initialize()` of the original.
    fn initialize(&self) {
        let this: Ref<Visual> = self.to_ref().upcast();
        let Some(self_visual) = ElementComposition::get_element_visual(&this) else {
            self.show_error("The control has no composition visual".to_string());
            return;
        };
        let compositor = self_visual.compositor().clone();
        *self.compositor.borrow_mut() = Some(compositor.clone());

        let surface = compositor.create_drawing_surface();
        let visual = compositor.create_surface_visual();
        let bounds = self.bounds();
        visual.set_size(Vector::new(bounds.width, bounds.height));
        visual.set_surface(Some((*surface).clone()));
        *self.surface.borrow_mut() = Some(surface.clone());
        *self.visual.borrow_mut() = Some(visual.clone());
        ElementComposition::set_element_child_visual(&this, Some((*visual).clone()));

        let this = self.to_ref();
        drop(start(async move {
            let (res, info) = this.do_initialize(&compositor, &surface).await;
            *this.info.borrow_mut() = info.clone();
            if let Some(parent_control) = this.parent_control() {
                parent_control.set_info(info);
            }
            this.initialized.set(res);
            this.queue_next_frame();
        }));
    }

    fn update_frame(&self) {
        self.update_queued.set(false);
        let Some(source) = self.get_presentation_source() else {
            return;
        };

        let bounds = self.bounds();
        if let Some(visual) = self.visual.borrow().as_ref() {
            visual.set_size(Vector::new(bounds.width, bounds.height));
        }
        let size = PixelSize::from_size(bounds.size(), source.render_scaling());
        self.render_frame(size);
        if self.supports_disco() && self.disco.get() > 0.0 {
            self.queue_next_frame();
        }
    }

    fn queue_next_frame(&self) {
        if self.initialized.get() && !self.update_queued.get() {
            let compositor = self.compositor.borrow().clone();
            if let Some(compositor) = compositor {
                self.update_queued.set(true);
                let this = self.to_ref();
                compositor.request_composition_update(move || this.update_frame());
            }
        }
    }

    /// `DoInitialize`: the interop of the compositor, then the resources of the demo for it.
    async fn do_initialize(
        &self,
        compositor: &Rc<Compositor>,
        composition_drawing_surface: &CompositionDrawingSurface,
    ) -> (bool, String) {
        let Some(interop) = compositor.try_get_composition_gpu_interop().await else {
            return (false, "Compositor doesn't support interop for the current backend".to_string());
        };
        self.initialize_graphics_resources(compositor, composition_drawing_surface, &interop)
    }
}
