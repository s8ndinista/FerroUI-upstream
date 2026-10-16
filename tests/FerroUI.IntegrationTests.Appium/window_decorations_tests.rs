//! Port of `WindowDecorationsTests.cs`.

use crate::infrastructure::{
    Collection, DefaultAppFixture, Disposable, Session, TestBase, TestClass, TestPlatforms, Tests, WindowChrome,
};
use std::time::Duration;

pub struct WindowDecorationsTests {
    base: TestBase,
}

impl TestClass for WindowDecorationsTests {
    fn dispose(&self) {
        self.set_parameters(false, -1);
        self.apply_to_current_window();
    }
}

impl WindowDecorationsTests {
    pub fn new(fixture: &DefaultAppFixture) -> Self {
        WindowDecorationsTests { base: TestBase::new(fixture, "Window Decorations") }
    }

    fn session(&self) -> &Session {
        self.base.session()
    }

    pub fn window_size_should_be_consistent_between_toggles(&self) {
        let window = self.session().find_element_by_accessibility_id("MainWindow");
        let original = window.size();

        // Step 1: keep extend client area to false, but adjust some value that should not have any effect.
        self.set_parameters(false, 10);
        self.apply_to_current_window();
        assert_eq!(original, window.size());

        // Step 2: enable and disable extended client area.
        self.set_parameters(true, 20);
        self.apply_to_current_window();
        self.set_parameters(false, 20);
        self.apply_to_current_window();
        assert_eq!(original, window.size());
    }

    pub fn can_restore_to_non_extended_state(&self) {
        self.set_parameters(true, 20);
        self.apply_to_current_window();

        self.set_parameters(false, 1000);
        self.apply_to_current_window();

        let current_window = self.session().get_current_single_window();
        let system_chrome = current_window.get_system_chrome_buttons();
        let client_chrome = current_window.get_client_chrome_buttons();

        self.assert_system_chrome(&system_chrome, &client_chrome, false);

        let props = self.session().find_element_by_accessibility_id("WindowDecorationProperties");
        assert_eq!("0 0 False", props.text());
    }

    // We don't have client chrome on macOS
    pub fn should_apply_client_chrome(&self, title_bar_height: i32) {
        self.set_parameters(true, title_bar_height);

        self.apply_to_current_window();

        std::thread::sleep(Duration::from_millis(500));

        let current_window = self.session().get_current_single_window();
        let system_chrome = current_window.get_system_chrome_buttons();
        let client_chrome = current_window.get_client_chrome_buttons();

        self.assert_client_chrome(&system_chrome, &client_chrome, title_bar_height);

        let props = self.session().find_element_by_accessibility_id("WindowDecorationProperties");
        if title_bar_height > 0 {
            assert_eq!(format!("0 {title_bar_height} True"), props.text());
        }
    }

    pub fn should_apply_system_chrome(&self, title_bar_height: i32) {
        self.set_parameters(true, title_bar_height);

        self.apply_to_current_window();

        std::thread::sleep(Duration::from_millis(500));

        let current_window = self.session().get_current_single_window();
        let system_chrome = current_window.get_system_chrome_buttons();
        let client_chrome = current_window.get_client_chrome_buttons();

        self.assert_system_chrome(&system_chrome, &client_chrome, true);

        let props = self.session().find_element_by_accessibility_id("WindowDecorationProperties");
        if title_bar_height > 0 {
            assert_eq!(format!("0 {title_bar_height} True"), props.text());
        }
    }

    // We don't have client chrome on macOS
    pub fn should_apply_client_chrome_on_new_window(&self, title_bar_height: i32) {
        self.set_parameters(true, title_bar_height);

        let _window = self.apply_on_new_window();
        std::thread::sleep(Duration::from_millis(500));

        let secondary_window = self.session().get_window_by_id("SecondaryWindow");
        let system_chrome = secondary_window.get_system_chrome_buttons();
        let client_chrome = secondary_window.get_client_chrome_buttons();

        self.assert_client_chrome(&system_chrome, &client_chrome, title_bar_height);
    }

    pub fn should_apply_system_chrome_on_new_window(&self, title_bar_height: i32) {
        self.set_parameters(true, title_bar_height);

        let _window = self.apply_on_new_window();
        std::thread::sleep(Duration::from_millis(500));

        let secondary_window = self.session().get_window_by_id("SecondaryWindow");
        let system_chrome = secondary_window.get_system_chrome_buttons();
        let client_chrome = secondary_window.get_client_chrome_buttons();

        self.assert_system_chrome(&system_chrome, &client_chrome, true);
    }

