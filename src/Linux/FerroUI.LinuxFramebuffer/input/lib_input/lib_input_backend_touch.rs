//! The touch events of the `libinput` backend (the port of
//! `LibInputBackend.Touch.cs`).

use super::lib_input_backend::{LibInputEvent, LibInputState};
use super::lib_input_native_unsafe_methods::lib_input_event_type::*;
use crate::input::ev_dev::EvDevRawEvent;
use ferroui_base::input::raw::RawPointerEventType;
use ferroui_base::{Point, Size};

impl LibInputState {
    pub(crate) fn handle_touch(
        &mut self,
        ev: &LibInputEvent,
        info: Size,
        has_input_root: bool,
        out: &mut Vec<EvDevRawEvent>,
    ) {
        let LibInputEvent::Touch { type_, slot, time_usec, x, y } = *ev else {
            return;
        };

        if type_ < LIBINPUT_EVENT_TOUCH_FRAME {
            let pt;
            if type_ == LIBINPUT_EVENT_TOUCH_DOWN || type_ == LIBINPUT_EVENT_TOUCH_MOTION {
                // The library transforms to a whole width and height.
                pt = Point::new(x * (info.width as i32) as f64, y * (info.height as i32) as f64);
                self.pointers.insert(slot, pt);
            } else {
                pt = self.pointers.remove(&slot).unwrap_or_default();
            }

            let ts = time_usec / 1000;
            if !has_input_root {
                return;
            }

            out.push(EvDevRawEvent::Touch {
                type_: if type_ == LIBINPUT_EVENT_TOUCH_DOWN {
                    RawPointerEventType::TouchBegin
                } else if type_ == LIBINPUT_EVENT_TOUCH_UP {
                    RawPointerEventType::TouchEnd
                } else if type_ == LIBINPUT_EVENT_TOUCH_MOTION {
                    RawPointerEventType::TouchUpdate
                } else {
                    RawPointerEventType::TouchCancel
                },
                timestamp: ts,
                position: pt,
                raw_pointer_id: slot as i64,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    fn touch(type_: i32, slot: i32, x: f64, y: f64) -> LibInputEvent {
        LibInputEvent::Touch { type_, slot, time_usec: 7_000, x, y }
    }

    fn expected(type_: RawPointerEventType, x: f64, y: f64, id: i64) -> EvDevRawEvent {
        EvDevRawEvent::Touch { type_, timestamp: 7, position: Point::new(x, y), raw_pointer_id: id }
    }

    #[test]
    fn contacts_are_kept_by_slot_and_end_where_they_were_last() {
        let mut state = LibInputState::new();
        let screen = Size::new(1000.0, 500.0);
        let mut out = Vec::new();
        for ev in [
            touch(LIBINPUT_EVENT_TOUCH_DOWN, 0, 0.1, 0.2),
            touch(LIBINPUT_EVENT_TOUCH_DOWN, 1, 0.5, 0.5),
            touch(LIBINPUT_EVENT_TOUCH_MOTION, 0, 0.2, 0.2),
            // An up and a cancel carry no position.
            touch(LIBINPUT_EVENT_TOUCH_UP, 0, 0.0, 0.0),
            touch(LIBINPUT_EVENT_TOUCH_CANCEL, 1, 0.0, 0.0),
            // A slot that is not known ends at the origin.
            touch(LIBINPUT_EVENT_TOUCH_UP, 5, 0.0, 0.0),
        ] {
            state.handle(&ev, screen, true, &mut out);
        }
        assert_eq!(
            out,
            [
                expected(RawPointerEventType::TouchBegin, 100.0, 100.0, 0),
                expected(RawPointerEventType::TouchBegin, 500.0, 250.0, 1),
                expected(RawPointerEventType::TouchUpdate, 200.0, 100.0, 0),
                expected(RawPointerEventType::TouchEnd, 200.0, 100.0, 0),
                expected(RawPointerEventType::TouchCancel, 500.0, 250.0, 1),
                expected(RawPointerEventType::TouchEnd, 0.0, 0.0, 5),
            ]
        );
        assert!(state.pointers.is_empty());
    }

    #[test]
    fn without_an_input_root_the_contact_is_remembered_and_nothing_is_reported() {
        let mut state = LibInputState::new();
        let mut out = Vec::new();
        state.handle(&touch(LIBINPUT_EVENT_TOUCH_DOWN, 0, 0.5, 0.5), Size::new(100.0, 100.0), false, &mut out);
        assert!(out.is_empty());
        state.handle(&touch(LIBINPUT_EVENT_TOUCH_UP, 0, 0.0, 0.0), Size::new(100.0, 100.0), true, &mut out);
        assert_eq!(out, [expected(RawPointerEventType::TouchEnd, 50.0, 50.0, 0)]);
    }
}
