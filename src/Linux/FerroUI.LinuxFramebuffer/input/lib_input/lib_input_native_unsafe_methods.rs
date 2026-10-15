//! The functions and enumerations of `libinput` (the port of
//! `LibInputNativeUnsafeMethods.cs`).
//!
//! The reference declares platform invokes of `libinput.so.10`, which its
//! runtime resolves when one is first called. Here the library is opened
//! with `dlopen` and every function is resolved when the backend starts
//! (`native_library.rs`; docs/porting/linux-framebuffer-platform.md,
//! section 3): nothing links the library, and a machine without it gets an
//! error that names it.

#![allow(non_camel_case_types)]

/// `libinput_event_type`.
pub mod lib_input_event_type {
    pub const LIBINPUT_EVENT_NONE: i32 = 0;
    pub const LIBINPUT_EVENT_DEVICE_ADDED: i32 = 1;
    pub const LIBINPUT_EVENT_DEVICE_REMOVED: i32 = 2;
    pub const LIBINPUT_EVENT_KEYBOARD_KEY: i32 = 300;
    pub const LIBINPUT_EVENT_POINTER_MOTION: i32 = 400;
    pub const LIBINPUT_EVENT_POINTER_MOTION_ABSOLUTE: i32 = 401;
    pub const LIBINPUT_EVENT_POINTER_BUTTON: i32 = 402;
    pub const LIBINPUT_EVENT_POINTER_AXIS: i32 = 403;
    pub const LIBINPUT_EVENT_POINTER_SCROLL_WHEEL: i32 = 404;
    pub const LIBINPUT_EVENT_POINTER_SCROLL_FINGER: i32 = 405;
    pub const LIBINPUT_EVENT_POINTER_SCROLL_CONTINUOUS: i32 = 406;
    pub const LIBINPUT_EVENT_TOUCH_DOWN: i32 = 500;
    pub const LIBINPUT_EVENT_TOUCH_UP: i32 = 501;
    pub const LIBINPUT_EVENT_TOUCH_MOTION: i32 = 502;
    pub const LIBINPUT_EVENT_TOUCH_CANCEL: i32 = 503;
    pub const LIBINPUT_EVENT_TOUCH_FRAME: i32 = 504;
    pub const LIBINPUT_EVENT_TABLET_TOOL_AXIS: i32 = 600;
    pub const LIBINPUT_EVENT_TABLET_TOOL_PROXIMITY: i32 = 601;
    pub const LIBINPUT_EVENT_TABLET_TOOL_TIP: i32 = 602;
    pub const LIBINPUT_EVENT_TABLET_TOOL_BUTTON: i32 = 603;
    pub const LIBINPUT_EVENT_TABLET_PAD_BUTTON: i32 = 700;
    pub const LIBINPUT_EVENT_TABLET_PAD_RING: i32 = 701;
    pub const LIBINPUT_EVENT_TABLET_PAD_STRIP: i32 = 702;
    pub const LIBINPUT_EVENT_GESTURE_SWIPE_BEGIN: i32 = 800;
    pub const LIBINPUT_EVENT_GESTURE_SWIPE_UPDATE: i32 = 801;
    pub const LIBINPUT_EVENT_GESTURE_SWIPE_END: i32 = 802;
    pub const LIBINPUT_EVENT_GESTURE_PINCH_BEGIN: i32 = 803;
    pub const LIBINPUT_EVENT_GESTURE_PINCH_UPDATE: i32 = 804;
    pub const LIBINPUT_EVENT_GESTURE_PINCH_END: i32 = 805;
    pub const LIBINPUT_EVENT_SWITCH_TOGGLE: i32 = 900;
}

/// `libinput_pointer_axis_source`.
pub mod lib_input_pointer_axis_source {
    /// The event is caused by the rotation of a wheel.
    pub const LIBINPUT_POINTER_AXIS_SOURCE_WHEEL: i32 = 1;
    /// The event is caused by the movement of one or more fingers on a device.
    pub const LIBINPUT_POINTER_AXIS_SOURCE_FINGER: i32 = 2;
    /// The event is caused by the motion of some device.
    pub const LIBINPUT_POINTER_AXIS_SOURCE_CONTINUOUS: i32 = 3;
    /// The event is caused by the tilting of a mouse wheel rather than
    /// its rotation. Deprecated as of libinput 1.16.
    pub const LIBINPUT_POINTER_AXIS_SOURCE_WHEEL_TILT: i32 = 4;
}

/// `libinput_pointer_axis`.
pub mod lib_input_pointer_axis {
    pub const LIBINPUT_POINTER_AXIS_SCROLL_VERTICAL: i32 = 0;
    pub const LIBINPUT_POINTER_AXIS_SCROLL_HORIZONTAL: i32 = 1;
}

#[cfg(target_os = "linux")]
pub use imp::LibInput;

#[cfg(target_os = "linux")]
mod imp {
    use std::ffi::{c_char, c_double, c_int, c_void, CString};

