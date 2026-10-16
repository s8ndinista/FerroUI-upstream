//! Port of `Embedding/Win32TextBoxFactory.cs`: the native text box of the
//! Windows platform, an `EDIT` control whose window procedure the sample
//! replaces to raise the events of the text box.

use super::win_api::{self, TME_HOVER, TME_LEAVE, WM_CONTEXTMENU, WM_MOUSEHOVER, WM_MOUSELEAVE, WM_MOUSEMOVE};
use super::{INativeTextBoxFactory, INativeTextBoxImpl, Win32WindowControlHandle};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_controls::platform::IPlatformHandle;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::TRACKMOUSEEVENT;

pub struct Win32TextBoxFactory;

impl INativeTextBoxFactory for Win32TextBoxFactory {
    fn create_control(&self, parent: Rc<dyn IPlatformHandle>) -> Rc<dyn INativeTextBoxImpl> {
        Win32TextBox::new(parent)
    }
}

/// The handlers of an event without arguments.
#[derive(Default)]
struct Handlers {
    list: Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
}

impl Handlers {
    fn add(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.list.borrow_mut().push(handler.clone());
        let list = Rc::downgrade(&self.list);
        Disposable::create(move || {
            if let Some(list) = list.upgrade() {
                let mut list = list.borrow_mut();
                if let Some(index) = list.iter().position(|other| Rc::ptr_eq(other, &handler)) {
                    list.remove(index);
                }
            }
        })
    }

    fn is_empty(&self) -> bool {
        self.list.borrow().is_empty()
    }

    fn invoke(&self) {
        // The handlers run after the borrow ends: a handler may add or remove one.
        let handlers: Vec<Rc<dyn Fn()>> = self.list.borrow().clone();
        for handler in handlers {
            handler();
        }
    }
}

ferroui_base::ferro_thread_local! {
    /// The text box of each window of the sample, by its handle: what the window procedure finds
    /// its text box with (the managed original keeps a delegate that captures the text box).
    static TEXT_BOXES: RefCell<HashMap<isize, Weak<Win32TextBox>>> = RefCell::new(HashMap::new());
}

/// `Win32TextBoxFactory.Win32TextBox`.
struct Win32TextBox {
    hwnd: isize,
    old_wnd_proc: Cell<isize>,
    track_mouse_event: Cell<TRACKMOUSEEVENT>,
    handle: Rc<Win32WindowControlHandle>,
    context_menu_requested: Handlers,
    hovered: Handlers,
    pointer_exited: Handlers,
}

impl Win32TextBox {
    fn new(parent: Rc<dyn IPlatformHandle>) -> Rc<Win32TextBox> {
        let handle = win_api::create_window_ex(
            0,
            "EDIT",
            "",
            win_api::WS_CHILD | win_api::WS_VISIBLE | win_api::WS_BORDER,
            parent.handle() as HWND,
        );
        assert!(!handle.is_null(), "the native text box is created");

        let track_mouse_event = TRACKMOUSEEVENT {
            cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
            dwFlags: TME_HOVER | TME_LEAVE,
            hwndTrack: handle,
            dwHoverTime: 400,
        };

        let this = Rc::new(Win32TextBox {
            hwnd: handle as isize,
            old_wnd_proc: Cell::new(0),
            track_mouse_event: Cell::new(track_mouse_event),
            handle: Rc::new(Win32WindowControlHandle::new(handle as isize, "HWND")),
            context_menu_requested: Handlers::default(),
            hovered: Handlers::default(),
            pointer_exited: Handlers::default(),
        });
        TEXT_BOXES.with(|text_boxes| text_boxes.borrow_mut().insert(this.hwnd, Rc::downgrade(&this)));
        // SAFETY: the procedure passes every message it does not handle to the procedure
        // this returns, which is kept before a message can arrive (the window is of this
        // thread).
        let old = unsafe { win_api::set_window_procedure(handle, wnd_proc) };
        this.old_wnd_proc.set(old);
        this
    }

    /// The text box of the window `hwnd`.
    fn of(hwnd: HWND) -> Option<Rc<Win32TextBox>> {
        TEXT_BOXES.with(|text_boxes| text_boxes.borrow().get(&(hwnd as isize)).and_then(Weak::upgrade))
    }
}

impl Drop for Win32TextBox {
    fn drop(&mut self) {
        let hwnd = self.hwnd;
        let _ = TEXT_BOXES.try_with(|text_boxes| text_boxes.borrow_mut().remove(&hwnd));
    }
}

/// `WndProc`: the events of the text box, then the procedure of the class.
unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    let Some(this) = Win32TextBox::of(hwnd) else {
        // SAFETY: no text box: the window is being destroyed after its text box went away;
        // the procedure of the class of the system answers.
        return unsafe { windows_sys::Win32::UI::WindowsAndMessaging::DefWindowProcW(hwnd, msg, w_param, l_param) };
    };
    match msg {
        WM_CONTEXTMENU => {
            if !this.context_menu_requested.is_empty() {
                this.context_menu_requested.invoke();
                return 0;
            }
        }
        WM_MOUSELEAVE => this.pointer_exited.invoke(),
        WM_MOUSEHOVER => this.hovered.invoke(),
        WM_MOUSEMOVE => {
            let mut event = this.track_mouse_event.get();
            win_api::track_mouse_event(&mut event);
            this.track_mouse_event.set(event);
        }
        _ => {}
    }

    // SAFETY: the procedure the window had before the sample replaced it.
    unsafe { win_api::call_window_proc(this.old_wnd_proc.get(), hwnd, msg, w_param, l_param) }
}

impl INativeTextBoxImpl for Win32TextBox {
    fn handle(&self) -> Rc<dyn IPlatformHandle> {
        self.handle.clone()
    }

    fn text(&self) -> String {
        win_api::get_window_text(self.hwnd as HWND, 256)
    }

    fn set_text(&self, value: &str) {
        win_api::set_window_text(self.hwnd as HWND, value);
    }

    fn context_menu_requested(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.context_menu_requested.add(handler)
    }

    fn hovered(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.hovered.add(handler)
    }

    fn pointer_exited(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.pointer_exited.add(handler)
    }
}
