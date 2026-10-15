//! A WinUI 3 application and its windows for a Rust program.
//!
//! The reference hosts the panel in an application written in C# and XAML,
//! which the tools of the SDK compile: `App : Application` with its
//! `OnLaunched`, `MainWindow : Window` with a grid that holds the panel, and
//! a `Main` that calls `Application.Start`. A Rust program has no XAML
//! compiler; this module is what that generated code does: [`XamlApplication::start`]
//! runs the application with a closure in place of `OnLaunched` (the
//! application object is composed, `interop/composable.rs`, and overrides
//! `OnLaunched`), and [`XamlWindow`] and [`XamlGrid`] are the window and the
//! grid of the sample's markup (docs/porting/winui.md, section 3.4).

#[cfg(windows)]
pub use imp::{XamlApplication, XamlGrid, XamlWindow};

#[cfg(windows)]
mod imp {
    use crate::interop::composable::{compose, Composed, ComposedBase};
    use crate::interop::event_handlers::{application_initialization_callback, window_closed_handler};
    use crate::interop::win_rt::{factory, hstring};
    use crate::interop::*;
    use ferroui_microcom::{ComPtr, HResult};
    use ferroui_win32::wnd_proc_guard;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[cfg_attr(not(target_arch = "x86"), link(name = "combase", kind = "raw-dylib"))]
    #[cfg_attr(target_arch = "x86", link(name = "combase", kind = "raw-dylib", import_name_type = "undecorated"))]
    extern "system" {
        fn RoInitialize(init_type: i32) -> i32;
    }

    /// `RO_INIT_SINGLETHREADED`: the thread of a XAML application is a
    /// single-threaded apartment (`[STAThread]` of the C# application).
    const RO_INIT_SINGLETHREADED: i32 = 0;

    type Launched = RefCell<Option<Box<dyn FnOnce(&XamlApplication)>>>;

    /// The outer object of the composed `Application`: `App` of the
    /// reference's sample, whose `OnLaunched` it overrides.
    struct ApplicationOuter {
        base: ComposedBase,
        launched: Rc<Launched>,
    }

    impl Composed for ApplicationOuter {
        fn composed_base(&self) -> &ComposedBase {
            &self.base
        }
    }

    inspectable!(ApplicationOuter, "FerroUI.WinUI.XamlApplication", [IApplicationOverrides]);

    impl IApplicationOverridesImpl for ApplicationOuter {
        fn on_launched(&self, _args: Option<&IInspectable>) -> Result<(), HResult> {
            let launched = self.launched.borrow_mut().take();
            if let Some(launched) = launched {
                wnd_proc_guard::guard((), || launched(&XamlApplication));
            }
            Ok(())
        }
    }

    /// The application. See the module.
    pub struct XamlApplication;

    impl XamlApplication {
        /// Initialises the thread as a single-threaded apartment and runs
        /// `Application.Start`: the message loop of the application, until it
        /// exits. `on_launched` is called once, from `OnLaunched`. A panic of
        /// a handler the system called ends the loop and is raised again here.
        pub fn start(on_launched: impl FnOnce(&XamlApplication) + 'static) -> Result<(), HResult> {
            // SAFETY: takes a number. S_FALSE (initialised already) is a
            // success.
            let hr = unsafe { RoInitialize(RO_INIT_SINGLETHREADED) };
            if hr < 0 {
                return Err(HResult(hr as u32));
            }

            wnd_proc_guard::set_pending_hook(Some(Rc::new(|| XamlApplication.exit())));

            let launched: Rc<Launched> = Rc::new(RefCell::new(Some(Box::new(on_launched))));
            let application: Rc<RefCell<Option<ComPtr<IApplicationOverrides>>>> = Rc::new(RefCell::new(None));
            let callback = {
                let application = application.clone();
                application_initialization_callback(move |_| {
                    // `Application.Start(p => new App())` of the generated
                    // program: the application object is created on the
                    // thread of the application.
                    let created = factory::<IApplicationFactory>("Microsoft.UI.Xaml.Application").and_then(|application_factory| {
                        compose::<IApplicationOverrides, ApplicationOuter, 0>(
                            ApplicationOuter { base: ComposedBase::default(), launched: launched.clone() },
                            [],
                            |outer, inner| {
                                // SAFETY: the outer object and a place of the
                                // caller's frame.
                                unsafe { application_factory.create_instance(Some(outer), inner) }.map(|_| ())
                            },
                        )
                    });
                    match created {
                        Ok(created) => *application.borrow_mut() = Some(created),
                        Err(error) => panic!("creating the XAML application failed: {error}"),
                    }
                })
            };

            let statics = factory::<IApplicationStatics>("Microsoft.UI.Xaml.Application")?;
            let result = statics.start(Some(&callback));
            drop(application.borrow_mut().take());
            wnd_proc_guard::set_pending_hook(None);
            wnd_proc_guard::resume_pending();
            result
        }

