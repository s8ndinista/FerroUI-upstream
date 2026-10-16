//! Port of `AutomationTests.cs`.

use crate::infrastructure::{Collection, DefaultAppFixture, Session, TestBase, TestClass, Tests};

pub struct AutomationTests {
    base: TestBase,
}

impl TestClass for AutomationTests {}

impl AutomationTests {
    pub fn new(fixture: &DefaultAppFixture) -> Self {
        AutomationTests { base: TestBase::new(fixture, "Automation") }
    }

    fn session(&self) -> &Session {
        self.base.session()
    }

    pub fn automation_id(&self) {
        // AutomationID can be specified by the Name or AutomationProperties.AutomationId
        // properties, with the latter taking precedence.
        let _by_name = self.session().find_element_by_accessibility_id("TextBlockWithName");
        let _by_automation_id = self.session().find_element_by_accessibility_id("TextBlockWithNameAndAutomationId");
    }

    pub fn labeled_by(&self) {
        let label = self.session().find_element_by_accessibility_id("TextBlockAsLabel");
        let labeled_text_box = self.session().find_element_by_accessibility_id("LabeledByTextBox");

        assert_eq!("Label for TextBox", label.text());
        assert_eq!("Label for TextBox", labeled_text_box.get_name());
    }
}

pub fn tests(tests: &mut Tests) {
    tests
        .class("automation_tests", Collection::Default, AutomationTests::new)
        .fact("automation_id", AutomationTests::automation_id)
        .fact("labeled_by", AutomationTests::labeled_by);
}
