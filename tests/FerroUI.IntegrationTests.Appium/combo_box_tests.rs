//! Port of `ComboBoxTests.cs`: the tests run in the application of the
//! default collection (`ComboBoxTests.Default`) and in the one with overlay
//! popups (`ComboBoxTests.OverlayPopups`).

use crate::infrastructure::{
    Collection, DefaultAppFixture, Keys, Session, TestBase, TestClass, TestPlatforms, Tests,
};

pub struct ComboBoxTests {
    base: TestBase,
}

impl TestClass for ComboBoxTests {}

impl ComboBoxTests {
    pub fn new(fixture: &DefaultAppFixture) -> Self {
        ComboBoxTests { base: TestBase::new(fixture, "ComboBox") }
    }

    fn session(&self) -> &Session {
        self.base.session()
    }

    pub fn can_change_selection_using_mouse(&self) {
        let combo_box = self.session().find_element_by_accessibility_id("BasicComboBox");

        self.session().find_element_by_accessibility_id("ComboBoxSelectFirst").click();
        assert_eq!("Item 0", combo_box.get_combo_box_value());

        combo_box.click();
        self.session().find_element_by_name("Item 1").send_click();

        assert_eq!("Item 1", combo_box.get_combo_box_value());
    }

    pub fn can_change_selection_from_unselected_using_mouse(&self) {
        let combo_box = self.session().find_element_by_accessibility_id("BasicComboBox");

        self.session().find_element_by_accessibility_id("ComboBoxSelectionClear").click();
        assert_eq!("", combo_box.get_combo_box_value());

        combo_box.click();
        self.session().find_element_by_name("Item 0").send_click();

        assert_eq!("Item 0", combo_box.get_combo_box_value());
    }

    pub fn can_change_selection_with_keyboard_when_closed(&self) {
        let combo_box = self.session().find_element_by_accessibility_id("BasicComboBox");
        let wrap = self.session().find_element_by_accessibility_id("ComboBoxWrapSelection");

        if wrap.get_is_checked() != Some(false) {
            wrap.click();
        }

        self.session().find_element_by_accessibility_id("ComboBoxSelectionClear").click();

        combo_box.send_keys(Keys::ARROW_DOWN);
        assert_eq!("Item 0", combo_box.get_combo_box_value());

        combo_box.send_keys(Keys::ARROW_DOWN);
        assert_eq!("Item 1", combo_box.get_combo_box_value());

        combo_box.send_keys(Keys::ARROW_DOWN);
        assert_eq!("Item 1", combo_box.get_combo_box_value());

        combo_box.send_keys(Keys::ARROW_UP);
        assert_eq!("Item 0", combo_box.get_combo_box_value());

        combo_box.send_keys(Keys::ARROW_UP);
        assert_eq!("Item 0", combo_box.get_combo_box_value());
    }

    pub fn can_change_wrapping_selection_with_keyboard_when_closed(&self) {
        let combo_box = self.session().find_element_by_accessibility_id("BasicComboBox");
        let wrap = self.session().find_element_by_accessibility_id("ComboBoxWrapSelection");

        if wrap.get_is_checked() != Some(true) {
            wrap.click();
        }

        self.session().find_element_by_accessibility_id("ComboBoxSelectionClear").click();

        combo_box.send_keys(Keys::ARROW_DOWN);
        assert_eq!("Item 0", combo_box.get_combo_box_value());

        combo_box.send_keys(Keys::ARROW_DOWN);
        assert_eq!("Item 1", combo_box.get_combo_box_value());

        combo_box.send_keys(Keys::ARROW_DOWN);
        assert_eq!("Item 0", combo_box.get_combo_box_value());

        combo_box.send_keys(Keys::ARROW_DOWN);
        assert_eq!("Item 1", combo_box.get_combo_box_value());

        combo_box.send_keys(Keys::ARROW_UP);
        assert_eq!("Item 0", combo_box.get_combo_box_value());

        combo_box.send_keys(Keys::ARROW_UP);
        assert_eq!("Item 1", combo_box.get_combo_box_value());
    }

    pub fn can_change_selection_when_open_with_keyboard(&self) {
        let combo_box = self.session().find_element_by_accessibility_id("BasicComboBox");

        self.session().find_element_by_accessibility_id("ComboBoxSelectFirst").click();
        assert_eq!("Item 0", combo_box.get_combo_box_value());

        combo_box.send_keys(&format!("{}{}", Keys::LEFT_ALT, Keys::ARROW_DOWN));
        combo_box.send_keys(Keys::ARROW_DOWN);

        let item = self.session().find_element_by_name("Item 1");
        item.send_keys(Keys::ENTER);

        assert_eq!("Item 1", combo_box.get_combo_box_value());
    }

    pub fn can_change_selection_when_open_with_keyboard_from_unselected(&self) {
        let combo_box = self.session().find_element_by_accessibility_id("BasicComboBox");

        self.session().find_element_by_accessibility_id("ComboBoxSelectionClear").click();
        assert_eq!("", combo_box.get_combo_box_value());

        combo_box.send_keys(&format!("{}{}", Keys::LEFT_ALT, Keys::ARROW_DOWN));
        combo_box.send_keys(Keys::ARROW_DOWN);

        let item = self.session().find_element_by_name("Item 0");
        item.send_keys(Keys::ENTER);

        assert_eq!("Item 0", combo_box.get_combo_box_value());
    }

    pub fn can_cancel_keyboard_selection_with_escape(&self) {
        let combo_box = self.session().find_element_by_accessibility_id("BasicComboBox");

        self.session().find_element_by_accessibility_id("ComboBoxSelectionClear").click();
        assert_eq!("", combo_box.get_combo_box_value());

        combo_box.send_keys(&format!("{}{}", Keys::LEFT_ALT, Keys::ARROW_DOWN));
        combo_box.send_keys(Keys::ARROW_DOWN);

        let item = self.session().find_element_by_name("Item 0");
        item.send_keys(Keys::ESCAPE);

        assert_eq!("", combo_box.get_combo_box_value());
    }
}

pub fn tests(tests: &mut Tests) {
    for (module, collection) in [
        ("combo_box_tests::default", Collection::Default),
        ("combo_box_tests::overlay_popups", Collection::OverlayPopups),
    ] {
        tests
            .class(module, collection, ComboBoxTests::new)
            .fact("can_change_selection_using_mouse", ComboBoxTests::can_change_selection_using_mouse)
            .fact(
                "can_change_selection_from_unselected_using_mouse",
                ComboBoxTests::can_change_selection_from_unselected_using_mouse,
            )
            .platform_fact(
                "can_change_selection_with_keyboard_when_closed",
                TestPlatforms::WINDOWS,
                ComboBoxTests::can_change_selection_with_keyboard_when_closed,
            )
            .platform_fact(
                "can_change_wrapping_selection_with_keyboard_when_closed",
                TestPlatforms::WINDOWS,
                ComboBoxTests::can_change_wrapping_selection_with_keyboard_when_closed,
            )
            .platform_fact(
                "can_change_selection_when_open_with_keyboard",
                TestPlatforms::WINDOWS,
                ComboBoxTests::can_change_selection_when_open_with_keyboard,
            )
            .platform_fact(
                "can_change_selection_when_open_with_keyboard_from_unselected",
                TestPlatforms::WINDOWS,
                ComboBoxTests::can_change_selection_when_open_with_keyboard_from_unselected,
            )
            .platform_fact(
                "can_cancel_keyboard_selection_with_escape",
                TestPlatforms::WINDOWS,
                ComboBoxTests::can_cancel_keyboard_selection_with_escape,
            );
    }
}
