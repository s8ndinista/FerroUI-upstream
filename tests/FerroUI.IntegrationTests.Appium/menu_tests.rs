//! Port of `MenuTests.cs`: the tests run in the application of the default
//! collection (`MenuTests.Default`) and in the one with overlay popups
//! (`MenuTests.OverlayPopups`).

use crate::infrastructure::{Actions, Collection, DefaultAppFixture, Keys, Session, TestBase, TestClass, TestPlatforms, Tests};
use std::time::Duration;

pub struct MenuTests {
    base: TestBase,
}

impl TestClass for MenuTests {
    fn dispose(&self) {
        // Click the reset button so that any menu still open gets closed
        let reset = self.session().find_element_by_accessibility_id("MenuClickedMenuItemReset");
        reset.click();
    }
}

impl MenuTests {
    pub fn new(fixture: &DefaultAppFixture) -> Self {
        let base = TestBase::new(fixture, "Menu");
        let reset = base.session().find_element_by_accessibility_id("MenuClickedMenuItemReset");
        reset.click();

        let clicked_menu_item = base.session().find_element_by_accessibility_id("ClickedMenuItem");
        assert_eq!("None", clicked_menu_item.text());
        MenuTests { base }
    }

    fn session(&self) -> &Session {
        self.base.session()
    }

    pub fn click_child(&self) {
        let root_menu_item = self.session().find_element_by_accessibility_id("RootMenuItem");

        root_menu_item.send_click();

        let child_menu_item = self.session().find_element_by_accessibility_id("Child1MenuItem");
        child_menu_item.send_click();

        let clicked_menu_item = self.session().find_element_by_accessibility_id("ClickedMenuItem");
        assert_eq!("_Child 1", clicked_menu_item.text());
    }

    pub fn click_grandchild(&self) {
        let root_menu_item = self.session().find_element_by_accessibility_id("RootMenuItem");

        root_menu_item.send_click();

        let child_menu_item = self.session().find_element_by_accessibility_id("Child2MenuItem");
        child_menu_item.send_click();

        let grandchild_menu_item = self.session().find_element_by_accessibility_id("GrandchildMenuItem");
        grandchild_menu_item.send_click();

        let clicked_menu_item = self.session().find_element_by_accessibility_id("ClickedMenuItem");
        assert_eq!("_Grandchild", clicked_menu_item.text());
    }

    pub fn select_child_with_alt_arrow_keys(&self) {
        self.move_pointer_out_of_the_way();

        let window = self.session().find_element_by_accessibility_id("MainWindow");
        window.click();

        window.send_keys(Keys::LEFT_ALT);
        std::thread::sleep(Duration::from_millis(150));
        window.send_keys(&format!("{}{}", Keys::DOWN, Keys::ENTER));

        let clicked_menu_item = self.session().find_element_by_accessibility_id("ClickedMenuItem");
        assert_eq!("_Child 1", clicked_menu_item.text());
    }

    pub fn select_grandchild_with_alt_arrow_keys(&self) {
        self.move_pointer_out_of_the_way();

        let window = self.session().find_element_by_accessibility_id("MainWindow");
        window.click();

        window.send_keys(Keys::LEFT_ALT);
        std::thread::sleep(Duration::from_millis(150));
        window.send_keys(&format!("{}{}{}{}", Keys::DOWN, Keys::DOWN, Keys::RIGHT, Keys::ENTER));

        let clicked_menu_item = self.session().find_element_by_accessibility_id("ClickedMenuItem");
        assert_eq!("_Grandchild", clicked_menu_item.text());
    }

    pub fn select_child_with_alt_access_keys(&self) {
        self.move_pointer_out_of_the_way();

        let window = self.session().find_element_by_accessibility_id("MainWindow");
        window.click();

        window.send_keys(Keys::LEFT_ALT);
        std::thread::sleep(Duration::from_millis(150));
        window.send_keys("r");
        std::thread::sleep(Duration::from_millis(75));
        window.send_keys("c");

        let clicked_menu_item = self.session().find_element_by_accessibility_id("ClickedMenuItem");
        assert_eq!("_Child 1", clicked_menu_item.text());
    }

