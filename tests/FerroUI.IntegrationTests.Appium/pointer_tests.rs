//! Port of `PointerTests.cs`.

use crate::infrastructure::{Collection, DefaultAppFixture, Session, TestBase, TestClass, Tests};

pub struct PointerTests {
    base: TestBase,
}

impl TestClass for PointerTests {}

impl PointerTests {
    pub fn new(fixture: &DefaultAppFixture) -> Self {
        PointerTests { base: TestBase::new(fixture, "Pointer") }
    }

    fn session(&self) -> &Session {
        self.base.session()
    }

    pub fn pointer_capture_is_released_when_showing_dialog(&self) {
        let button = self.session().find_element_by_accessibility_id("PointerPageShowDialog");

        button.open_window_with_click().dispose();

        let status = self.session().find_element_by_accessibility_id("PointerCaptureStatus");
        assert_eq!("None", status.text());
    }
}

pub fn tests(tests: &mut Tests) {
    tests.class("pointer_tests", Collection::Default, PointerTests::new).fact(
        "pointer_capture_is_released_when_showing_dialog",
        PointerTests::pointer_capture_is_released_when_showing_dialog,
    );
}
