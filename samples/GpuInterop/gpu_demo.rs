//! Port of `GpuDemo.xaml.cs`: the class of the document `GpuDemo.xaml` (the demo with the
//! sliders that turn and colour what it draws) and the contract of a demo.

use crate::markup::xaml_class;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, DirectProperty, FerroObject,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl,
    VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentControlImpl, ControlImpl, UserControl};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[repr(C)]
pub struct GpuDemo {
    base: UserControl,
    yaw: Cell<f32>,
    pitch: Cell<f32>,
    roll: Cell<f32>,
    disco: Cell<f32>,
    info: RefCell<String>,
    disco_visible: Cell<bool>,
    demo: RefCell<Option<Rc<dyn IGpuDemo>>>,
}

ferro_class!(GpuDemo: UserControl);
ferro_impl_classes!(
    GpuDemo: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(GpuDemo { new: GpuDemo::new });
xaml_class!(GpuDemo, "/GpuDemo.xaml");

ferro_properties! {
    impl GpuDemo {
        pub fn yaw_property() -> DirectProperty<GpuDemo, f32> {
            FerroProperty::register_direct::<GpuDemo, _>("Yaw", |o| o.yaw(), Some(|o: &GpuDemo, v| o.set_yaw(v)), 0.0)
        }

        pub fn pitch_property() -> DirectProperty<GpuDemo, f32> {
            FerroProperty::register_direct::<GpuDemo, _>(
                "Pitch",
                |o| o.pitch(),
                Some(|o: &GpuDemo, v| o.set_pitch(v)),
                0.0,
            )
        }

        pub fn roll_property() -> DirectProperty<GpuDemo, f32> {
            FerroProperty::register_direct::<GpuDemo, _>(
                "Roll",
                |o| o.roll(),
                Some(|o: &GpuDemo, v| o.set_roll(v)),
                0.0,
            )
        }

        pub fn disco_property() -> DirectProperty<GpuDemo, f32> {
            FerroProperty::register_direct::<GpuDemo, _>(
                "Disco",
                |o| o.disco(),
                Some(|o: &GpuDemo, v| o.set_disco(v)),
                0.0,
            )
        }

        pub fn info_property() -> DirectProperty<GpuDemo, String> {
            FerroProperty::register_direct::<GpuDemo, _>(
                "Info",
                |o| o.info(),
                Some(|o: &GpuDemo, v| o.set_info(v)),
                String::new(),
            )
        }

        // The setter of the registration writes the field without raising the change, as the
        // original's does.
        pub fn disco_visible_property() -> DirectProperty<GpuDemo, bool> {
            FerroProperty::register_direct::<GpuDemo, _>(
                "DiscoVisible",
                |o| o.disco_visible(),
                Some(|o: &GpuDemo, v| o.disco_visible.set(v)),
                false,
            )
        }

        // The setter of the registration writes the field without raising the change, as the
        // original's does.
        pub fn demo_property() -> DirectProperty<GpuDemo, Option<Rc<dyn IGpuDemo>>> {
            // A demo is a control: the content control of the document, whose content is
            // bound to this property, shows the control the handle holds.
            ValueTypes::register_interface_object::<Rc<dyn IGpuDemo>>(|demo| demo.as_object());
            FerroProperty::register_direct::<GpuDemo, _>(
                "Demo",
                |o| o.demo(),
                Some(|o: &GpuDemo, v| *o.demo.borrow_mut() = v),
                None,
            )
        }
    }
}

impl FerroObjectImpl for GpuDemo {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        let property = change.property();
        if property == Self::yaw_property().as_property()
            || property == Self::pitch_property().as_property()
            || property == Self::roll_property().as_property()
            || property == Self::disco_property().as_property()
            || property == Self::demo_property().as_property()
        {
            if property == Self::demo_property().as_property() {
                if let Some(old) = change.get_old_value::<Option<Rc<dyn IGpuDemo>>>().flatten() {
                    old.update(None, 0.0, 0.0, 0.0, 0.0);
                }
            }
            let demo = this.demo.borrow().clone();
            if let Some(demo) = demo {
                demo.update(Some(&this.to_ref()), this.yaw(), this.pitch(), this.roll(), this.disco());
            }
        }

        Self::parent_on_property_changed(this, change);
    }
}

impl GpuDemo {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            yaw: Cell::new(5.0),
            pitch: Cell::new(5.0),
            roll: Cell::new(5.0),
            disco: Cell::new(0.0),
            info: RefCell::new(String::new()),
            disco_visible: Cell::new(false),
            demo: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    pub fn yaw(&self) -> f32 {
        self.yaw.get()
    }

    pub fn set_yaw(&self, value: f32) {
        self.set_and_raise_cell(Self::yaw_property(), &self.yaw, value);
    }

    pub fn pitch(&self) -> f32 {
        self.pitch.get()
    }

    pub fn set_pitch(&self, value: f32) {
        self.set_and_raise_cell(Self::pitch_property(), &self.pitch, value);
    }

    pub fn roll(&self) -> f32 {
        self.roll.get()
    }

    pub fn set_roll(&self, value: f32) {
        self.set_and_raise_cell(Self::roll_property(), &self.roll, value);
    }

    pub fn disco(&self) -> f32 {
        self.disco.get()
    }

    pub fn set_disco(&self, value: f32) {
        self.set_and_raise_cell(Self::disco_property(), &self.disco, value);
    }

    pub fn info(&self) -> String {
        self.info.borrow().clone()
    }

    pub fn set_info(&self, value: String) {
        self.set_and_raise(Self::info_property(), &self.info, value);
    }

    pub fn disco_visible(&self) -> bool {
        self.disco_visible.get()
    }

    pub fn set_disco_visible(&self, value: bool) {
        self.set_and_raise_cell(Self::disco_visible_property(), &self.disco_visible, value);
    }

    /// The demo the control shows.
    pub fn demo(&self) -> Option<Rc<dyn IGpuDemo>> {
        self.demo.borrow().clone()
    }

    pub fn set_demo(&self, value: Option<Rc<dyn IGpuDemo>>) {
        self.set_and_raise(Self::demo_property(), &self.demo, value);
    }
}

/// A demo that [`GpuDemo`] shows and passes the values of its sliders to. The demos of the
/// sample are controls that implement it ([`DrawingSurfaceDemoBase`](crate::DrawingSurfaceDemoBase),
/// whose handle of the contract is `DrawingSurfaceDemoBase::as_gpu_demo`).
pub trait IGpuDemo {
    fn update(&self, parent: Option<&Ref<GpuDemo>>, yaw: f32, pitch: f32, roll: f32, disco: f32);

    /// The object that implements the contract, if one does: `is`/`as` on the interface
    /// (`as_object()?.cast::<T>()`).
    fn as_object(&self) -> Option<Ref<FerroObject>>;
}

/// Handles of the contract compare by the object behind them.
impl PartialEq for dyn IGpuDemo {
    fn eq(&self, other: &Self) -> bool {
        match (self.as_object(), other.as_object()) {
            (Some(a), Some(b)) => a == b,
            _ => std::ptr::addr_eq(self, other),
        }
    }
}
