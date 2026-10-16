//! Port of `WindowTests_MacOS.cs`.
//!
//! `WindowOrder_Owned_Dialog_Stays_InFront_Of_Parent` and
//! `WindowOrder_Owned_Dialog_Stays_InFront_Of_FullScreen_Parent` are tests
//! upstream only with Appium 2 (`#if APPIUM2`); the harness of the port is not
//! Appium 1, so they are registered.

use crate::infrastructure::{
    Actions, Collection, DefaultAppFixture, Disposable, Element, Session, TestBase, TestClass, TestPlatforms, Tests,
};
use ferroui_base::PixelSize;
use ferroui_controls::{WindowDecorations, WindowStartupLocation};
use std::time::Duration;

/// `ShowWindowMode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShowWindowMode {
    NonOwned,
    Owned,
    Modal,
}

/// The options of `OpenWindow`, with upstream's defaults.
#[derive(Clone, Copy)]
pub struct OpenWindow {
    pub size: Option<PixelSize>,
    pub mode: ShowWindowMode,
    pub location: WindowStartupLocation,
    pub can_resize: bool,
    pub window_decorations: WindowDecorations,
    pub extend_client_area: bool,
}

impl Default for OpenWindow {
    fn default() -> Self {
        OpenWindow {
            size: None,
            mode: ShowWindowMode::NonOwned,
            location: WindowStartupLocation::Manual,
            can_resize: true,
            window_decorations: WindowDecorations::Full,
            extend_client_area: false,
        }
    }
}

fn open(size: Option<PixelSize>, mode: ShowWindowMode, location: WindowStartupLocation) -> OpenWindow {
    OpenWindow { size, mode, location, ..Default::default() }
}

pub struct WindowTestsMacOS {
    base: TestBase,
}

impl TestClass for WindowTestsMacOS {}

impl WindowTestsMacOS {
    pub fn new(fixture: &DefaultAppFixture) -> Self {
        WindowTestsMacOS { base: TestBase::new(fixture, "Window") }
    }

    fn session(&self) -> &Session {
        self.base.session()
    }

    pub fn window_order_modal_dialog_stays_in_front_of_parent(&self) {
        let main_window = self.session().find_element_by_accessibility_id("MainWindow");

        let _window =
            self.open_window(open(Some(PixelSize::new(200, 100)), ShowWindowMode::Modal, WindowStartupLocation::Manual));
        main_window.click();

        std::thread::sleep(Duration::from_millis(300)); // sync with timer

        let secondary_window_index = self.get_window_order("SecondaryWindow");
        assert_eq!(1, secondary_window_index);
    }

    pub fn window_order_modal_dialog_stays_in_front_of_parent_when_clicking_resize_grip(&self) {
        let _main_window = self.get_window("MainWindow");

        let _window =
            self.open_window(open(Some(PixelSize::new(200, 100)), ShowWindowMode::Modal, WindowStartupLocation::Manual));
        let child_window = self.get_window("SecondaryWindow");

        Actions::new(self.session()).move_to_element_offset(&child_window, 100, 1).click_and_hold().perform();

        let secondary_window_index = self.get_window_order("SecondaryWindow");

        Actions::new(self.session()).move_to_element_offset(&child_window, 100, 1).release().perform();

        assert_eq!(1, secondary_window_index);
    }

