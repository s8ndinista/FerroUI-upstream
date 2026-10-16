//! Port of `ButtonTests.cs`.

use crate::infrastructure::{Collection, DefaultAppFixture, Session, TestBase, TestClass, TestPlatforms, Tests};

pub struct ButtonTests {
    base: TestBase,
}

impl TestClass for ButtonTests {}

impl ButtonTests {
    pub fn new(fixture: &DefaultAppFixture) -> Self {
        ButtonTests { base: TestBase::new(fixture, "Button") }
    }

    fn session(&self) -> &Session {
        self.base.session()
    }

    pub fn disabled_button(&self) {
        let button = self.session().find_element_by_accessibility_id("DisabledButton");

        assert_eq!("Disabled Button", button.text());
        assert!(!button.enabled());
    }

    pub fn effectively_disabled_button(&self) {
        let button = self.session().find_element_by_accessibility_id("EffectivelyDisabledButton");

        assert_eq!("Effectively Disabled Button", button.text());
        assert!(!button.enabled());
    }

    pub fn basic_button(&self) {
        let button = self.session().find_element_by_accessibility_id("BasicButton");

        assert_eq!("Basic Button", button.text());
        assert!(button.enabled());
    }

    pub fn button_with_text_block(&self) {
        let button = self.session().find_element_by_accessibility_id("ButtonWithTextBlock");

        assert_eq!("Button with TextBlock", button.text());
    }

    pub fn button_with_accelerator_key(&self) {
        let button = self.session().find_element_by_accessibility_id("ButtonWithAcceleratorKey");

        assert_eq!(Some("Ctrl+B"), button.get_attribute("AcceleratorKey").as_deref());
    }
}

pub fn tests(tests: &mut Tests) {
    tests
        .class("button_tests", Collection::Default, ButtonTests::new)
        .fact("disabled_button", ButtonTests::disabled_button)
        .fact("effectively_disabled_button", ButtonTests::effectively_disabled_button)
        .fact("basic_button", ButtonTests::basic_button)
        .fact("button_with_text_block", ButtonTests::button_with_text_block)
        .platform_fact("button_with_accelerator_key", TestPlatforms::WINDOWS, ButtonTests::button_with_accelerator_key);
}
