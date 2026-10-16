//! Port of `ListBoxTests.cs`.

use crate::infrastructure::{
    Actions, Collection, DefaultAppFixture, Element, Keys, Session, TestBase, TestClass, TestPlatforms, Tests,
};

pub struct ListBoxTests {
    base: TestBase,
}

impl TestClass for ListBoxTests {}

impl ListBoxTests {
    pub fn new(fixture: &DefaultAppFixture) -> Self {
        ListBoxTests { base: TestBase::new(fixture, "ListBox") }
    }

    fn session(&self) -> &Session {
        self.base.session()
    }

    pub fn can_select_item_by_clicking(&self) {
        let list_box = self.get_target();
        let item2 = list_box.find_element_by_name("Item 2");
        let item4 = list_box.find_element_by_name("Item 4");

        assert!(!item2.selected());
        assert!(!item4.selected());

        item2.send_click();
        assert!(item2.selected());
        assert!(!item4.selected());

        item4.send_click();
        assert!(!item2.selected());
        assert!(item4.selected());
    }

    pub fn can_select_items_by_ctrl_clicking(&self) {
        let list_box = self.get_target();
        let item2 = list_box.find_element_by_name("Item 2");
        let item4 = list_box.find_element_by_name("Item 4");

        assert!(!item2.selected());
        assert!(!item4.selected());

        Actions::new(self.session())
            .click_on(&item2)
            .key_down(Keys::CONTROL)
            .click_on(&item4)
            .key_up(Keys::CONTROL)
            .perform();

        assert!(item2.selected());
        assert!(item4.selected());
    }

    // appium-mac2-driver just hangs
    pub fn can_select_range_by_shift_clicking(&self) {
        let list_box = self.get_target();
        let item2 = list_box.find_element_by_name("Item 2");
        let item3 = list_box.find_element_by_name("Item 3");
        let item4 = list_box.find_element_by_name("Item 4");

        assert!(!item2.selected());
        assert!(!item3.selected());
        assert!(!item4.selected());

        Actions::new(self.session())
            .click_on(&item2)
            .key_down(Keys::SHIFT)
            .click_on(&item4)
            .key_up(Keys::SHIFT)
            .perform();

        assert!(item2.selected());
        assert!(item3.selected());
        assert!(item4.selected());
    }

    pub fn is_virtualized(&self) {
        let list_box = self.get_target();
        let children = list_box.get_children();

        assert!(children.len() < 100);
    }

    fn get_target(&self) -> Element {
        self.session().find_element_by_accessibility_id("ListBoxSelectionClear").click();
        self.session().find_element_by_accessibility_id("BasicListBox")
    }
}

pub fn tests(tests: &mut Tests) {
    tests
        .class("list_box_tests", Collection::Default, ListBoxTests::new)
        .fact("can_select_item_by_clicking", ListBoxTests::can_select_item_by_clicking)
        .add(
            "can_select_items_by_ctrl_clicking",
            TestPlatforms::ALL,
            Some("WinAppDriver seems unable to consistently send a Ctrl key and appium-mac2-driver just hangs"),
            ListBoxTests::can_select_items_by_ctrl_clicking,
        )
        .platform_fact("can_select_range_by_shift_clicking", TestPlatforms::WINDOWS, ListBoxTests::can_select_range_by_shift_clicking)
        .fact("is_virtualized", ListBoxTests::is_virtualized);
}
