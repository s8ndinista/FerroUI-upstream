//! Port of `DragDropTests.cs`.

use crate::infrastructure::{
    ActionBuilder, Actions, Collection, DefaultAppFixture, MouseButton, PointerInputDevice, Session, TestBase, TestClass,
    TestPlatforms, Tests,
};
use std::time::Duration;

pub struct DragDropTests {
    base: TestBase,
}

impl TestClass for DragDropTests {}

impl DragDropTests {
    pub fn new(fixture: &DefaultAppFixture) -> Self {
        let base = TestBase::new(fixture, "DragDrop");
        let reset = base.session().find_element_by_accessibility_id("ResetDragDrop");
        reset.click();
        DragDropTests { base }
    }

    fn session(&self) -> &Session {
        self.base.session()
    }

    pub fn drag_drop_coordinates_correct_when_controls_offset_from_origin(&self) {
        // This test verifies the fix for drag-drop coordinate calculation when
        // controls are positioned away from the window origin.
        // Issue: In embedded views or when controls have margin/offset from origin,
        // the drag-drop coordinates were incorrectly calculated relative to the
        // window rather than the view.

        let drag_source = self.session().find_element_by_accessibility_id("DragSource");
        let drop_target = self.session().find_element_by_accessibility_id("DropTarget");
        let drop_position = self.session().find_element_by_accessibility_id("DropPosition");
        let drag_drop_status = self.session().find_element_by_accessibility_id("DragDropStatus");

        // Perform drag from source to target
        Actions::new(self.session())
            .move_to_element(&drag_source)
            .click_and_hold()
            .move_to_element(&drop_target)
            .release()
            .perform();

        std::thread::sleep(Duration::from_millis(500)); // Allow UI to update

        // Verify the drop was successful
        let status = drag_drop_status.text();
        assert!(status == "Drop OK" || status == "Copied", "Expected drop to succeed, but status was: {status}");

        // Verify the drop position is within the target bounds
        // If the coordinate calculation bug exists, the position would be
        // offset by the spacer/margin and would show negative coordinates
        // or coordinates outside the target bounds
        let position_text = drop_position.text();
        assert!(position_text.starts_with("Drop:"), "{position_text:?} does not start with \"Drop:\"");

        // The DropTargetText should not contain "ERROR"
        let drop_target_text = self.session().find_element_by_accessibility_id("DropTargetText");
        assert!(!drop_target_text.text().contains("ERROR"));
    }

    pub fn drag_drop_position_updates_during_drag_over(&self) {
        // Verifies that position is correctly reported during drag-over events
        let drag_source = self.session().find_element_by_accessibility_id("DragSource");
        let drop_target = self.session().find_element_by_accessibility_id("DropTarget");
        let drop_position = self.session().find_element_by_accessibility_id("DropPosition");

        // Start drag and move over target, then release
        let device = PointerInputDevice::mouse();
        let mut builder = ActionBuilder::new();

        // Move to drag source and start drag
        builder.add_action(device.create_pointer_move(&drag_source, 0, 0, Duration::from_millis(100)));
        builder.add_action(device.create_pointer_down(MouseButton::Left));

        // Move to drop target (this triggers DragOver events)
        builder.add_action(device.create_pointer_move(&drop_target, 0, 0, Duration::from_millis(200)));

        // Pause to allow DragOver events to be processed
        builder.add_action(device.create_pause(Duration::from_millis(200)));

        // Release at current position (completes the drag)
        builder.add_action(device.create_pointer_up(MouseButton::Left));

        self.session().perform_actions(builder.to_action_sequence_list());

        std::thread::sleep(Duration::from_millis(200));

        // Check that position was recorded (either DragOver or Drop)
        let position_text = drop_position.text();

        // Position should have been updated during drag-over or drop
        assert!(
            position_text.contains("DragOver:") || position_text.contains("Drop:"),
            "Expected position to be updated during drag, but got: {position_text}"
        );
    }

    pub fn drag_drop_can_be_cancelled(&self) {
        let drag_source = self.session().find_element_by_accessibility_id("DragSource");
        let drag_drop_status = self.session().find_element_by_accessibility_id("DragDropStatus");

        // Start drag but release outside any drop target
        Actions::new(self.session())
            .move_to_element(&drag_source)
            .click_and_hold()
            .move_by_offset(-200, 0) // Move away from drop target
            .release()
            .perform();

        std::thread::sleep(Duration::from_millis(500));

        // Status should indicate cancelled
        let status = drag_drop_status.text();
        assert_eq!("Cancelled", status);
    }
}

pub fn tests(tests: &mut Tests) {
    tests
        .class("drag_drop_tests", Collection::Default, DragDropTests::new)
        .platform_fact(
            "drag_drop_coordinates_correct_when_controls_offset_from_origin",
            TestPlatforms::MAC_OS,
            DragDropTests::drag_drop_coordinates_correct_when_controls_offset_from_origin,
        )
        .platform_fact(
            "drag_drop_position_updates_during_drag_over",
            TestPlatforms::MAC_OS,
            DragDropTests::drag_drop_position_updates_during_drag_over,
        )
        .fact("drag_drop_can_be_cancelled", DragDropTests::drag_drop_can_be_cancelled);
}
