//! Port of `EmbeddingTests.cs`.

use crate::infrastructure::{Actions, Collection, DefaultAppFixture, Element, Keys, Session, TestBase, TestClass, TestPlatforms, Tests};
use std::time::Duration;

pub struct EmbeddingTests {
    base: TestBase,
}

impl TestClass for EmbeddingTests {}

impl EmbeddingTests {
    pub fn new(fixture: &DefaultAppFixture) -> Self {
        let base = TestBase::new(fixture, "Embedding");
        let reset = base.session().find_element_by_accessibility_id("Reset");
        reset.click();
        EmbeddingTests { base }
    }

    fn session(&self) -> &Session {
        self.base.session()
    }

    /// The text box inside the host of the native control: the drivers have
    /// different XPath syntax between Windows and macOS.
    fn native_text_box(&self, id: &str) -> Element {
        if cfg!(windows) {
            self.session().find_element_by_xpath(&format!("//*[@AutomationId='{id}']//*[1]"))
        } else {
            self.session().find_element_by_xpath(&format!("//*[@identifier='{id}']//*[1]"))
        }
    }

    pub fn can_edit_native_text_box(&self) {
        // Appium has different XPath syntax between Windows and macOS.
        let text_box = self.native_text_box("NativeTextBox");

        assert_eq!("Native text box", text_box.text());

        text_box.send_keys("Hello world!");

        // SendKeys behaves differently between Windows and macOS. On Windows it inserts at the start
        // of the text box, on macOS it replaces the text for some reason. Sigh.
        let expected = if cfg!(windows) { "Hello world!Native text box" } else { "Hello world!" };
        assert_eq!(expected, text_box.text());
    }

    pub fn can_edit_native_text_box_in_popup(&self) {
        let check_box = self.session().find_element_by_accessibility_id("EmbeddingPopupOpenCheckBox");
        check_box.click();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // Appium has different XPath syntax between Windows and macOS.
            let text_box = self.native_text_box("NativeTextBoxInPopup");

            assert_eq!("Native text box", text_box.text());

            text_box.send_keys("Hello world!");

            // SendKeys behaves differently between Windows and macOS. On Windows it inserts at the start
            // of the text box, on macOS it replaces the text for some reason. Sigh.
            let expected = if cfg!(windows) { "Hello world!Native text box" } else { "Hello world!" };
            assert_eq!(expected, text_box.text());
        }));

        // finally
        check_box.click();
        if let Err(panic) = result {
            std::panic::resume_unwind(panic);
        }
    }

    pub fn showing_tool_tip_does_not_steal_focus_from_native_text_box(&self) {
        // Appium has different XPath syntax between Windows and macOS.
        let text_box = self.native_text_box("NativeTextBox");

        // Clicking on the text box causes the cursor to hover over it, opening the tooltip.
        text_box.click();
        std::thread::sleep(Duration::from_millis(1000));

        // Ensure the tooltip has opened.
        self.session().find_element_by_accessibility_id("NativeTextBoxToolTip");

        // The tooltip should not have stolen focus from the text box, so text entry should work.
        Actions::new(self.session()).send_keys("Hello world!").perform();

        // SendKeys behaves differently between Windows and macOS. On Windows it inserts at the start
        // of the text box, on macOS it replaces the text for some reason. Sigh.
        let expected = if cfg!(windows) { "Native text boxHello world!" } else { "Hello world!" };

        assert_eq!(expected, text_box.text());
    }

    pub fn showing_context_menu_steals_focus_from_native_text_box(&self) {
        // Appium has different XPath syntax between Windows and macOS.
        let text_box = self.native_text_box("NativeTextBox");

        // Click on the text box the right-click to show the context menu.
        text_box.click();
        Actions::new(self.session()).context_click_on(&text_box).perform();

        // Ensure the context menu has opened.
        self.session().find_element_by_accessibility_id("NativeTextBoxContextMenu");

        // Select the first menu item with the keyboard.
        Actions::new(self.session()).send_keys(Keys::ARROW_DOWN).send_keys(Keys::ENTER).perform();

        assert_eq!("Context menu item clicked", text_box.text());
    }
}

pub fn tests(tests: &mut Tests) {
    const NOT_ON_MAC_OS: &str = "Not yet working on macOS";
    tests
        .class("embedding_tests", Collection::Default, EmbeddingTests::new)
        .platform_fact_because(
            "can_edit_native_text_box",
            TestPlatforms::WINDOWS,
            NOT_ON_MAC_OS,
            EmbeddingTests::can_edit_native_text_box,
        )
        .platform_fact_because(
            "can_edit_native_text_box_in_popup",
            TestPlatforms::WINDOWS,
            NOT_ON_MAC_OS,
            EmbeddingTests::can_edit_native_text_box_in_popup,
        )
        .platform_fact_because(
            "showing_tool_tip_does_not_steal_focus_from_native_text_box",
            TestPlatforms::WINDOWS,
            NOT_ON_MAC_OS,
            EmbeddingTests::showing_tool_tip_does_not_steal_focus_from_native_text_box,
        )
        .platform_fact_because(
            "showing_context_menu_steals_focus_from_native_text_box",
            TestPlatforms::WINDOWS,
            NOT_ON_MAC_OS,
            EmbeddingTests::showing_context_menu_steals_focus_from_native_text_box,
        );
}
