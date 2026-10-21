use crate::markup::xaml_class;
use crate::ReaderView;
use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
use ferroui_controls::{Application, ApplicationImpl, ApplicationImplExt, Control, NewApplication, Window};
use ferroui_themes_winui::WinUITheme;
use std::cell::RefCell;
use std::rc::Rc;
#[repr(C)]
pub struct App { base: Application, theme: RefCell<Option<Ref<WinUITheme>>> }
ferro_class!(App: Application);
ferro_class_info!(App { new: App::new });
ferro_impl_classes!(App: FerroObjectImpl);
xaml_class!(App, "/App.xaml");
impl NewApplication for App { fn new_application() -> Ref<Self> { Self::new() } }
impl ApplicationImpl for App {
    fn initialize(this: &Self) {
        crate::register_types();
        let theme = WinUITheme::new();
        crate::theme::set_accent(&theme, "#E85D32");
        this.styles().add(theme.as_style());
        *this.theme.borrow_mut() = Some(theme);
        this.initialize_component();
    }
    fn on_framework_initialization_completed(this: &Self) {
        let lifetime = this.application_lifetime();
        if let Some(desktop) = lifetime.as_ref().and_then(|v| v.as_classic_desktop_style_application_lifetime()) {
            let window = Window::new();
            window.set_title(Some("Ember · Hacker News".into()));
            window.set_width(1440.0); window.set_height(940.0);
            window.set_min_width(340.0); window.set_min_height(480.0);
            window.set_content(Some(Control::boxed(&ReaderView::new())));
            desktop.set_main_window(Some(window));
        } else if let Some(activity) = lifetime.as_ref().and_then(|v| v.as_activity_application_lifetime()) {
            activity.set_main_view_factory(Some(Rc::new(|| ReaderView::new().upcast())));
        } else if let Some(single) = lifetime.as_ref().and_then(|v| v.as_single_view_application_lifetime()) {
            single.set_main_view(Some(ReaderView::new().upcast()));
        }
        Self::parent_on_framework_initialization_completed(this);
    }
}
impl App {
    pub fn construct() -> Self { Self { base: Application::construct(), theme: RefCell::new(None) } }
    pub fn new() -> Ref<Self> { instantiate(Self::construct()) }
    pub(crate) fn theme() -> Option<Ref<WinUITheme>> {
        Application::current().and_then(|app| app.cast::<App>()).and_then(|app| app.theme.borrow().clone())
    }
}
