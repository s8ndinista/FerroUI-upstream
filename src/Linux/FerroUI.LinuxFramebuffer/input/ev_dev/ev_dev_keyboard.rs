//! The handler of a keyboard.
//!
//! An addition (docs/porting/linux-framebuffer-platform.md, section 7):
//! the reference reads no keys. The keys are those of a US keyboard: no
//! layout is read from the system.

use super::ev_dev_key_table::physical_key_from_ev_key;
use super::{EvDevDeviceHandler, EvDevRawEvent};
use crate::native_unsafe_methods::{ev_type, input_event};
use ferroui_base::input::raw::RawKeyEventType;
use ferroui_base::input::{PhysicalKey, RawInputModifiers};
use ferroui_base::Size;

/// The description of a keyboard: `(path):type=keyboard`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EvDevKeyboardDeviceDescription {
    pub path: Option<String>,
}

#[derive(Default)]
pub struct EvDevKeyboard {
    /// The modifier keys that are held.
    held: Vec<PhysicalKey>,
    caps_lock: bool,
}

fn modifier_of(key: PhysicalKey) -> Option<RawInputModifiers> {
    match key {
        PhysicalKey::AltLeft | PhysicalKey::AltRight => Some(RawInputModifiers::ALT),
        PhysicalKey::ControlLeft | PhysicalKey::ControlRight => Some(RawInputModifiers::CONTROL),
        PhysicalKey::ShiftLeft | PhysicalKey::ShiftRight => Some(RawInputModifiers::SHIFT),
        PhysicalKey::MetaLeft | PhysicalKey::MetaRight => Some(RawInputModifiers::META),
        _ => None,
    }
}

impl EvDevKeyboard {
    pub fn new() -> Self {
        Self::default()
    }

    fn modifiers(&self) -> RawInputModifiers {
        self.held.iter().filter_map(|&key| modifier_of(key)).fold(RawInputModifiers::empty(), |all, one| all | one)
    }
}

impl EvDevDeviceHandler for EvDevKeyboard {
    fn handle_event(&mut self, ev: &input_event, _screen_size: Size, out: &mut Vec<EvDevRawEvent>) {
        if ev.type_ != ev_type::EV_KEY {
            return;
        }
        let physical_key = physical_key_from_ev_key(ev.code);
        if physical_key == PhysicalKey::None {
            return;
        }

        // 1 is a press, 2 a repeat of a held key, 0 a release.
        let down = ev.value != 0;
        if modifier_of(physical_key).is_some() {
            self.held.retain(|&held| held != physical_key);
            if down {
                self.held.push(physical_key);
            }
        }
        if physical_key == PhysicalKey::CapsLock && ev.value == 1 {
            self.caps_lock = !self.caps_lock;
        }

        let modifiers = self.modifiers();
        let shift = modifiers.contains(RawInputModifiers::SHIFT);
        let is_letter = (PhysicalKey::A as i32..=PhysicalKey::Z as i32).contains(&(physical_key as i32));
        let key_symbol = physical_key.to_qwerty_key_symbol(shift != (self.caps_lock && is_letter));
        let timestamp = ev.timestamp();

        out.push(EvDevRawEvent::Key {
            type_: if down { RawKeyEventType::KeyDown } else { RawKeyEventType::KeyUp },
            timestamp,
            key: physical_key.to_qwerty_key(),
            modifiers,
            physical_key,
            key_symbol: key_symbol.map(str::to_owned),
        });

        let commands = RawInputModifiers::CONTROL | RawInputModifiers::ALT | RawInputModifiers::META;
        if let Some(text) = key_symbol {
            let printable = text.chars().all(|c| !c.is_control());
            if down && printable && !modifiers.intersects(commands) {
                out.push(EvDevRawEvent::Text { timestamp, text: text.to_owned() });
            }
        }
    }

