//! The input backend over event devices (the port of `EvDevBackend.cs`).
//!
//! A thread waits for the devices and reads their events. The reference
//! handles them on that thread and the top-level posts the raw events to
//! the UI thread; raw events hold objects of the UI thread here, so the
//! thread posts what it read and the handlers run on the UI thread
//! (docs/porting/linux-framebuffer-platform.md, section 4).

use super::ev_dev_touch_screen::handle_events;
use super::{
    EvDevDevice, EvDevDeviceDescription, EvDevDeviceHandler, EvDevKeyboard, EvDevMouse, EvDevMultiTouchScreen,
    EvDevRawEvent, EvDevSingleTouchScreen,
};
use crate::input::{IInputBackend, IScreenInfoProvider};
use crate::native_unsafe_methods::input_event;
use crate::raw_event_grouping::RawEvent;
use ferroui_base::input::raw::{
    RawKeyEventArgs, RawMouseWheelEventArgs, RawPointerEventArgs, RawTextInputEventArgs, RawTouchEventArgs,
};
use ferroui_base::input::{IInputDevice, IInputRoot, KeyDeviceType, MouseDevice, TouchDevice};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::os::fd::RawFd;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;

/// The prefix of the environment variables that name devices.
const DEVICE_VARIABLE_PREFIX: &str = "FERROUI_EVDEV_DEVICE_";

struct DeviceEntry {
    /// Keeps the device open for the thread that reads it.
    _device: EvDevDevice,
    handler: Box<dyn EvDevDeviceHandler>,
    /// Whether the kernel dropped events and the next report is awaited.
    dropped: bool,
    /// The device of the pointer and touch events of the handler.
    pointer_device: Rc<dyn IInputDevice>,
}

#[derive(Default)]
struct State {
    handlers: RefCell<Vec<DeviceEntry>>,
    on_input: RefCell<Option<Rc<dyn Fn(RawEvent)>>>,
    input_root: RefCell<Option<Rc<dyn IInputRoot>>>,
    screen: RefCell<Option<Rc<dyn IScreenInfoProvider>>>,
    warned_about_the_input_root: Cell<bool>,
}

thread_local! {
    /// The backends of the UI thread, by the number their input threads
    /// post with.
    static BACKENDS: RefCell<HashMap<u64, Weak<State>>> = RefCell::new(HashMap::new());
}

static NEXT_BACKEND: AtomicU64 = AtomicU64::new(1);

pub struct EvDevBackend {
    device_descriptions: Vec<EvDevDeviceDescription>,
    state: Rc<State>,
}

impl State {
    /// `HandleEvents` of the handler of device `index`, on the UI thread.
    fn handle(&self, index: usize, events: &[input_event]) {
        let Some(screen) = self.screen.borrow().clone() else {
            return;
        };
        let mut out = Vec::new();
        let pointer_device = {
            let mut handlers = self.handlers.borrow_mut();
            let Some(entry) = handlers.get_mut(index) else {
                return;
            };
            handle_events(entry.handler.as_mut(), &mut entry.dropped, events, screen.scaled_size(), &mut out);
            entry.pointer_device.clone()
        };
        if out.is_empty() {
            return;
        }

        let Some(input_root) = self.input_root.borrow().clone() else {
            // The reference throws here, on its input thread, which
            // prints the exception and goes on.
            if !self.warned_about_the_input_root.replace(true) {
                eprintln!("InputRoot hasn't been set");
            }
            return;
        };
        let Some(on_input) = self.on_input.borrow().clone() else {
            return;
        };

        for event in out {
            let args: RawEvent = match event {
                EvDevRawEvent::Touch { type_, timestamp, position, raw_pointer_id } => Rc::new(RawTouchEventArgs::new(
                    pointer_device.clone(),
                    timestamp,
                    input_root.clone(),
                    type_,
                    position,
                    ferroui_base::input::RawInputModifiers::empty(),
                    raw_pointer_id,
                )),
                EvDevRawEvent::Pointer { type_, timestamp, position, modifiers } => Rc::new(RawPointerEventArgs::new(
                    pointer_device.clone(),
                    timestamp,
                    input_root.clone(),
                    type_,
                    position,
                    modifiers,
                )),
                EvDevRawEvent::Wheel { timestamp, position, delta, modifiers } => Rc::new(RawMouseWheelEventArgs::new(
                    pointer_device.clone(),
                    timestamp,
                    input_root.clone(),
                    position,
                    delta,
                    modifiers,
                )),
                EvDevRawEvent::Key { type_, timestamp, key, modifiers, physical_key, key_symbol } => {
                    let Some(keyboard) = keyboard_device() else {
                        continue;
                    };
                    Rc::new(RawKeyEventArgs::new(
                        keyboard,
                        timestamp,
                        input_root.clone(),
                        type_,
                        key,
                        modifiers,
                        physical_key,
                        key_symbol,
                        KeyDeviceType::Keyboard,
                    ))
                }
                EvDevRawEvent::Text { timestamp, text } => {
                    let Some(keyboard) = keyboard_device() else {
                        continue;
                    };
                    Rc::new(RawTextInputEventArgs::new(keyboard, timestamp, input_root.clone(), text))
                }
            };
            on_input(args);
        }
    }
}

