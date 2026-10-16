//! Port of `KeyboardTests.cs`.
//!
//! Upstream builds these tests as `[Fact]` with Appium 2 and as Windows-only
//! with Appium 1, whose Mac2 driver does not implement the key actions of the
//! protocol (`RequiresW3CKeyActions`). The harness of the port posts the keys
//! itself on both platforms, so the tests are the `[Fact]` of the Appium 2
//! configuration.

use crate::infrastructure::{Actions, Collection, DefaultAppFixture, Keys, Session, TestBase, TestClass, Tests};

pub struct KeyboardTests {
    base: TestBase,
}

impl TestClass for KeyboardTests {}

impl KeyboardTests {
    pub fn new(fixture: &DefaultAppFixture) -> Self {
        let base = TestBase::new(fixture, "Keyboard");
        let reset = base.session().find_element_by_accessibility_id("ResetKeyboard");
        reset.click();
        KeyboardTests { base }
    }

    fn session(&self) -> &Session {
        self.base.session()
    }

    pub fn key_binding_without_modifier_is_raised_while_text_box_is_focused(&self) {
        let text_box = self.session().find_element_by_accessibility_id("GestureTextBox");
        let last_key_binding = self.session().find_element_by_accessibility_id("LastKeyBinding");

        text_box.click();
        Actions::new(self.session()).send_keys(Keys::SPACE).perform();

        assert_eq!("Space", last_key_binding.text());
    }

    pub fn key_binding_on_character_key_is_raised_while_text_box_is_focused(&self) {
        let text_box = self.session().find_element_by_accessibility_id("GestureTextBox");
        let last_key_binding = self.session().find_element_by_accessibility_id("LastKeyBinding");

        text_box.click();
        Actions::new(self.session()).send_keys("a").perform();

        assert_eq!("A", last_key_binding.text());
    }

    pub fn key_binding_with_modifier_is_raised_while_text_box_is_focused(&self) {
        let text_box = self.session().find_element_by_accessibility_id("GestureTextBox");
        let last_key_binding = self.session().find_element_by_accessibility_id("LastKeyBinding");

        text_box.click();
        Actions::new(self.session()).key_down(Keys::CONTROL).send_keys("g").key_up(Keys::CONTROL).perform();

        assert_eq!("Ctrl+G", last_key_binding.text());
    }

    pub fn handled_key_binding_does_not_insert_text(&self) {
        let text_box = self.session().find_element_by_accessibility_id("GestureTextBox");
        let content = self.session().find_element_by_accessibility_id("GestureTextBoxContent");

        text_box.click();
        Actions::new(self.session()).send_keys(Keys::SPACE).perform();

        // A matched KeyBinding marks the key event as handled, which must prevent the
        // platform from producing text for it.
        assert_eq!("", content.text());
    }

    pub fn unhandled_key_still_produces_text(&self) {
        let text_box = self.session().find_element_by_accessibility_id("KeyDownTextBox");
        let last_key_down = self.session().find_element_by_accessibility_id("LastKeyDown");
        let last_text_input = self.session().find_element_by_accessibility_id("LastTextInput");

        text_box.click();
        Actions::new(self.session()).send_keys("b").perform();

        assert_eq!("b", text_box.text());
        assert_eq!("B|B|[b]", last_key_down.text());
        assert_eq!("[b]", last_text_input.text());
    }

    pub fn unhandled_space_produces_exactly_one_key_down_and_text(&self) {
        let text_box = self.session().find_element_by_accessibility_id("KeyDownTextBox");
        let last_key_down = self.session().find_element_by_accessibility_id("LastKeyDown");
        let key_down_count = self.session().find_element_by_accessibility_id("KeyDownCount");
        let last_text_input = self.session().find_element_by_accessibility_id("LastTextInput");

        text_box.click();
        Actions::new(self.session()).send_keys(Keys::SPACE).perform();

        // The input context must not produce a second key down for the same NSEvent.
        assert_eq!("1", key_down_count.text());
        assert_eq!("Space|Space|[ ]", last_key_down.text());
        assert_eq!("[ ]", last_text_input.text());
    }
}

pub fn tests(tests: &mut Tests) {
    tests
        .class("keyboard_tests", Collection::Default, KeyboardTests::new)
        .fact(
            "key_binding_without_modifier_is_raised_while_text_box_is_focused",
            KeyboardTests::key_binding_without_modifier_is_raised_while_text_box_is_focused,
        )
        .fact(
            "key_binding_on_character_key_is_raised_while_text_box_is_focused",
            KeyboardTests::key_binding_on_character_key_is_raised_while_text_box_is_focused,
        )
        .fact(
            "key_binding_with_modifier_is_raised_while_text_box_is_focused",
            KeyboardTests::key_binding_with_modifier_is_raised_while_text_box_is_focused,
        )
        .fact("handled_key_binding_does_not_insert_text", KeyboardTests::handled_key_binding_does_not_insert_text)
        .fact("unhandled_key_still_produces_text", KeyboardTests::unhandled_key_still_produces_text)
        .fact(
            "unhandled_space_produces_exactly_one_key_down_and_text",
            KeyboardTests::unhandled_space_produces_exactly_one_key_down_and_text,
        );
}
