//! An application on the framebuffer platform that checks itself: the
//! Skia renderer in software and the Simple theme, one view filled with
//! one colour, on a framebuffer that is a plain file or a (virtual)
//! framebuffer device, with input from virtual devices it creates.
//!
//! ```text
//! cargo run -p ferroui-linuxframebuffer --features example --example linuxfb_smoke -- --file /tmp/fb.raw
//! sudo .../linuxfb_smoke --fbdev /dev/fb1 --expect-id vkms --uinput
//! ```
//!
//! - `--file <path>`: the framebuffer is a file of 640 by 400 pixels
//!   (`--size=WxH`, `--format=bgra|rgba|rgb565`); works on every Linux
//!   machine and needs no privileges.
//! - `--fbdev <node> --expect-id <prefix>`: a framebuffer device of the
//!   kernel. The example writes to it only when the identification the
//!   device reports begins with the prefix, so that a virtual device
//!   (`Virtual FB` of `vfb`, `vkms` of the emulation of `vkms`) cannot be
//!   mistaken for the console of the machine.
//! - `--direct`, `--async`: draw directly into the mapped memory; copy
//!   frames on the blit thread.
//! - `--uinput`: create a touch screen, a keyboard, a mouse and a
//!   multi-touch screen through `/dev/uinput` (needs access to it), read
//!   them with the evdev backend and check that the events written to
//!   them arrive as raw input.
//! - `--dump <path>`: write the content of the framebuffer, as raw bytes,
//!   when the checks are done.
//!
//! Every check prints `[ ok ]` or `[FAILED]`; the last line is
//! `SMOKE PASSED` or `SMOKE FAILED` and the exit code follows it.

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("linuxfb_smoke: this example needs Linux");
}

#[cfg(target_os = "linux")]
fn main() -> std::process::ExitCode {
    app::run()
}

