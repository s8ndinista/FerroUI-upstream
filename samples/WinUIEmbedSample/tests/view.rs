//! The embedded view in a window of the headless shell (`sample_testing::Shell`), in place of
//! the swap chain panel: it lays out, renders, and its handlers change what it shows.

use crate::embedded_view::{fixed_0, key_modifiers_text};
use crate::{EmbeddedView, FerroApp, SAMPLE};
use ferroui_base::input::KeyModifiers;
use ferroui_base::{AnyValue, ObjectType, Point, Ref, Visual};
use ferroui_controls::{Application, Button, Control, TextBlock, Window};
use sample_testing::Shell;

/// The size of the window of the test: the right column of the window of the sample. The
/// view is taller: a test scrolls what it uses into view first.
const SIZE: (f64, f64) = (640.0, 900.0);

fn start() -> (Shell, Ref<Window>, Ref<EmbeddedView>) {
    let shell = Shell::start(&SAMPLE, SIZE.0, SIZE.1, || FerroApp::new().upcast());
    let view = EmbeddedView::new();
    let window = Window::new();
    window.set_content(Some(Control::boxed(view.clone())));
    window.show();
    shell.settle();
    (shell, window, view)
}

fn descendants<T: ObjectType>(root: &Visual) -> Vec<Ref<T>> {
    root.get_visual_descendants().filter_map(|visual| visual.cast::<T>()).collect()
}

fn named_text(view: &EmbeddedView, name: &str) -> String {
    view.get_control::<TextBlock>(name).text().unwrap_or_default()
}

/// The button whose content is the text.
fn button(root: &Visual, text: &str) -> Ref<Button> {
    descendants::<Button>(root)
        .into_iter()
        .find(|button| {
            button.content().is_some_and(|content| {
                let content: &dyn AnyValue = &*content;
                content.downcast_ref::<String>().is_some_and(|content| content == text)
            })
        })
        .unwrap_or_else(|| panic!("the button {text:?}"))
}

fn center(window: &Window, visual: &Visual) -> Point {
    let bounds = Shell::bounds_of(window, visual);
    Point::new(bounds.x + bounds.width / 2.0, bounds.y + bounds.height / 2.0)
}

#[test]
fn the_application_has_the_fluent_theme() {
    let (shell, window, _view) = start();
    let application = Application::current().expect("the application of the test");
    assert!(application.cast::<FerroApp>().is_some(), "the application is the application of the sample");
    assert_eq!(application.styles().count(), 1, "FerroApp.xaml declares one style: the Fluent theme");
    window.close();
    drop(shell);
}

#[test]
fn the_view_shows_its_document_and_renders() {
    let (shell, window, view) = start();
    assert_eq!(named_text(&view, "AvClickCount"), "Clicked 0 times");
    assert_eq!(named_text(&view, "AvSliderValue"), "Slider: 25");
    assert_eq!(named_text(&view, "HoverState"), "out");
    assert_eq!(named_text(&view, "InkStats"), "0 points / 0 intermediate");
    let shown: Vec<String> = descendants::<TextBlock>(&window).into_iter().filter_map(|text_block| text_block.text()).collect();
    assert!(shown.iter().any(|text| text == "FerroUI"), "the heading of the view: {shown:?}");
    assert!(shell.frames(0) > 0, "the compositor drew the window");
    let av_button = button(&window, "FerroUI button");
    assert!(
        shell.last_frame(0).colors(Shell::frame_rect_of(&window, &av_button)) > 1,
        "the button of the view is drawn"
    );
    window.close();
    drop(shell);
}

#[test]
fn the_button_counts_its_clicks() {
    let (shell, window, view) = start();
    let av_button = button(&window, "FerroUI button");
    shell.click(0, center(&window, &av_button));
    shell.click(0, center(&window, &av_button));
    assert_eq!(named_text(&view, "AvClickCount"), "Clicked 2 times");
    window.close();
    drop(shell);
}

#[test]
fn the_slider_shows_its_value() {
    let (shell, window, view) = start();
    view.av_slider().set_range_value(60.4);
    assert_eq!(named_text(&view, "AvSliderValue"), "Slider: 60");
    view.av_slider().set_range_value(70.5);
    assert_eq!(named_text(&view, "AvSliderValue"), "Slider: 71");
    window.close();
    drop(shell);
}

#[test]
fn ink_draws_the_points_of_the_pointer() {
    let (shell, window, view) = start();
    let canvas = view.get_control::<ferroui_controls::Canvas>("InkCanvas");
    canvas.bring_into_view();
    shell.settle();
    let bounds = Shell::bounds_of(&window, &canvas);
    let start = Point::new(bounds.x + 20.0, bounds.y + 20.0);
    shell.pointer_move(0, start);
    shell.pointer_down(0, start);
    shell.pointer_move(0, Point::new(start.x + 30.0, start.y + 10.0));
    shell.pointer_up(0, Point::new(start.x + 30.0, start.y + 10.0));
    let stats = named_text(&view, "InkStats");
    // The handler of the moves counts every move over the canvas, pressed or not, as the
    // reference does: the move to the start, the press, the move and the release.
    assert!(stats.starts_with("4 points"), "a move, a press, a move and a release: {stats}");
    assert_eq!(canvas.children().count(), 4 + stats_intermediate(&stats), "a dot per point");

    let clear = button(&window, "Clear");
    clear.bring_into_view();
    shell.settle();
    shell.click(0, center(&window, &clear));
    assert_eq!(named_text(&view, "InkStats"), "0 points / 0 intermediate");
    assert_eq!(canvas.children().count(), 0);
    window.close();
    drop(shell);
}

fn stats_intermediate(stats: &str) -> usize {
    stats.split(" / ").nth(1).and_then(|part| part.split(' ').next()).and_then(|count| count.parse().ok()).unwrap_or(0)
}

#[test]
fn hover_shows_the_state_of_the_pointer() {
    let (shell, window, view) = start();
    let hover_panel = view.get_control::<ferroui_controls::Border>("HoverPanel");
    hover_panel.bring_into_view();
    shell.settle();
    shell.pointer_move(0, center(&window, &hover_panel));
    assert_eq!(named_text(&view, "HoverState"), "over");
    shell.pointer_leave(0);
    assert_eq!(named_text(&view, "HoverState"), "out");
    window.close();
    drop(shell);
}

#[test]
fn the_texts_of_the_values_are_those_of_the_reference() {
    assert_eq!(key_modifiers_text(KeyModifiers::NONE), "None");
    assert_eq!(key_modifiers_text(KeyModifiers::CONTROL | KeyModifiers::SHIFT), "Control, Shift");
    assert_eq!(key_modifiers_text(KeyModifiers::ALT | KeyModifiers::META), "Alt, Meta");
    assert_eq!(fixed_0(25.0), "25");
    assert_eq!(fixed_0(0.5), "1");
    assert_eq!(fixed_0(2.5), "3");
}

