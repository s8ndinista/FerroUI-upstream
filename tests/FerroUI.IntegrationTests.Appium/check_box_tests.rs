//! Port of `CheckBoxTests.cs`.

use crate::infrastructure::{Collection, DefaultAppFixture, Session, TestBase, TestClass, Tests};

pub struct CheckBoxTests {
    base: TestBase,
}

impl TestClass for CheckBoxTests {}

impl CheckBoxTests {
    pub fn new(fixture: &DefaultAppFixture) -> Self {
        CheckBoxTests { base: TestBase::new(fixture, "CheckBox") }
    }

    fn session(&self) -> &Session {
        self.base.session()
    }

    pub fn unchecked_check_box(&self) {
        let check_box = self.session().find_element_by_accessibility_id("UncheckedCheckBox");

        assert_eq!("Unchecked", check_box.get_name());
        assert_eq!(Some(false), check_box.get_is_checked());

        check_box.click();
        assert_eq!(Some(true), check_box.get_is_checked());
    }

    pub fn checked_check_box(&self) {
        let check_box = self.session().find_element_by_accessibility_id("CheckedCheckBox");

        assert_eq!("Checked", check_box.get_name());
        assert_eq!(Some(true), check_box.get_is_checked());

        check_box.click();
        assert_eq!(Some(false), check_box.get_is_checked());
    }

    pub fn three_state_check_box(&self) {
        let check_box = self.session().find_element_by_accessibility_id("ThreeStateCheckBox");

        assert_eq!("ThreeState", check_box.get_name());
        assert_eq!(None, check_box.get_is_checked());

        check_box.click();
        assert_eq!(Some(false), check_box.get_is_checked());

        check_box.click();
        assert_eq!(Some(true), check_box.get_is_checked());

        check_box.click();
        assert_eq!(None, check_box.get_is_checked());
    }
}

pub fn tests(tests: &mut Tests) {
    tests
        .class("check_box_tests", Collection::Default, CheckBoxTests::new)
        .fact("unchecked_check_box", CheckBoxTests::unchecked_check_box)
        .fact("checked_check_box", CheckBoxTests::checked_check_box)
        .fact("three_state_check_box", CheckBoxTests::three_state_check_box);
}