    pub fn window_order_modal_dialog_stays_in_front_of_parent_when_in_fullscreen(&self) {
        let main_window = self.get_window("MainWindow");
        let full_screen = main_window.find_element_by_accessibility_id("_XCUI:FullScreenWindow");

        full_screen.click();

        std::thread::sleep(Duration::from_millis(500));

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _window = self.open_window(open(
                Some(PixelSize::new(200, 100)),
                ShowWindowMode::Modal,
                WindowStartupLocation::Manual,
            ));
            let secondary_window_index = self.get_window_order("SecondaryWindow");
            assert_eq!(1, secondary_window_index);
        }));

        // finally
        self.session().find_element_by_accessibility_id("IntegrationTestApp_ExitFullscreen").click();
        if let Err(panic) = result {
            std::panic::resume_unwind(panic);
        }
    }

    pub fn window_order_owned_dialog_stays_in_front_of_parent(&self) {
        let main_window = self.session().find_element_by_accessibility_id("MainWindow");

        let _window =
            self.open_window(open(Some(PixelSize::new(200, 100)), ShowWindowMode::Owned, WindowStartupLocation::Manual));
        main_window.send_click();
        let secondary_window_index = self.get_window_order("SecondaryWindow");
        assert_eq!(1, secondary_window_index);
    }

    pub fn window_order_owned_dialog_stays_in_front_of_full_screen_parent(&self) {
        let mut main_window = self.session().find_element_by_accessibility_id("MainWindow");

        // Enter fullscreen
        main_window.find_element_by_accessibility_id("EnterFullscreen").click();

        // Wait for fullscreen transition.
        std::thread::sleep(Duration::from_millis(1000));

        // Make sure we entered fullscreen.
        let mut window_state = main_window.find_element_by_accessibility_id("MainWindowState");
        assert_eq!("FullScreen", window_state.text());

        // Open child window.
        {
            let _window = self.open_window(open(
                Some(PixelSize::new(200, 100)),
                ShowWindowMode::Owned,
                WindowStartupLocation::Manual,
            ));
            main_window.send_click();
            let secondary_window_index = self.get_window_order("SecondaryWindow");
            assert_eq!(1, secondary_window_index);
        }

        // Exit fullscreen by menu shortcut Command+R
        main_window.find_element_by_accessibility_id("IntegrationTestApp_ExitFullscreen").click();

        // Wait for restore transition.
        std::thread::sleep(Duration::from_millis(1000));

        // Make sure we exited fullscreen.
        main_window = self.session().find_element_by_accessibility_id("MainWindow");
        window_state = main_window.find_element_by_accessibility_id("MainWindowState");
        assert_eq!("Normal", window_state.text());
    }

    pub fn window_order_owned_dialog_stays_in_front_of_parent_after_modal_closed(&self) {
        let _window =
            self.open_window(open(Some(PixelSize::new(200, 300)), ShowWindowMode::Owned, WindowStartupLocation::Manual));
        self.open_window(open(None, ShowWindowMode::Modal, WindowStartupLocation::Manual)).dispose();

        let secondary_window_index = self.get_window_order("SecondaryWindow");
        assert_eq!(1, secondary_window_index);
    }

    pub fn does_not_switch_space_from_full_screen_to_main_desktop_when_full_screen_window_clicked(&self) {
        // Issue #9565
        let mut main_window = self.session().find_element_by_accessibility_id("MainWindow");
        let mut window_state: Element;

        // Open child window.
        {
            let _window = self.open_window(open(
                Some(PixelSize::new(200, 100)),
                ShowWindowMode::Owned,
                WindowStartupLocation::Manual,
            ));
            // Enter fullscreen
            main_window.find_element_by_accessibility_id("EnterFullscreen").click();

            // Wait for fullscreen transition.
            std::thread::sleep(Duration::from_millis(1000));

            // Make sure we entered fullscreen.
            main_window = self.session().find_element_by_accessibility_id("MainWindow");
            window_state = main_window.find_element_by_accessibility_id("MainWindowState");
            assert_eq!("FullScreen", window_state.text());

            // Click on main window
            main_window.click();

            // Failed here due to #9565: main window is no longer visible as the main space is now shown instead
            // of the fullscreen space.
            main_window.find_element_by_accessibility_id("IntegrationTestApp_ExitFullscreen").click();

            // Wait for restore transition.
            std::thread::sleep(Duration::from_millis(1000));
        }

        // Make sure we exited fullscreen.
        main_window = self.session().find_element_by_accessibility_id("MainWindow");
        window_state = main_window.find_element_by_accessibility_id("MainWindowState");
        assert_eq!("Normal", window_state.text());
    }

    pub fn window_order_non_owned_window_does_not_stay_in_front_of_parent(&self) {
        let main_window = self.session().find_element_by_accessibility_id("MainWindow");

        let _window = self.open_window(open(
            Some(PixelSize::new(800, 100)),
            ShowWindowMode::NonOwned,
            WindowStartupLocation::Manual,
        ));
        main_window.click();

        let secondary_window_index = self.get_window_order("SecondaryWindow");

        assert_eq!(2, secondary_window_index);

        let send_to_back = self.session().find_element_by_accessibility_id("SendToBack");
        send_to_back.click();
    }

    pub fn window_order_owned_window_stays_in_front_of_topmost_owner(&self) {
        self.session().find_element_by_accessibility_id("ShowTopmostWindow").click();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            std::thread::sleep(Duration::from_millis(1000));

            let owner_window_index = self.get_window_order("OwnerWindow");
            let owned_window_index = self.get_window_order("OwnedWindow");

            // orderedIndex counts from the front, so the owned window needs the lower index.
            assert!(
                owned_window_index < owner_window_index,
                "Expected the owned window in front of its topmost owner, but the owner was at {owner_window_index} \
                 and the owned window at {owned_window_index}."
            );
        }));

        // finally
        self.close_window("OwnedWindow");
        self.close_window("OwnerWindow");
        if let Err(panic) = result {
            std::panic::resume_unwind(panic);
        }
    }

    pub fn window_order_owned_is_correct_after_closing_window(&self) {
        let _window = self.open_window(open(
            Some(PixelSize::new(300, 500)),
            ShowWindowMode::Owned,
            WindowStartupLocation::CenterOwner,
        ));
        // Open a second child window, and close it.
        {
            let _second = self.open_window(open(
                Some(PixelSize::new(200, 200)),
                ShowWindowMode::Owned,
                WindowStartupLocation::CenterOwner,
            ));
        }

        let secondary_window_index = self.get_window_order("SecondaryWindow");

        assert_eq!(1, secondary_window_index);
    }

    pub fn parent_window_has_disabled_chrome_buttons_when_modal_dialog_shown(&self) {
        let window = self.get_window("MainWindow");
        let window_chrome = window.get_system_chrome_buttons();

        assert!(window_chrome.close.as_ref().expect("the close button").enabled());
        assert!(window_chrome.full_screen.as_ref().expect("the full screen button").enabled());
        assert!(window_chrome.minimize.as_ref().expect("the minimize button").enabled());
        assert!(window_chrome.maximize.is_none());

        let _window = self.open_window(open(
            Some(PixelSize::new(200, 100)),
            ShowWindowMode::Modal,
            WindowStartupLocation::CenterOwner,
        ));
        assert!(!window_chrome.close.as_ref().unwrap().enabled());
        assert!(!window_chrome.full_screen.as_ref().unwrap().enabled());
        assert!(!window_chrome.minimize.as_ref().unwrap().enabled());
    }

    pub fn minimize_button_is_disabled_on_modal_dialog(&self) {
        let _window = self.open_window(open(
            Some(PixelSize::new(200, 100)),
            ShowWindowMode::Modal,
            WindowStartupLocation::CenterOwner,
        ));
        let secondary_window = self.get_window("SecondaryWindow");
        let window_chrome = secondary_window.get_system_chrome_buttons();

        assert!(window_chrome.close.expect("the close button").enabled());
        assert!(window_chrome.maximize.expect("the zoom button").enabled());
        assert!(!window_chrome.minimize.expect("the minimize button").enabled());
    }

    pub fn minimize_button_disabled_owned_window(&self, mode: ShowWindowMode) {
        let _window = self.open_window(open(Some(PixelSize::new(200, 100)), mode, WindowStartupLocation::Manual));
        let secondary_window = self.get_window("SecondaryWindow");
        let miniaturize_button = secondary_window.find_element_by_accessibility_id("_XCUI:MinimizeWindow");

        assert!(!miniaturize_button.enabled());
    }

    pub fn minimize_button_minimizes_window(&self, mode: ShowWindowMode) {
        let _window = self.open_window(open(Some(PixelSize::new(200, 100)), mode, WindowStartupLocation::Manual));
        let secondary_window = self.get_window("SecondaryWindow");
        let miniaturize_button = secondary_window.find_element_by_accessibility_id("_XCUI:MinimizeWindow");

        miniaturize_button.click();
        std::thread::sleep(Duration::from_millis(1000));

        let hittable: Vec<String> = self
            .session()
            .find_elements_by_xpath("/XCUIElementTypeApplication/XCUIElementTypeWindow")
            .iter()
            .map(|x| x.get_attribute("hittable").unwrap_or_default())
            .collect();
        assert_eq!(vec!["true", "false"], hittable);

        self.session().find_element_by_accessibility_id("RestoreAll").click();
        std::thread::sleep(Duration::from_millis(1000));

        let hittable: Vec<String> = self
            .session()
            .find_elements_by_xpath("/XCUIElementTypeApplication/XCUIElementTypeWindow")
            .iter()
            .map(|x| x.get_attribute("hittable").unwrap_or_default())
            .collect();
        assert_eq!(vec!["true", "true"], hittable);
    }

    pub fn hidden_child_window_is_not_reshown_when_parent_clicked(&self) {
        let main_window = self.session().find_element_by_accessibility_id("MainWindow");

        // We don't use dispose to close the window here, because it seems that hiding and re-showing a window
        // causes Appium to think it's a different window.
        std::mem::forget(self.open_window(open(None, ShowWindowMode::Owned, WindowStartupLocation::Manual)));

        let mut secondary_window = self.get_window("SecondaryWindow");
        let hide_button = secondary_window.find_element_by_accessibility_id("HideButton");

        hide_button.click();

        let windows = self.session().find_elements_by_xpath("XCUIElementTypeWindow");
        assert_eq!(1, windows.len());

        main_window.click();

        let windows = self.session().find_elements_by_xpath("XCUIElementTypeWindow");
        assert_eq!(1, windows.len());

        self.session().find_element_by_accessibility_id("RestoreAll").click();

        // Close the window manually.
        secondary_window = self.get_window("SecondaryWindow");
        secondary_window.find_element_by_accessibility_id("_XCUI:CloseWindow").click();
    }

    pub fn toggling_system_decorations_should_preserve_extend_client_area(&self) {
        // #10650
        let _window = self.open_window(OpenWindow { extend_client_area: true, ..Default::default() });
        let secondary_window = self.get_window("SecondaryWindow");

        // The XPath of the title bar text _should_ be "XCUIElementTypeStaticText"
        // but Appium seems to put a fake node between the window and the title bar
        // https://stackoverflow.com/a/71914227/6448
        let mut title_bar = secondary_window.find_elements_by_xpath("/*/XCUIElementTypeStaticText").len();

        assert_eq!(0, title_bar);

        secondary_window.find_element_by_accessibility_id("CurrentSystemDecorations").click();
        self.session().find_element_by_accessibility_id("SystemDecorationsNone").send_click();
        secondary_window.find_element_by_accessibility_id("CurrentSystemDecorations").click();
        self.session().find_element_by_accessibility_id("SystemDecorationsFull").send_click();

        title_bar = secondary_window.find_elements_by_xpath("/*/XCUIElementTypeStaticText").len();
        assert_eq!(0, title_bar);
    }

    pub fn extend_client_area_system_decorations_shows_correct_buttons(&self, decorations: WindowDecorations) {
        // #10650
        let _window = self.open_window(OpenWindow {
            extend_client_area: true,
            window_decorations: decorations,
            ..Default::default()
        });
        let secondary_window = self.get_window("SecondaryWindow");

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let chrome = secondary_window.get_system_chrome_buttons();

            if decorations == WindowDecorations::Full {
                assert!(chrome.close.is_some());
                assert!(chrome.minimize.is_some());
                assert!(chrome.full_screen.is_some());
            } else {
                assert!(chrome.close.is_none());
                assert!(chrome.minimize.is_none());
                assert!(chrome.full_screen.is_none());
            }
        }));

        // finally
        if decorations != WindowDecorations::Full {
            secondary_window.find_element_by_accessibility_id("CurrentSystemDecorations").click();
            self.session().find_element_by_accessibility_id("SystemDecorationsFull").send_click();
        }
        if let Err(panic) = result {
            std::panic::resume_unwind(panic);
        }
    }

    fn open_window(&self, options: OpenWindow) -> Disposable {
        let session = self.session();
        let size_text_box = session.find_element_by_accessibility_id("ShowWindowSize");
        let mode_combo_box = session.find_element_by_accessibility_id("ShowWindowMode");
        let location_combo_box = session.find_element_by_accessibility_id("ShowWindowLocation");
        let can_resize_check_box = session.find_element_by_accessibility_id("ShowWindowCanResize");
        let show_button = session.find_element_by_accessibility_id("ShowWindow");
        let system_decorations_combo_box = session.find_element_by_accessibility_id("ShowWindowSystemDecorations");
        let extend_client_area_check_box =
            session.find_element_by_accessibility_id("ShowWindowExtendClientAreaToDecorationsHint");

        if let Some(size) = options.size {
            size_text_box.send_keys(&format!("{}, {}", size.width, size.height));
        }

        let mode = format!("{:?}", options.mode);
        if mode_combo_box.get_combo_box_value() != mode {
            mode_combo_box.click();
            session.find_element_by_name(&mode).send_click();
        }

        let location = format!("{:?}", options.location);
        if location_combo_box.get_combo_box_value() != location {
            location_combo_box.click();
            session.find_element_by_name(&location).send_click();
        }

        if can_resize_check_box.get_is_checked() != Some(options.can_resize) {
            can_resize_check_box.click();
        }

        let decorations = format!("{:?}", options.window_decorations);
        if system_decorations_combo_box.get_combo_box_value() != decorations {
            system_decorations_combo_box.click();
            session.find_element_by_name(&decorations).send_click();
        }

        if extend_client_area_check_box.get_is_checked() != Some(options.extend_client_area) {
            extend_client_area_check_box.click();
        }

        show_button.open_window_with_click()
    }

    fn get_window(&self, identifier: &str) -> Element {
        self.session().get_window_by_id(identifier)
    }

    fn get_window_order(&self, identifier: &str) -> i32 {
        let window = self.get_window(identifier);
        let order = window.find_element_by_xpath("//*[@identifier='CurrentOrder']");
        let text = order.text();
        text.parse().unwrap_or_else(|error| panic!("CurrentOrder {text:?}: {error}"))
    }

    fn close_window(&self, identifier: &str) {
        self.get_window(identifier).find_element_by_accessibility_id("_XCUI:CloseWindow").click();
        std::thread::sleep(Duration::from_millis(1000));
    }
}

