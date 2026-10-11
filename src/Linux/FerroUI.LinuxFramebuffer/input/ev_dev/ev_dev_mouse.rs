//! The handler of a pointer device: a mouse, or an absolute pointer such
//! as the one of a virtual machine.
//!
//! An addition (docs/porting/linux-framebuffer-platform.md, section 7):
//! the reference handles pointers in its `libinput` backend only, and
//! there only absolute ones.

use super::{EvDevDeviceHandler, EvDevDeviceInfo, EvDevRawEvent};
use crate::native_unsafe_methods::{abs_axis, axis_event_code, ev_key, ev_type, input_absinfo, input_event};
use ferroui_base::input::raw::RawPointerEventType;
use ferroui_base::input::RawInputModifiers;
use ferroui_base::{Point, Size, Vector};

/// The description of a pointer device: `(path):type=mouse`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EvDevMouseDeviceDescription {
    pub path: Option<String>,
}

pub struct EvDevMouse {
    axis_x: Option<input_absinfo>,
    axis_y: Option<input_absinfo>,
    /// The position of the pointer on the screen.
    position: Point,
    abs_x: i32,
    abs_y: i32,
    has_relative_movement: bool,
    has_absolute_movement: bool,
    /// The button changes of the report, in order.
    buttons: Vec<(u16, bool)>,
    wheel: Vector,
    modifiers: RawInputModifiers,
}

impl EvDevMouse {
    pub fn new(device: &EvDevDeviceInfo) -> Self {
        Self {
            axis_x: device.abs_x(),
            axis_y: device.abs_y(),
            position: Point::default(),
            abs_x: 0,
            abs_y: 0,
            has_relative_movement: false,
            has_absolute_movement: false,
            buttons: Vec::new(),
            wheel: Vector::default(),
            modifiers: RawInputModifiers::empty(),
        }
    }

    fn scale(value: i32, axis: &input_absinfo, size: f64) -> f64 {
        let range = (axis.maximum - axis.minimum) as f64;
        if range <= 0.0 {
            return 0.0;
        }
        (value - axis.minimum) as f64 * size / range
    }
}

impl EvDevDeviceHandler for EvDevMouse {
    fn handle_event(&mut self, ev: &input_event, screen_size: Size, out: &mut Vec<EvDevRawEvent>) {
        match ev.type_ {
            ev_type::EV_REL => match ev.code {
                axis_event_code::REL_X => {
                    self.position = Point::new(self.position.x + ev.value as f64, self.position.y);
                    self.has_relative_movement = true;
                }
                axis_event_code::REL_Y => {
                    self.position = Point::new(self.position.x, self.position.y + ev.value as f64);
                    self.has_relative_movement = true;
                }
                // A step away from the user scrolls up; a step to the
                // right scrolls right, which is a negative delta.
                axis_event_code::REL_WHEEL => self.wheel = Vector::new(self.wheel.x, self.wheel.y + ev.value as f64),
                axis_event_code::REL_HWHEEL => self.wheel = Vector::new(self.wheel.x - ev.value as f64, self.wheel.y),
                _ => {}
            },
            ev_type::EV_ABS => match ev.code {
                abs_axis::ABS_X => {
                    self.abs_x = ev.value;
                    self.has_absolute_movement = true;
                }
                abs_axis::ABS_Y => {
                    self.abs_y = ev.value;
                    self.has_absolute_movement = true;
                }
                _ => {}
            },
            ev_type::EV_KEY => {
                if matches!(ev.code, ev_key::BTN_LEFT | ev_key::BTN_RIGHT | ev_key::BTN_MIDDLE) && ev.value != 2 {
                    self.buttons.push((ev.code, ev.value != 0));
                }
            }
            ev_type::EV_SYN => {
                let timestamp = ev.timestamp();
                if self.has_absolute_movement {
                    if let (Some(axis_x), Some(axis_y)) = (&self.axis_x, &self.axis_y) {
                        self.position = Point::new(
                            Self::scale(self.abs_x, axis_x, screen_size.width),
                            Self::scale(self.abs_y, axis_y, screen_size.height),
                        );
                    }
                }
                if self.has_relative_movement {
                    // The pointer stays on the screen.
                    self.position = Point::new(
                        self.position.x.clamp(0.0, (screen_size.width - 1.0).max(0.0)),
                        self.position.y.clamp(0.0, (screen_size.height - 1.0).max(0.0)),
                    );
                }
                if self.has_relative_movement || self.has_absolute_movement {
                    out.push(EvDevRawEvent::Pointer {
                        type_: RawPointerEventType::Move,
                        timestamp,
                        position: self.position,
                        modifiers: self.modifiers,
                    });
                }
                for (button, down) in std::mem::take(&mut self.buttons) {
                    let (flag, pressed, released) = match button {
                        ev_key::BTN_LEFT => (
                            RawInputModifiers::LEFT_MOUSE_BUTTON,
                            RawPointerEventType::LeftButtonDown,
                            RawPointerEventType::LeftButtonUp,
                        ),
                        ev_key::BTN_RIGHT => (
                            RawInputModifiers::RIGHT_MOUSE_BUTTON,
                            RawPointerEventType::RightButtonDown,
                            RawPointerEventType::RightButtonUp,
                        ),
                        _ => (
                            RawInputModifiers::MIDDLE_MOUSE_BUTTON,
                            RawPointerEventType::MiddleButtonDown,
                            RawPointerEventType::MiddleButtonUp,
                        ),
                    };
                    self.modifiers.set(flag, down);
                    out.push(EvDevRawEvent::Pointer {
                        type_: if down { pressed } else { released },
                        timestamp,
                        position: self.position,
                        modifiers: self.modifiers,
                    });
                }
                if self.wheel != Vector::default() {
                    out.push(EvDevRawEvent::Wheel {
                        timestamp,
                        position: self.position,
                        delta: self.wheel,
                        modifiers: self.modifiers,
                    });
                }
                self.has_relative_movement = false;
                self.has_absolute_movement = false;
                self.wheel = Vector::default();
            }
            _ => {}
        }
    }

