//! The tests of `ScreenTests.cs`: a change of the displays, broadcast to the
//! windows of the application, refreshes the screens and the rate of the
//! render timer whether the application has a window or not (a tray-only
//! application has only the message window of the platform).

use crate::TestCase;

pub fn tests(cases: &mut Vec<TestCase>) {
    #[cfg(windows)]
    {
        for create_window in [false, true] {
            cases.push(TestCase::new(
                format!("screen_tests::display_changes_refresh_screens_with_or_without_a_window({create_window})"),
                move || windows::display_changes_refresh_screens_with_or_without_a_window(create_window),
            ));
        }
        cases.push(TestCase::new(
            "screen_tests::display_changes_update_render_timer_without_a_window".to_string(),
            windows::display_changes_update_render_timer_without_a_window,
        ));
    }
    #[cfg(not(windows))]
    let _ = cases;
}

#[cfg(windows)]
mod windows {
    use ferroui_base::reactive::IDisposable;
    use ferroui_base::threading::{Dispatcher, DispatcherPriority};
    use ferroui_controls::platform::{IPlatformHandle, IScreenImpl, ITopLevelImpl, IWindowingPlatform, ScreensBaseImplExt};
    use ferroui_win32::interop::unmanaged_methods::{send_message, WindowsMessage};
    use ferroui_win32::Win32Platform;
    use std::cell::Cell;
    use std::rc::Rc;

    pub fn display_changes_refresh_screens_with_or_without_a_window(create_window: bool) {
        let platform = Win32Platform::instance();
        let screens = platform.screen();
        let window = create_window.then(|| platform.create_window());
        let changes = Rc::new(Cell::new(0));
        // `screens.Changed += OnChanged`: the event of the port is one callback, so the one
        // that was there is called as well and given back at the end.
        let previous = screens.changed();
        {
            let changes = changes.clone();
            let previous = previous.clone();
            screens.set_changed(Some(Rc::new(move || {
                changes.set(changes.get() + 1);
                if let Some(previous) = &previous {
                    previous();
                }
            })));
        }

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // Deliver the broadcast to this application's HWNDs without changing the desktop.
            send_display_change(platform.handle());
            send_display_change(platform.handle());
            if let Some(window) = &window {
                let handle = window.handle().expect("a window has a handle");
                send_display_change(handle.handle());
            }

            // `Assert.NotSame(cachedScreens, screens.AllScreens)` has no counterpart: the list of
            // the port is a new vector on every read. That the cache was dropped shows in the
            // count read again from the system and in the one change notification below.
            assert_eq!(screens.screen_count() as usize, screens.all_screens().len());
            let _ = Dispatcher::ui_thread().invoke_local_with_priority(|| {}, DispatcherPriority::BACKGROUND);
            assert_eq!(1, changes.get());
        }));

        screens.set_changed(previous);
        if let Some(window) = window {
            window.dispose();
        }
        if let Err(panic) = result {
            std::panic::resume_unwind(panic);
        }
    }

    pub fn display_changes_update_render_timer_without_a_window() {
        let platform = Win32Platform::instance();
        // The reference binds a timer of its own in a scope of the services; the timer of the
        // port is the platform's, which the test sets to one frame a second.
        let timer = platform.sleep_loop_render_timer().expect("the platform renders with the timer of the sleep loop");
        timer.set_desired_fps(1);
        let expected_fps =
            platform.screen().all_platform_screens().iter().map(|screen| screen.frequency()).max().unwrap_or(0).max(60);

        send_display_change(platform.handle());

        assert_eq!(expected_fps, timer.desired_fps());
        let _ = Dispatcher::ui_thread().invoke_local_with_priority(|| {}, DispatcherPriority::BACKGROUND);
    }

    fn send_display_change(hwnd: isize) {
        send_message(hwnd, WindowsMessage::WM_DISPLAYCHANGE, 0, 0);
    }
}