/// The keyboard device of the platform, as an input device.
fn keyboard_device() -> Option<Rc<dyn IInputDevice>> {
    crate::LinuxFramebufferPlatform::keyboard_device().map(|keyboard| keyboard as Rc<dyn IInputDevice>)
}

/// `InputThread`: waits for the devices and posts what it reads.
fn input_thread(epoll: RawFd, fds: Vec<RawFd>, backend: u64, dispatcher: Arc<Dispatcher>) {
    const MAX_EVENTS: usize = 16;
    let mut events = [libc::epoll_event { events: 0, u64: 0 }; MAX_EVENTS];
    loop {
        // SAFETY: `events` has room for `MAX_EVENTS` events.
        let event_count = unsafe { libc::epoll_wait(epoll, events.as_mut_ptr(), MAX_EVENTS as i32, 1000) };
        let event_count = event_count.clamp(0, MAX_EVENTS as i32) as usize;
        for ev in &events[..event_count] {
            let index = ev.u64 as usize;
            let Some(&fd) = fds.get(index) else {
                continue;
            };
            let read = EvDevDevice::read_events(fd);
            if read.is_empty() {
                continue;
            }
            dispatcher.post(
                move || {
                    let state = BACKENDS.with(|backends| backends.borrow().get(&backend).and_then(Weak::upgrade));
                    if let Some(state) = state {
                        state.handle(index, &read);
                    }
                },
                DispatcherPriority::SEND,
            );
        }
    }
}

impl EvDevBackend {
    pub fn new(devices: Vec<EvDevDeviceDescription>) -> Self {
        Self { device_descriptions: devices, state: Rc::new(State::default()) }
    }

    /// The backend of the devices the environment names: one variable
    /// `FERROUI_EVDEV_DEVICE_{name}` per device, with the format
    /// `(path):type=(touchscreen|multitouch|keyboard|mouse):[calibration=..]`.
    pub fn create_from_environment() -> Result<Self, String> {
        let mut variables: Vec<(String, String)> =
            std::env::vars().filter(|(key, _)| key.starts_with(DEVICE_VARIABLE_PREFIX)).collect();
        variables.sort();

        let mut device_descriptions = Vec::new();
        for (_, value) in variables {
            device_descriptions.push(EvDevDeviceDescription::parse_from_env(&value)?);
        }

        if device_descriptions.is_empty() {
            return Err(format!(
                "No device device description found, specify devices by adding {DEVICE_VARIABLE_PREFIX}{{name}} environment variables"
            ));
        }

        Ok(Self::new(device_descriptions))
    }
}

impl IInputBackend for EvDevBackend {
    /// # Panics
    /// Panics when a device cannot be opened or is not of the kind its
    /// description names.
    fn initialize(&self, info: Rc<dyn IScreenInfoProvider>, on_input: Rc<dyn Fn(RawEvent)>) {
        *self.state.on_input.borrow_mut() = Some(on_input);
        *self.state.screen.borrow_mut() = Some(info);
        // SAFETY: a plain system call.
        let epoll = unsafe { libc::epoll_create1(0) };
        let mut fds = Vec::new();

        for (c, description) in self.device_descriptions.iter().enumerate() {
            let dev =
                EvDevDevice::open(description.path().unwrap_or_default()).unwrap_or_else(|error| panic!("{error}"));
            let failed = |error: String| -> ! { panic!("{error}: {}", description.path().unwrap_or_default()) };
            let (handler, pointer_device): (Box<dyn EvDevDeviceHandler>, Rc<dyn IInputDevice>) = match description {
                EvDevDeviceDescription::TouchScreen(touch) => (
                    Box::new(EvDevSingleTouchScreen::new(dev.info(), touch).unwrap_or_else(|error| failed(error))),
                    TouchDevice::new(),
                ),
                EvDevDeviceDescription::MultiTouchScreen(touch) => (
                    Box::new(EvDevMultiTouchScreen::new(dev.info(), touch).unwrap_or_else(|error| failed(error))),
                    TouchDevice::new(),
                ),
                EvDevDeviceDescription::Keyboard(_) => (Box::new(EvDevKeyboard::new()), MouseDevice::new()),
                EvDevDeviceDescription::Mouse(_) => (Box::new(EvDevMouse::new(dev.info())), MouseDevice::new()),
            };

            let fd = dev.fd();
            fds.push(fd);
            self.state.handlers.borrow_mut().push(DeviceEntry {
                _device: dev,
                handler,
                dropped: false,
                pointer_device,
            });

            let mut ev = libc::epoll_event { events: libc::EPOLLIN as u32, u64: c as u64 };
            // SAFETY: `ev` is valid for the call; the descriptors are open.
            unsafe {
                libc::epoll_ctl(epoll, libc::EPOLL_CTL_ADD, fd, &mut ev);
            }
        }

        let backend = NEXT_BACKEND.fetch_add(1, Ordering::Relaxed);
        BACKENDS.with(|backends| backends.borrow_mut().insert(backend, Rc::downgrade(&self.state)));
        let dispatcher = Dispatcher::ui_thread();
        thread::Builder::new()
            .name("evdev input".to_owned())
            .spawn(move || input_thread(epoll, fds, backend, dispatcher))
            .unwrap_or_else(|error| panic!("Unable to start the input thread: {error}"));
    }

    fn set_input_root(&self, root: Rc<dyn IInputRoot>) {
        *self.state.input_root.borrow_mut() = Some(root);
    }
}
