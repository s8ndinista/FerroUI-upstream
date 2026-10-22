use crate::FontStyle;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum StandardTokenType {
    #[default]
    Other = 0,
    Comment = 1,
    String = 2,
    RegEx = 3,
}

/// Packed attributes with the same bit layout as the TextMate token format.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EncodedTokenAttributes(pub u32);
impl EncodedTokenAttributes {
    pub const fn new(
        language: u8,
        token_type: StandardTokenType,
        balanced: bool,
        style: FontStyle,
        foreground: u16,
        background: u8,
    ) -> Self {
        Self(
            language as u32
                | (token_type as u32) << 8
                | (balanced as u32) << 10
                | ((style.0 as u32) & 15) << 11
                | ((foreground as u32) & 511) << 15
                | (background as u32) << 24,
        )
    }
    pub const fn language_id(self) -> u8 {
        self.0 as u8
    }
    pub const fn token_type(self) -> StandardTokenType {
        match (self.0 >> 8) & 3 {
            1 => StandardTokenType::Comment,
            2 => StandardTokenType::String,
            3 => StandardTokenType::RegEx,
            _ => StandardTokenType::Other,
        }
    }
    pub const fn contains_balanced_brackets(self) -> bool {
        self.0 & (1 << 10) != 0
    }
    pub const fn font_style(self) -> FontStyle {
        FontStyle(((self.0 >> 11) & 15) as u8)
    }
    pub const fn foreground(self) -> u16 {
        ((self.0 >> 15) & 511) as u16
    }
    pub const fn background(self) -> u8 {
        (self.0 >> 24) as u8
    }
    pub fn infer_token_type(scopes: &[String]) -> StandardTokenType {
        scopes
            .iter()
            .rev()
            .find_map(|scope| {
                if scope == "comment" || scope.starts_with("comment.") {
                    Some(StandardTokenType::Comment)
                } else if scope == "string" || scope.starts_with("string.") {
                    Some(StandardTokenType::String)
                } else if scope == "regex" || scope.starts_with("regex.") {
                    Some(StandardTokenType::RegEx)
                } else if scope == "meta.embedded" || scope.starts_with("meta.embedded.") {
                    Some(StandardTokenType::Other)
                } else {
                    None
                }
            })
            .unwrap_or_default()
    }
}
