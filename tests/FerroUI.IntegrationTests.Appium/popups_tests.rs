//! Port of `PopupsTests.cs`: the tests run in the application of the default
//! collection (`PopupsTests.Default`) and in the one with overlay popups
//! (`PopupsTests.OverlayPopups`).

use crate::infrastructure::{Collection, DefaultAppFixture, Session, TestBase, TestClass, TestPlatforms, Tests};
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::time::Duration;

pub struct PopupsTests {
    base: TestBase,
    is_overlay_popups: bool,
}

impl TestClass for PopupsTests {}

impl PopupsTests {
    pub fn new(is_overlay_popups: bool, fixture: &DefaultAppFixture) -> Self {
        PopupsTests { base: TestBase::new(fixture, "Popups"), is_overlay_popups }
    }

    fn session(&self) -> &Session {
        self.base.session()
    }

    pub fn light_dismiss_popup_should_open_and_close(&self) {
        // Open popup
        let button = self.session().find_element_by_accessibility_id("ShowLightDismissPopup");
        button.click();
        std::thread::sleep(Duration::from_millis(500));

        // Assert - Popup is visible
        self.session().find_element_by_accessibility_id("LightDismissPopupContent");

        // Act - Click outside to dismiss
        let dismiss_border = self.session().find_element_by_accessibility_id("DismissButton");
        dismiss_border.click();

        std::thread::sleep(Duration::from_millis(500));

        // Assert - Popup is closed
        assert!(self.session().try_find_element_by_accessibility_id("LightDismissPopupContent").is_err());
    }

    pub fn stays_open_popup_should_stay_open(&self) {
        // Open popup
        let button = self.session().find_element_by_accessibility_id("ShowStaysOpenPopup");
        button.click();
        std::thread::sleep(Duration::from_millis(500));

        let result = catch_unwind(AssertUnwindSafe(|| {
            // Assert - Popup is visible
            self.session().find_element_by_accessibility_id("StaysOpenPopupCloseButton");

            // Act - Click outside
            let dismiss_border = self.session().find_element_by_accessibility_id("DismissButton");
            dismiss_border.click();

            std::thread::sleep(Duration::from_millis(500));

            // Assert - Popup is still visible
            self.session().find_element_by_accessibility_id("StaysOpenPopupCloseButton");
        }));

        // finally
        // Act - Close popup with button
        self.session().find_element_by_accessibility_id("StaysOpenPopupCloseButton").click();

        std::thread::sleep(Duration::from_millis(400));

        // Assert - Popup is closed
        assert!(self.session().try_find_element_by_accessibility_id("StaysOpenPopupCloseButton").is_err());
        if let Err(panic) = result {
            resume_unwind(panic);
        }
    }

    pub fn stays_open_popup_text_box_should_be_editable(&self) {
        // Open popup
        let button = self.session().find_element_by_accessibility_id("ShowStaysOpenPopup");
        button.click();
        std::thread::sleep(Duration::from_millis(500));

        let result = catch_unwind(AssertUnwindSafe(|| {
            // Find and edit the TextBox
            let text_box = self.session().find_element_by_accessibility_id("StaysOpenTextBox");
            text_box.clear();
            text_box.send_keys("New text value");
            std::thread::sleep(Duration::from_millis(500));

            // Verify text was changed
            assert_eq!("New text value", text_box.text());
        }));

        // finally
        // Cleanup - close popup
        self.session().find_element_by_accessibility_id("StaysOpenPopupCloseButton").click();
        if let Err(panic) = result {
            resume_unwind(panic);
        }
    }

