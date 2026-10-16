//! Port of `App.xaml.cs`: the application of the sample.

use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
use ferroui_controls::{Application, ApplicationImpl, NewApplication};

#[repr(C)]
pub struct App {
    base: Application,
}

ferro_class!(App: Application);
ferro_class_info!(App { new: App::new });
ferro_impl_classes!(App: FerroObjectImpl);

impl NewApplication for App {
    fn new_application() -> Ref<Self> {
        Self::new()
    }
}

impl ApplicationImpl for App {
    /// The load of the document by the XAML loader (`Load(this)`). `App.xaml` names no class,
    /// so its markup is not the compiled markup of this class: the load fails, as the load of
    /// the reference does (it finds no precompiled markup for the class). The program never
    /// initializes the application (`program.rs`).
    fn initialize(this: &Self) {
        crate::register_types();
        crate::SAMPLE.load_component(&this.to_ref(), "/App.xaml");
    }
}

impl App {
    pub fn construct() -> Self {
        Self { base: Application::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
