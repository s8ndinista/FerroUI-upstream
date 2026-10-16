//! Port of `PointerTests_MacOS.cs`: a class with helpers and a `Dispose`,
//! and no tests at the tracked commit. Nothing registers it; its helpers are
//! kept so that a test added to it upstream has them.

#![allow(dead_code)]

use crate::infrastructure::{DefaultAppFixture, Element, Session, TestBase, TestClass, Tests};

pub struct PointerTestsMacOS {
    base: TestBase,
}

impl PointerTestsMacOS {
    pub fn new(fixture: &DefaultAppFixture) -> Self {
        PointerTestsMacOS { base: TestBase::new(fixture, "Window Decorations") }
    }

    fn session(&self) -> &Session {
        self.base.session()
    }

    fn set_parameters(&self, extend_client_area: bool, show_title_area_control: bool) {
        let extend_client_area_check_box =
            self.session().find_element_by_accessibility_id("WindowExtendClientAreaToDecorationsHint");
        let show_title_area_control_check_box = self.session().find_element_by_accessibility_id("WindowShowTitleAreaControl");

        if extend_client_area_check_box.get_is_checked() != Some(extend_client_area) {
            extend_client_area_check_box.click();
        }
        if show_title_area_control_check_box.get_is_checked() != Some(show_title_area_control) {
            show_title_area_control_check_box.click();
        }
    }

    fn count(window: &Element, id: &str) -> i32 {
        let text_box = window.find_element_by_accessibility_id(id);
        // `int.Parse(text ?? "0")`: the text of an element is never null here.
        let text = text_box.text();
        text.parse().unwrap_or_else(|error| panic!("{id}: {text:?}: {error}"))
    }

    fn get_move_count(window: &Element) -> i32 {
        Self::count(window, "MouseMoveCount")
    }

    fn get_release_count(window: &Element) -> i32 {
        Self::count(window, "MouseReleaseCount")
    }

    fn get_mouse_down_count(window: &Element) -> i32 {
        Self::count(window, "MouseDownCount")
    }

    fn get_double_click_count(window: &Element) -> i32 {
        Self::count(window, "DoubleClickCount")
    }
}

impl TestClass for PointerTestsMacOS {
    fn dispose(&self) {
        self.set_parameters(false, false);
        let apply_button = self.session().find_element_by_accessibility_id("ApplyWindowDecorations");
        apply_button.click();
    }
}

/// The class has no tests at the tracked commit.
pub fn tests(_tests: &mut Tests) {}
