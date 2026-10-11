//! The handler of a touch screen that reports several contacts with the
//! multi-touch protocol B of the kernel (slots).
//!
//! An addition (docs/porting/linux-framebuffer-platform.md, section 7):
//! the reference's evdev backend knows a single contact, and reads
//! several contacts through `libinput` only.

use super::ev_dev_touch_screen::touch_position;
use super::{EvDevDeviceHandler, EvDevDeviceInfo, EvDevRawEvent};
use crate::native_unsafe_methods::{abs_axis, ev_type, input_absinfo, input_event};
use ferroui_base::input::raw::RawPointerEventType;
use ferroui_base::{Matrix, Size};

/// The description of a multi-touch screen:
/// `(path):type=multitouch:[calibration=m11,m12,m21,m22,m31,m32]`.
#[derive(Clone, Debug, PartialEq)]
pub struct EvDevMultiTouchScreenDeviceDescription {
    pub path: Option<String>,
    pub calibration_matrix: Matrix,
}

impl Default for EvDevMultiTouchScreenDeviceDescription {
    fn default() -> Self {
        Self { path: None, calibration_matrix: Matrix::IDENTITY }
    }
}

#[derive(Clone, Copy, Default)]
struct Slot {
    /// Whether a contact is in the slot, as far as was reported.
    active: bool,
    /// The position that was last reported for the slot.
    reported: (i32, i32),
    x: i32,
    y: i32,
    // What the current report changes:
    /// the contact that was in the slot ends;
    ends_first: bool,
    /// a contact begins;
    begins: bool,
    /// the contact that begins ends as well;
    ends_after_begin: bool,
    /// the position changed.
    moved: bool,
}

pub struct EvDevMultiTouchScreen {
    calibration: Matrix,
    axis_x: input_absinfo,
    axis_y: input_absinfo,
    slots: Vec<Slot>,
    current_slot: usize,
}

/// More slots than any device has; a slot number beyond it is ignored.
const MAX_SLOTS: usize = 64;

impl EvDevMultiTouchScreen {
    /// Fails when the device does not have the two position axes of the
    /// protocol.
    pub fn new(device: &EvDevDeviceInfo, description: &EvDevMultiTouchScreenDeviceDescription) -> Result<Self, String> {
        let axis = |code: u16| device.abs.get(&code).copied();
        let (Some(axis_x), Some(axis_y)) = (axis(abs_axis::ABS_MT_POSITION_X), axis(abs_axis::ABS_MT_POSITION_Y))
        else {
            return Err("Device is not a multi-touch screen".to_owned());
        };
        let slot_count =
            axis(abs_axis::ABS_MT_SLOT).map_or(1, |slot| (slot.maximum.max(0) as usize + 1).min(MAX_SLOTS));
        Ok(Self {
            calibration: description.calibration_matrix,
            axis_x,
            axis_y,
            slots: vec![Slot::default(); slot_count],
            current_slot: 0,
        })
    }

    fn slot(&mut self) -> Option<&mut Slot> {
        if self.current_slot >= MAX_SLOTS {
            return None;
        }
        if self.current_slot >= self.slots.len() {
            self.slots.resize(self.current_slot + 1, Slot::default());
        }
        self.slots.get_mut(self.current_slot)
    }

    fn report(&mut self, timestamp: u64, screen_size: Size, out: &mut Vec<EvDevRawEvent>) {
        let (axis_x, axis_y, calibration) = (self.axis_x, self.axis_y, self.calibration);
        for (index, slot) in self.slots.iter_mut().enumerate() {
            let mut push = |type_: RawPointerEventType, (x, y): (i32, i32)| {
                let position = touch_position(x, y, &axis_x, &axis_y, calibration, screen_size);
                out.push(EvDevRawEvent::Touch { type_, timestamp, position, raw_pointer_id: index as i64 });
            };
            // A contact that is replaced in one report ends, where it was
            // last seen, before the new one begins.
            if slot.ends_first {
                push(RawPointerEventType::TouchEnd, slot.reported);
                slot.active = false;
            }
            if slot.begins {
                push(RawPointerEventType::TouchBegin, (slot.x, slot.y));
                slot.active = true;
                if slot.ends_after_begin {
                    push(RawPointerEventType::TouchEnd, (slot.x, slot.y));
                    slot.active = false;
                }
                slot.reported = (slot.x, slot.y);
            } else if slot.moved && slot.active {
                push(RawPointerEventType::TouchUpdate, (slot.x, slot.y));
                slot.reported = (slot.x, slot.y);
            }
            slot.ends_first = false;
            slot.begins = false;
            slot.ends_after_begin = false;
            slot.moved = false;
        }
    }
}