    pub fn select_grandchild_with_alt_access_keys(&self) {
        self.move_pointer_out_of_the_way();

        let window = self.session().find_element_by_accessibility_id("MainWindow");
        window.click();

        window.send_keys(Keys::LEFT_ALT);
        std::thread::sleep(Duration::from_millis(150));
        window.send_keys("r");
        std::thread::sleep(Duration::from_millis(75));
        window.send_keys("h");
        std::thread::sleep(Duration::from_millis(75));
        window.send_keys("g");

        let clicked_menu_item = self.session().find_element_by_accessibility_id("ClickedMenuItem");
        assert_eq!("_Grandchild", clicked_menu_item.text());
    }

    pub fn select_child_with_click_arrow_keys(&self) {
        let root_menu_item = self.session().find_element_by_accessibility_id("RootMenuItem");
        root_menu_item.send_click();

        self.move_pointer_out_of_the_way();

        Actions::new(self.session()).send_keys(&format!("{}{}", Keys::DOWN, Keys::ENTER)).perform();

        let clicked_menu_item = self.session().find_element_by_accessibility_id("ClickedMenuItem");
        assert_eq!("_Child 1", clicked_menu_item.text());
    }

    pub fn select_grandchild_with_click_arrow_keys(&self) {
        let root_menu_item = self.session().find_element_by_accessibility_id("RootMenuItem");
        root_menu_item.send_click();

        self.move_pointer_out_of_the_way();

        Actions::new(self.session())
            .send_keys(&format!("{}{}{}{}", Keys::DOWN, Keys::DOWN, Keys::RIGHT, Keys::ENTER))
            .perform();

        let clicked_menu_item = self.session().find_element_by_accessibility_id("ClickedMenuItem");
        assert_eq!("_Grandchild", clicked_menu_item.text());
    }

    pub fn child_accelerator_key(&self) {
        let root_menu_item = self.session().find_element_by_accessibility_id("RootMenuItem");

        root_menu_item.send_click();

        let child_menu_item = self.session().find_element_by_accessibility_id("Child1MenuItem");

        assert_eq!(Some("Ctrl+O"), child_menu_item.get_attribute("AcceleratorKey").as_deref());
    }

    pub fn pointer_over_does_not_steal_focus(&self) {
        // Issue #7906
        let text_box = self.session().find_element_by_accessibility_id("MenuFocusTest");
        text_box.click();

        assert!(text_box.get_is_focused());

        let root_menu_item = self.session().find_element_by_accessibility_id("RootMenuItem");
        root_menu_item.move_pointer_over();

        assert!(text_box.get_is_focused());
    }

    fn move_pointer_out_of_the_way(&self) {
        // Move the pointer to the menu tab item so that it's not over the menu in preparation
        // for key press tests. This prevents the mouse accidentally selecting the wrong item
        // by hovering.
        let tabs = self.session().find_element_by_accessibility_id("Pager");
        let tab = tabs.find_element_by_name("Menu");
        tab.move_pointer_over();
    }
}

pub fn tests(tests: &mut Tests) {
    for (module, collection) in
        [("menu_tests::default", Collection::Default), ("menu_tests::overlay_popups", Collection::OverlayPopups)]
    {
        tests
            .class(module, collection, MenuTests::new)
            .fact("click_child", MenuTests::click_child)
            .fact("click_grandchild", MenuTests::click_grandchild)
            .platform_fact("select_child_with_alt_arrow_keys", TestPlatforms::WINDOWS, MenuTests::select_child_with_alt_arrow_keys)
            .platform_fact(
                "select_grandchild_with_alt_arrow_keys",
                TestPlatforms::WINDOWS,
                MenuTests::select_grandchild_with_alt_arrow_keys,
            )
            .platform_fact(
                "select_child_with_alt_access_keys",
                TestPlatforms::WINDOWS,
                MenuTests::select_child_with_alt_access_keys,
            )
            .platform_fact(
                "select_grandchild_with_alt_access_keys",
                TestPlatforms::WINDOWS,
                MenuTests::select_grandchild_with_alt_access_keys,
            )
            .platform_fact(
                "select_child_with_click_arrow_keys",
                TestPlatforms::WINDOWS,
                MenuTests::select_child_with_click_arrow_keys,
            )
            .platform_fact(
                "select_grandchild_with_click_arrow_keys",
                TestPlatforms::WINDOWS,
                MenuTests::select_grandchild_with_click_arrow_keys,
            )
            .platform_fact("child_accelerator_key", TestPlatforms::WINDOWS, MenuTests::child_accelerator_key)
            .platform_fact(
                "pointer_over_does_not_steal_focus",
                TestPlatforms::WINDOWS,
                MenuTests::pointer_over_does_not_steal_focus,
            );
    }
}
