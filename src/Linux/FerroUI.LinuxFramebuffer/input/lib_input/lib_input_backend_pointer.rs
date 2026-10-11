//! The pointer events of the `libinput` backend (the port of
//! `LibInputBackend.Pointer.cs`).

use super::lib_input_backend::{LibInputEvent, LibInputState};
use super::lib_input_native_unsafe_methods::lib_input_pointer_axis_source::*;
use crate::input::ev_dev::EvDevRawEvent;
use crate::native_unsafe_methods::ev_key;
use ferroui_base::input::raw::RawPointerEventType;
use ferroui_base::input::RawInputModifiers;
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::{Point, Size, Vector};

const POINTER: &str = "LinuxFramebuffer/Input/LibInput/Pointer";

fn log(level: LogEventLevel, message: &str) {
    if let Some(logger) = Logger::try_get(level, POINTER) {
        logger.log(None, message);
    }
}

impl LibInputState {
    pub(crate) fn handle_pointer(&mut self, ev: &LibInputEvent, info: Size, out: &mut Vec<EvDevRawEvent>) {
        let modifiers = RawInputModifiers::empty(); //TODO: support input modifiers
        match *ev {
            LibInputEvent::PointerMotionAbsolute { time_usec, x, y } => {
                // The library transforms to a whole width and height.
                self.mouse_position = Point::new(x * (info.width as i32) as f64, y * (info.height as i32) as f64);
                out.push(EvDevRawEvent::Pointer {
                    type_: RawPointerEventType::Move,
                    timestamp: time_usec / 1000,
                    position: self.mouse_position,
                    modifiers,
                });
            }
            LibInputEvent::PointerButton { time_usec, button, button_state } => {
                // The states of the library are 0 (released) and 1
                // (pressed). The reference maps the right button to "up"
                // for 1 and to "down" for 2, and the middle button to
                // "up" for 2, states that do not exist; corrected here
                // (DEVIATIONS.md).
                let type_ = match (u16::try_from(button).unwrap_or(0), button_state) {
                    (ev_key::BTN_LEFT, 1) => Some(RawPointerEventType::LeftButtonDown),
                    (ev_key::BTN_LEFT, 0) => Some(RawPointerEventType::LeftButtonUp),
                    (ev_key::BTN_RIGHT, 1) => Some(RawPointerEventType::RightButtonDown),
                    (ev_key::BTN_RIGHT, 0) => Some(RawPointerEventType::RightButtonUp),
                    (ev_key::BTN_MIDDLE, 1) => Some(RawPointerEventType::MiddleButtonDown),
                    (ev_key::BTN_MIDDLE, 0) => Some(RawPointerEventType::MiddleButtonUp),
                    _ => None,
                };
                match type_ {
                    Some(type_) => out.push(EvDevRawEvent::Pointer {
                        type_,
                        timestamp: time_usec / 1000,
                        position: self.mouse_position,
                        modifiers,
                    }),
                    None => log(LogEventLevel::Warning, &format!("The button {button} is not associated")),
                }
            }
            // Backward compatibility with low-res wheel
            LibInputEvent::PointerAxis { time_usec, source_axis, value_discrete } => match source_axis {
                LIBINPUT_POINTER_AXIS_SOURCE_WHEEL => out.push(EvDevRawEvent::Wheel {
                    timestamp: time_usec / 1000,
                    position: self.mouse_position,
                    delta: Vector::new(0.0, -value_discrete),
                    modifiers,
                }),
                _ => log(LogEventLevel::Debug, &format!("The pointer axis {source_axis} is not managed.")),
            },
            // Hi-Res wheel
            LibInputEvent::PointerScrollWheel { time_usec, value_v120 } => out.push(EvDevRawEvent::Wheel {
                timestamp: time_usec / 1000,
                position: self.mouse_position,
                delta: Vector::new(0.0, -value_v120 / 120.0),
                modifiers,
            }),
            LibInputEvent::PointerOther { type_ } => {
                log(LogEventLevel::Warning, &format!("The pointer event {type_} is not mapped."));
            }
            LibInputEvent::Touch { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::super::lib_input_backend::{calibration_matrix, is_pointer_event, is_touch_event};
    use super::super::lib_input_native_unsafe_methods::lib_input_event_type::*;
    use super::*;
    use ferroui_base::platform::SurfaceOrientation;

    fn map(state: &mut LibInputState, ev: LibInputEvent) -> Vec<EvDevRawEvent> {
        let mut out = Vec::new();
        state.handle(&ev, Size::new(800.5, 480.0), true, &mut out);
        out
    }

    fn pointer(type_: RawPointerEventType, timestamp: u64, x: f64, y: f64) -> EvDevRawEvent {
        EvDevRawEvent::Pointer { type_, timestamp, position: Point::new(x, y), modifiers: RawInputModifiers::empty() }
    }

    #[test]
    fn absolute_motion_moves_the_pointer_and_buttons_are_at_its_position() {
        let mut state = LibInputState::new();
        let moved = map(&mut state, LibInputEvent::PointerMotionAbsolute { time_usec: 5_000, x: 0.5, y: 0.25 });
        // The width is truncated to a whole number, as the reference passes it.
        assert_eq!(moved, [pointer(RawPointerEventType::Move, 5, 400.0, 120.0)]);

        let button = |button: u16, button_state| LibInputEvent::PointerButton {
            time_usec: 6_000,
            button: button as u32,
            button_state,
        };
        for (code, state_value, expected) in [
            (ev_key::BTN_LEFT, 1, RawPointerEventType::LeftButtonDown),
            (ev_key::BTN_LEFT, 0, RawPointerEventType::LeftButtonUp),
            (ev_key::BTN_RIGHT, 1, RawPointerEventType::RightButtonDown),
            (ev_key::BTN_RIGHT, 0, RawPointerEventType::RightButtonUp),
            (ev_key::BTN_MIDDLE, 1, RawPointerEventType::MiddleButtonDown),
            (ev_key::BTN_MIDDLE, 0, RawPointerEventType::MiddleButtonUp),
        ] {
            assert_eq!(map(&mut state, button(code, state_value)), [pointer(expected, 6, 400.0, 120.0)]);
        }
        // A button the framework has no event for.
        assert_eq!(map(&mut state, button(0x113, 1)), []);
    }

    #[test]
    fn a_wheel_step_scrolls_and_other_axis_sources_do_not() {
        let mut state = LibInputState::new();
        let wheel = |y: f64| EvDevRawEvent::Wheel {
            timestamp: 1,
            position: Point::default(),
            delta: Vector::new(0.0, y),
            modifiers: RawInputModifiers::empty(),
        };
        let axis =
            |source_axis, value_discrete| LibInputEvent::PointerAxis { time_usec: 1_000, source_axis, value_discrete };
        // A step towards the user is positive for the library and scrolls down.
        assert_eq!(map(&mut state, axis(LIBINPUT_POINTER_AXIS_SOURCE_WHEEL, 1.0)), [wheel(-1.0)]);
        assert_eq!(map(&mut state, axis(LIBINPUT_POINTER_AXIS_SOURCE_FINGER, 0.0)), []);
        assert_eq!(
            map(&mut state, LibInputEvent::PointerScrollWheel { time_usec: 1_000, value_v120: -240.0 }),
            [wheel(2.0)]
        );
        assert_eq!(map(&mut state, LibInputEvent::PointerOther { type_: LIBINPUT_EVENT_POINTER_MOTION }), []);
    }

    #[test]
    fn the_thread_forwards_touch_events_and_pointer_events_up_to_the_legacy_axis() {
        assert!(is_touch_event(LIBINPUT_EVENT_TOUCH_DOWN) && is_touch_event(LIBINPUT_EVENT_TOUCH_CANCEL));
        assert!(!is_touch_event(LIBINPUT_EVENT_TOUCH_FRAME));
        assert!(is_pointer_event(LIBINPUT_EVENT_POINTER_MOTION) && is_pointer_event(LIBINPUT_EVENT_POINTER_AXIS));
        assert!(!is_pointer_event(LIBINPUT_EVENT_POINTER_SCROLL_WHEEL));
        assert!(!is_pointer_event(LIBINPUT_EVENT_KEYBOARD_KEY));
    }

    #[test]
    fn the_calibration_follows_the_orientation() {
        assert_eq!(calibration_matrix(SurfaceOrientation::Rotation0), [1.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
        assert_eq!(calibration_matrix(SurfaceOrientation::Rotation90), [0.0, 1.0, 0.0, -1.0, 0.0, 1.0]);
        assert_eq!(calibration_matrix(SurfaceOrientation::Rotation180), [-1.0, 0.0, 1.0, 0.0, -1.0, 1.0]);
        assert_eq!(calibration_matrix(SurfaceOrientation::Rotation270), [0.0, -1.0, 1.0, 1.0, 0.0, 0.0]);
    }
}
