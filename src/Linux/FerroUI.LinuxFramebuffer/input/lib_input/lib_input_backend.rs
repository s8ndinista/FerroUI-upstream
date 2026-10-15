//! The input backend over `libinput` (the port of `LibInputBackend.cs`).
//!
//! A thread owns the context of the library and reads its events. The
//! reference maps them to raw events on that thread; raw events hold
//! objects of the UI thread here, so the thread posts each event as a
//! value ([`LibInputEvent`]) and the mapping runs on the UI thread
//! (docs/porting/linux-framebuffer-platform.md, section 4).

use super::lib_input_native_unsafe_methods::lib_input_event_type::*;
use super::LibInputBackendOptions;
use crate::input::ev_dev::EvDevRawEvent;
use ferroui_base::platform::SurfaceOrientation;
use ferroui_base::{Point, Size};
use std::collections::HashMap;

/// The range positions are asked of the library in: the thread does not
/// know the size of the screen, so it asks for a position in this range
/// and the UI thread scales it (the transformation of the library is
/// linear).
pub(crate) const POSITION_RANGE: u32 = 1 << 20;

/// What the mapping needs of an event of the library.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LibInputEvent {
    /// A touch event below `LIBINPUT_EVENT_TOUCH_FRAME`: the position is a
    /// fraction of the screen, and is meaningful for a down and a motion.
    Touch { type_: i32, slot: i32, time_usec: u64, x: f64, y: f64 },
    /// `LIBINPUT_EVENT_POINTER_MOTION_ABSOLUTE`, the position as a fraction.
    PointerMotionAbsolute { time_usec: u64, x: f64, y: f64 },
    /// `LIBINPUT_EVENT_POINTER_BUTTON`.
    PointerButton { time_usec: u64, button: u32, button_state: i32 },
    /// `LIBINPUT_EVENT_POINTER_AXIS`: the source and, for a wheel, the
    /// vertical value in steps.
    PointerAxis { time_usec: u64, source_axis: i32, value_discrete: f64 },
    /// `LIBINPUT_EVENT_POINTER_SCROLL_WHEEL`.
    PointerScrollWheel { time_usec: u64, value_v120: f64 },
    /// Another event of the pointer range.
    PointerOther { type_: i32 },
}

/// The calibration matrix (the first two rows) that turns the input of a
/// device with the orientation of the screen.
pub(crate) fn calibration_matrix(screen_orientation: SurfaceOrientation) -> [f32; 6] {
    match screen_orientation {
        SurfaceOrientation::Rotation90 => [0.0, 1.0, 0.0, -1.0, 0.0, 1.0],
        SurfaceOrientation::Rotation180 => [-1.0, 0.0, 1.0, 0.0, -1.0, 1.0],
        SurfaceOrientation::Rotation270 => [0.0, -1.0, 1.0, 1.0, 0.0, 0.0],
        _ => [1.0, 0.0, 0.0, 0.0, 1.0, 0.0], // Normal
    }
}

/// Whether the thread hands an event of this type to the touch mapping.
pub(crate) fn is_touch_event(type_: i32) -> bool {
    (LIBINPUT_EVENT_TOUCH_DOWN..=LIBINPUT_EVENT_TOUCH_CANCEL).contains(&type_)
}

/// Whether the thread hands an event of this type to the pointer
/// mapping. As in the reference the range ends at the legacy axis event:
/// the wheel scroll event that follows it in the enumeration describes
/// the same step of the wheel.
pub(crate) fn is_pointer_event(type_: i32) -> bool {
    (LIBINPUT_EVENT_POINTER_MOTION..=LIBINPUT_EVENT_POINTER_AXIS).contains(&type_)
}

/// The state the mapping keeps between events (the fields of the three
/// partial files of the reference that are not objects of the UI thread).
#[derive(Default)]
pub struct LibInputState {
    pub(crate) mouse_position: Point,
    pub(crate) pointers: HashMap<i32, Point>,
}

impl LibInputState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Maps one event. `info` is the scaled size of the screen;
    /// `has_input_root` whether an input root was set (touch events
    /// before that are dropped after the state is updated, as in the
    /// reference).
    pub fn handle(&mut self, ev: &LibInputEvent, info: Size, has_input_root: bool, out: &mut Vec<EvDevRawEvent>) {
        match ev {
            LibInputEvent::Touch { .. } => self.handle_touch(ev, info, has_input_root, out),
            _ => self.handle_pointer(ev, info, out),
        }
    }
}

