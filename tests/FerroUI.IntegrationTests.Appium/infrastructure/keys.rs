//! The keys of the WebDriver protocol (`OpenQA.Selenium.Keys` of upstream's
//! tests): characters of the private use area that name a key that is not a
//! character. A string the tests send is a mix of such keys and of text.

/// A key that is not a character of text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NamedKey {
    Null,
    Cancel,
    Help,
    Backspace,
    Tab,
    Clear,
    Return,
    Enter,
    Shift,
    Control,
    Alt,
    Pause,
    Escape,
    Space,
    PageUp,
    PageDown,
    End,
    Home,
    Left,
    Up,
    Right,
    Down,
    Insert,
    Delete,
    F(u8),
    Meta,
}

impl NamedKey {
    /// Whether the key is a modifier: pressed by one occurrence in a string
    /// of keys and released by the next (the rule of the protocol).
    pub fn is_modifier(self) -> bool {
        matches!(self, NamedKey::Shift | NamedKey::Control | NamedKey::Alt | NamedKey::Meta)
    }
}

/// One key of a string of keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Named(NamedKey),
    Char(char),
}

/// The constants of `Keys` the tests use, as strings, so that they are
/// concatenated as upstream concatenates them (`Keys.LeftAlt + Keys.ArrowDown`).
pub struct Keys;

impl Keys {
    pub const NULL: &'static str = "\u{E000}";
    pub const BACKSPACE: &'static str = "\u{E003}";
    pub const TAB: &'static str = "\u{E004}";
    pub const RETURN: &'static str = "\u{E006}";
    pub const ENTER: &'static str = "\u{E007}";
    pub const SHIFT: &'static str = "\u{E008}";
    pub const LEFT_SHIFT: &'static str = "\u{E008}";
    pub const CONTROL: &'static str = "\u{E009}";
    pub const LEFT_CONTROL: &'static str = "\u{E009}";
    pub const ALT: &'static str = "\u{E00A}";
    pub const LEFT_ALT: &'static str = "\u{E00A}";
    pub const ESCAPE: &'static str = "\u{E00C}";
    pub const SPACE: &'static str = "\u{E00D}";
    pub const PAGE_UP: &'static str = "\u{E00E}";
    pub const PAGE_DOWN: &'static str = "\u{E00F}";
    pub const END: &'static str = "\u{E010}";
    pub const HOME: &'static str = "\u{E011}";
    pub const LEFT: &'static str = "\u{E012}";
    pub const ARROW_LEFT: &'static str = "\u{E012}";
    pub const UP: &'static str = "\u{E013}";
    pub const ARROW_UP: &'static str = "\u{E013}";
    pub const RIGHT: &'static str = "\u{E014}";
    pub const ARROW_RIGHT: &'static str = "\u{E014}";
    pub const DOWN: &'static str = "\u{E015}";
    pub const ARROW_DOWN: &'static str = "\u{E015}";
    pub const INSERT: &'static str = "\u{E016}";
    pub const DELETE: &'static str = "\u{E017}";
    pub const F4: &'static str = "\u{E034}";
    pub const META: &'static str = "\u{E03D}";
    pub const COMMAND: &'static str = "\u{E03D}";
}

/// The key a character of a string of keys stands for.
pub fn key_of(c: char) -> Key {
    let named = match c as u32 {
        0xE000 => NamedKey::Null,
        0xE001 => NamedKey::Cancel,
        0xE002 => NamedKey::Help,
        0xE003 => NamedKey::Backspace,
        0xE004 => NamedKey::Tab,
        0xE005 => NamedKey::Clear,
        0xE006 => NamedKey::Return,
        0xE007 => NamedKey::Enter,
        0xE008 => NamedKey::Shift,
        0xE009 => NamedKey::Control,
        0xE00A => NamedKey::Alt,
        0xE00B => NamedKey::Pause,
        0xE00C => NamedKey::Escape,
        0xE00D => NamedKey::Space,
        0xE00E => NamedKey::PageUp,
        0xE00F => NamedKey::PageDown,
        0xE010 => NamedKey::End,
        0xE011 => NamedKey::Home,
        0xE012 => NamedKey::Left,
        0xE013 => NamedKey::Up,
        0xE014 => NamedKey::Right,
        0xE015 => NamedKey::Down,
        0xE016 => NamedKey::Insert,
        0xE017 => NamedKey::Delete,
        code @ 0xE031..=0xE03C => NamedKey::F((code - 0xE030) as u8),
        0xE03D => NamedKey::Meta,
        _ => return Key::Char(c),
    };
    Key::Named(named)
}

