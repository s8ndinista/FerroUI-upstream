//! The functions of GTK 3, GDK 3 and GObject the sample calls: the counterpart of the GtkSharp
//! package the reference references (not a file of the reference). The libraries are opened at
//! run time, as the X11 platform opens GTK for its dialogs; a missing library or function ends
//! the program with its name.
//!
//! GTK belongs to the thread that initialized it, the main thread of the program, which runs
//! the main loop of GLib that the X11 platform shares (`UseGLibMainLoop`). Every function here
//! is called on that thread.

use ferroui_base::native_functions;
use ferroui_base::platform::native_library::NativeLibrary;
use std::ffi::{c_char, c_int, c_ulong, c_void, CStr, CString};
use std::sync::OnceLock;

/// `GtkAllocation` (`GdkRectangle`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GtkAllocation {
    pub x: c_int,
    pub y: c_int,
    pub width: c_int,
    pub height: c_int,
}

/// `GdkRGBA`: the channels from 0 to 1.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GdkRgba {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

/// `GTK_WINDOW_TOPLEVEL`.
pub const GTK_WINDOW_TOPLEVEL: c_int = 0;
/// `GTK_ORIENTATION_VERTICAL`.
pub const GTK_ORIENTATION_VERTICAL: c_int = 1;
/// `GTK_STATE_FLAG_NORMAL`.
pub const GTK_STATE_FLAG_NORMAL: c_int = 0;
/// `G_CONNECT_AFTER`.
pub const G_CONNECT_AFTER: c_int = 1;

native_functions! {
    /// The functions, as C declares them (a `gboolean` is an `int`).
    pub struct GtkApi(gtk, gdk, gobject) {
        gtk::gtk_init: unsafe extern "C" fn(*mut c_int, *mut *mut *mut c_char);
        gtk::gtk_main: unsafe extern "C" fn();
        gtk::gtk_main_quit: unsafe extern "C" fn();
        gtk::gtk_window_new: unsafe extern "C" fn(c_int) -> *mut c_void;
        gtk::gtk_window_set_title: unsafe extern "C" fn(*mut c_void, *const c_char);
        gtk::gtk_socket_new: unsafe extern "C" fn() -> *mut c_void;
        gtk::gtk_socket_add_id: unsafe extern "C" fn(*mut c_void, c_ulong);
        gtk::gtk_box_new: unsafe extern "C" fn(c_int, c_int) -> *mut c_void;
        gtk::gtk_box_pack_end: unsafe extern "C" fn(*mut c_void, *mut c_void, c_int, c_int, u32);
        gtk::gtk_label_new: unsafe extern "C" fn(*const c_char) -> *mut c_void;
        gtk::gtk_entry_new: unsafe extern "C" fn() -> *mut c_void;
        gtk::gtk_button_new: unsafe extern "C" fn() -> *mut c_void;
        gtk::gtk_container_add: unsafe extern "C" fn(*mut c_void, *mut c_void);
        gtk::gtk_widget_set_size_request: unsafe extern "C" fn(*mut c_void, c_int, c_int);
        gtk::gtk_widget_realize: unsafe extern "C" fn(*mut c_void);
        gtk::gtk_widget_show_all: unsafe extern "C" fn(*mut c_void);
        gtk::gtk_widget_destroy: unsafe extern "C" fn(*mut c_void);
        gtk::gtk_widget_get_scale_factor: unsafe extern "C" fn(*mut c_void) -> c_int;
        gtk::gtk_widget_get_allocated_size: unsafe extern "C" fn(*mut c_void, *mut GtkAllocation, *mut c_int);
        gtk::gtk_widget_get_style_context: unsafe extern "C" fn(*mut c_void) -> *mut c_void;
        gtk::gtk_style_context_get_background_color: unsafe extern "C" fn(*mut c_void, c_int, *mut GdkRgba);
        gdk::gdk_set_allowed_backends: unsafe extern "C" fn(*const c_char);
        gdk::gdk_display_get_default: unsafe extern "C" fn() -> *mut c_void;
        gdk::gdk_display_sync: unsafe extern "C" fn(*mut c_void);
        gobject::g_signal_connect_data:
            unsafe extern "C" fn(*mut c_void, *const c_char, *const c_void, *mut c_void, *const c_void, c_int) -> c_ulong;
    }
}

const GTK_NAME: &CStr = c"libgtk-3.so.0";
const GDK_NAME: &CStr = c"libgdk-3.so.0";
const GOBJECT_NAME: &CStr = c"libgobject-2.0.so.0";

/// The functions, the libraries opened at the first call (the loader never closes a library).
///
/// # Panics
/// Panics with the name of a library or a function that cannot be found.
pub fn api() -> &'static GtkApi {
    static API: OnceLock<GtkApi> = OnceLock::new();
    API.get_or_init(|| {
        let open = |name: &'static CStr| NativeLibrary::open(name).unwrap_or_else(|error| panic!("{error}"));
        GtkApi::load(&open(GTK_NAME), &open(GDK_NAME), &open(GOBJECT_NAME)).unwrap_or_else(|error| panic!("{error}"))
    })
}

/// A text as C takes it; a text with a zero in it ends there.
pub fn c_string(text: &str) -> CString {
    let end = text.find('\0').unwrap_or(text.len());
    CString::new(&text[..end]).unwrap_or_default()
}

/// A widget of GTK (or another GObject), by its address.
pub type Widget = *mut c_void;

/// `g_signal_connect` (`after` false) or `g_signal_connect_after` (`after` true) for a handler
/// with the given C signature, passed as an untyped address.
///
/// # Safety
/// `instance` is a live GObject that has the signal, and `handler` is an `extern "C"` function
/// whose signature is the one of the signal with `data` as its last argument, valid while the
/// handler may run.
pub unsafe fn signal_connect(instance: Widget, signal: &CStr, handler: *const c_void, data: *mut c_void, after: bool) {
    unsafe {
        (api().g_signal_connect_data)(
            instance,
            signal.as_ptr(),
            handler,
            data,
            std::ptr::null(),
            if after { G_CONNECT_AFTER } else { 0 },
        )
    };
}
