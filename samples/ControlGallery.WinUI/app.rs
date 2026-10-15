//! `App` of the reference's sample: the WinUI application, which sets the
//! framework up with the catalog's application and a single view lifetime,
//! and shows the main window.

use crate::main_window::MainWindow;
use ferroui_base::Ref;
use ferroui_controls::application_lifetimes::{IApplicationLifetime, ISingleViewApplicationLifetime};
use ferroui_controls::{AppBuilder, Control};
use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
use ferroui_skia::SkiaApplicationExtensions;
use ferroui_win32::Win32ApplicationExtensions;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    /// `App.Lifetime` of the reference.
    static LIFETIME: Rc<SingleViewLifetime> = Rc::new(SingleViewLifetime::default());
    /// `_window` of the reference's application.
    static WINDOW: RefCell<Option<Rc<MainWindow>>> = const { RefCell::new(None) };
}

pub struct App;

impl App {
    /// `App.Lifetime`.
    pub fn lifetime() -> Rc<SingleViewLifetime> {
        LIFETIME.with(Rc::clone)
    }

    /// `OnLaunched`.
    pub fn on_launched(smoke: bool) {
        let lifetime: Rc<dyn IApplicationLifetime> = Self::lifetime();
        AppBuilder::configure::<control_catalog::App>()
            .use_win32()
            .use_skia()
            .use_harfbuzz()
            .setup_with_lifetime(lifetime);
        crate::smoke::report(true, "the framework is set up: the Windows backend, Skia and HarfBuzz");

        let window = match MainWindow::new() {
            Ok(window) => window,
            Err(error) => panic!("creating the main window failed: {error}"),
        };
        if let Err(error) = window.activate() {
            panic!("activating the main window failed: {error}");
        }
        if smoke {
            crate::smoke::start(window.clone());
        }
        WINDOW.with(|slot| *slot.borrow_mut() = Some(window));
    }
}

/// `SingleViewLifetime` of the reference's sample.
#[derive(Default)]
pub struct SingleViewLifetime {
    main_view: RefCell<Option<Ref<Control>>>,
}

impl IApplicationLifetime for SingleViewLifetime {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_single_view_application_lifetime(&self) -> Option<&dyn ISingleViewApplicationLifetime> {
        Some(self)
    }
}

impl ISingleViewApplicationLifetime for SingleViewLifetime {
    fn main_view(&self) -> Option<Ref<Control>> {
        self.main_view.borrow().clone()
    }

    fn set_main_view(&self, value: Option<Ref<Control>>) {
        *self.main_view.borrow_mut() = value;
    }
}
