//! The handler of a touch screen that reports one contact, and what the
//! handlers of event devices share (the port of `EvDevTouchScreen.cs`).
//!
//! A handler is a state machine over the events of its device. It runs on
//! the UI thread and reports raw events as values ([`EvDevRawEvent`]),
//! which the backend turns into the raw input events of the framework
//! with its devices and the input root; that is what lets the state
//! machines be tested against recorded event streams.

use super::{EvDevDeviceInfo, EvDevTouchScreenDeviceDescription};
use crate::native_unsafe_methods::{abs_axis, ev_key, ev_type, input_absinfo, input_event, syn_code};
use ferroui_base::input::raw::{RawKeyEventType, RawPointerEventType};
use ferroui_base::input::{Key, PhysicalKey, RawInputModifiers};
use ferroui_base::{Matrix, Point, Size, Vector};

/// A raw input event of an event device, before it has a device object
/// and an input root.
#[derive(Clone, Debug, PartialEq)]
pub enum EvDevRawEvent {
    Touch {
        type_: RawPointerEventType,
        timestamp: u64,
        position: Point,
        raw_pointer_id: i64,
    },
    Pointer {
        type_: RawPointerEventType,
        timestamp: u64,
        position: Point,
        modifiers: RawInputModifiers,
    },
    Wheel {
        timestamp: u64,
        position: Point,
        delta: Vector,
        modifiers: RawInputModifiers,
    },
    Key {
        type_: RawKeyEventType,
        timestamp: u64,
        key: Key,
        modifiers: RawInputModifiers,
        physical_key: PhysicalKey,
        key_symbol: Option<String>,
    },
    Text {
        timestamp: u64,
        text: String,
    },
}

/// The handler of the events of one device (`EvDevDeviceHandler`).
pub trait EvDevDeviceHandler {
    /// `HandleEvent`. `screen_size` is the scaled size of the screen.
    fn handle_event(&mut self, ev: &input_event, screen_size: Size, out: &mut Vec<EvDevRawEvent>);

    /// Ends what is in progress after the kernel dropped events of the
    /// device. Not in the reference, whose library re-synchronises.
    fn reset(&mut self, _timestamp: u64, _screen_size: Size, _out: &mut Vec<EvDevRawEvent>) {}
}

/// `HandleEvents`: hands the events of one read to the handler.
///
/// After `SYN_DROPPED` the events up to the next `SYN_REPORT` belong to a
/// report the kernel could not deliver whole: they are passed over and
/// the handler is reset. `dropped` carries that state from one read to
/// the next.
pub fn handle_events(
    handler: &mut dyn EvDevDeviceHandler,
    dropped: &mut bool,
    events: &[input_event],
    screen_size: Size,
    out: &mut Vec<EvDevRawEvent>,
) {
    for ev in events {
        if ev.type_ == ev_type::EV_SYN && ev.code == syn_code::SYN_DROPPED {
            *dropped = true;
            continue;
        }
        if *dropped {
            if ev.type_ == ev_type::EV_SYN && ev.code == syn_code::SYN_REPORT {
                *dropped = false;
                handler.reset(ev.timestamp(), screen_size, out);
            }
            continue;
        }
        handler.handle_event(ev, screen_size, out);
    }
}

/// The position on the screen of a point of a touch device: normalised to
/// the range of the axes, calibrated, and scaled to the screen.
pub(crate) fn touch_position(
    x: i32,
    y: i32,
    axis_x: &input_absinfo,
    axis_y: &input_absinfo,
    calibration: Matrix,
    screen_size: Size,
) -> Point {
    let touch_width = (axis_x.maximum - axis_x.minimum) as f64;
    let touch_height = (axis_y.maximum - axis_y.minimum) as f64;

    // Normalize to 0-(max-min)
    let point = Point::new((x - axis_x.minimum) as f64, (y - axis_y.minimum) as f64);

    // Apply calibration matrix
    let point = point * calibration;

    // Transform to display pixel grid
    Point::new(point.x * screen_size.width / touch_width, point.y * screen_size.height / touch_height)
}

pub struct EvDevSingleTouchScreen {
    calibration: Matrix,
    axis_x: input_absinfo,
    axis_y: input_absinfo,
    current_x: i32,
    current_y: i32,
    has_movement: bool,
    press_action: Option<bool>,
}

