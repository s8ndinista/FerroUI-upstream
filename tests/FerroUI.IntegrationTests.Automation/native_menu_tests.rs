//! Port of `NativeMenuTests.cs`: the classes `NativeMenuTests` and
//! `DockMenuTests`.

use crate::infrastructure::{Collection, DefaultAppFixture, Session, TestBase, TestClass, TestPlatforms, Tests};
use std::time::Duration;

pub struct NativeMenuTests {
    base: TestBase,
}

impl TestClass for NativeMenuTests {}

impl NativeMenuTests {
    pub fn new(fixture: &DefaultAppFixture) -> Self {
        NativeMenuTests { base: TestBase::new(fixture, "Automation") }
    }

    fn session(&self) -> &Session {
        self.base.session()
    }

    pub fn mac_os_view_menu_select_button_tab(&self) {
        let tabs = self.session().find_element_by_accessibility_id("Pager");
        let button_tab = tabs.find_element_by_name("Button");
        let menu_bar = self.session().find_element_by_xpath("/XCUIElementTypeApplication/XCUIElementTypeMenuBar");
        let view_menu = menu_bar.find_element_by_name("View");

        assert!(!button_tab.selected());

        view_menu.click();
        let button_menu = view_menu.find_element_by_name("Button");
        button_menu.click();

        assert!(button_tab.selected());
    }

    pub fn win32_view_menu_select_button_tab(&self) {
        let tabs = self.session().find_element_by_accessibility_id("Pager");
        let button_tab = tabs.find_element_by_name("Button");
        let view_menu = self.session().find_element_by_xpath("//MenuItem[@Name='View']");

        assert!(!button_tab.selected());

        view_menu.click();
        let button_menu = view_menu.find_element_by_name("Button");
        button_menu.click();

        assert!(button_tab.selected());
    }

    pub fn mac_os_sanitizes_access_key_markers_when_included_in_menu_title(&self) {
        let menu_bar = self.session().find_element_by_xpath("/XCUIElementTypeApplication/XCUIElementTypeMenuBar");

        assert!(menu_bar.find_elements_by_name("_Options").is_empty());
        assert!(menu_bar.find_elements_by_name("Options").len() == 1);
    }

    pub fn win32_ferro_menu_has_tool_tip_if_defined(&self) {
        let view_menu = self.session().find_element_by_xpath("//MenuItem[@Name='View']");
        view_menu.click();

        let button_menu_item = view_menu.find_element_by_name("Button");
        button_menu_item.move_pointer_over();

        // Wait for tooltip to open.
        std::thread::sleep(Duration::from_millis(2000));

        let tool_tip_candidates = self.session().find_elements_by_class_name("TextBlock");
        assert!(tool_tip_candidates.iter().any(|x| x.text() == "Tip:Button"));
    }

    pub fn mac_os_native_menu_has_tool_tip_if_defined(&self) {
        let menu_bar = self.session().find_element_by_xpath("/XCUIElementTypeApplication/XCUIElementTypeMenuBar");
        let view_menu = menu_bar.find_element_by_name("View");
        view_menu.click();

        let button_menu_item = view_menu.find_element_by_name("Button");
        button_menu_item.move_pointer_over();

        // Wait for tooltip to open.
        std::thread::sleep(Duration::from_millis(4000));

        let tool_tip_candidates = self.session().find_elements_by_class_name("XCUIElementTypeStaticText");
        assert!(tool_tip_candidates.iter().any(|x| x.text() == "Tip:Button"));
    }
}

pub struct DockMenuTests {
    base: TestBase,
}

impl TestClass for DockMenuTests {}

impl DockMenuTests {
    #[allow(dead_code)]
    const DOCK_APP_NAME: &'static str = "IntegrationTestApp";

    pub fn new(fixture: &DefaultAppFixture) -> Self {
        DockMenuTests { base: TestBase::new(fixture, "DesktopPage") }
    }

    fn session(&self) -> &Session {
        self.base.session()
    }

    pub fn mac_os_dock_menu_can_add_items_dynamically(&self) {
        let mut count_text = self.session().find_element_by_accessibility_id("DockMenuItemCount");
        assert_eq!("0", count_text.text());

        let add_button = self.session().find_element_by_accessibility_id("AddDockMenuItem");
        add_button.click();

        std::thread::sleep(Duration::from_millis(500));

        count_text = self.session().find_element_by_accessibility_id("DockMenuItemCount");
        assert_eq!("2", count_text.text());

        add_button.click();

        std::thread::sleep(Duration::from_millis(500));

        count_text = self.session().find_element_by_accessibility_id("DockMenuItemCount");
        assert_eq!("3", count_text.text());
    }
}

pub fn tests(tests: &mut Tests) {
    tests
        .class("native_menu_tests", Collection::Default, NativeMenuTests::new)
        .platform_fact(
            "mac_os_view_menu_select_button_tab",
            TestPlatforms::MAC_OS,
            NativeMenuTests::mac_os_view_menu_select_button_tab,
        )
        .platform_fact(
            "win32_view_menu_select_button_tab",
            TestPlatforms::WINDOWS,
            NativeMenuTests::win32_view_menu_select_button_tab,
        )
        .platform_fact(
            "mac_os_sanitizes_access_key_markers_when_included_in_menu_title",
            TestPlatforms::MAC_OS,
            NativeMenuTests::mac_os_sanitizes_access_key_markers_when_included_in_menu_title,
        )
        .platform_fact(
            "win32_ferro_menu_has_tool_tip_if_defined",
            TestPlatforms::WINDOWS,
            NativeMenuTests::win32_ferro_menu_has_tool_tip_if_defined,
        )
        .add(
            "mac_os_native_menu_has_tool_tip_if_defined",
            TestPlatforms::MAC_OS,
            Some("Flaky test"),
            NativeMenuTests::mac_os_native_menu_has_tool_tip_if_defined,
        );
    tests.class("dock_menu_tests", Collection::Default, DockMenuTests::new).platform_fact(
        "mac_os_dock_menu_can_add_items_dynamically",
        TestPlatforms::MAC_OS,
        DockMenuTests::mac_os_dock_menu_can_add_items_dynamically,
    );
}