    fn reset(&mut self, _timestamp: u64, _screen_size: Size, _out: &mut Vec<EvDevRawEvent>) {
        self.held.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::super::ev_dev_touch_screen::tests::{ev, run, syn};
    use super::*;
    use ferroui_base::input::Key;

    const KEY_A: u16 = 30;
    const KEY_1: u16 = 2;
    const KEY_ENTER: u16 = 28;
    const KEY_LEFTSHIFT: u16 = 42;
    const KEY_LEFTCTRL: u16 = 29;
    const KEY_CAPSLOCK: u16 = 58;

    fn key(code: u16, value: i32) -> input_event {
        ev(10, ev_type::EV_KEY, code, value)
    }

    /// The events as `(down, key, modifiers, symbol)` and texts.
    fn summary(events: &[input_event]) -> Vec<String> {
        let mut keyboard = EvDevKeyboard::new();
        run(&mut keyboard, events, Size::default())
            .into_iter()
            .map(|event| match event {
                EvDevRawEvent::Key { type_, key, modifiers, key_symbol, .. } => {
                    let direction = if type_ == RawKeyEventType::KeyDown { "down" } else { "up" };
                    format!("{direction} {key:?} {} {:?}", modifiers.bits(), key_symbol.unwrap_or_default())
                }
                EvDevRawEvent::Text { text, .. } => format!("text {text:?}"),
                other => panic!("unexpected {other:?}"),
            })
            .collect()
    }

    #[test]
    fn a_key_is_a_down_with_its_text_and_an_up() {
        // The scan code and the report that surround a key are passed over.
        let events = [ev(10, ev_type::EV_MSC, 4, 0x70004), key(KEY_A, 1), syn(10), key(KEY_A, 0), syn(11)];
        assert_eq!(summary(&events), ["down A 0 \"a\"", "text \"a\"", "up A 0 \"a\""]);
    }

    #[test]
    fn a_held_key_repeats_as_further_downs() {
        let events = [key(KEY_A, 1), key(KEY_A, 2), key(KEY_A, 2), key(KEY_A, 0)];
        assert_eq!(
            summary(&events),
            [
                "down A 0 \"a\"",
                "text \"a\"",
                "down A 0 \"a\"",
                "text \"a\"",
                "down A 0 \"a\"",
                "text \"a\"",
                "up A 0 \"a\""
            ]
        );
    }

    #[test]
    fn shift_changes_the_symbol_and_is_a_modifier_while_held() {
        let events = [key(KEY_LEFTSHIFT, 1), key(KEY_A, 1), key(KEY_1, 1), key(KEY_LEFTSHIFT, 0), key(KEY_A, 1)];
        assert_eq!(
            summary(&events),
            [
                "down LeftShift 4 \"\"",
                "down A 4 \"A\"",
                "text \"A\"",
                "down D1 4 \"!\"",
                "text \"!\"",
                "up LeftShift 0 \"\"",
                "down A 0 \"a\"",
                "text \"a\"",
            ]
        );
    }

    #[test]
    fn caps_lock_changes_letters_only_and_shift_undoes_it() {
        let events = [
            key(KEY_CAPSLOCK, 1),
            key(KEY_CAPSLOCK, 0),
            key(KEY_A, 1),
            key(KEY_1, 1),
            key(KEY_LEFTSHIFT, 1),
            key(KEY_A, 1),
        ];
        let summary = summary(&events);
        assert_eq!(
            summary[2..],
            [
                "down A 0 \"A\"",
                "text \"A\"",
                "down D1 0 \"1\"",
                "text \"1\"",
                "down LeftShift 4 \"\"",
                "down A 4 \"a\"",
                "text \"a\""
            ]
        );
    }

    #[test]
    fn a_key_with_control_and_a_key_without_a_printable_symbol_have_no_text() {
        let events = [key(KEY_ENTER, 1), key(KEY_LEFTCTRL, 1), key(KEY_A, 1)];
        assert_eq!(summary(&events), ["down Return 0 \"\\r\"", "down LeftCtrl 2 \"\"", "down A 2 \"a\""]);
        assert_eq!(physical_key_from_ev_key(KEY_ENTER).to_qwerty_key(), Key::Return);
    }
}
