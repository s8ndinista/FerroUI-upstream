//! The real application of the sample, headless (`sample_testing::Shell`): the application
//! populates itself from `App.xaml`, the main window is created and shown as the desktop
//! lifetime shows it, and its frames are rendered through the compositor with Skia.

use crate::{App, MainView, MainWindow, SAMPLE};
use ferroui_base::{AnyValue, Ref};
use ferroui_controls::{Application, Button, TextBox};
use sample_testing::Shell;

fn start() -> (Shell, Ref<MainWindow>) {
    let shell = Shell::start(&SAMPLE, 720.0, 1280.0, || App::new().upcast());
    let window = MainWindow::new();
    window.show();
    shell.settle();
    (shell, window)
}

#[test]
fn the_application_has_its_name_and_the_fluent_theme() {
    let (shell, window) = start();
    let application = Application::current().expect("the application of the test");
    assert!(application.cast::<App>().is_some(), "the application is the application of the sample");
    assert_eq!(application.name().as_deref(), Some("Single Project Sandbox"), "the name App.xaml sets");
    assert_eq!(application.styles().count(), 1, "App.xaml declares one style: the Fluent theme");
    window.close();
    drop(shell);
}

#[test]
fn the_main_window_shows_the_login_view() {
    let (shell, window) = start();
    assert!(!window.can_resize(), "MainWindow.xaml: CanResize=False");
    assert_eq!((window.width(), window.height()), (720.0, 1280.0));
    assert_eq!(window.title().as_deref(), Some("Single Project Sandbox"));

    let view = window.get_visual_descendants().find_map(|visual| visual.cast::<MainView>()).expect("the main view");
    let text_boxes: Vec<Ref<TextBox>> = view.get_visual_descendants().filter_map(|visual| visual.cast::<TextBox>()).collect();
    let placeholders: Vec<String> = text_boxes.iter().filter_map(|text_box| text_box.placeholder_text()).collect();
    assert_eq!(placeholders, ["Text", "Username", "Password", "Pin"]);
    assert!(text_boxes[0].accepts_return(), "the first text box accepts returns");
    assert_eq!(text_boxes[2].password_char(), '*');

    let button = view.get_visual_descendants().find_map(|visual| visual.cast::<Button>()).expect("the login button");
    let content = button.content().expect("the content of the button");
    let content: &dyn AnyValue = &*content;
    assert_eq!(content.downcast_ref::<String>().map(String::as_str), Some("Login"));

    assert!(shell.frames(0) > 0, "the compositor drew the window");
    assert!(shell.last_frame(0).colors((0.0, 0.0, 720.0, 1280.0)) > 2, "the view draws its controls");
    window.close();
    drop(shell);
}
