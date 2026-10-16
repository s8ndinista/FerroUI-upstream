//! The application of the gallery (the class of `App.xaml`): the theme under test as the first
//! of its styles, then the styles of its document; the main window with the classic desktop
//! lifetime. With another lifetime (the single view of the comparison host of Windows) the host
//! shows what it wants itself.

use crate::main_window::MainWindow;
use crate::markup::xaml_class;
use crate::theme::{self, GalleryTheme};
use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
use ferroui_controls::{Application, ApplicationImpl, ApplicationImplExt, NewApplication};
use std::cell::RefCell;

#[repr(C)]
pub struct App {
    base: Application,
    theme: RefCell<Option<Ref<GalleryTheme>>>,
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
    /// `Initialize()`: the theme, then the document of the application.
    fn initialize(this: &Self) {
        crate::register_types();
        let gallery_theme = theme::create();
        this.styles().add(theme::as_style(&gallery_theme));
        *this.theme.borrow_mut() = Some(gallery_theme);
        this.initialize_component();
    }

    fn on_framework_initialization_completed(this: &Self) {
        let lifetime = this.application_lifetime();
        if let Some(desktop) = lifetime.as_ref().and_then(|l| l.as_classic_desktop_style_application_lifetime()) {
            desktop.set_main_window(Some(MainWindow::new().upcast()));
        }

        Self::parent_on_framework_initialization_completed(this);
    }
}

impl App {
    pub fn construct() -> Self {
        Self { base: Application::construct(), theme: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The application of the gallery, once the framework has created it.
    pub fn current() -> Option<Ref<App>> {
        Application::current().and_then(|application| application.cast::<App>())
    }

    /// The theme the application was initialised with.
    ///
    /// # Panics
    /// Panics before `initialize()`.
    pub fn theme(&self) -> Ref<GalleryTheme> {
        self.theme.borrow().clone().expect("the theme is created by initialize()")
    }

    /// The theme of the current application of the gallery.
    pub fn current_theme() -> Option<Ref<GalleryTheme>> {
        Self::current().map(|app| app.theme())
    }
}