    /// `struct libinput_interface`: how the library opens and closes the
    /// devices.
    #[repr(C)]
    pub(crate) struct libinput_interface {
        open_restricted: unsafe extern "C" fn(path: *const c_char, flags: c_int, user_data: *mut c_void) -> c_int,
        close_restricted: unsafe extern "C" fn(fd: c_int, user_data: *mut c_void),
    }

    unsafe extern "C" fn open_restricted(path: *const c_char, flags: c_int, _user_data: *mut c_void) -> c_int {
        // SAFETY: the library passes a valid C string.
        let fd = unsafe { libc::open(path, flags, 0) };
        if fd == -1 {
            return -std::io::Error::last_os_error().raw_os_error().unwrap_or(libc::EIO);
        }
        fd
    }

    unsafe extern "C" fn close_restricted(fd: c_int, _user_data: *mut c_void) {
        // SAFETY: the library closes a descriptor `open_restricted` gave it.
        unsafe {
            libc::close(fd);
        }
    }

    static INTERFACE: libinput_interface = libinput_interface { open_restricted, close_restricted };

    ferroui_base::native_functions! {
        /// The functions of the library, resolved.
        pub struct LibInput(lib) {
            lib::libinput_path_create_context: unsafe extern "C" fn(*const libinput_interface, *mut c_void) -> *mut c_void;
            lib::libinput_path_add_device: unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void;
            lib::libinput_path_remove_device: unsafe extern "C" fn(*mut c_void);
            lib::libinput_device_config_calibration_set_matrix: unsafe extern "C" fn(*mut c_void, *const f32) -> c_int;
            lib::libinput_get_fd: unsafe extern "C" fn(*mut c_void) -> c_int;
            lib::libinput_dispatch: unsafe extern "C" fn(*mut c_void) -> c_int;
            lib::libinput_get_event: unsafe extern "C" fn(*mut c_void) -> *mut c_void;
            lib::libinput_event_get_type: unsafe extern "C" fn(*mut c_void) -> c_int;
            lib::libinput_event_destroy: unsafe extern "C" fn(*mut c_void);
            lib::libinput_event_get_touch_event: unsafe extern "C" fn(*mut c_void) -> *mut c_void;
            lib::libinput_event_touch_get_slot: unsafe extern "C" fn(*mut c_void) -> c_int;
            lib::libinput_event_touch_get_time_usec: unsafe extern "C" fn(*mut c_void) -> u64;
            lib::libinput_event_touch_get_x_transformed: unsafe extern "C" fn(*mut c_void, u32) -> c_double;
            lib::libinput_event_touch_get_y_transformed: unsafe extern "C" fn(*mut c_void, u32) -> c_double;
            lib::libinput_event_get_pointer_event: unsafe extern "C" fn(*mut c_void) -> *mut c_void;
            lib::libinput_event_pointer_get_time_usec: unsafe extern "C" fn(*mut c_void) -> u64;
            lib::libinput_event_pointer_get_absolute_x_transformed: unsafe extern "C" fn(*mut c_void, u32) -> c_double;
            lib::libinput_event_pointer_get_absolute_y_transformed: unsafe extern "C" fn(*mut c_void, u32) -> c_double;
            lib::libinput_event_pointer_get_button: unsafe extern "C" fn(*mut c_void) -> u32;
            lib::libinput_event_pointer_get_button_state: unsafe extern "C" fn(*mut c_void) -> c_int;
            lib::libinput_event_pointer_get_axis_source: unsafe extern "C" fn(*mut c_void) -> c_int;
            lib::libinput_event_pointer_get_axis_value_discrete: unsafe extern "C" fn(*mut c_void, c_int) -> c_double;
            lib::libinput_event_pointer_get_scroll_value_v120: unsafe extern "C" fn(*mut c_void, c_int) -> c_double;
        }
    }