/// The devices of the options, or every event device of the system.
pub(crate) fn event_devices(options: Option<&LibInputBackendOptions>) -> Vec<String> {
    if let Some(events) = options.and_then(|options| options.events.clone()) {
        return events;
    }
    let mut files: Vec<String> = std::fs::read_dir("/dev/input")
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|entry| entry.file_name().to_string_lossy().starts_with("event"))
                .map(|entry| entry.path().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

#[cfg(target_os = "linux")]
pub use imp::LibInputBackend;

#[cfg(target_os = "linux")]
mod imp {
    use super::super::lib_input_native_unsafe_methods::LibInput;
    use crate::native_library::load_library;
    use super::*;
    use crate::input::{IInputBackend, IScreenInfoProvider};
    use crate::raw_event_grouping::RawEvent;
    use ferroui_base::input::raw::{RawMouseWheelEventArgs, RawPointerEventArgs, RawTouchEventArgs};
    use ferroui_base::input::{IInputDevice, IInputRoot, MouseDevice, RawInputModifiers, TouchDevice};
    use ferroui_base::logging::{LogEventLevel, Logger};
    use ferroui_base::threading::{Dispatcher, DispatcherPriority};
    use std::cell::RefCell;
    use std::rc::{Rc, Weak};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    use std::thread;

    struct State {
        screen: RefCell<Option<Rc<dyn IScreenInfoProvider>>>,
        input_root: RefCell<Option<Rc<dyn IInputRoot>>>,
        on_input: RefCell<Option<Rc<dyn Fn(RawEvent)>>>,
        mapping: RefCell<LibInputState>,
        mouse: Rc<MouseDevice>,
        touch: Rc<TouchDevice>,
    }

    ferroui_base::ferro_thread_local! {
        /// The backends of the UI thread, by the number their input
        /// threads post with.
        static BACKENDS: RefCell<HashMap<u64, Weak<State>>> = RefCell::new(HashMap::new());
    }

    static NEXT_BACKEND: AtomicU64 = AtomicU64::new(1);

    pub struct LibInputBackend {
        options: Option<LibInputBackendOptions>,
        state: Rc<State>,
    }

    impl State {
        /// The mapping of one event, on the UI thread.
        fn handle(&self, ev: &LibInputEvent) {
            let Some(screen) = self.screen.borrow().clone() else {
                return;
            };
            let input_root = self.input_root.borrow().clone();
            let mut out = Vec::new();
            self.mapping.borrow_mut().handle(ev, screen.scaled_size(), input_root.is_some(), &mut out);
            // The reference throws on its input thread when a pointer
            // event arrives before the input root is set.
            let (Some(input_root), Some(on_input)) = (input_root, self.on_input.borrow().clone()) else {
                return;
            };
            let mouse: Rc<dyn IInputDevice> = self.mouse.clone();
            let touch: Rc<dyn IInputDevice> = self.touch.clone();
            for event in out {
                let args: RawEvent = match event {
                    EvDevRawEvent::Touch { type_, timestamp, position, raw_pointer_id } => {
                        Rc::new(RawTouchEventArgs::new(
                            touch.clone(),
                            timestamp,
                            input_root.clone(),
                            type_,
                            position,
                            RawInputModifiers::empty(),
                            raw_pointer_id,
                        ))
                    }
                    EvDevRawEvent::Pointer { type_, timestamp, position, modifiers } => {
                        Rc::new(RawPointerEventArgs::new(
                            mouse.clone(),
                            timestamp,
                            input_root.clone(),
                            type_,
                            position,
                            modifiers,
                        ))
                    }
                    EvDevRawEvent::Wheel { timestamp, position, delta, modifiers } => {
                        Rc::new(RawMouseWheelEventArgs::new(
                            mouse.clone(),
                            timestamp,
                            input_root.clone(),
                            position,
                            delta,
                            modifiers,
                        ))
                    }
                    EvDevRawEvent::Key { .. } | EvDevRawEvent::Text { .. } => continue,
                };
                on_input(args);
            }
        }
    }

    /// `InputThread`.
    fn input_thread(lib: LibInput, events: Vec<String>, matrix: [f32; 6], backend: u64, dispatcher: Arc<Dispatcher>) {
        let Some(ctx) = lib.path_create_context() else {
            eprintln!("libinput: the context could not be created");
            return;
        };
        let fd = ctx.get_fd();

        for file in &events {
            ctx.path_add_device(file, &matrix);
        }

        loop {
            ctx.dispatch();
            while let Some(ev) = ctx.get_event() {
                let type_ = ev.get_type();
                let value = if is_touch_event(type_) {
                    let with_position = type_ == LIBINPUT_EVENT_TOUCH_DOWN || type_ == LIBINPUT_EVENT_TOUCH_MOTION;
                    ev.touch(with_position, POSITION_RANGE).map(|(slot, time_usec, x, y)| LibInputEvent::Touch {
                        type_,
                        slot,
                        time_usec,
                        x: x / POSITION_RANGE as f64,
                        y: y / POSITION_RANGE as f64,
                    })
                } else if is_pointer_event(type_) {
                    Some(match type_ {
                        LIBINPUT_EVENT_POINTER_MOTION_ABSOLUTE => {
                            let (x, y) = ev.pointer_absolute(POSITION_RANGE);
                            LibInputEvent::PointerMotionAbsolute {
                                time_usec: ev.pointer_time_usec(),
                                x: x / POSITION_RANGE as f64,
                                y: y / POSITION_RANGE as f64,
                            }
                        }
                        LIBINPUT_EVENT_POINTER_BUTTON => {
                            let (button, button_state) = ev.pointer_button();
                            LibInputEvent::PointerButton { time_usec: ev.pointer_time_usec(), button, button_state }
                        }
                        LIBINPUT_EVENT_POINTER_AXIS => {
                            let (source_axis, value_discrete) = ev.pointer_axis();
                            LibInputEvent::PointerAxis {
                                time_usec: ev.pointer_time_usec(),
                                source_axis,
                                value_discrete,
                            }
                        }
                        LIBINPUT_EVENT_POINTER_SCROLL_WHEEL => LibInputEvent::PointerScrollWheel {
                            time_usec: ev.pointer_time_usec(),
                            value_v120: ev.pointer_scroll_value_v120(),
                        },
                        _ => LibInputEvent::PointerOther { type_ },
                    })
                } else {
                    None
                };
                drop(ev);

                if let Some(value) = value {
                    dispatcher.post(
                        move || {
                            let state =
                                BACKENDS.with(|backends| backends.borrow().get(&backend).and_then(Weak::upgrade));
                            if let Some(state) = state {
                                state.handle(&value);
                            }
                        },
                        DispatcherPriority::SEND,
                    );
                }
                ctx.dispatch();
            }

            let mut pfd = libc::pollfd { fd, events: libc::POLLIN, revents: 0 };
            // SAFETY: one valid entry for the call.
            unsafe {
                libc::poll(&mut pfd, 1, 10);
            }
        }
    }

    impl LibInputBackend {
        pub fn new() -> Self {
            Self::create(None)
        }

        pub fn with_options(options: LibInputBackendOptions) -> Self {
            Self::create(Some(options))
        }

        fn create(options: Option<LibInputBackendOptions>) -> Self {
            Self {
                options,
                state: Rc::new(State {
                    screen: RefCell::new(None),
                    input_root: RefCell::new(None),
                    on_input: RefCell::new(None),
                    mapping: RefCell::new(LibInputState::new()),
                    mouse: MouseDevice::new(),
                    touch: TouchDevice::new(),
                }),
            }
        }
    }

    impl Default for LibInputBackend {
        fn default() -> Self {
            Self::new()
        }
    }

    impl IInputBackend for LibInputBackend {
        /// # Panics
        /// Panics when the library cannot be loaded.
        fn initialize(&self, screen: Rc<dyn IScreenInfoProvider>, on_input: Rc<dyn Fn(RawEvent)>) {
            let screen_orientation = screen
                .as_surface_orientation()
                .map_or(SurfaceOrientation::Rotation0, |surface_orientation| surface_orientation.orientation());
            *self.state.screen.borrow_mut() = Some(screen);
            *self.state.on_input.borrow_mut() = Some(on_input);
            let lib = load_library(c"libinput.so.10", LibInput::load).unwrap_or_else(|error| panic!("{error}"));
            let events = event_devices(self.options.as_ref());
            if let Some(logger) = Logger::try_get(LogEventLevel::Debug, "LinuxFramebuffer/Input/LibInput") {
                logger.log(None, &format!("{} event devices", events.len()));
            }

            let backend = NEXT_BACKEND.fetch_add(1, Ordering::Relaxed);
            BACKENDS.with(|backends| backends.borrow_mut().insert(backend, Rc::downgrade(&self.state)));
            let dispatcher = Dispatcher::ui_thread();
            let matrix = calibration_matrix(screen_orientation);
            thread::Builder::new()
                .name("Input Manager Worker".to_owned())
                .spawn(move || input_thread(lib, events, matrix, backend, dispatcher))
                .unwrap_or_else(|error| panic!("Unable to start the input thread: {error}"));
        }

        fn set_input_root(&self, root: Rc<dyn IInputRoot>) {
            *self.state.input_root.borrow_mut() = Some(root);
        }
    }
}