impl EvDevSingleTouchScreen {
    /// Fails when the device does not have the two absolute axes.
    pub fn new(device: &EvDevDeviceInfo, description: &EvDevTouchScreenDeviceDescription) -> Result<Self, String> {
        let (Some(axis_x), Some(axis_y)) = (device.abs_x(), device.abs_y()) else {
            return Err("Device is not a touchscreen".to_owned());
        };
        Ok(Self {
            calibration: description.calibration_matrix,
            axis_x,
            axis_y,
            current_x: 0,
            current_y: 0,
            has_movement: false,
            press_action: None,
        })
    }

    fn raise_event(&self, type_: RawPointerEventType, timestamp: u64, screen_size: Size, out: &mut Vec<EvDevRawEvent>) {
        let position =
            touch_position(self.current_x, self.current_y, &self.axis_x, &self.axis_y, self.calibration, screen_size);
        out.push(EvDevRawEvent::Touch { type_, timestamp, position, raw_pointer_id: 1 });
    }
}

impl EvDevDeviceHandler for EvDevSingleTouchScreen {
    fn handle_event(&mut self, ev: &input_event, screen_size: Size, out: &mut Vec<EvDevRawEvent>) {
        if ev.type_ == ev_type::EV_ABS {
            if ev.code == abs_axis::ABS_X {
                self.current_x = ev.value;
                self.has_movement = true;
            }

            if ev.code == abs_axis::ABS_Y {
                self.current_y = ev.value;
                self.has_movement = true;
            }
        }

        if ev.type_ == ev_type::EV_KEY && ev.code == ev_key::BTN_TOUCH {
            self.press_action = Some(ev.value != 0);
        }

        if ev.type_ == ev_type::EV_SYN {
            if let Some(press_action) = self.press_action {
                let type_ = if press_action { RawPointerEventType::TouchBegin } else { RawPointerEventType::TouchEnd };
                self.raise_event(type_, ev.timestamp(), screen_size, out);
            } else if self.has_movement {
                self.raise_event(RawPointerEventType::TouchUpdate, ev.timestamp(), screen_size, out);
            }

            self.has_movement = false;
            self.press_action = None;
        }
    }

    fn reset(&mut self, _timestamp: u64, _screen_size: Size, _out: &mut Vec<EvDevRawEvent>) {
        self.has_movement = false;
        self.press_action = None;
    }
}

#[cfg(test)]
pub(crate) mod tests {
    // Not from the reference, which has no tests for this file. The
    // streams are what `evtest` prints for such a device.
    use super::*;

    pub fn info(axes: &[(u16, i32, i32)]) -> EvDevDeviceInfo {
        EvDevDeviceInfo {
            name: Some("test".to_owned()),
            event_types: vec![ev_type::EV_SYN, ev_type::EV_KEY, ev_type::EV_ABS],
            abs: axes
                .iter()
                .map(|&(code, minimum, maximum)| (code, input_absinfo { minimum, maximum, ..Default::default() }))
                .collect(),
        }
    }

    pub fn ev(ms: i64, type_: u16, code: u16, value: i32) -> input_event {
        input_event { tv_sec: ms / 1000, tv_usec: (ms % 1000) * 1000, type_, code, value }
    }

    pub fn syn(ms: i64) -> input_event {
        ev(ms, ev_type::EV_SYN, syn_code::SYN_REPORT, 0)
    }

    pub fn run(handler: &mut dyn EvDevDeviceHandler, events: &[input_event], screen: Size) -> Vec<EvDevRawEvent> {
        let mut out = Vec::new();
        let mut dropped = false;
        handle_events(handler, &mut dropped, events, screen, &mut out);
        out
    }

    fn touch(type_: RawPointerEventType, timestamp: u64, x: f64, y: f64) -> EvDevRawEvent {
        EvDevRawEvent::Touch { type_, timestamp, position: Point::new(x, y), raw_pointer_id: 1 }
    }

