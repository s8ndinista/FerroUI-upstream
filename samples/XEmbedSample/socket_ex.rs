//! Port of `SocketEx.cs`: a socket of GTK that embeds a plug of the framework
//! (`FerroXEmbedGtkSocket`, the socket class of the reference).
//!
//! The reference derives from the socket class of GtkSharp and overrides `OnRealized`,
//! `OnSizeAllocated` and `OnDestroyed`; GtkSharp calls those overrides from the class handlers
//! of the signals `realize`, `size-allocate` and `destroy`. The port makes a socket of GTK and
//! connects handlers to the same signals: after the handler of the class for `realize` and
//! `size-allocate` (the overrides call the base first), before it for `destroy` (the override
//! calls the base last).

use crate::gtk::{self, api, GdkRgba, GtkAllocation, Widget};
use ferroui_base::media::Color;
use ferroui_base::{BoxedValue, PixelSize};
use ferroui_x11::XEmbedPlug;
use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;

/// A socket of GTK whose plug is a window of the framework showing [`content`](Self::content).
pub struct FerroXEmbedGtkSocket {
    widget: Widget,
    background_color: GdkRgba,
    plug: RefCell<Option<XEmbedPlug>>,
    content: RefCell<Option<BoxedValue>>,
}

impl FerroXEmbedGtkSocket {
    /// A socket whose plug has `background_color`, the background of the window of GTK.
    pub fn new(background_color: GdkRgba) -> Rc<FerroXEmbedGtkSocket> {
        // SAFETY: GTK is initialized on this thread (`program.rs`).
        let widget = unsafe { (api().gtk_socket_new)() };
        let this = Rc::new(FerroXEmbedGtkSocket {
            widget,
            background_color,
            plug: RefCell::new(None),
            content: RefCell::new(None),
        });
        // The handlers hold the socket: one strong reference, released by the handler of
        // `destroy` once the widget is gone.
        let data = Rc::into_raw(this.clone()) as *mut c_void;
        // SAFETY: the widget is the socket just made; each handler has the C signature of its
        // signal with the data last, and the data stays valid until `destroy` releases it.
        unsafe {
            gtk::signal_connect(widget, c"realize", on_realized as *const c_void, data, true);
            gtk::signal_connect(widget, c"size-allocate", on_size_allocated as *const c_void, data, true);
            gtk::signal_connect(widget, c"destroy", on_destroyed as *const c_void, data, false);
        }
        this
    }

    /// The widget of GTK.
    pub fn widget(&self) -> Widget {
        self.widget
    }

    pub fn content(&self) -> Option<BoxedValue> {
        self.content.borrow().clone()
    }

    pub fn set_content(&self, value: Option<BoxedValue>) {
        *self.content.borrow_mut() = value.clone();
        if let Some(plug) = self.plug.borrow().as_ref() {
            plug.set_content(value);
        }
    }

    /// The window of the plug on the X server, once the socket is realized.
    pub fn plug_handle(&self) -> Option<u64> {
        self.plug.borrow().as_ref().map(|plug| plug.handle() as u64)
    }

    /// `ScaleFactor` of the widget.
    fn scale_factor(&self) -> i32 {
        // SAFETY: the widget is alive while its handlers run.
        unsafe { (api().gtk_widget_get_scale_factor)(self.widget) }
    }

    fn on_realized(&self) {
        if self.plug.borrow().is_none() {
            *self.plug.borrow_mut() = Some(XEmbedPlug::create());
        }
        // The handlers GTK runs while the plug is added (`size-allocate`) read the plug too.
        let plug = self.plug.borrow();
        let Some(plug) = plug.as_ref() else { return };
        plug.set_scale_factor(self.scale_factor() as f64);
        plug.set_background_color(Color::from_rgb(
            (self.background_color.red * 255.0) as u8,
            (self.background_color.green * 255.0) as u8,
            (self.background_color.blue * 255.0) as u8,
        ));
        plug.set_content(self.content.borrow().clone());
        Self::apply_interactive_resize(self.widget, Some(plug));
        // SAFETY: the widget is the socket; the window is the plug's, which exists.
        unsafe { (api().gtk_socket_add_id)(self.widget, plug.handle() as _) };
    }

    /// Not a part of XEmbed, but allows a smooth resize.
    fn apply_interactive_resize(widget: Widget, plug: Option<&XEmbedPlug>) {
        let mut rect = GtkAllocation::default();
        // SAFETY: the widget is alive while its handlers run; the out-arguments are locals.
        let scale = unsafe {
            (api().gtk_widget_get_allocated_size)(widget, &mut rect, std::ptr::null_mut());
            (api().gtk_widget_get_scale_factor)(widget)
        };
        if let Some(plug) = plug {
            plug.process_interactive_resize(PixelSize::new(rect.width * scale, rect.height * scale));
        }
    }

    fn on_size_allocated(&self) {
        // SAFETY: GDK is initialized; the display is the default one.
        unsafe { (api().gdk_display_sync)((api().gdk_display_get_default)()) };
        Self::apply_interactive_resize(self.widget, self.plug.borrow().as_ref());
    }

    fn on_destroyed(&self) {
        let plug = self.plug.borrow_mut().take();
        if let Some(plug) = plug {
            plug.dispose();
        }
    }
}

/// The socket a handler was connected with.
///
/// # Safety
/// `data` is the pointer [`FerroXEmbedGtkSocket::new`] made, not released yet.
unsafe fn socket<'a>(data: *mut c_void) -> &'a FerroXEmbedGtkSocket {
    unsafe { &*(data as *const FerroXEmbedGtkSocket) }
}

extern "C" fn on_realized(_widget: Widget, data: *mut c_void) {
    // SAFETY: the data of the handler (`new`), released only by `destroy`.
    unsafe { socket(data) }.on_realized();
}

extern "C" fn on_size_allocated(_widget: Widget, _allocation: *mut GtkAllocation, data: *mut c_void) {
    // SAFETY: as above.
    unsafe { socket(data) }.on_size_allocated();
}

extern "C" fn on_destroyed(_widget: Widget, data: *mut c_void) {
    // SAFETY: as above; `destroy` is emitted once, and the reference the handlers hold is
    // released after the last of them ran.
    unsafe {
        socket(data).on_destroyed();
        drop(Rc::from_raw(data as *const FerroXEmbedGtkSocket));
    }
}
