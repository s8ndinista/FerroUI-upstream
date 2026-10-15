//! Port of the code of the application document of the upstream sample (`FerroApp.xaml` after
//! the conversion of its name): the application of the framework inside the WinUI application,
//! which its document gives the Fluent theme. The WinUI application sets the framework up with
//! it in its `OnLaunched`.

use crate::markup::xaml_class;
use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
use ferroui_controls::{Application, ApplicationImpl, NewApplication};

#[repr(C)]
pub struct FerroApp {
    base: Application,
}

ferro_class!(FerroApp: Application);
ferro_class_info!(FerroApp { new: FerroApp::new });
ferro_impl_classes!(FerroApp: FerroObjectImpl);
xaml_class!(FerroApp, "/FerroApp.xaml");

impl NewApplication for FerroApp {
    fn new_application() -> Ref<Self> {
        Self::new()
    }
}

impl ApplicationImpl for FerroApp {
    /// `Initialize()`: the loader of the markup populates the application from its document.
    fn initialize(this: &Self) {
        crate::register_types();
        this.initialize_component();
    }
}

impl FerroApp {
    pub fn construct() -> Self {
        Self { base: Application::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
