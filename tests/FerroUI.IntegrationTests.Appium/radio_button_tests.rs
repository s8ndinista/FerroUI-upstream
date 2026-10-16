//! Port of `RadioButtonTests.cs`.

use crate::infrastructure::{Collection, DefaultAppFixture, Session, TestBase, TestClass, Tests};

pub struct RadioButtonTests {
    base: TestBase,
}

impl TestClass for RadioButtonTests {}

impl RadioButtonTests {
    pub fn new(fixture: &DefaultAppFixture) -> Self {
        RadioButtonTests { base: TestBase::new(fixture, "RadioButton") }
    }

    fn session(&self) -> &Session {
        self.base.session()
    }

    pub fn radio_button_is_checked_true_when_clicked(&self) {
        let button = self.session().find_element_by_accessibility_id("BasicRadioButton");
        assert_eq!(Some(false), button.get_is_checked());
        button.click();
        assert_eq!(Some(true), button.get_is_checked());
    }

    pub fn three_state_radio_button_is_checked_false_when_other_three_state_radio_button_checked(&self) {
        let button1 = self.session().find_element_by_accessibility_id("ThreeStatesRadioButton1");
        let button2 = self.session().find_element_by_accessibility_id("ThreeStatesRadioButton2");
        assert_eq!(Some(true), button1.get_is_checked());
        assert_eq!(Some(false), button2.get_is_checked());
        button2.click();
        assert_eq!(Some(false), button1.get_is_checked());
        assert_eq!(Some(true), button2.get_is_checked());
    }
}

pub fn tests(tests: &mut Tests) {
    tests
        .class("radio_button_tests", Collection::Default, RadioButtonTests::new)
        .fact("radio_button_is_checked_true_when_clicked", RadioButtonTests::radio_button_is_checked_true_when_clicked)
        .fact(
            "three_state_radio_button_is_checked_false_when_other_three_state_radio_button_checked",
            RadioButtonTests::three_state_radio_button_is_checked_false_when_other_three_state_radio_button_checked,
        );
}
