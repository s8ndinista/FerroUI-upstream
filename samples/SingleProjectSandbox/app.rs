//! Port of `App.xaml.cs`: the class of the document `App.xaml`.

use crate::markup::xaml_class;
use crate::{MainView, MainWindow};
use ferroui_base::logging::LogEventLevel;
use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
use ferroui_controls::{AppBuilder, Application, ApplicationImpl, ApplicationImplExt, Control, NewApplication};
use std::rc::Rc;

#[repr(C)]
pub struct App {
    base: Application,
}

ferro_class!(App: Application);
ferro_class_info!(App { new: App::new });
ferro_impl_classes!(App: FerroObjectImpl);
xaml_class!(App, "/App.xaml");

impl NewApplication for App {
    fn new_application() -> Ref<Self> {
        Self::new()
    }
}

impl ApplicationImpl for App {
    fn initialize(this: &Self) {
        crate::register_types();
        this.initialize_component();
    }

    fn on_framework_initialization_completed(this: &Self) {
        let lifetime = this.application_lifetime();
        if let Some(desktop_lifetime) = lifetime.as_ref().and_then(|l| l.as_classic_desktop_style_application_lifetime()) {
            desktop_lifetime.set_main_window(Some(MainWindow::new().upcast()));
        } else if let Some(single_view_factory_application_lifetime) =
            lifetime.as_ref().and_then(|l| l.as_activity_application_lifetime())
        {
            single_view_factory_application_lifetime
                .set_main_view_factory(Some(Rc::new(|| -> Ref<Control> { MainView::new().upcast() })));
        } else if let Some(single_view_lifetime) = lifetime.as_ref().and_then(|l| l.as_single_view_application_lifetime())
        {
            single_view_lifetime.set_main_view(Some(MainView::new().upcast()));
        }

        Self::parent_on_framework_initialization_completed(this);
    }
}

impl App {
    pub fn construct() -> Self {
        Self { base: Application::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The method of the reference that makes the application builder: every entry point
    /// starts from it and selects its platform.
    pub fn build_ferro_app() -> AppBuilder {
        AppBuilder::configure::<App>().log_to_trace(LogEventLevel::Warning, &[])
    }
}
