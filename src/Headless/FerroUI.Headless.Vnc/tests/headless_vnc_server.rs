//! The VNC server of the headless platform, driven by a VNC client over a
//! loopback port (not from upstream, which has no tests of the project).
//!
//! An application with a button and a text box starts with the server; the
//! client of `support/rfb_client.rs` connects without authentication,
//! receives the framebuffer, which must be the frame the window rendered,
//! clicks the button and types into the text box, and the controls must
//! have received the input.
//!
//! The application runs on a thread of its own, the UI thread, for the
//! whole test; the test is the only one of this file, as an application is
//! set up once a process.

#[path = "support/rfb_client.rs"]
mod rfb_client;

use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref,
};
use ferroui_controls::{
    AppBuilder, Application, ApplicationImpl, Button, Control, NewApplication, ShutdownMode,
    StackPanel, TextBox, Window,
};
use ferroui_headless::HeadlessWindowExtensions;
use ferroui_headless_vnc::HeadlessVncPlatformExtensions;
use ferroui_skia::SkiaApplicationExtensions;
use ferroui_themes_simple::SimpleTheme;
use rfb_client::{RfbClient, DESKTOP_SIZE, RAW};
use std::cell::RefCell;
use std::net::TcpListener;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

#[repr(C)]
struct TestApplication {
    base: Application,
}

ferro_class!(TestApplication: Application);
ferro_impl_classes!(TestApplication: FerroObjectImpl);

impl NewApplication for TestApplication {
    fn new_application() -> Ref<Self> {
        let this = instantiate(Self {
            base: Application::construct(),
        });
        this.styles().add(SimpleTheme::new().as_style());
        this
    }
}

impl ApplicationImpl for TestApplication {}

/// The controls of the main window, on the UI thread.
struct MainWindow {
    window: Ref<Window>,
    text_box: Ref<TextBox>,
}

ferroui_base::ferro_thread_local! {
    static MAIN_WINDOW: RefCell<Option<MainWindow>> = const { RefCell::new(None) };
}

/// The number of clicks of the button.
static CLICKS: AtomicUsize = AtomicUsize::new(0);

/// The main window: a button of 120 by 40 at the top left, a text box of
/// 200 by 32 below it.
fn create_main_window() -> MainWindow {
    let button = Button::new();
    let content: BoxedValue = Rc::new("Click".to_string());
    button.set_content(Some(content));
    button.set_width(120.0);
    button.set_height(40.0);
    button.set_horizontal_alignment(HorizontalAlignment::Left);
    button.click(|_, _| {
        CLICKS.fetch_add(1, Ordering::SeqCst);
    });

    let text_box = TextBox::new();
    text_box.set_width(200.0);
    text_box.set_height(32.0);
    text_box.set_horizontal_alignment(HorizontalAlignment::Left);

    let panel = StackPanel::new();
    panel.children().add(&button);
    panel.children().add(&text_box);

    let window = Window::new();
    window.set_width(320.0);
    window.set_height(200.0);
    window.set_content(Some(Control::boxed(panel)));
    MainWindow { window, text_box }
}

/// Runs `f` on the UI thread and returns its result.
fn on_ui_thread<R: Send + 'static>(
    dispatcher: &Arc<Dispatcher>,
    f: impl FnOnce() -> R + Send + 'static,
) -> R {
    dispatcher.invoke(f).expect("the UI thread runs")
}

/// The last frame the main window rendered: its size and its pixels, four
/// bytes each.
fn last_rendered_frame(dispatcher: &Arc<Dispatcher>) -> Option<(i32, i32, Vec<u8>)> {
    on_ui_thread(dispatcher, || {
        MAIN_WINDOW.with(|main_window| {
            let main_window = main_window.borrow();
            let frame = main_window.as_ref()?.window.get_last_rendered_frame()?;
            let size = frame.pixel_size();
            let mut pixels = vec![0u8; (size.width * size.height * 4) as usize];
            frame.copy_pixels(
                ferroui_base::PixelRect::from_size(size),
                &mut pixels,
                size.width * 4,
            );
            frame.dispose();
            Some((size.width, size.height, pixels))
        })
    })
}