pub fn tests(tests: &mut Tests) {
    const MAC_OS: TestPlatforms = TestPlatforms::MAC_OS;
    let mut class = tests.class("window_tests_mac_os", Collection::Default, WindowTestsMacOS::new);
    class
        .platform_fact(
            "window_order_modal_dialog_stays_in_front_of_parent",
            MAC_OS,
            WindowTestsMacOS::window_order_modal_dialog_stays_in_front_of_parent,
        )
        .platform_fact(
            "window_order_modal_dialog_stays_in_front_of_parent_when_clicking_resize_grip",
            MAC_OS,
            WindowTestsMacOS::window_order_modal_dialog_stays_in_front_of_parent_when_clicking_resize_grip,
        )
        .platform_fact(
            "window_order_modal_dialog_stays_in_front_of_parent_when_in_fullscreen",
            MAC_OS,
            WindowTestsMacOS::window_order_modal_dialog_stays_in_front_of_parent_when_in_fullscreen,
        )
        .platform_fact(
            "window_order_owned_dialog_stays_in_front_of_parent",
            MAC_OS,
            WindowTestsMacOS::window_order_owned_dialog_stays_in_front_of_parent,
        )
        .platform_fact(
            "window_order_owned_dialog_stays_in_front_of_full_screen_parent",
            MAC_OS,
            WindowTestsMacOS::window_order_owned_dialog_stays_in_front_of_full_screen_parent,
        )
        .platform_fact(
            "window_order_owned_dialog_stays_in_front_of_parent_after_modal_closed",
            MAC_OS,
            WindowTestsMacOS::window_order_owned_dialog_stays_in_front_of_parent_after_modal_closed,
        )
        .add(
            "does_not_switch_space_from_full_screen_to_main_desktop_when_full_screen_window_clicked",
            MAC_OS,
            Some("Flaky test"),
            WindowTestsMacOS::does_not_switch_space_from_full_screen_to_main_desktop_when_full_screen_window_clicked,
        )
        .platform_fact(
            "window_order_non_owned_window_does_not_stay_in_front_of_parent",
            MAC_OS,
            WindowTestsMacOS::window_order_non_owned_window_does_not_stay_in_front_of_parent,
        )
        .platform_fact(
            "window_order_owned_window_stays_in_front_of_topmost_owner",
            MAC_OS,
            WindowTestsMacOS::window_order_owned_window_stays_in_front_of_topmost_owner,
        )
        .platform_fact(
            "window_order_owned_is_correct_after_closing_window",
            MAC_OS,
            WindowTestsMacOS::window_order_owned_is_correct_after_closing_window,
        )
        .platform_fact(
            "parent_window_has_disabled_chrome_buttons_when_modal_dialog_shown",
            MAC_OS,
            WindowTestsMacOS::parent_window_has_disabled_chrome_buttons_when_modal_dialog_shown,
        )
        .platform_fact(
            "minimize_button_is_disabled_on_modal_dialog",
            MAC_OS,
            WindowTestsMacOS::minimize_button_is_disabled_on_modal_dialog,
        )
        .platform_fact("minimize_button_disabled_owned_window(Owned)", MAC_OS, |t: &WindowTestsMacOS| {
            t.minimize_button_disabled_owned_window(ShowWindowMode::Owned)
        })
        .platform_fact("minimize_button_minimizes_window(NonOwned)", MAC_OS, |t: &WindowTestsMacOS| {
            t.minimize_button_minimizes_window(ShowWindowMode::NonOwned)
        })
        .platform_fact(
            "hidden_child_window_is_not_reshown_when_parent_clicked",
            MAC_OS,
            WindowTestsMacOS::hidden_child_window_is_not_reshown_when_parent_clicked,
        )
        .platform_fact(
            "toggling_system_decorations_should_preserve_extend_client_area",
            MAC_OS,
            WindowTestsMacOS::toggling_system_decorations_should_preserve_extend_client_area,
        );
    for decorations in [WindowDecorations::None, WindowDecorations::BorderOnly, WindowDecorations::Full] {
        class.platform_fact(
            &format!("extend_client_area_system_decorations_shows_correct_buttons({decorations:?})"),
            MAC_OS,
            move |t: &WindowTestsMacOS| t.extend_client_area_system_decorations_shows_correct_buttons(decorations),
        );
    }
}
