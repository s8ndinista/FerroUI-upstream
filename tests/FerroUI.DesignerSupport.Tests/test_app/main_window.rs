//! Port of `MainWindow.xaml.cs`: the window of the document `MainWindow.xaml`.

use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroLocator, FerroObjectImpl,
    LocatorExtensions, ObjectType, Ref, StyledElementImpl, VisualImpl,
};
use ferroui_controls::platform::PlatformManager;
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentControlImpl, ControlImpl, TopLevelImpl, Window, WindowBaseImpl, WindowImpl};
use ferroui_markup_xaml::{IRuntimeXamlLoader, RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument};
use std::rc::Rc;

#[repr(C)]
pub struct MainWindow {
    base: Window,
}

ferro_class!(MainWindow: Window);
ferro_impl_classes!(
    MainWindow: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    TopLevelImpl,
    WindowBaseImpl,
    WindowImpl
);
ferro_class_info!(MainWindow { new: MainWindow::new });

impl MainWindow {
    pub fn construct() -> Self {
        Self { base: Window::construct(PlatformManager::create_window()) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        // `FerroXamlLoader.Load(this)`.
        load(&this, include_str!("MainWindow.xaml"));
        this
    }
}

/// `FerroXamlLoader.Load(this)`: populates `this` from the text of the
/// document of its class with the run-time loader.
///
/// # Panics
/// Panics if the document fails to load (an exception of the constructor in
/// the managed original).
pub(crate) fn load<T: ObjectType>(this: &Ref<T>, xaml: &str) {
    let loader = FerroLocator::current().get_required_service::<dyn IRuntimeXamlLoader>();
    let root: BoxedValue = Rc::new(this.clone());
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.local_assembly = Some(&crate::ASSEMBLY);
    configuration.use_compiled_bindings_by_default = true;
    if let Err(error) = loader.load(RuntimeXamlLoaderDocument::with_root_instance(Some(root), xaml), configuration) {
        panic!("{}", error.message());
    }
}