    #[test]
    fn a_press_a_move_and_a_release_are_begin_update_and_end_scaled_to_the_screen() {
        let info = info(&[(abs_axis::ABS_X, 0, 4095), (abs_axis::ABS_Y, 0, 4095)]);
        let mut handler = EvDevSingleTouchScreen::new(&info, &EvDevTouchScreenDeviceDescription::default()).unwrap();
        let screen = Size::new(819.0, 409.5);
        let events = [
            ev(1000, ev_type::EV_ABS, abs_axis::ABS_X, 1000),
            ev(1000, ev_type::EV_ABS, abs_axis::ABS_Y, 2000),
            ev(1000, ev_type::EV_KEY, ev_key::BTN_TOUCH, 1),
            syn(1000),
            ev(1016, ev_type::EV_ABS, abs_axis::ABS_X, 1500),
            syn(1016),
            // A report without a change reports nothing.
            syn(1020),
            ev(1032, ev_type::EV_KEY, ev_key::BTN_TOUCH, 0),
            syn(1032),
        ];
        assert_eq!(
            run(&mut handler, &events, screen),
            [
                touch(RawPointerEventType::TouchBegin, 1000, 200.0, 200.0),
                touch(RawPointerEventType::TouchUpdate, 1016, 300.0, 200.0),
                touch(RawPointerEventType::TouchEnd, 1032, 300.0, 200.0),
            ]
        );
    }

    #[test]
    fn the_axis_minimum_and_the_calibration_are_applied_before_the_scale() {
        let info = info(&[(abs_axis::ABS_X, 100, 1100), (abs_axis::ABS_Y, 200, 700)]);
        // Swaps the axes.
        let description = EvDevTouchScreenDeviceDescription {
            path: None,
            calibration_matrix: Matrix::new(0.0, 1.0, 1.0, 0.0, 0.0, 0.0),
        };
        let mut handler = EvDevSingleTouchScreen::new(&info, &description).unwrap();
        let events = [
            ev(5, ev_type::EV_ABS, abs_axis::ABS_X, 600),
            ev(5, ev_type::EV_ABS, abs_axis::ABS_Y, 300),
            ev(5, ev_type::EV_KEY, ev_key::BTN_TOUCH, 1),
            syn(5),
        ];
        // (500, 100) swapped is (100, 500); scaled by 1000/1000 and 500/500.
        assert_eq!(
            run(&mut handler, &events, Size::new(1000.0, 500.0)),
            [touch(RawPointerEventType::TouchBegin, 5, 100.0, 500.0)]
        );
    }

    #[test]
    fn a_device_without_the_two_axes_is_not_a_touch_screen() {
        let info = info(&[(abs_axis::ABS_X, 0, 10)]);
        let error = EvDevSingleTouchScreen::new(&info, &EvDevTouchScreenDeviceDescription::default()).err();
        assert_eq!(error.as_deref(), Some("Device is not a touchscreen"));
    }

    #[test]
    fn the_events_of_a_dropped_report_are_passed_over() {
        let info = info(&[(abs_axis::ABS_X, 0, 100), (abs_axis::ABS_Y, 0, 100)]);
        let mut handler = EvDevSingleTouchScreen::new(&info, &EvDevTouchScreenDeviceDescription::default()).unwrap();
        let screen = Size::new(100.0, 100.0);
        let mut out = Vec::new();
        let mut dropped = false;
        let first = [
            ev(1, ev_type::EV_ABS, abs_axis::ABS_X, 10),
            ev(1, ev_type::EV_SYN, syn_code::SYN_DROPPED, 0),
            ev(1, ev_type::EV_KEY, ev_key::BTN_TOUCH, 1),
        ];
        handle_events(&mut handler, &mut dropped, &first, screen, &mut out);
        assert!(dropped && out.is_empty());
        // The state is carried to the next read.
        let second = [
            ev(2, ev_type::EV_ABS, abs_axis::ABS_Y, 20),
            syn(2),
            ev(3, ev_type::EV_ABS, abs_axis::ABS_X, 50),
            ev(3, ev_type::EV_ABS, abs_axis::ABS_Y, 60),
            syn(3),
        ];
        handle_events(&mut handler, &mut dropped, &second, screen, &mut out);
        assert!(!dropped);
        assert_eq!(out, [touch(RawPointerEventType::TouchUpdate, 3, 50.0, 60.0)]);
    }
}