    fn reset(&mut self, timestamp: u64, _screen_size: Size, out: &mut Vec<EvDevRawEvent>) {
        // The buttons that were held are released where the pointer is.
        for (flag, released) in [
            (RawInputModifiers::LEFT_MOUSE_BUTTON, RawPointerEventType::LeftButtonUp),
            (RawInputModifiers::RIGHT_MOUSE_BUTTON, RawPointerEventType::RightButtonUp),
            (RawInputModifiers::MIDDLE_MOUSE_BUTTON, RawPointerEventType::MiddleButtonUp),
        ] {
            if self.modifiers.contains(flag) {
                self.modifiers.remove(flag);
                out.push(EvDevRawEvent::Pointer {
                    type_: released,
                    timestamp,
                    position: self.position,
                    modifiers: self.modifiers,
                });
            }
        }
        self.buttons.clear();
        self.wheel = Vector::default();
        self.has_relative_movement = false;
        self.has_absolute_movement = false;
    }
}

#[cfg(test)]
mod tests {
    use super::super::ev_dev_touch_screen::tests::{ev, info, run, syn};
    use super::*;
    use crate::native_unsafe_methods::syn_code;

    fn pointer(
        type_: RawPointerEventType,
        timestamp: u64,
        x: f64,
        y: f64,
        modifiers: RawInputModifiers,
    ) -> EvDevRawEvent {
        EvDevRawEvent::Pointer { type_, timestamp, position: Point::new(x, y), modifiers }
    }

    #[test]
    fn relative_motion_accumulates_and_stays_on_the_screen() {
        let mut mouse = EvDevMouse::new(&info(&[]));
        let screen = Size::new(100.0, 50.0);
        let none = RawInputModifiers::empty();
        let events = [
            ev(1, ev_type::EV_REL, axis_event_code::REL_X, 30),
            ev(1, ev_type::EV_REL, axis_event_code::REL_Y, 20),
            syn(1),
            ev(2, ev_type::EV_REL, axis_event_code::REL_X, 500),
            syn(2),
            ev(3, ev_type::EV_REL, axis_event_code::REL_X, -10),
            ev(3, ev_type::EV_REL, axis_event_code::REL_Y, -100),
            syn(3),
        ];
        assert_eq!(
            run(&mut mouse, &events, screen),
            [
                pointer(RawPointerEventType::Move, 1, 30.0, 20.0, none),
                pointer(RawPointerEventType::Move, 2, 99.0, 20.0, none),
                pointer(RawPointerEventType::Move, 3, 89.0, 0.0, none),
            ]
        );
    }