        /// `Application.Current.Exit()`: ends the message loop.
        pub fn exit(&self) {
            let current = factory::<IApplicationStatics>("Microsoft.UI.Xaml.Application")
                .and_then(|statics| statics.current())
                .ok()
                .flatten();
            if let Some(current) = current {
                let _ = current.exit();
            }
        }
    }

    /// A window of WinUI (`Microsoft.UI.Xaml.Window`).
    pub struct XamlWindow {
        window: ComPtr<IWindow>,
    }

    impl XamlWindow {
        pub fn new() -> Result<XamlWindow, HResult> {
            let window_factory = factory::<IWindowFactory>("Microsoft.UI.Xaml.Window")?;
            let mut inner: *mut IInspectable = std::ptr::null_mut();
            // SAFETY: no outer object (the window is not derived from); the
            // inner object is not asked for and released below.
            let window = unsafe { window_factory.create_instance(None, &mut inner) }?.ok_or(HResult::POINTER)?;
            if !inner.is_null() {
                // SAFETY: an owned reference the factory wrote.
                unsafe { (*(inner as *const ferroui_microcom::IUnknown)).release() };
            }
            Ok(XamlWindow { window })
        }

        pub fn set_title(&self, title: &str) -> Result<(), HResult> {
            self.window.set_title(hstring(title)?.handle())
        }

        pub fn set_content(&self, content: &IUIElement) -> Result<(), HResult> {
            self.window.set_content(Some(content))
        }

        pub fn activate(&self) -> Result<(), HResult> {
            self.window.activate()
        }

        pub fn close(&self) -> Result<(), HResult> {
            self.window.close()
        }

        /// `Closed`.
        pub fn on_closed(&self, handler: impl Fn() + 'static) -> Result<(), HResult> {
            self.window.add_closed(Some(&window_closed_handler(move |_, _| handler()))).map(|_| ())
        }

        /// The window of the bindings.
        pub fn window(&self) -> &ComPtr<IWindow> {
            &self.window
        }
    }

    /// A grid of WinUI (`Microsoft.UI.Xaml.Controls.Grid`).
    pub struct XamlGrid {
        grid: ComPtr<IInspectable>,
    }

    impl XamlGrid {
        pub fn new() -> Result<XamlGrid, HResult> {
            let grid_factory = factory::<IGridFactory>("Microsoft.UI.Xaml.Controls.Grid")?;
            let mut inner: *mut IInspectable = std::ptr::null_mut();
            // SAFETY: as `XamlWindow::new`.
            let grid = unsafe { grid_factory.create_instance(None, &mut inner) }?.ok_or(HResult::POINTER)?;
            if !inner.is_null() {
                // SAFETY: an owned reference the factory wrote.
                unsafe { (*(inner as *const ferroui_microcom::IUnknown)).release() };
            }
            Ok(XamlGrid { grid })
        }

        pub fn add_child(&self, child: &IUIElement) -> Result<(), HResult> {
            self.grid.cast::<IPanel>()?.children()?.ok_or(HResult::POINTER)?.append(Some(child))
        }

        pub fn as_ui_element(&self) -> Result<ComPtr<IUIElement>, HResult> {
            self.grid.cast::<IUIElement>()
        }
    }
}
