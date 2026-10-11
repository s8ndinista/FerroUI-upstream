//! The kernel's key codes as physical keys.
//!
//! An addition (docs/porting/linux-framebuffer-platform.md, section 7):
//! the reference reads no keys. The codes are those of
//! `linux/input-event-codes.h`; they name positions on the keyboard, which
//! is what a physical key is.

use ferroui_base::input::PhysicalKey;

/// The physical key of a key code (`KEY_*`), or `None` for a code that is
/// not a key of a keyboard the framework knows.
pub fn physical_key_from_ev_key(code: u16) -> PhysicalKey {
    use PhysicalKey::*;
    match code {
        1 => Escape,
        2 => Digit1,
        3 => Digit2,
        4 => Digit3,
        5 => Digit4,
        6 => Digit5,
        7 => Digit6,
        8 => Digit7,
        9 => Digit8,
        10 => Digit9,
        11 => Digit0,
        12 => Minus,
        13 => Equal,
        14 => Backspace,
        15 => Tab,
        16 => Q,
        17 => W,
        18 => E,
        19 => R,
        20 => T,
        21 => Y,
        22 => U,
        23 => I,
        24 => O,
        25 => P,
        26 => BracketLeft,
        27 => BracketRight,
        28 => Enter,
        29 => ControlLeft,
        30 => A,
        31 => S,
        32 => D,
        33 => F,
        34 => G,
        35 => H,
        36 => J,
        37 => K,
        38 => L,
        39 => Semicolon,
        40 => Quote,
        41 => Backquote,
        42 => ShiftLeft,
        43 => Backslash,
        44 => Z,
        45 => X,
        46 => C,
        47 => V,
        48 => B,
        49 => N,
        50 => M,
        51 => Comma,
        52 => Period,
        53 => Slash,
        54 => ShiftRight,
        55 => NumPadMultiply,
        56 => AltLeft,
        57 => Space,
        58 => CapsLock,
        59 => F1,
        60 => F2,
        61 => F3,
        62 => F4,
        63 => F5,
        64 => F6,
        65 => F7,
        66 => F8,
        67 => F9,
        68 => F10,
        69 => NumLock,
        70 => ScrollLock,
        71 => NumPad7,
        72 => NumPad8,
        73 => NumPad9,
        74 => NumPadSubtract,
        75 => NumPad4,
        76 => NumPad5,
        77 => NumPad6,
        78 => NumPadAdd,
        79 => NumPad1,
        80 => NumPad2,
        81 => NumPad3,
        82 => NumPad0,
        83 => NumPadDecimal,
        85 => Lang5,
        86 => IntlBackslash,
        87 => F11,
        88 => F12,
        89 => IntlRo,
        90 => Lang3,
        91 => Lang4,
        92 => Convert,
        93 => KanaMode,
        94 => NonConvert,
        95 => NumPadComma,
        96 => NumPadEnter,
        97 => ControlRight,
        98 => NumPadDivide,
        99 => PrintScreen,
        100 => AltRight,
        102 => Home,
        103 => ArrowUp,
        104 => PageUp,
        105 => ArrowLeft,
        106 => ArrowRight,
        107 => End,
        108 => ArrowDown,
        109 => PageDown,
        110 => Insert,
        111 => Delete,
        113 => AudioVolumeMute,
        114 => AudioVolumeDown,
        115 => AudioVolumeUp,
        116 => Power,
        117 => NumPadEqual,
        119 => Pause,
        121 => NumPadComma,
        122 => Lang1,
        123 => Lang2,
        124 => IntlYen,
        125 => MetaLeft,
        126 => MetaRight,
        127 => ContextMenu,
        128 => BrowserStop,
        129 => Again,
        130 => Props,
        131 => Undo,
        133 => Copy,
        134 => Open,
        135 => Paste,
        136 => Find,
        137 => Cut,
        138 => Help,
        140 => LaunchApp2,
        142 => Sleep,
        143 => WakeUp,
        155 => LaunchMail,
        156 => BrowserFavorites,
        158 => BrowserBack,
        159 => BrowserForward,
        161 => Eject,
        163 => MediaTrackNext,
        164 => MediaPlayPause,
        165 => MediaTrackPrevious,
        166 => MediaStop,
        172 => BrowserHome,
        173 => BrowserRefresh,
        179 => NumPadParenLeft,
        180 => NumPadParenRight,
        183 => F13,
        184 => F14,
        185 => F15,
        186 => F16,
        187 => F17,
        188 => F18,
        189 => F19,
        190 => F20,
        191 => F21,
        192 => F22,
        193 => F23,
        194 => F24,
        217 => BrowserSearch,
        226 => MediaSelect,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::input::Key;
    use std::collections::HashSet;

    #[test]
    fn the_rows_of_a_keyboard_are_in_their_places() {
        // KEY_Q .. KEY_P, KEY_A .. KEY_L, KEY_Z .. KEY_M.
        let row = |codes: std::ops::RangeInclusive<u16>| -> String {
            codes.map(|code| physical_key_from_ev_key(code).to_qwerty_key_symbol(false).unwrap()).collect()
        };
        assert_eq!(row(16..=25), "qwertyuiop");
        assert_eq!(row(30..=38), "asdfghjkl");
        assert_eq!(row(44..=50), "zxcvbnm");
        assert_eq!(row(2..=11), "1234567890");
        assert_eq!(row(71..=73), "789");
    }

    #[test]
    fn the_keys_the_framework_names_are_the_kernels() {
        for (code, key) in [
            (1, Key::Escape),
            (14, Key::Back),
            (15, Key::Tab),
            (28, Key::Return),
            (57, Key::Space),
            (59, Key::F1),
            (88, Key::F12),
            (96, Key::Return),
            (102, Key::Home),
            (103, Key::Up),
            (105, Key::Left),
            (106, Key::Right),
            (108, Key::Down),
            (111, Key::Delete),
        ] {
            assert_eq!(physical_key_from_ev_key(code).to_qwerty_key(), key, "code {code}");
        }
        assert_eq!(physical_key_from_ev_key(0), PhysicalKey::None);
        // The buttons are not keys.
        assert_eq!(physical_key_from_ev_key(0x110), PhysicalKey::None);
    }

    #[test]
    fn no_two_codes_but_the_two_keypad_commas_share_a_key() {
        let mut seen = HashSet::new();
        for code in 0..0x300u16 {
            let key = physical_key_from_ev_key(code);
            if key != PhysicalKey::None && code != 121 {
                assert!(seen.insert(key), "code {code} repeats {key:?}");
            }
        }
        assert!(seen.len() > 140);
    }
}
