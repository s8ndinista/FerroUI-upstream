//! Port of `ScrollBarTests.cs`.

use crate::infrastructure::{Collection, DefaultAppFixture, Session, TestBase, TestClass, Tests};

pub struct ScrollBarTests {
    base: TestBase,
}

impl TestClass for ScrollBarTests {}

fn parse_double(text: &str) -> f64 {
    text.parse().unwrap_or_else(|error| panic!("{text:?}: {error}"))
}

impl ScrollBarTests {
    pub fn new(fixture: &DefaultAppFixture) -> Self {
        ScrollBarTests { base: TestBase::new(fixture, "ScrollBar") }
    }

    fn session(&self) -> &Session {
        self.base.session()
    }

    pub fn scroll_bar_increases_value_by_large_change_when_increase_button_is_clicked(&self) {
        let button = self.session().find_element_by_accessibility_id("MyScrollBar");
        assert!(parse_double(&button.text()) == 20.0);

        button.click();

        // Default LargeChange value is 10 so when clicking the IncreaseButton
        // ScrollBar value should be increased by 10.
        assert_eq!(30.0, parse_double(&button.text()));
    }
}

pub fn tests(tests: &mut Tests) {
    tests.class("scroll_bar_tests", Collection::Default, ScrollBarTests::new).fact(
        "scroll_bar_increases_value_by_large_change_when_increase_button_is_clicked",
        ScrollBarTests::scroll_bar_increases_value_by_large_change_when_increase_button_is_clicked,
    );
}