    fn assert_client_chrome(&self, system_chrome: &WindowChrome, client_chrome: &WindowChrome, title_bar_height: i32) {
        // Ignore windows, it always reports full sized and enabled buttons and title bar. Just drawn behind.
        if !cfg!(windows) {
            // All system chrome buttons are hidden.
            assert!(!system_chrome.is_any_button_enabled());
            assert_eq!(-1, system_chrome.max_button_height());
            assert!(matches!(system_chrome.title_bar_height(), -1 | 0));
        }

        // At least some client chrome buttons are shown.
        assert!(client_chrome.is_any_button_enabled());
        assert!(client_chrome.max_button_height() > 0);
        if title_bar_height != -1 {
            assert_eq!(title_bar_height, client_chrome.title_bar_height());
        }
    }

    fn assert_system_chrome(&self, system_chrome: &WindowChrome, client_chrome: &WindowChrome, is_extended: bool) {
        // At least some server chrome buttons are shown.
        assert!(system_chrome.is_any_button_enabled());
        assert!(system_chrome.max_button_height() > 0);

        // All client chrome buttons are hidden.
        assert!(!client_chrome.is_any_button_enabled());
        assert_eq!(-1, client_chrome.max_button_height());

        // System chrome is always 0px height, when client area is extended.
        if is_extended {
            assert!(matches!(system_chrome.title_bar_height(), -1 | 0));
        }
        // Can't get titlebar height on macOS.
        else if !cfg!(target_os = "macos") {
            assert!(system_chrome.title_bar_height() > 0);
        }
    }

    fn set_parameters(&self, extend_client_area: bool, title_bar_height: i32) {
        let extend_client_area_check_box =
            self.session().find_element_by_accessibility_id("WindowExtendClientAreaToDecorationsHint");
        let title_bar_height_box = self.session().find_element_by_accessibility_id("WindowTitleBarHeightHint");

        if extend_client_area_check_box.get_is_checked() != Some(extend_client_area) {
            extend_client_area_check_box.click();
        }

        title_bar_height_box.click();
        title_bar_height_box.clear();
        if title_bar_height >= 0 {
            title_bar_height_box.send_keys(&title_bar_height.to_string());
        }
    }

    fn apply_to_current_window(&self) {
        let apply_window_decorations = self.session().find_element_by_accessibility_id("ApplyWindowDecorations");
        apply_window_decorations.click();
    }

    fn apply_on_new_window(&self) -> Disposable {
        let show_new_window_decorations = self.session().find_element_by_accessibility_id("ShowNewWindowDecorations");
        show_new_window_decorations.open_window_with_click()
    }
}

pub fn tests(tests: &mut Tests) {
    let mut class = tests.class("window_decorations_tests", Collection::WindowDecorations, WindowDecorationsTests::new);
    class
        .fact(
            "window_size_should_be_consistent_between_toggles",
            WindowDecorationsTests::window_size_should_be_consistent_between_toggles,
        )
        .fact("can_restore_to_non_extended_state", WindowDecorationsTests::can_restore_to_non_extended_state);
    for title_bar_height in [-1, 25, 50] {
        class
            .platform_fact(
                &format!("should_apply_client_chrome({title_bar_height})"),
                TestPlatforms::WINDOWS,
                move |t: &WindowDecorationsTests| t.should_apply_client_chrome(title_bar_height),
            )
            .platform_fact(
                &format!("should_apply_system_chrome({title_bar_height})"),
                TestPlatforms::MAC_OS,
                move |t: &WindowDecorationsTests| t.should_apply_system_chrome(title_bar_height),
            )
            .platform_fact(
                &format!("should_apply_client_chrome_on_new_window({title_bar_height})"),
                TestPlatforms::WINDOWS,
                move |t: &WindowDecorationsTests| t.should_apply_client_chrome_on_new_window(title_bar_height),
            )
            .platform_fact(
                &format!("should_apply_system_chrome_on_new_window({title_bar_height})"),
                TestPlatforms::MAC_OS,
                move |t: &WindowDecorationsTests| t.should_apply_system_chrome_on_new_window(title_bar_height),
            );
    }
}
