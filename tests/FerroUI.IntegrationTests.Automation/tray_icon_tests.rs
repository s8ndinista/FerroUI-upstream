//! Port of `TrayIconTests.cs`.

use crate::infrastructure::{
    Actions, By, Collection, DefaultAppFixture, Element, Session, TestBase, TestClass, TestPlatforms, Tests,
};
use std::time::Duration;

const TRAY_ICON_NAME: &str = "IntegrationTestApp TrayIcon";

pub struct TrayIconTests {
    base: TestBase,
    root_session: Option<Session>,
}

impl TestClass for TrayIconTests {
    fn dispose(&self) {
        // The nested session of the desktop starts no application: nothing to end.
    }
}

impl TrayIconTests {
    pub fn new(fixture: &DefaultAppFixture) -> Self {
        let base = TestBase::new(fixture, "Desktop");
        // "Root" is a special name for windows the desktop session, that has access to task bar.
        let root_session = if cfg!(windows) { Some(fixture.create_nested_session("Root")) } else { None };
        TrayIconTests { base, root_session }
    }

    fn session(&self) -> &Session {
        self.base.session()
    }

    fn root_or_session(&self) -> &Session {
        self.root_session.as_ref().unwrap_or_else(|| self.session())
    }

    // Left click is only supported on Windows.
    pub fn should_handle_left_click(&self) {
        let tray_icon_button = get_tray_icon_button(self.root_or_session(), TRAY_ICON_NAME);
        let tray_icon_button = tray_icon_button.expect("the tray icon");

        tray_icon_button.send_click();

        std::thread::sleep(Duration::from_millis(2000));

        let check_box = self.session().find_element_by_accessibility_id("TrayIconClicked");
        assert_eq!(Some(true), check_box.get_is_checked());
    }

    pub fn should_handle_context_menu_item_click(&self) {
        let tray_icon_button = get_tray_icon_button(self.root_or_session(), TRAY_ICON_NAME);
        let tray_icon_button = tray_icon_button.expect("the tray icon");

        let context_menu = show_and_get_tray_menu(&tray_icon_button, TRAY_ICON_NAME);

        let menu_item = context_menu.find_element_by_name("Raise Menu Clicked");
        menu_item.send_click();

        std::thread::sleep(Duration::from_millis(2000));

        let check_box = self.session().find_element_by_accessibility_id("TrayIconMenuClicked");
        assert_eq!(Some(true), check_box.get_is_checked());
    }

    pub fn can_toggle_tray_icon_visibility(&self) {
        let tray_icon_button = get_tray_icon_button(self.root_or_session(), TRAY_ICON_NAME);
        assert!(tray_icon_button.is_some());

        let toggle_button = self.session().find_element_by_accessibility_id("ToggleTrayIconVisible");
        toggle_button.send_click();

        let tray_icon_button = get_tray_icon_button(self.root_or_session(), TRAY_ICON_NAME);
        assert!(tray_icon_button.is_none());

        toggle_button.send_click();

        let tray_icon_button = get_tray_icon_button(self.root_or_session(), TRAY_ICON_NAME);
        assert!(tray_icon_button.is_some());
    }
}

fn get_tray_icon_button(session: &Session, tray_icon_name: &str) -> Option<Element> {
    fn try_to_get_icon(parent: &Element, tray_icon_name: &str) -> Option<Element> {
        parent
            .find_elements_by_name(tray_icon_name)
            .into_iter()
            .last()
            // Some icons (including the framework's) for some reason include leading whitespace in their name.
            // Couldn't find any info on that, which is weird.
            .or_else(|| parent.find_elements_by_name(&format!(" {tray_icon_name}")).into_iter().last())
    }

    if cfg!(windows) {
        let task_bar = session
            .find_elements_by_class_name("Shell_TrayWnd")
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("Couldn't find Taskbar on current system."));

        if let Some(tray_icon) = try_to_get_icon(&task_bar, tray_icon_name) {
            return Some(tray_icon);
        }

        // Add a sleep here, as previous test might still run popup closing animation.
        std::thread::sleep(Duration::from_millis(1000));

        // win11: SystemTrayIcon
        // win10: Notification Chevron
        let tray_icons_button = task_bar
            .find_elements_by_accessibility_id("SystemTrayIcon")
            .into_iter()
            .next()
            .or_else(|| task_bar.find_elements_by_name("Notification Chevron").into_iter().next())
            .unwrap_or_else(|| panic!("SystemTrayIcon cannot be found."));
        tray_icons_button.click();

        // win11: TopLevelWindowForOverflowXamlIsland
        // win10: NotifyIconOverflowWindow
        let tray_icons_flyout = session
            .find_elements_by_class_name("TopLevelWindowForOverflowXamlIsland")
            .into_iter()
            .next()
            .or_else(|| session.find_elements_by_class_name("NotifyIconOverflowWindow").into_iter().next())
            .unwrap_or_else(|| panic!("System tray overflow window cannot be found."));
        return try_to_get_icon(&tray_icons_flyout, tray_icon_name);
    }
    if cfg!(target_os = "macos") {
        return session.find_elements_by_xpath("//XCUIElementTypeStatusItem").into_iter().next();
    }

    panic!("PlatformNotSupportedException");
}

fn show_and_get_tray_menu(tray_icon: &Element, tray_icon_name: &str) -> Element {
    if cfg!(windows) {
        let session = tray_icon.session();
        Actions::new(session).context_click_on(tray_icon).perform();

        std::thread::sleep(Duration::from_millis(1000));

        session.find_element(By::XPath(format!("//Window[@AutomationId='FerroTrayPopupRoot_{tray_icon_name}']")))
    } else {
        tray_icon.click();
        tray_icon.find_element_by_xpath("//XCUIElementTypeStatusItem/XCUIElementTypeMenu")
    }
}

pub fn tests(tests: &mut Tests) {
    tests
        .class("tray_icon_tests", Collection::Default, TrayIconTests::new)
        .add("should_handle_left_click", TestPlatforms::WINDOWS, Some("Flaky test"), TrayIconTests::should_handle_left_click)
        .add(
            "should_handle_context_menu_item_click",
            TestPlatforms::ALL,
            Some("Flaky test"),
            TrayIconTests::should_handle_context_menu_item_click,
        )
        .add(
            "can_toggle_tray_icon_visibility",
            TestPlatforms::ALL,
            Some("Flaky test"),
            TrayIconTests::can_toggle_tray_icon_visibility,
        );
}
