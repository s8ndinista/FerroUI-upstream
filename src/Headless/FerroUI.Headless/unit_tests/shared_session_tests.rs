//! Tests of the port (not in upstream): what a test leaves to the next one
//! in the session of a test assembly with the per-assembly isolation, where
//! the application, its compositor and its media context are shared.
//!
//! A test that closes a window ends with a composition batch in flight: the
//! window hides before its renderer is disposed, which asks for a commit,
//! and the commit is made by the jobs that run after the test. The render
//! timer of the headless platform ticks on the dispatcher of the session,
//! which does not run between two dispatches, so nothing processes that
//! batch before the next test starts. The media context then holds every
//! commit of the next test back until a tick, and a window the next test
//! shows has no composition visual attached to its target after its jobs
//! have run (`should_render_to_a_compositor_snapshot_capture` of the
//! rendering tests failed so when the leak test ran before it).

use super::oneshot;
use super::test_application::{run, run_async, PER_ASSEMBLY};
use crate::HeadlessWindowExtensions;
use ferroui_base::input::{MouseButton, RawInputModifiers};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::Brushes;
use ferroui_base::rendering::composition::ElementComposition;
use ferroui_base::threading::Dispatcher;
use ferroui_base::Point;
use ferroui_controls::{Button, ContentControl, Control, SizeToContent, Window};

/// What the leak test does: a window is shown, used and closed.
fn show_use_and_close_a_window() {
    let window = Window::new();
    let button = Button::new();
    window.set_content(Some(Control::boxed(button.clone())));
    window.show();
    button.focus();
    window.mouse_down(Point::new(1.0, 1.0), MouseButton::Left, RawInputModifiers::NONE);
    window.close();
}

/// What the snapshot test of the rendering tests does: a window is shown,
/// its jobs run, and its composition visual is attached to its target and
/// can be drawn into a snapshot.
async fn show_a_window_and_take_a_snapshot() {
    let content_control = ContentControl::new();
    content_control.set_horizontal_alignment(HorizontalAlignment::Stretch);
    content_control.set_vertical_alignment(VerticalAlignment::Stretch);
    content_control.set_width(100.0);
    content_control.set_height(100.0);
    content_control.set_background(Some(Brushes::green()));
    let window = Window::new();
    window.set_content(Some(Control::boxed(content_control)));
    window.set_size_to_content(SizeToContent::WIDTH_AND_HEIGHT);

    window.show();

    Dispatcher::ui_thread().run_jobs(None);

    let composition_visual = ElementComposition::get_element_visual(&window).unwrap();
    assert!(composition_visual.root().is_some(), "the composition visual of the window is attached to its target");
    let task = composition_visual.compositor().create_composition_visual_snapshot(&composition_visual, 1.0);
    let (completed, completion) = oneshot();
    task.on_completed(move || completed.send(()));
    completion.await;
    let snapshot = task.take_result().expect("the outcome of the snapshot").expect("the snapshot");

    assert_eq!(100.0, snapshot.size().width);
    assert_eq!(100.0, snapshot.size().height);
}

/// A test that closed a window leaves no composition batch in flight to the
/// next test of the shared session: the next test's window is composed by
/// its own jobs. Five pairs, one after the other.
#[test]
fn a_test_after_one_that_closed_a_window_composes_its_own_window() {
    for _ in 0..5 {
        run(&PER_ASSEMBLY, show_use_and_close_a_window);
        run_async(&PER_ASSEMBLY, show_a_window_and_take_a_snapshot);
    }
}