#[cfg(target_os = "linux")]
mod app {
    use ferroui_base::input::raw::{
        RawKeyEventArgs, RawKeyEventType, RawMouseWheelEventArgs, RawPointerEventArgs, RawTextInputEventArgs,
        RawTouchEventArgs,
    };
    use ferroui_base::input::IInputRoot;
    use ferroui_base::media::immutable::ImmutableSolidColorBrush;
    use ferroui_base::media::Color;
    use ferroui_base::platform::PixelFormat;
    use ferroui_base::reactive::IDisposable;
    use ferroui_base::styling::Styles;
    use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
    use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Point, Ref};
    use ferroui_controls::application_lifetimes::IApplicationLifetime;
    use ferroui_controls::{
        AppBuilder, Application, ApplicationImpl, ApplicationImplExt, Border, Control, NewApplication,
    };
    use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
    use ferroui_linuxframebuffer::input::ev_dev::{
        EvDevBackend, EvDevDeviceDescription, EvDevKeyboardDeviceDescription, EvDevMouseDeviceDescription,
        EvDevMultiTouchScreenDeviceDescription, EvDevTouchScreenDeviceDescription,
    };
    use ferroui_linuxframebuffer::input::null_input::NullInputBackend;
    use ferroui_linuxframebuffer::input::{IInputBackend, IScreenInfoProvider};
    use ferroui_linuxframebuffer::native_unsafe_methods::{
        abs_axis, axis_event_code, ev_key, ev_type, fb_bitfield, fb_fix_screeninfo, fb_var_screeninfo, ioc, syn_code,
        IOC_READ, IOC_WRITE,
    };
    use ferroui_linuxframebuffer::output::{
        FbDevOutputOptions, FbdevOutput, FileFbDevice, IFbDevice, IOutputBackend, LinuxFbDevice,
    };
    use ferroui_linuxframebuffer::raw_event_grouping::RawEvent;
    use ferroui_linuxframebuffer::LinuxFramebufferPlatformExtensions;
    use ferroui_skia::SkiaApplicationExtensions;
    use ferroui_themes_simple::SimpleTheme;
    use std::cell::{Cell, RefCell};
    use std::fs::File;
    use std::io::{Read, Write};
    use std::os::fd::AsRawFd;
    use std::process::ExitCode;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    /// The colour the view is filled with.
    const FILL: (u8, u8, u8) = (0x33, 0x66, 0x99);
    const RUN_TIMEOUT: Duration = Duration::from_secs(60);

    /// Whether every check passed.
    static SMOKE_PASSED: AtomicBool = AtomicBool::new(false);

    /// What the application needs of the command line and of `run`.
    struct Smoke {
        node: String,
        var_info: fb_var_screeninfo,
        fixed_info: fb_fix_screeninfo,
        dump: Option<String>,
        devices: Option<VirtualDevices>,
        log: Rc<RefCell<Vec<Logged>>>,
        failed: Cell<u32>,
        passed: Cell<u32>,
    }

    thread_local! {
        static SMOKE: RefCell<Option<Rc<Smoke>>> = const { RefCell::new(None) };
        /// The timer of the checks, kept for the length of the run.
        static TIMER: RefCell<Option<Rc<dyn IDisposable>>> = const { RefCell::new(None) };
    }

    impl Smoke {
        fn check(&self, name: &str, passed: bool, detail: String) {
            println!("  [{}] {name}: {detail}", if passed { " ok " } else { "FAILED" });
            let counter = if passed { &self.passed } else { &self.failed };
            counter.set(counter.get() + 1);
        }
    }

    #[repr(C)]
    pub struct App {
        base: Application,
    }

    ferro_class!(App: Application);
    ferro_impl_classes!(App: FerroObjectImpl);

    impl NewApplication for App {
        fn new_application() -> Ref<Self> {
            instantiate(Self { base: Application::construct() })
        }
    }

    impl ApplicationImpl for App {
        fn initialize(this: &Self) {
            Self::parent_initialize(this);
            this.styles().add(SimpleTheme::new().upcast::<Styles>());
        }

        fn on_framework_initialization_completed(this: &Self) {
            let lifetime = this.application_lifetime();
            if let Some(single_view) =
                lifetime.as_ref().and_then(|lifetime| lifetime.as_single_view_application_lifetime())
            {
                let border = Border::new();
                border.set_background(Some(Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(
                    FILL.0, FILL.1, FILL.2,
                )))));
                single_view.set_main_view(Some(border.upcast::<Control>()));
            }

            if let (Some(smoke), Some(lifetime)) = (SMOKE.with(|smoke| smoke.borrow().clone()), lifetime) {
                start_checks(smoke, lifetime);
            }

            Self::parent_on_framework_initialization_completed(this);
        }
    }

    /// A raw input event as the checks compare it.
    #[derive(Clone, Debug)]
    struct Logged {
        what: String,
        position: Option<Point>,
    }

    fn logged(what: impl Into<String>, x: f64, y: f64) -> Logged {
        Logged { what: what.into(), position: Some(Point::new(x, y)) }
    }

    fn logged_without_position(what: impl Into<String>) -> Logged {
        Logged { what: what.into(), position: None }
    }

    fn same(expected: &[Logged], actual: &[Logged]) -> bool {
        expected.len() == actual.len()
            && expected.iter().zip(actual).all(|(expected, actual)| {
                expected.what == actual.what
                    && match (expected.position, actual.position) {
                        (Some(a), Some(b)) => (a.x - b.x).abs() <= 1.0 && (a.y - b.y).abs() <= 1.0,
                        (None, None) => true,
                        _ => false,
                    }
            })
    }

    fn describe(e: &RawEvent) -> Logged {
        if let Some(touch) = e.downcast_ref::<RawTouchEventArgs>() {
            let position = touch.position();
            return logged(format!("touch {:?} {}", touch.type_(), touch.raw_pointer_id()), position.x, position.y);
        }
        if let Some(wheel) = e.downcast_ref::<RawMouseWheelEventArgs>() {
            let delta = wheel.delta();
            return logged_without_position(format!("wheel {} {}", delta.x, delta.y));
        }
        if let Some(pointer) = e.downcast_ref::<RawPointerEventArgs>() {
            let position = pointer.position();
            return logged(format!("pointer {:?}", pointer.type_()), position.x, position.y);
        }
        if let Some(key) = e.downcast_ref::<RawKeyEventArgs>() {
            let direction = if key.type_() == RawKeyEventType::KeyDown { "down" } else { "up" };
            return logged_without_position(format!("key {direction} {:?}", key.key()));
        }
        if let Some(text) = e.downcast_ref::<RawTextInputEventArgs>() {
            return logged_without_position(format!("text {:?}", text.text()));
        }
        logged_without_position("other")
    }

    /// An input backend that notes every raw event of another one.
    struct RecordingBackend {
        inner: Rc<dyn IInputBackend>,
        log: Rc<RefCell<Vec<Logged>>>,
    }

    impl IInputBackend for RecordingBackend {
        fn initialize(&self, info: Rc<dyn IScreenInfoProvider>, on_input: Rc<dyn Fn(RawEvent)>) {
            let log = self.log.clone();
            self.inner.initialize(
                info,
                Rc::new(move |e: RawEvent| {
                    log.borrow_mut().push(describe(&e));
                    on_input(e);
                }),
            );
        }

        fn set_input_root(&self, root: Rc<dyn IInputRoot>) {
            self.inner.set_input_root(root);
        }
    }

    /// A virtual input device, created through `/dev/uinput`.
    struct UinputDevice {
        file: File,
        event_node: String,
    }

    const UI_DEV_CREATE: u32 = ioc(0, b'U', 1, 0);
    const UI_DEV_DESTROY: u32 = ioc(0, b'U', 2, 0);
    const UI_DEV_SETUP: u32 = ioc(IOC_WRITE, b'U', 3, std::mem::size_of::<libc::uinput_setup>());
    const UI_ABS_SETUP: u32 = ioc(IOC_WRITE, b'U', 4, std::mem::size_of::<libc::uinput_abs_setup>());
    const UI_SET_EVBIT: u32 = ioc(IOC_WRITE, b'U', 100, 4);
    const UI_SET_KEYBIT: u32 = ioc(IOC_WRITE, b'U', 101, 4);
    const UI_SET_RELBIT: u32 = ioc(IOC_WRITE, b'U', 102, 4);
    const UI_SET_ABSBIT: u32 = ioc(IOC_WRITE, b'U', 103, 4);

    impl UinputDevice {
        fn set_bit(file: &File, request: u32, bit: u16) -> std::io::Result<()> {
            // SAFETY: the request takes an integer argument.
            let rv = unsafe { libc::ioctl(file.as_raw_fd(), request as _, bit as libc::c_int) };
            if rv < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        }

        fn create(name: &str, keys: &[u16], rel: &[u16], abs: &[(u16, i32, i32)]) -> std::io::Result<Self> {
            let file = std::fs::OpenOptions::new().write(true).open("/dev/uinput")?;
            let fd = file.as_raw_fd();
            Self::set_bit(&file, UI_SET_EVBIT, ev_type::EV_SYN)?;
            if !keys.is_empty() {
                Self::set_bit(&file, UI_SET_EVBIT, ev_type::EV_KEY)?;
            }
            for &key in keys {
                Self::set_bit(&file, UI_SET_KEYBIT, key)?;
            }
            if !rel.is_empty() {
                Self::set_bit(&file, UI_SET_EVBIT, ev_type::EV_REL)?;
            }
            for &axis in rel {
                Self::set_bit(&file, UI_SET_RELBIT, axis)?;
            }
            if !abs.is_empty() {
                Self::set_bit(&file, UI_SET_EVBIT, ev_type::EV_ABS)?;
            }
            for &(axis, minimum, maximum) in abs {
                Self::set_bit(&file, UI_SET_ABSBIT, axis)?;
                // SAFETY: an all-zero structure is valid; it is local and
                // the request reads one of its size.
                unsafe {
                    let mut setup: libc::uinput_abs_setup = std::mem::zeroed();
                    setup.code = axis;
                    setup.absinfo.minimum = minimum;
                    setup.absinfo.maximum = maximum;
                    if libc::ioctl(fd, UI_ABS_SETUP as _, &setup) < 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                }
            }

            // SAFETY: as above, for the description of the device; then
            // the request that creates it, which has no argument.
            unsafe {
                let mut setup: libc::uinput_setup = std::mem::zeroed();
                setup.id.bustype = 0x06; // BUS_VIRTUAL
                setup.id.vendor = 0x1209;
                setup.id.product = 0x0001;
                for (target, source) in setup.name.iter_mut().zip(name.bytes().take(79)) {
                    *target = source as _;
                }
                if libc::ioctl(fd, UI_DEV_SETUP as _, &setup) < 0 || libc::ioctl(fd, UI_DEV_CREATE as _) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
            }

            // The name of the device in sysfs, and from it the event node.
            let mut sysname = [0u8; 64];
            // SAFETY: the length of the buffer is part of the request.
            let rv = unsafe { libc::ioctl(fd, ioc(IOC_READ, b'U', 44, sysname.len()) as _, sysname.as_mut_ptr()) };
            if rv < 0 {
                return Err(std::io::Error::last_os_error());
            }
            let sysname =
                String::from_utf8_lossy(&sysname[..sysname.iter().position(|&b| b == 0).unwrap_or(0)]).into_owned();
            let started = Instant::now();
            loop {
                let event = std::fs::read_dir(format!("/sys/devices/virtual/input/{sysname}"))?
                    .filter_map(|entry| entry.ok().map(|entry| entry.file_name().to_string_lossy().into_owned()))
                    .find(|name| name.starts_with("event"));
                if let Some(event) = event {
                    let event_node = format!("/dev/input/{event}");
                    if std::path::Path::new(&event_node).exists() {
                        return Ok(Self { file, event_node });
                    }
                }
                if started.elapsed() > Duration::from_secs(5) {
                    return Err(std::io::Error::other(format!("no event node appeared for {sysname}")));
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }

        fn emit(&self, type_: u16, code: u16, value: i32) {
            // SAFETY: an all-zero event is valid.
            let mut ev: libc::input_event = unsafe { std::mem::zeroed() };
            ev.type_ = type_;
            ev.code = code;
            ev.value = value;
            // SAFETY: the bytes of a plain structure, for its size.
            let bytes = unsafe {
                std::slice::from_raw_parts((&ev as *const libc::input_event).cast::<u8>(), std::mem::size_of_val(&ev))
            };
            let _ = (&self.file).write_all(bytes);
        }

        /// One report: the events and the `SYN_REPORT` that ends it.
        fn report(&self, events: &[(u16, u16, i32)]) {
            for &(type_, code, value) in events {
                self.emit(type_, code, value);
            }
            self.emit(ev_type::EV_SYN, syn_code::SYN_REPORT, 0);
        }
    }

    impl Drop for UinputDevice {
        fn drop(&mut self) {
            // SAFETY: the request has no argument.
            unsafe {
                libc::ioctl(self.file.as_raw_fd(), UI_DEV_DESTROY as _);
            }
        }
    }

    struct VirtualDevices {
        touch: UinputDevice,
        keyboard: UinputDevice,
        mouse: UinputDevice,
        multi_touch: UinputDevice,
    }

    const KEY_A: u16 = 30;
    const KEY_LEFTSHIFT: u16 = 42;
    const KEY_ENTER: u16 = 28;

    impl VirtualDevices {
        fn create() -> std::io::Result<Self> {
            let touch = UinputDevice::create(
                "FerroUI smoke touch screen",
                &[ev_key::BTN_TOUCH],
                &[],
                &[(abs_axis::ABS_X, 0, 1000), (abs_axis::ABS_Y, 0, 1000)],
            )?;
            let keys: Vec<u16> = (1..=120).collect();
            let keyboard = UinputDevice::create("FerroUI smoke keyboard", &keys, &[], &[])?;
            let mouse = UinputDevice::create(
                "FerroUI smoke mouse",
                &[ev_key::BTN_LEFT, ev_key::BTN_RIGHT, ev_key::BTN_MIDDLE],
                &[axis_event_code::REL_X, axis_event_code::REL_Y, axis_event_code::REL_WHEEL],
                &[],
            )?;
            let multi_touch = UinputDevice::create(
                "FerroUI smoke multi-touch screen",
                &[ev_key::BTN_TOUCH],
                &[],
                &[
                    (abs_axis::ABS_MT_SLOT, 0, 9),
                    (abs_axis::ABS_MT_TRACKING_ID, 0, 65535),
                    (abs_axis::ABS_MT_POSITION_X, 0, 1000),
                    (abs_axis::ABS_MT_POSITION_Y, 0, 1000),
                ],
            )?;
            Ok(Self { touch, keyboard, mouse, multi_touch })
        }

        fn descriptions(&self) -> Vec<EvDevDeviceDescription> {
            vec![
                EvDevDeviceDescription::TouchScreen(EvDevTouchScreenDeviceDescription {
                    path: Some(self.touch.event_node.clone()),
                    ..Default::default()
                }),
                EvDevDeviceDescription::Keyboard(EvDevKeyboardDeviceDescription {
                    path: Some(self.keyboard.event_node.clone()),
                }),
                EvDevDeviceDescription::Mouse(EvDevMouseDeviceDescription {
                    path: Some(self.mouse.event_node.clone()),
                }),
                EvDevDeviceDescription::MultiTouchScreen(EvDevMultiTouchScreenDeviceDescription {
                    path: Some(self.multi_touch.event_node.clone()),
                    ..Default::default()
                }),
            ]
        }
    }

    /// The bytes of the framebuffer, read from its node.
    fn read_framebuffer(smoke: &Smoke) -> std::io::Result<Vec<u8>> {
        let length = smoke.fixed_info.line_length as u64 * smoke.var_info.yres as u64;
        let mut content = Vec::new();
        File::open(&smoke.node)?.take(length).read_to_end(&mut content)?;
        Ok(content)
    }

    /// The colour of a pixel of the framebuffer as red, green and blue.
    fn pixel(smoke: &Smoke, content: &[u8], x: u32, y: u32) -> Option<(u8, u8, u8)> {
        let bytes = smoke.var_info.bits_per_pixel as usize / 8;
        let offset = y as usize * smoke.fixed_info.line_length as usize + x as usize * bytes;
        let px = content.get(offset..offset + bytes)?;
        if bytes == 2 {
            // The 16-bit format of the renderer: five bits of red at the top.
            let value = u16::from_le_bytes([px[0], px[1]]);
            let expand = |v: u16, bits: u32| ((v as u32 * 255) / ((1 << bits) - 1)) as u8;
            return Some((expand(value >> 11, 5), expand((value >> 5) & 0x3f, 6), expand(value & 0x1f, 5)));
        }
        if smoke.var_info.blue.offset == 16 {
            Some((px[0], px[1], px[2]))
        } else {
            Some((px[2], px[1], px[0]))
        }
    }

    fn is_fill(colour: Option<(u8, u8, u8)>) -> bool {
        let close = |a: u8, b: u8| (a as i32 - b as i32).abs() <= 8;
        colour.is_some_and(|(r, g, b)| close(r, FILL.0) && close(g, FILL.1) && close(b, FILL.2))
    }

    /// The steps of the run, one per tick of a timer on the UI thread.
    fn start_checks(smoke: Rc<Smoke>, lifetime: Rc<dyn IApplicationLifetime>) {
        println!("Phase: frame");
        let started = Instant::now();
        let step = Cell::new(0u32);
        let waited = Cell::new(0u32);
        let (width, height) = (smoke.var_info.xres, smoke.var_info.yres);
        let (w, h) = (width as f64, height as f64);

        let finish = {
            let smoke = smoke.clone();
            move || {
                if let Some(dump) = &smoke.dump {
                    match read_framebuffer(&smoke).and_then(|content| std::fs::write(dump, content)) {
                        Ok(()) => println!(
                            "The framebuffer was written to {dump} ({width}x{height}, {} bits, line length {})",
                            smoke.var_info.bits_per_pixel, smoke.fixed_info.line_length
                        ),
                        Err(error) => smoke.check("dump", false, format!("{error}")),
                    }
                }
                SMOKE_PASSED.store(smoke.failed.get() == 0 && smoke.passed.get() > 0, Ordering::SeqCst);
                if let Some(controlled) = lifetime.as_controlled_application_lifetime() {
                    controlled.shutdown(0);
                }
            }
        };

        // What is written to a device in a step, and what must have
        // arrived a few ticks later.
        let expect = {
            let smoke = smoke.clone();
            move |name: &str, expected: Vec<Logged>| {
                let actual = std::mem::take(&mut *smoke.log.borrow_mut());
                let passed = same(&expected, &actual);
                let detail = if passed {
                    format!("{} raw events, as expected", actual.len())
                } else {
                    format!("expected {expected:?}, received {actual:?}")
                };
                smoke.check(name, passed, detail);
            }
        };

        let timer = DispatcherTimer::run(
            move || {
                match step.get() {
                    0 => {
                        // The frame: the middle and the corners have the colour of the view.
                        let content = read_framebuffer(&smoke).unwrap_or_default();
                        let points = [(width / 2, height / 2), (0, 0), (width - 1, height - 1)];
                        let colours: Vec<_> = points.iter().map(|&(x, y)| pixel(&smoke, &content, x, y)).collect();
                        let arrived = colours.iter().all(|&colour| is_fill(colour));
                        if !arrived && started.elapsed() < Duration::from_secs(10) {
                            return true;
                        }
                        smoke.check(
                            "frame",
                            arrived,
                            format!(
                                "the pixels at the middle and at two corners are {colours:?} after {} ms (the view is filled with {FILL:?})",
                                started.elapsed().as_millis()
                            ),
                        );
                        if smoke.devices.is_none() {
                            finish();
                            return false;
                        }
                        println!("Phase: input");
                        smoke.log.borrow_mut().clear();
                    }
                    step_number => {
                        let Some(devices) = &smoke.devices else {
                            return false;
                        };
                        // Odd steps write, even steps look (three ticks later).
                        if step_number % 2 == 0 {
                            if waited.get() < 3 {
                                waited.set(waited.get() + 1);
                                return true;
                            }
                            waited.set(0);
                        }
                        match step_number {
                            1 => {
                                let touch = &devices.touch;
                                touch.report(&[
                                    (ev_type::EV_ABS, abs_axis::ABS_X, 500),
                                    (ev_type::EV_ABS, abs_axis::ABS_Y, 250),
                                    (ev_type::EV_KEY, ev_key::BTN_TOUCH, 1),
                                ]);
                                touch.report(&[(ev_type::EV_ABS, abs_axis::ABS_X, 600)]);
                                touch.report(&[(ev_type::EV_KEY, ev_key::BTN_TOUCH, 0)]);
                            }
                            2 => expect(
                                "touch screen",
                                vec![
                                    logged("touch TouchBegin 1", w * 0.5, h * 0.25),
                                    logged("touch TouchUpdate 1", w * 0.6, h * 0.25),
                                    logged("touch TouchEnd 1", w * 0.6, h * 0.25),
                                ],
                            ),
                            3 => {
                                let keyboard = &devices.keyboard;
                                keyboard.report(&[(ev_type::EV_KEY, KEY_A, 1)]);
                                keyboard.report(&[(ev_type::EV_KEY, KEY_A, 0)]);
                                keyboard.report(&[(ev_type::EV_KEY, KEY_LEFTSHIFT, 1)]);
                                keyboard.report(&[(ev_type::EV_KEY, KEY_A, 1)]);
                                keyboard.report(&[(ev_type::EV_KEY, KEY_A, 0)]);
                                keyboard.report(&[(ev_type::EV_KEY, KEY_LEFTSHIFT, 0)]);
                                keyboard.report(&[(ev_type::EV_KEY, KEY_ENTER, 1)]);
                                keyboard.report(&[(ev_type::EV_KEY, KEY_ENTER, 0)]);
                            }
                            4 => expect(
                                "keyboard",
                                [
                                    "key down A",
                                    "text \"a\"",
                                    "key up A",
                                    "key down LeftShift",
                                    "key down A",
                                    "text \"A\"",
                                    "key up A",
                                    "key up LeftShift",
                                    "key down Return",
                                    "key up Return",
                                ]
                                .into_iter()
                                .map(logged_without_position)
                                .collect(),
                            ),
                            5 => {
                                let mouse = &devices.mouse;
                                mouse.report(&[
                                    (ev_type::EV_REL, axis_event_code::REL_X, 40),
                                    (ev_type::EV_REL, axis_event_code::REL_Y, 30),
                                ]);
                                mouse.report(&[(ev_type::EV_KEY, ev_key::BTN_LEFT, 1)]);
                                mouse.report(&[(ev_type::EV_KEY, ev_key::BTN_LEFT, 0)]);
                                mouse.report(&[(ev_type::EV_REL, axis_event_code::REL_WHEEL, -1)]);
                            }
                            6 => expect(
                                "mouse",
                                vec![
                                    logged("pointer Move", 40.0, 30.0),
                                    logged("pointer LeftButtonDown", 40.0, 30.0),
                                    logged("pointer LeftButtonUp", 40.0, 30.0),
                                    logged_without_position("wheel 0 -1"),
                                ],
                            ),
                            7 => {
                                let touch = &devices.multi_touch;
                                touch.report(&[
                                    (ev_type::EV_ABS, abs_axis::ABS_MT_SLOT, 0),
                                    (ev_type::EV_ABS, abs_axis::ABS_MT_TRACKING_ID, 10),
                                    (ev_type::EV_ABS, abs_axis::ABS_MT_POSITION_X, 100),
                                    (ev_type::EV_ABS, abs_axis::ABS_MT_POSITION_Y, 200),
                                    (ev_type::EV_ABS, abs_axis::ABS_MT_SLOT, 1),
                                    (ev_type::EV_ABS, abs_axis::ABS_MT_TRACKING_ID, 11),
                                    (ev_type::EV_ABS, abs_axis::ABS_MT_POSITION_X, 800),
                                    (ev_type::EV_ABS, abs_axis::ABS_MT_POSITION_Y, 700),
                                ]);
                                touch.report(&[(ev_type::EV_ABS, abs_axis::ABS_MT_POSITION_X, 900)]);
                                touch.report(&[(ev_type::EV_ABS, abs_axis::ABS_MT_TRACKING_ID, -1)]);
                                touch.report(&[
                                    (ev_type::EV_ABS, abs_axis::ABS_MT_SLOT, 0),
                                    (ev_type::EV_ABS, abs_axis::ABS_MT_TRACKING_ID, -1),
                                ]);
                            }
                            8 => expect(
                                "multi-touch screen",
                                vec![
                                    logged("touch TouchBegin 0", w * 0.1, h * 0.2),
                                    logged("touch TouchBegin 1", w * 0.8, h * 0.7),
                                    logged("touch TouchUpdate 1", w * 0.9, h * 0.7),
                                    logged("touch TouchEnd 1", w * 0.9, h * 0.7),
                                    logged("touch TouchEnd 0", w * 0.1, h * 0.2),
                                ],
                            ),
                            _ => {
                                finish();
                                return false;
                            }
                        }
                    }
                }
                step.set(step.get() + 1);
                true
            },
            Duration::from_millis(50),
            DispatcherPriority::BACKGROUND,
        );
        TIMER.with(|slot| *slot.borrow_mut() = Some(timer));
    }

    fn argument(name: &str) -> Option<String> {
        let args: Vec<String> = std::env::args().collect();
        let prefix = format!("{name}=");
        args.iter()
            .position(|arg| arg == name)
            .and_then(|index| args.get(index + 1).cloned())
            .or_else(|| args.iter().find_map(|arg| arg.strip_prefix(&prefix).map(str::to_owned)))
    }

    fn flag(name: &str) -> bool {
        std::env::args().any(|arg| arg == name)
    }

    fn file_var_info() -> fb_var_screeninfo {
        let size = argument("--size").unwrap_or_else(|| "640x400".to_owned());
        let (width, height) =
            size.split_once('x').and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?))).unwrap_or((640, 400));
        let eight = |offset| fb_bitfield { offset, length: 8, msb_right: 0 };
        let mut var_info = fb_var_screeninfo { xres: width, yres: height, bits_per_pixel: 32, ..Default::default() };
        match argument("--format").as_deref() {
            Some("rgba") => {
                (var_info.red, var_info.green, var_info.blue, var_info.transp) =
                    (eight(0), eight(8), eight(16), eight(24))
            }
            Some("rgb565") => var_info.bits_per_pixel = 16,
            _ => {
                (var_info.red, var_info.green, var_info.blue, var_info.transp) =
                    (eight(16), eight(8), eight(0), eight(24))
            }
        }
        var_info
    }

    pub fn run() -> ExitCode {
        let fail = |message: String| {
            println!("  [FAILED] setup: {message}");
            println!("SMOKE FAILED");
            ExitCode::FAILURE
        };

        println!("Phase: output");
        let (node, device): (String, Box<dyn IFbDevice>) = if let Some(path) = argument("--file") {
            match FileFbDevice::create(&path, file_var_info()) {
                Ok(device) => (path, Box::new(device)),
                Err(error) => return fail(format!("the file {path} cannot be created: {error}")),
            }
        } else if let Some(node) = argument("--fbdev") {
            let Some(expected) = argument("--expect-id") else {
                return fail("--fbdev needs --expect-id, the beginning of the identification of the device".to_owned());
            };
            let device = match LinuxFbDevice::open(&node) {
                Ok(device) => device,
                Err(error) => return fail(format!("{node}: {error}")),
            };
            // Nothing is written to a device that is not the expected one.
            let id = device.get_fix_screen_info().map(|info| info.id_string()).unwrap_or_default();
            if !id.starts_with(&expected) {
                return fail(format!("{node} is \"{id}\", not a device whose identification begins with \"{expected}\"; nothing was written to it"));
            }
            (node, Box::new(device))
        } else {
            return fail("one of --file <path> and --fbdev <node> --expect-id <prefix> is needed".to_owned());
        };

        let options = FbDevOutputOptions {
            render_directly_to_mapped_memory: flag("--direct"),
            use_async_front_buffer_blit: flag("--async").then_some(true),
            ..Default::default()
        };
        let (var_info, fixed_info) = match (device.get_var_screen_info(), device.get_fix_screen_info()) {
            (Ok(var_info), Ok(fixed_info)) => (var_info, fixed_info),
            (Err(error), _) | (_, Err(error)) => return fail(format!("{node}: {error}")),
        };
        let output = match FbdevOutput::with_device(device, options) {
            Ok(output) => output,
            Err(error) => return fail(format!("{node}: {error}")),
        };

        let log = Rc::new(RefCell::new(Vec::new()));
        let devices = if flag("--uinput") {
            match VirtualDevices::create() {
                Ok(devices) => Some(devices),
                Err(error) => {
                    return fail(format!("the virtual input devices cannot be created through /dev/uinput: {error}"))
                }
            }
        } else {
            None
        };
        let input_backend: Rc<dyn IInputBackend> = match &devices {
            Some(devices) => {
                for description in devices.descriptions() {
                    println!("  virtual input device {}", description.path().unwrap_or_default());
                }
                Rc::new(RecordingBackend {
                    inner: Rc::new(EvDevBackend::new(devices.descriptions())),
                    log: log.clone(),
                })
            }
            None => Rc::new(NullInputBackend::new()),
        };

        let smoke = Rc::new(Smoke {
            node: node.clone(),
            var_info,
            fixed_info,
            dump: argument("--dump"),
            devices,
            log,
            failed: Cell::new(0),
            passed: Cell::new(0),
        });
        let format = match (var_info.bits_per_pixel, var_info.blue.offset) {
            (16, _) => PixelFormat::RGB565,
            (32, 16) => PixelFormat::RGBA8888,
            _ => PixelFormat::BGRA8888,
        };
        let size = output.pixel_size();
        smoke.check(
            "output",
            size.width as u32 == var_info.xres
                && size.height as u32 == var_info.yres
                && size.width > 0
                && size.height > 0,
            format!(
                "{node} is \"{}\": {}x{}, {} bits per pixel, lines of {} bytes, drawn as {format:?}",
                output.id(),
                size.width,
                size.height,
                var_info.bits_per_pixel,
                fixed_info.line_length
            ),
        );
        SMOKE.with(|slot| *slot.borrow_mut() = Some(smoke));

        // The run must end by itself.
        std::thread::spawn(|| {
            std::thread::sleep(RUN_TIMEOUT);
            println!("  [FAILED] run: the smoke run did not finish within {} seconds", RUN_TIMEOUT.as_secs());
            println!("SMOKE FAILED");
            std::process::exit(1);
        });

        let args: Vec<String> = std::env::args().collect();
        let exit_code = AppBuilder::configure::<App>()
            .log_to_text_writer(std::io::stderr(), ferroui_base::logging::LogEventLevel::Warning, &[])
            .use_harfbuzz()
            .use_skia()
            .start_linux_direct(&args, output, Some(input_backend));
        println!("The application ended with exit code {exit_code}");

        // The virtual devices are removed with the state of the run.
        SMOKE.with(|slot| slot.borrow_mut().take());
        if exit_code == 0 && SMOKE_PASSED.load(Ordering::SeqCst) {
            println!("SMOKE PASSED");
            ExitCode::SUCCESS
        } else {
            println!("SMOKE FAILED");
            ExitCode::FAILURE
        }
    }
}
