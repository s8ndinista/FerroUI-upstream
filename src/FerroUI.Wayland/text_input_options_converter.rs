//! Maps the [`TextInputOptions`] of the framework onto the
//! `zwp_text_input_v3` `(content_hint, content_purpose)` pair (the port of
//! `TextInputOptionsConverter.cs`).
//!
//! The reference returns the enumerations of its generated bindings. The
//! bindings of the port exist on Linux only, so the two types are declared
//! here with the values of the protocol (`text-input-unstable-v3.xml`) and are
//! turned into the types of the bindings where a request is sent
//! (`server/transient/wayland_text_input_v3.rs`); the conversion is the same
//! on every system, which is where its tests run.

use ferroui_base::input::text_input::{TextInputContentType, TextInputOptions};

bitflags::bitflags! {
    /// `zwp_text_input_v3.content_hint`: a bit field.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct ContentHint: u32 {
        const NONE = 0x0;
        const COMPLETION = 0x1;
        const SPELLCHECK = 0x2;
        const AUTO_CAPITALIZATION = 0x4;
        const LOWERCASE = 0x8;
        const UPPERCASE = 0x10;
        const TITLECASE = 0x20;
        const HIDDEN_TEXT = 0x40;
        const SENSITIVE_DATA = 0x80;
        const LATIN = 0x100;
        const MULTILINE = 0x200;
    }
}

/// `zwp_text_input_v3.content_purpose`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ContentPurpose {
    Normal = 0,
    Alpha = 1,
    Digits = 2,
    Number = 3,
    Phone = 4,
    Url = 5,
    Email = 6,
    Name = 7,
    Password = 8,
    Pin = 9,
    Date = 10,
    Time = 11,
    Datetime = 12,
    Terminal = 13,
}

pub fn convert(options: &TextInputOptions) -> (ContentHint, ContentPurpose) {
    let mut hint = ContentHint::NONE;
    let mut purpose = ContentPurpose::Normal;

    match options.content_type {
        TextInputContentType::Normal => {}
        TextInputContentType::Alpha => hint |= ContentHint::LATIN,
        TextInputContentType::Digits => purpose = ContentPurpose::Digits,
        TextInputContentType::Pin => purpose = ContentPurpose::Pin,
        TextInputContentType::Number => purpose = ContentPurpose::Number,
        TextInputContentType::Email => purpose = ContentPurpose::Email,
        TextInputContentType::Url => purpose = ContentPurpose::Url,
        TextInputContentType::Name => purpose = ContentPurpose::Name,
        TextInputContentType::Password => {
            purpose = ContentPurpose::Password;
            hint |= ContentHint::HIDDEN_TEXT | ContentHint::SENSITIVE_DATA;
        }
        TextInputContentType::Social | TextInputContentType::Search => {}
    }

    if options.multiline {
        hint |= ContentHint::MULTILINE;
    }
    if options.is_sensitive {
        hint |= ContentHint::SENSITIVE_DATA;
    }
    if options.lowercase {
        hint |= ContentHint::LOWERCASE;
    }
    if options.uppercase {
        hint |= ContentHint::UPPERCASE;
    }
    if options.auto_capitalization {
        hint |= ContentHint::AUTO_CAPITALIZATION;
    }

    if options.show_suggestions == Some(true) {
        hint |= ContentHint::COMPLETION | ContentHint::SPELLCHECK;
    }

    // Password always overrides: even with ShowSuggestions=true, never reveal text.
    if options.content_type == TextInputContentType::Password {
        hint &= !(ContentHint::COMPLETION | ContentHint::SPELLCHECK);
        hint |= ContentHint::HIDDEN_TEXT | ContentHint::SENSITIVE_DATA;
    }

    (hint, purpose)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_none_normal() {
        let (h, p) = convert(&TextInputOptions::default());
        assert_eq!(h, ContentHint::NONE);
        assert_eq!(p, ContentPurpose::Normal);
    }

    #[test]
    fn content_type_maps_purpose() {
        let cases = [
            (TextInputContentType::Digits, ContentPurpose::Digits),
            (TextInputContentType::Pin, ContentPurpose::Pin),
            (TextInputContentType::Number, ContentPurpose::Number),
            (TextInputContentType::Email, ContentPurpose::Email),
            (TextInputContentType::Url, ContentPurpose::Url),
            (TextInputContentType::Name, ContentPurpose::Name),
        ];
        for (content_type, expected) in cases {
            let (_, p) = convert(&TextInputOptions { content_type, ..Default::default() });
            assert_eq!(p, expected);
        }
    }

    #[test]
    fn alpha_adds_latin_hint() {
        let (h, _) = convert(&TextInputOptions { content_type: TextInputContentType::Alpha, ..Default::default() });
        assert!(h.contains(ContentHint::LATIN));
    }

    #[test]
    fn password_forces_hidden_and_sensitive_even_with_show_suggestions() {
        let (h, p) = convert(&TextInputOptions {
            content_type: TextInputContentType::Password,
            show_suggestions: Some(true),
            ..Default::default()
        });
        assert_eq!(p, ContentPurpose::Password);
        assert!(h.contains(ContentHint::HIDDEN_TEXT));
        assert!(h.contains(ContentHint::SENSITIVE_DATA));
        assert!(!h.contains(ContentHint::COMPLETION));
        assert!(!h.contains(ContentHint::SPELLCHECK));
    }

    #[test]
    fn flags_compose_into_hint() {
        let (h, _) = convert(&TextInputOptions {
            multiline: true,
            is_sensitive: true,
            lowercase: true,
            uppercase: true,
            auto_capitalization: true,
            show_suggestions: Some(true),
            ..Default::default()
        });
        assert!(h.contains(ContentHint::MULTILINE));
        assert!(h.contains(ContentHint::SENSITIVE_DATA));
        assert!(h.contains(ContentHint::LOWERCASE));
        assert!(h.contains(ContentHint::UPPERCASE));
        assert!(h.contains(ContentHint::AUTO_CAPITALIZATION));
        assert!(h.contains(ContentHint::COMPLETION));
        assert!(h.contains(ContentHint::SPELLCHECK));
    }

    #[test]
    fn show_suggestions_false_omits_completion() {
        let (h, _) = convert(&TextInputOptions { show_suggestions: Some(false), ..Default::default() });
        assert!(!h.contains(ContentHint::COMPLETION));
        assert!(!h.contains(ContentHint::SPELLCHECK));
    }
}