impl EvDevDeviceHandler for EvDevMultiTouchScreen {
    fn handle_event(&mut self, ev: &input_event, screen_size: Size, out: &mut Vec<EvDevRawEvent>) {
        if ev.type_ == ev_type::EV_ABS {
            match ev.code {
                abs_axis::ABS_MT_SLOT => self.current_slot = ev.value.max(0) as usize,
                abs_axis::ABS_MT_TRACKING_ID => {
                    let begins = ev.value >= 0;
                    if let Some(slot) = self.slot() {
                        if begins {
                            // A new identifier in a slot that has a
                            // contact replaces it.
                            if !slot.begins {
                                slot.ends_first = slot.active;
                                slot.begins = true;
                            }
                        } else if slot.begins {
                            slot.ends_after_begin = true;
                        } else if slot.active {
                            slot.ends_first = true;
                        }
                    }
                }
                abs_axis::ABS_MT_POSITION_X => {
                    if let Some(slot) = self.slot() {
                        slot.x = ev.value;
                        slot.moved = true;
                    }
                }
                abs_axis::ABS_MT_POSITION_Y => {
                    if let Some(slot) = self.slot() {
                        slot.y = ev.value;
                        slot.moved = true;
                    }
                }
                _ => {}
            }
        }

        if ev.type_ == ev_type::EV_SYN {
            self.report(ev.timestamp(), screen_size, out);
        }
    }

    fn reset(&mut self, timestamp: u64, screen_size: Size, out: &mut Vec<EvDevRawEvent>) {
        // The contacts that were reported end where they were last seen.
        for slot in &mut self.slots {
            slot.begins = false;
            slot.ends_after_begin = false;
            slot.moved = false;
            slot.ends_first = slot.active;
        }
        self.report(timestamp, screen_size, out);
    }
}

#[cfg(test)]
mod tests {
    use super::super::ev_dev_touch_screen::tests::{ev, info, run, syn};
    use super::*;
    use crate::native_unsafe_methods::syn_code;
    use ferroui_base::Point;

    fn abs(ms: i64, code: u16, value: i32) -> input_event {
        ev(ms, ev_type::EV_ABS, code, value)
    }

    fn touch(type_: RawPointerEventType, timestamp: u64, x: f64, y: f64, id: i64) -> EvDevRawEvent {
        EvDevRawEvent::Touch { type_, timestamp, position: Point::new(x, y), raw_pointer_id: id }
    }

    fn screen_and_handler() -> (Size, EvDevMultiTouchScreen) {
        let info = info(&[
            (abs_axis::ABS_MT_SLOT, 0, 9),
            (abs_axis::ABS_MT_POSITION_X, 0, 1000),
            (abs_axis::ABS_MT_POSITION_Y, 0, 500),
        ]);
        let handler = EvDevMultiTouchScreen::new(&info, &EvDevMultiTouchScreenDeviceDescription::default()).unwrap();
        (Size::new(1000.0, 500.0), handler)
    }

    #[test]
    fn two_fingers_are_two_contacts_with_the_slot_as_identifier() {
        let (screen, mut handler) = screen_and_handler();
        use RawPointerEventType::{TouchBegin, TouchEnd, TouchUpdate};
        let events = [
            // The first finger: the slot is 0 until the device says otherwise.
            abs(1, abs_axis::ABS_MT_TRACKING_ID, 45),
            abs(1, abs_axis::ABS_MT_POSITION_X, 100),
            abs(1, abs_axis::ABS_MT_POSITION_Y, 200),
            syn(1),
            // The second finger.
            abs(2, abs_axis::ABS_MT_SLOT, 1),
            abs(2, abs_axis::ABS_MT_TRACKING_ID, 46),
            abs(2, abs_axis::ABS_MT_POSITION_X, 300),
            abs(2, abs_axis::ABS_MT_POSITION_Y, 400),
            syn(2),
            // Both move in one report.
            abs(3, abs_axis::ABS_MT_SLOT, 0),
            abs(3, abs_axis::ABS_MT_POSITION_X, 110),
            abs(3, abs_axis::ABS_MT_SLOT, 1),
            abs(3, abs_axis::ABS_MT_POSITION_Y, 410),
            syn(3),
            // The first is lifted; the second stays.
            abs(4, abs_axis::ABS_MT_SLOT, 0),
            abs(4, abs_axis::ABS_MT_TRACKING_ID, -1),
            syn(4),
            abs(5, abs_axis::ABS_MT_SLOT, 1),
            abs(5, abs_axis::ABS_MT_TRACKING_ID, -1),
            syn(5),
        ];
        assert_eq!(
            run(&mut handler, &events, screen),
            [
                touch(TouchBegin, 1, 100.0, 200.0, 0),
                touch(TouchBegin, 2, 300.0, 400.0, 1),
                touch(TouchUpdate, 3, 110.0, 200.0, 0),
                touch(TouchUpdate, 3, 300.0, 410.0, 1),
                touch(TouchEnd, 4, 110.0, 200.0, 0),
                touch(TouchEnd, 5, 300.0, 410.0, 1),
            ]
        );
    }

