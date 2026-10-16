//! Port of `ContextMenuTests.cs`.

use crate::infrastructure::{Actions, Collection, DefaultAppFixture, Keys, Session, TestBase, TestClass, TestPlatforms, Tests};

pub struct ContextMenuTests {
    base: TestBase,
}

impl TestClass for ContextMenuTests {}

impl ContextMenuTests {
    pub fn new(fixture: &DefaultAppFixture) -> Self {
        ContextMenuTests { base: TestBase::new(fixture, "ContextMenu") }
    }

    fn session(&self) -> &Session {
        self.base.session()
    }

    pub fn select_first_item_with_down_arrow_key(&self) {
        let control = self.session().find_element_by_accessibility_id("ShowContextMenu");

        Actions::new(self.session()).context_click_on(&control).send_keys(Keys::ARROW_DOWN).perform();

        let clicked_menu_item = self.session().find_element_by_accessibility_id("ContextMenuItem1");
        assert!(clicked_menu_item.get_is_focused());
    }
}

pub fn tests(tests: &mut Tests) {
    tests.class("context_menu_tests", Collection::Default, ContextMenuTests::new).platform_fact(
        "select_first_item_with_down_arrow_key",
        TestPlatforms::WINDOWS,
        ContextMenuTests::select_first_item_with_down_arrow_key,
    );
}