fn text_of_the_text_box(dispatcher: &Arc<Dispatcher>) -> Option<String> {
    on_ui_thread(dispatcher, || {
        MAIN_WINDOW.with(|main_window| {
            main_window
                .borrow()
                .as_ref()
                .and_then(|main_window| main_window.text_box.text())
        })
    })
}

/// Waits until `condition` holds, for at most ten seconds.
fn wait_for(what: &str, mut condition: impl FnMut() -> bool) {
    let start = Instant::now();
    while !condition() {
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "timed out waiting for {what}"
        );
        thread::sleep(Duration::from_millis(20));
    }
}

/// The directory the frames of the test are written to.
fn output_directory() -> PathBuf {
    let directory = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("headless-vnc");
    std::fs::create_dir_all(&directory).unwrap();
    directory
}

#[test]
fn a_vnc_client_sees_the_window_and_drives_its_controls() {
    // A free port of the loopback interface.
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();

    let (dispatcher_sender, dispatcher_receiver) = mpsc::channel();
    let application = thread::Builder::new()
        .name("UI thread".to_owned())
        .spawn(move || {
            AppBuilder::configure::<TestApplication>()
                .use_skia()
                .after_setup(move |builder| {
                    let main_window = create_main_window();
                    let lifetime = builder
                        .instance()
                        .and_then(|instance| instance.application_lifetime());
                    let desktop = lifetime.as_ref().and_then(|lifetime| {
                        lifetime.as_classic_desktop_style_application_lifetime()
                    });
                    desktop
                        .expect("a classic desktop lifetime")
                        .set_main_window(Some(main_window.window.clone()));
                    MAIN_WINDOW.with(|slot| *slot.borrow_mut() = Some(main_window));
                    dispatcher_sender.send(Dispatcher::ui_thread()).unwrap();
                })
                .start_with_headless_vnc_platform(
                    Some("127.0.0.1"),
                    port,
                    &[],
                    ShutdownMode::OnMainWindowClose,
                )
        })
        .unwrap();
    let dispatcher = dispatcher_receiver
        .recv_timeout(Duration::from_secs(60))
        .expect("the application starts");
    // The size a session announces is the size of the last frame: the window has rendered one.
    wait_for("the first frame of the window", || {
        last_rendered_frame(&dispatcher).is_some()
    });

    // The handshake: the server init message has the size of the window and the pixel format
    // of the server (32 bits, depth 24, little endian, true colour, blue first).
    let mut client = RfbClient::connect(&format!("127.0.0.1:{port}")).expect("the client connects");
    assert_eq!("FerroUI", client.name);
    assert_eq!((320, 200), (client.width, client.height));
    assert_eq!(
        [32, 24, 0, 1, 0, 255, 0, 255, 0, 255, 16, 8, 0],
        client.pixel_format[..13]
    );
    client.set_encodings(&[RAW, DESKTOP_SIZE]).unwrap();

    // The framebuffer is the frame the window rendered. A frame that was rendered between the
    // update and the comparison is the reason to ask again.
    let mut equal = false;
    for _ in 0..20 {
        client.fetch_frame().unwrap();
        let (width, height, pixels) =
            last_rendered_frame(&dispatcher).expect("the window rendered a frame");
        assert_eq!(
            (320, 200),
            (width, height),
            "the frame has the size of the window"
        );
        assert_eq!(
            (320, 200),
            (i32::from(client.width), i32::from(client.height))
        );
        assert_eq!(pixels.len(), client.framebuffer.len());
        // The fourth byte of a pixel is not part of the format of the server.
        equal = pixels
            .chunks_exact(4)
            .zip(client.framebuffer.chunks_exact(4))
            .all(|(a, b)| a[..3] == b[..3]);
        if equal {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    assert!(
        equal,
        "the framebuffer of the client is the last rendered frame"
    );
    assert!(
        client
            .framebuffer
            .chunks_exact(4)
            .any(|pixel| pixel[..3] != client.framebuffer[..3]),
        "the frame is not of one colour"
    );
    client
        .save_png(&output_directory().join("01-window.png"))
        .unwrap();

    // A pointer event reaches the button: a click at its centre.
    client.click(60, 20).unwrap();
    wait_for("the click of the button", || {
        CLICKS.load(Ordering::SeqCst) == 1
    });

    // Key events reach the text box: a click gives it the focus, two keys type two letters.
    client.click(100, 56).unwrap();
    client.type_key(0x61).unwrap(); // a
    client.type_key(0x62).unwrap(); // b
    wait_for("the text of the text box", || {
        text_of_the_text_box(&dispatcher).as_deref() == Some("ab")
    });
    assert_eq!(
        1,
        CLICKS.load(Ordering::SeqCst),
        "the click on the text box is not a click of the button"
    );

    // An incremental update brings the change: the text box shows the text.
    let before = client.framebuffer.clone();
    let start = Instant::now();
    while client.framebuffer == before {
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "an update shows the text"
        );
        client
            .request_update(true, 0, 0, client.width, client.height)
            .unwrap();
        client.read_update().unwrap();
    }
    client
        .save_png(&output_directory().join("02-typed.png"))
        .unwrap();

    // Closing the main window ends the application.
    drop(client);
    on_ui_thread(&dispatcher, || {
        MAIN_WINDOW.with(|main_window| {
            if let Some(main_window) = main_window.borrow_mut().take() {
                main_window.window.close();
            }
        })
    });
    let exit_code = application
        .join()
        .expect("the application ends without a panic");
    assert_eq!(0, exit_code);
}

/// Takes a picture of a running VNC server through the client of the tests
/// (`cargo test -p ferroui-headless-vnc --test headless_vnc_server -- --ignored`):
/// `FERROUI_VNC_ADDRESS` (default `127.0.0.1:5901`) is the server,
/// `FERROUI_VNC_SNAPSHOT` the PNG file to write (default in the temporary
/// directory of the tests). With `FERROUI_VNC_CLICK=x,y` the client clicks
/// there first and waits `FERROUI_VNC_WAIT_MS` milliseconds (default 1000)
/// before the picture.
#[test]
#[ignore = "needs a running VNC server"]
fn snapshot_of_a_running_server() {
    let address =
        std::env::var("FERROUI_VNC_ADDRESS").unwrap_or_else(|_| "127.0.0.1:5901".to_owned());
    let file = std::env::var_os("FERROUI_VNC_SNAPSHOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| output_directory().join("snapshot.png"));
    let mut client = RfbClient::connect(&address).expect("the client connects");
    client.set_encodings(&[RAW, DESKTOP_SIZE]).unwrap();
    client.fetch_frame().unwrap();

    if let Ok(click) = std::env::var("FERROUI_VNC_CLICK") {
        let (x, y) = click.split_once(',').expect("FERROUI_VNC_CLICK is x,y");
        client
            .click(x.trim().parse().unwrap(), y.trim().parse().unwrap())
            .unwrap();
        let wait = std::env::var("FERROUI_VNC_WAIT_MS")
            .ok()
            .and_then(|ms| ms.parse().ok())
            .unwrap_or(1000);
        thread::sleep(Duration::from_millis(wait));
        client.fetch_frame().unwrap();
    }

    if let Some(directory) = file.parent() {
        std::fs::create_dir_all(directory).unwrap();
    }
    client.save_png(&file).unwrap();
    println!(
        "{} by {} ({}): {}",
        client.width,
        client.height,
        client.name,
        file.display()
    );
}