    #[test]
    fn buttons_follow_the_move_of_their_report_and_are_modifiers_while_held() {
        let mut mouse = EvDevMouse::new(&info(&[]));
        let screen = Size::new(100.0, 100.0);
        let left = RawInputModifiers::LEFT_MOUSE_BUTTON;
        let right = RawInputModifiers::RIGHT_MOUSE_BUTTON;
        let events = [
            ev(1, ev_type::EV_KEY, ev_key::BTN_LEFT, 1),
            ev(1, ev_type::EV_REL, axis_event_code::REL_X, 5),
            syn(1),
            ev(2, ev_type::EV_KEY, ev_key::BTN_RIGHT, 1),
            syn(2),
            ev(3, ev_type::EV_KEY, ev_key::BTN_RIGHT, 0),
            ev(3, ev_type::EV_KEY, ev_key::BTN_LEFT, 0),
            syn(3),
            ev(4, ev_type::EV_KEY, ev_key::BTN_MIDDLE, 1),
            syn(4),
        ];
        assert_eq!(
            run(&mut mouse, &events, screen),
            [
                pointer(RawPointerEventType::Move, 1, 5.0, 0.0, RawInputModifiers::empty()),
                pointer(RawPointerEventType::LeftButtonDown, 1, 5.0, 0.0, left),
                pointer(RawPointerEventType::RightButtonDown, 2, 5.0, 0.0, left | right),
                pointer(RawPointerEventType::RightButtonUp, 3, 5.0, 0.0, left),
                pointer(RawPointerEventType::LeftButtonUp, 3, 5.0, 0.0, RawInputModifiers::empty()),
                pointer(RawPointerEventType::MiddleButtonDown, 4, 5.0, 0.0, RawInputModifiers::MIDDLE_MOUSE_BUTTON),
            ]
        );
    }

    #[test]
    fn the_wheel_is_a_delta_at_the_position() {
        let mut mouse = EvDevMouse::new(&info(&[]));
        let events = [
            ev(1, ev_type::EV_REL, axis_event_code::REL_WHEEL, -1),
            syn(1),
            ev(2, ev_type::EV_REL, axis_event_code::REL_HWHEEL, 2),
            syn(2),
        ];
        let wheel = |timestamp, x, y| EvDevRawEvent::Wheel {
            timestamp,
            position: Point::default(),
            delta: Vector::new(x, y),
            modifiers: RawInputModifiers::empty(),
        };
        assert_eq!(run(&mut mouse, &events, Size::new(10.0, 10.0)), [wheel(1, 0.0, -1.0), wheel(2, -2.0, 0.0)]);
    }

    #[test]
    fn an_absolute_pointer_is_scaled_to_the_screen() {
        // The tablet of a virtual machine: 0 to 32767 on both axes.
        let mut mouse = EvDevMouse::new(&info(&[(abs_axis::ABS_X, 0, 32767), (abs_axis::ABS_Y, 0, 32767)]));
        let events = [
            ev(1, ev_type::EV_ABS, abs_axis::ABS_X, 32767),
            ev(1, ev_type::EV_ABS, abs_axis::ABS_Y, 0),
            ev(1, ev_type::EV_KEY, ev_key::BTN_LEFT, 1),
            syn(1),
        ];
        let left = RawInputModifiers::LEFT_MOUSE_BUTTON;
        assert_eq!(
            run(&mut mouse, &events, Size::new(1024.0, 768.0)),
            [
                pointer(RawPointerEventType::Move, 1, 1024.0, 0.0, RawInputModifiers::empty()),
                pointer(RawPointerEventType::LeftButtonDown, 1, 1024.0, 0.0, left),
            ]
        );
    }

    #[test]
    fn dropped_events_release_the_held_buttons() {
        let mut mouse = EvDevMouse::new(&info(&[]));
        let events = [
            ev(1, ev_type::EV_KEY, ev_key::BTN_LEFT, 1),
            syn(1),
            ev(2, ev_type::EV_SYN, syn_code::SYN_DROPPED, 0),
            ev(2, ev_type::EV_REL, axis_event_code::REL_X, 5),
            syn(2),
        ];
        let out = run(&mut mouse, &events, Size::new(10.0, 10.0));
        assert_eq!(out[1], pointer(RawPointerEventType::LeftButtonUp, 2, 0.0, 0.0, RawInputModifiers::empty()));
        assert_eq!(out.len(), 2);
    }
}