    #[test]
    fn a_contact_that_is_replaced_ends_before_the_new_one_begins() {
        let (screen, mut handler) = screen_and_handler();
        use RawPointerEventType::{TouchBegin, TouchEnd};
        let events = [
            abs(1, abs_axis::ABS_MT_TRACKING_ID, 1),
            abs(1, abs_axis::ABS_MT_POSITION_X, 10),
            abs(1, abs_axis::ABS_MT_POSITION_Y, 10),
            syn(1),
            abs(2, abs_axis::ABS_MT_TRACKING_ID, 2),
            abs(2, abs_axis::ABS_MT_POSITION_X, 50),
            syn(2),
            // A tap within one report.
            abs(3, abs_axis::ABS_MT_SLOT, 3),
            abs(3, abs_axis::ABS_MT_TRACKING_ID, 3),
            abs(3, abs_axis::ABS_MT_POSITION_X, 70),
            abs(3, abs_axis::ABS_MT_POSITION_Y, 80),
            abs(3, abs_axis::ABS_MT_TRACKING_ID, -1),
            syn(3),
        ];
        assert_eq!(
            run(&mut handler, &events, screen),
            [
                touch(TouchBegin, 1, 10.0, 10.0, 0),
                touch(TouchEnd, 2, 10.0, 10.0, 0),
                touch(TouchBegin, 2, 50.0, 10.0, 0),
                touch(TouchBegin, 3, 70.0, 80.0, 3),
                touch(TouchEnd, 3, 70.0, 80.0, 3),
            ]
        );
    }

    #[test]
    fn a_position_without_a_contact_and_a_lift_without_one_report_nothing() {
        let (screen, mut handler) = screen_and_handler();
        let events = [
            abs(1, abs_axis::ABS_MT_POSITION_X, 10),
            syn(1),
            abs(2, abs_axis::ABS_MT_TRACKING_ID, -1),
            syn(2),
            // A slot beyond any device is passed over.
            abs(3, abs_axis::ABS_MT_SLOT, 5000),
            abs(3, abs_axis::ABS_MT_TRACKING_ID, 9),
            syn(3),
        ];
        assert_eq!(run(&mut handler, &events, screen), []);
    }

    #[test]
    fn dropped_events_end_the_contacts() {
        let (screen, mut handler) = screen_and_handler();
        let events = [
            abs(1, abs_axis::ABS_MT_TRACKING_ID, 1),
            abs(1, abs_axis::ABS_MT_POSITION_X, 10),
            abs(1, abs_axis::ABS_MT_POSITION_Y, 20),
            syn(1),
            ev(2, ev_type::EV_SYN, syn_code::SYN_DROPPED, 0),
            abs(2, abs_axis::ABS_MT_POSITION_X, 500),
            syn(2),
        ];
        let out = run(&mut handler, &events, screen);
        assert_eq!(out[1], touch(RawPointerEventType::TouchEnd, 2, 10.0, 20.0, 0));
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn a_device_without_the_position_axes_is_not_a_multi_touch_screen() {
        let info = info(&[(abs_axis::ABS_X, 0, 10), (abs_axis::ABS_Y, 0, 10)]);
        assert!(EvDevMultiTouchScreen::new(&info, &EvDevMultiTouchScreenDeviceDescription::default()).is_err());
    }
}