    /// A context of the library with the path backend. It belongs to the
    /// thread that reads it.
    pub struct Context<'a> {
        lib: &'a LibInput,
        ctx: *mut c_void,
    }

    /// An event of a context, destroyed when dropped.
    pub struct Event<'a> {
        lib: &'a LibInput,
        ev: *mut c_void,
    }

    impl LibInput {
        /// `libinput_path_create_context` with the interface of this
        /// module.
        pub fn path_create_context(&self) -> Option<Context<'_>> {
            // SAFETY: the interface is a static of the layout the library
            // expects; the user data is not used.
            let ctx = unsafe { (self.libinput_path_create_context)(&INTERFACE, std::ptr::null_mut()) };
            (!ctx.is_null()).then_some(Context { lib: self, ctx })
        }
    }

    // The calls below pass pointers the library returned for this context
    // (or events of it that are alive), which is what each function
    // requires; a context is used by one thread.
    impl<'a> Context<'a> {
        /// Adds a device; returns whether the library took it. The
        /// calibration matrix (six numbers, the first two rows) is set on
        /// a device that was taken.
        pub fn path_add_device(&self, path: &str, calibration: &[f32; 6]) -> bool {
            let Ok(path) = CString::new(path) else {
                return false;
            };
            // SAFETY: see above; `path` is a valid C string.
            let device = unsafe { (self.lib.libinput_path_add_device)(self.ctx, path.as_ptr()) };
            if device.is_null() {
                return false;
            }
            // SAFETY: see above; the matrix has the six numbers the
            // function reads.
            unsafe {
                (self.lib.libinput_device_config_calibration_set_matrix)(device, calibration.as_ptr());
            }
            true
        }

        pub fn get_fd(&self) -> i32 {
            // SAFETY: see above.
            unsafe { (self.lib.libinput_get_fd)(self.ctx) }
        }

        pub fn dispatch(&self) {
            // SAFETY: see above.
            unsafe {
                (self.lib.libinput_dispatch)(self.ctx);
            }
        }

        pub fn get_event(&self) -> Option<Event<'a>> {
            // SAFETY: see above.
            let ev = unsafe { (self.lib.libinput_get_event)(self.ctx) };
            (!ev.is_null()).then_some(Event { lib: self.lib, ev })
        }
    }

    impl Event<'_> {
        pub fn get_type(&self) -> i32 {
            // SAFETY: see above.
            unsafe { (self.lib.libinput_event_get_type)(self.ev) }
        }

        /// The slot and the time (in microseconds) of a touch event, and
        /// its position as fractions of a range of `range` when
        /// `with_position` (a down or a motion).
        pub fn touch(&self, with_position: bool, range: u32) -> Option<(i32, u64, f64, f64)> {
            // SAFETY: see above; the accessors are called for the event
            // types they are defined for (the caller checks the type).
            unsafe {
                let tev = (self.lib.libinput_event_get_touch_event)(self.ev);
                if tev.is_null() {
                    return None;
                }
                let slot = (self.lib.libinput_event_touch_get_slot)(tev);
                let time = (self.lib.libinput_event_touch_get_time_usec)(tev);
                let (x, y) = if with_position {
                    (
                        (self.lib.libinput_event_touch_get_x_transformed)(tev, range),
                        (self.lib.libinput_event_touch_get_y_transformed)(tev, range),
                    )
                } else {
                    (0.0, 0.0)
                };
                Some((slot, time, x, y))
            }
        }

        fn pointer(&self) -> *mut c_void {
            // SAFETY: see above.
            unsafe { (self.lib.libinput_event_get_pointer_event)(self.ev) }
        }

        pub fn pointer_time_usec(&self) -> u64 {
            // SAFETY: see above.
            unsafe { (self.lib.libinput_event_pointer_get_time_usec)(self.pointer()) }
        }

        /// The position of an absolute motion in a range of `range`.
        pub fn pointer_absolute(&self, range: u32) -> (f64, f64) {
            // SAFETY: see above; called for absolute motion only.
            unsafe {
                let pev = self.pointer();
                (
                    (self.lib.libinput_event_pointer_get_absolute_x_transformed)(pev, range),
                    (self.lib.libinput_event_pointer_get_absolute_y_transformed)(pev, range),
                )
            }
        }

        /// The button and its state of a button event.
        pub fn pointer_button(&self) -> (u32, i32) {
            // SAFETY: see above; called for button events only.
            unsafe {
                let pev = self.pointer();
                (
                    (self.lib.libinput_event_pointer_get_button)(pev),
                    (self.lib.libinput_event_pointer_get_button_state)(pev),
                )
            }
        }

        /// The source of an axis event and, for a wheel, its vertical
        /// value in steps.
        pub fn pointer_axis(&self) -> (i32, f64) {
            // SAFETY: see above; called for axis events only, and the
            // discrete value is read for wheels only.
            unsafe {
                let pev = self.pointer();
                let source = (self.lib.libinput_event_pointer_get_axis_source)(pev);
                let value = if source == super::lib_input_pointer_axis_source::LIBINPUT_POINTER_AXIS_SOURCE_WHEEL {
                    (self.lib.libinput_event_pointer_get_axis_value_discrete)(
                        pev,
                        super::lib_input_pointer_axis::LIBINPUT_POINTER_AXIS_SCROLL_VERTICAL,
                    )
                } else {
                    0.0
                };
                (source, value)
            }
        }

        /// The vertical value of a wheel scroll event, in 120ths of a step.
        pub fn pointer_scroll_value_v120(&self) -> f64 {
            // SAFETY: see above; called for wheel scroll events only.
            unsafe {
                (self.lib.libinput_event_pointer_get_scroll_value_v120)(
                    self.pointer(),
                    super::lib_input_pointer_axis::LIBINPUT_POINTER_AXIS_SCROLL_VERTICAL,
                )
            }
        }
    }

    impl Drop for Event<'_> {
        fn drop(&mut self) {
            // SAFETY: the event is destroyed once.
            unsafe {
                (self.lib.libinput_event_destroy)(self.ev);
            }
        }
    }
}