    pub fn top_most_popup_should_stay_above_other_windows(&self) {
        // It's not possible to test overlay topmost with other windows.
        if self.is_overlay_popups {
            return;
        }

        let stays_open_popup = self.session().find_element_by_accessibility_id("ShowTopMostPopup");
        let main_window_handle = self.session().current_window_handle();

        // Show topmost popup.
        stays_open_popup.click();
        self.session().find_element_by_accessibility_id("TopMostPopupCloseButton");

        let mut has_closed_popup = false;

        let result = catch_unwind(AssertUnwindSafe(|| {
            // Open a child window.
            let _window = self.session().find_element_by_accessibility_id("OpenNewWindowButton").open_window_with_click();
            std::thread::sleep(Duration::from_millis(500));

            // Force window to front by maximizing child window.
            self.session().find_element_by_accessibility_id("CurrentWindowState").send_click();
            self.session().find_element_by_accessibility_id("WindowStateMaximized").send_click();

            // Switch back to the mainwindow context and verify tooltip is still accessible.
            self.session().switch_to_window(&main_window_handle);
            self.session().find_element_by_accessibility_id("TopMostPopupCloseButton");

            // Verify we can still interact with the popup by closing it via button.
            self.session().find_element_by_accessibility_id("TopMostPopupCloseButton").click();

            // Verify popup closed
            assert!(self.session().try_find_element_by_accessibility_id("TopMostPopupCloseButton").is_err());
            has_closed_popup = true;
        }));

        // finally
        if !has_closed_popup {
            self.session().find_element_by_accessibility_id("TopMostPopupCloseButton").click();
        }
        if let Err(panic) = result {
            resume_unwind(panic);
        }
    }

    pub fn non_top_most_popup_does_not_stay_above_other_windows(&self) {
        let topmost_button = self.session().find_element_by_accessibility_id("ShowStaysOpenPopup");
        let main_window_handle = self.session().current_window_handle();

        // Show topmost popup.
        topmost_button.click();
        self.session().find_element_by_accessibility_id("StaysOpenPopupCloseButton");

        let result = catch_unwind(AssertUnwindSafe(|| {
            // Open a child window.
            let new_window_button = self.session().find_element_by_accessibility_id("OpenNewWindowButton");
            let _window = new_window_button.open_window_with_click();

            // Force window to front by maximizing child window.
            self.session().find_element_by_accessibility_id("CurrentWindowState").send_click();
            self.session().find_element_by_accessibility_id("WindowStateMaximized").send_click();

            // Switch back to the mainwindow context and verify tooltip is still accessible.
            self.session().switch_to_window(&main_window_handle);
            self.session().find_element_by_accessibility_id("StaysOpenPopupCloseButton");

            // Verify we cannot interact with the popup by attempting closing it via button.
            self.session().find_element_by_accessibility_id("StaysOpenPopupCloseButton").click();

            // Verify popup is still accessible.
            self.session().find_element_by_accessibility_id("StaysOpenPopupCloseButton");
        }));

        // finally
        // At this point secondary window should be already closed. And safe to close the popup.
        self.session().find_element_by_accessibility_id("StaysOpenPopupCloseButton").click();
        if let Err(panic) = result {
            resume_unwind(panic);
        }
    }
}

pub fn tests(tests: &mut Tests) {
    for (module, collection, is_overlay_popups) in [
        ("popups_tests::default", Collection::Default, false),
        ("popups_tests::overlay_popups", Collection::OverlayPopups, true),
    ] {
        tests
            .class(module, collection, move |fixture| PopupsTests::new(is_overlay_popups, fixture))
            .platform_fact(
                "light_dismiss_popup_should_open_and_close",
                TestPlatforms::WINDOWS,
                PopupsTests::light_dismiss_popup_should_open_and_close,
            )
            .platform_fact(
                "stays_open_popup_should_stay_open",
                TestPlatforms::WINDOWS,
                PopupsTests::stays_open_popup_should_stay_open,
            )
            .platform_fact(
                "stays_open_popup_text_box_should_be_editable",
                TestPlatforms::WINDOWS,
                PopupsTests::stays_open_popup_text_box_should_be_editable,
            )
            .platform_fact(
                "top_most_popup_should_stay_above_other_windows",
                TestPlatforms::WINDOWS,
                PopupsTests::top_most_popup_should_stay_above_other_windows,
            )
            .platform_fact(
                "non_top_most_popup_does_not_stay_above_other_windows",
                TestPlatforms::WINDOWS,
                PopupsTests::non_top_most_popup_does_not_stay_above_other_windows,
            );
    }
}
