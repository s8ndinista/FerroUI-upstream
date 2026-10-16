//! Port of `GestureTests.cs`.

use crate::infrastructure::{
    ActionBuilder, Actions, Collection, CoordinateOrigin, DefaultAppFixture, MouseButton, PointerInputDevice, Session,
    TestBase, TestClass, TestPlatforms, Tests,
};
use std::time::Duration;

pub struct GestureTests {
    base: TestBase,
}

impl TestClass for GestureTests {}

impl GestureTests {
    pub fn new(fixture: &DefaultAppFixture) -> Self {
        let base = TestBase::new(fixture, "Gestures");
        let clear = base.session().find_element_by_accessibility_id("ResetGestures");
        clear.click();
        GestureTests { base }
    }

    fn session(&self) -> &Session {
        self.base.session()
    }

    pub fn tapped_is_raised(&self) {
        let border = self.session().find_element_by_accessibility_id("GestureBorder");
        let last_gesture = self.session().find_element_by_accessibility_id("LastGesture");

        Actions::new(self.session()).click_on(&border).perform();

        assert_eq!("Tapped", last_gesture.text());
    }

    pub fn tapped_is_raised_slow(&self) {
        let border = self.session().find_element_by_accessibility_id("GestureBorder");
        let last_gesture = self.session().find_element_by_accessibility_id("LastGesture");

        Actions::new(self.session()).click_and_hold_on(&border).perform();

        std::thread::sleep(Duration::from_millis(2000));

        Actions::new(self.session()).release_on(&border).perform();

        assert_eq!("Tapped", last_gesture.text());
    }

    pub fn tapped_is_not_raised_for_drag(&self) {
        let border = self.session().find_element_by_accessibility_id("GestureBorder");
        let last_gesture = self.session().find_element_by_accessibility_id("LastGesture");

        Actions::new(self.session()).click_and_hold_on(&border).move_by_offset(50, 50).release().perform();

        assert_eq!("", last_gesture.text());
    }

    pub fn double_tapped_is_raised(&self) {
        let border = self.session().find_element_by_accessibility_id("GestureBorder");
        let last_gesture = self.session().find_element_by_accessibility_id("LastGesture");

        Actions::new(self.session()).double_click_on(&border).perform();

        assert_eq!("DoubleTapped", last_gesture.text());
    }

    pub fn double_tapped_is_raised_2(&self) {
        let border = self.session().find_element_by_accessibility_id("GestureBorder");
        let last_gesture = self.session().find_element_by_accessibility_id("LastGesture");

        Actions::new(self.session()).click_and_hold_on(&border).release().perform();

        std::thread::sleep(Duration::from_millis(50));

        // DoubleTapped is raised on second pointer press, not release.
        Actions::new(self.session()).click_and_hold_on(&border).perform();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            assert_eq!("DoubleTapped", last_gesture.text());
        }));

        // finally
        Actions::new(self.session()).move_to_element(&last_gesture).release().perform();
        if let Err(panic) = result {
            std::panic::resume_unwind(panic);
        }
    }

    pub fn double_tapped_is_raised_not_raised_if_too_slow(&self) {
        let border = self.session().find_element_by_accessibility_id("GestureBorder");
        let last_gesture = self.session().find_element_by_accessibility_id("LastGesture");

        Actions::new(self.session()).click_and_hold_on(&border).release().perform();

        std::thread::sleep(Duration::from_millis(2000));

        Actions::new(self.session()).click_and_hold_on(&border).release().perform();

        assert_eq!("Tapped", last_gesture.text());
    }

    pub fn double_tapped_is_raised_after_control_changes(&self) {
        // #8733
        let border = self.session().find_element_by_accessibility_id("GestureBorder");
        let last_gesture = self.session().find_element_by_accessibility_id("LastGesture");

        Actions::new(self.session()).move_to_element(&border).double_click().perform();

        std::thread::sleep(Duration::from_millis(100));

        Actions::new(self.session()).move_to_element_offset(&last_gesture, 200, 200).double_click().perform();

        assert_eq!("DoubleTapped2", last_gesture.text());
    }

    pub fn right_tapped_is_raised(&self) {
        let border = self.session().find_element_by_accessibility_id("GestureBorder");
        let last_gesture = self.session().find_element_by_accessibility_id("LastGesture");

        Actions::new(self.session()).context_click_on(&border).perform();

        assert_eq!("RightTapped", last_gesture.text());
    }

    pub fn right_tapped_is_raised_2(&self) {
        let border = self.session().find_element_by_accessibility_id("GestureBorder");
        let last_gesture = self.session().find_element_by_accessibility_id("LastGesture");
        let device = PointerInputDevice::mouse();
        let mut b = ActionBuilder::new();

        b.add_action(device.create_pointer_move(&border, 50, 50, Duration::from_millis(50)));
        b.add_action(device.create_pointer_down(MouseButton::Right));
        b.add_action(device.create_pointer_move(&border, 52, 52, Duration::from_millis(50)));
        b.add_action(device.create_pointer_up(MouseButton::Right));
        self.session().perform_actions(b.to_action_sequence_list());

        assert_eq!("RightTapped", last_gesture.text());
    }

    pub fn right_tapped_is_not_raised_for_drag(&self) {
        let border = self.session().find_element_by_accessibility_id("GestureBorder");
        let last_gesture = self.session().find_element_by_accessibility_id("LastGesture");
        let device = PointerInputDevice::mouse();
        let mut b = ActionBuilder::new();

        b.add_action(device.create_pointer_move(&border, 50, 50, Duration::from_millis(100)));
        b.add_action(device.create_pointer_down(MouseButton::Right));
        b.add_action(device.create_pointer_move_from(CoordinateOrigin::Pointer, 50, 50, Duration::from_millis(100)));
        b.add_action(device.create_pointer_up(MouseButton::Right));
        // As upstream, the sequence is built and not performed.
        let _ = b;

        assert_eq!("", last_gesture.text());
    }
}

pub fn tests(tests: &mut Tests) {
    tests
        .class("gesture_tests", Collection::Default, GestureTests::new)
        .fact("tapped_is_raised", GestureTests::tapped_is_raised)
        .fact("tapped_is_raised_slow", GestureTests::tapped_is_raised_slow)
        .fact("tapped_is_not_raised_for_drag", GestureTests::tapped_is_not_raised_for_drag)
        .fact("double_tapped_is_raised", GestureTests::double_tapped_is_raised)
        .platform_fact(
            "double_tapped_is_raised_2",
            TestPlatforms::WINDOWS.or(TestPlatforms::LINUX),
            GestureTests::double_tapped_is_raised_2,
        )
        .fact("double_tapped_is_raised_not_raised_if_too_slow", GestureTests::double_tapped_is_raised_not_raised_if_too_slow)
        .fact("double_tapped_is_raised_after_control_changes", GestureTests::double_tapped_is_raised_after_control_changes)
        .fact("right_tapped_is_raised", GestureTests::right_tapped_is_raised)
        .platform_fact("right_tapped_is_raised_2", TestPlatforms::MAC_OS, GestureTests::right_tapped_is_raised_2)
        .platform_fact(
            "right_tapped_is_not_raised_for_drag",
            TestPlatforms::MAC_OS,
            GestureTests::right_tapped_is_not_raised_for_drag,
        );
}
