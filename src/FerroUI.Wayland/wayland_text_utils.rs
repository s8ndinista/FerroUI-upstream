//! Helpers for converting between UTF-8 byte offsets (used by Wayland
//! text-input protocols) and UTF-16 code unit indices, which is what the
//! text of the framework is indexed by (the port of `WaylandTextUtils.cs`).
//!
//! The Wayland text-input protocols address text by byte offset into the
//! UTF-8 encoding of the string. Conversion must respect multi-byte sequences
//! and surrogate pairs (a single non-BMP codepoint is 4 bytes in UTF-8 and 2
//! code units in UTF-16) and must be robust to negative offsets, out-of-range
//! indices and partially-truncated sequences.
//!
//! A string of the port is UTF-8, so an index that falls between the two
//! halves of a surrogate pair names half a character. The reference counts
//! such a half as the replacement character (three bytes); so does this.

/// The length of `text` in UTF-16 code units (`string.Length`).
pub fn utf16_length(text: &str) -> i32 {
    text.encode_utf16().count() as i32
}

/// The part of `text` between two UTF-16 indices (`Substring`). An index inside a
/// surrogate pair moves to the start of the pair.
pub fn utf16_substring(text: &str, start_char: i32, end_char: i32) -> &str {
    let byte_of = |index: i32| -> usize {
        let mut units = 0;
        for (byte, character) in text.char_indices() {
            if units + character.len_utf16() as i32 > index {
                return byte;
            }
            units += character.len_utf16() as i32;
        }
        text.len()
    };
    let start = byte_of(start_char.max(0));
    let end = byte_of(end_char.max(0)).max(start);
    &text[start..end]
}

/// One code point of a UTF-16 span: its length in UTF-8 bytes and in code units
/// (`Rune.DecodeFromUtf16`, which gives the replacement character and one unit for half a
/// surrogate pair).
fn decode(span: &[u16]) -> (i32, i32) {
    let first = span[0];
    if (0xD800..0xDC00).contains(&first) {
        if span.len() > 1 && (0xDC00..0xE000).contains(&span[1]) {
            return (4, 2);
        }
        return (3, 1);
    }
    if (0xDC00..0xE000).contains(&first) {
        return (3, 1);
    }
    let bytes = match first {
        0..=0x7F => 1,
        0x80..=0x7FF => 2,
        _ => 3,
    };
    (bytes, 1)
}

/// `Encoding.UTF8.GetByteCount` of a UTF-16 span.
fn utf8_byte_count(span: &[u16]) -> i32 {
    let mut bytes = 0;
    let mut i = 0;
    while i < span.len() {
        let (code_point_bytes, code_point_chars) = decode(&span[i..]);
        bytes += code_point_bytes;
        i += code_point_chars as usize;
    }
    bytes
}

/// Convert a wayland-side byte offset (relative to `base_char`) into a UTF-16
/// index in `text`. A negative `byte_length` means "go backwards from
/// base_char". Out-of-range offsets are clamped to `[0, text.Length]`.
pub fn char_index_from_utf8_offset(text: &str, byte_length: i32, base_char: i32) -> i32 {
    let units: Vec<u16> = text.encode_utf16().collect();
    let length = units.len() as i32;
    if byte_length == 0 {
        return base_char.clamp(0, length);
    }

    let base_char = base_char.clamp(0, length);

    if byte_length < 0 {
        let prefix = &units[..base_char as usize];
        let prefix_bytes = utf8_byte_count(prefix);
        let target_bytes = prefix_bytes.saturating_add(byte_length).max(0);
        count_chars_for_utf8_bytes(prefix, target_bytes)
    } else {
        let suffix = &units[base_char as usize..];
        let suffix_bytes = utf8_byte_count(suffix);
        let target_bytes = byte_length.min(suffix_bytes);
        base_char + count_chars_for_utf8_bytes(suffix, target_bytes)
    }
}

/// Convert a UTF-16 index `char_index` in `text` into a wayland-side byte
/// offset, relative to `base_char`. The returned value is the byte distance
/// between `base_char` and `char_index`; it is negative when
/// `char_index` < `base_char`.
pub fn utf8_offset_from_char_index(text: &str, char_index: i32, base_char: i32) -> i32 {
    if char_index == base_char {
        return 0;
    }

    let units: Vec<u16> = text.encode_utf16().collect();
    let length = units.len() as i32;
    let base_char = base_char.clamp(0, length) as usize;
    let char_index = char_index.clamp(0, length) as usize;

    if char_index >= base_char {
        return utf8_byte_count(&units[base_char..char_index]);
    }

    -utf8_byte_count(&units[char_index..base_char])
}

/// Convert an absolute UTF-16 index to its absolute UTF-8 byte index in
/// `text`. Equivalent to `utf8_offset_from_char_index(text, char_index, 0)`.
pub fn utf8_byte_index_from_char_index(text: &str, char_index: i32) -> i32 {
    let units: Vec<u16> = text.encode_utf16().collect();
    let char_index = char_index.clamp(0, units.len() as i32) as usize;
    utf8_byte_count(&units[..char_index])
}

/// Walk the UTF-16 `span` until either it is exhausted or the cumulative
/// UTF-8 byte count reaches `target_bytes`. Stops at a UTF-16 index that lies
/// on a UTF-8 codepoint boundary, i.e. it never returns a position halfway
/// through a surrogate pair.
fn count_chars_for_utf8_bytes(span: &[u16], target_bytes: i32) -> i32 {
    if target_bytes <= 0 {
        return 0;
    }

    let mut bytes = 0;
    let mut i = 0usize;
    while i < span.len() {
        let (code_point_bytes, code_point_chars) = decode(&span[i..]);

        if bytes + code_point_bytes > target_bytes {
            break;
        }

        bytes += code_point_bytes;
        i += code_point_chars as usize;
    }

    i as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    // Cyrillic 'й' = U+0439 → 2 UTF-8 bytes, 1 char.
    // CJK '日' = U+65E5 → 3 UTF-8 bytes, 1 char.
    // U+1F30D → 4 UTF-8 bytes, 2 chars (surrogate pair).
    // bytes: 1+2+3+4+1=11; chars: 1+1+1+2+1=6.
    const MIXED: &str = "a\u{439}\u{65e5}\u{1F30D}b";

    #[test]
    fn char_index_from_utf8_offset_cases() {
        let cases: &[(&str, i32, i32, i32)] = &[
            ("hello", 0, 0, 0),
            ("hello", 5, 0, 5),     // forward, ASCII
            ("hello", -3, 5, 2),    // backward, ASCII
            ("hello", 100, 0, 5),   // clamp forward
            ("hello", -100, 5, 0),  // clamp backward
            (MIXED, 0, 3, 3),       // zero distance from middle
            (MIXED, 1, 0, 1),       // 'a' = 1 byte
            (MIXED, 3, 0, 2),       // two characters = 3 bytes → 2 chars
            (MIXED, 6, 0, 3),       // three characters = 6 bytes → 3 chars
            (MIXED, 10, 0, 5),      // four characters = 10 bytes → 5 chars (surrogate pair counts as 2)
            (MIXED, 11, 0, 6),      // full
            (MIXED, -1, 6, 5),      // 'b' = 1 byte
            (MIXED, -5, 6, 3),      // 'b' + the pair = 5 bytes → drops 3 chars
            (MIXED, -11, 6, 0),     // back to start
            (MIXED, 1, 1, 1),       // partway: +1 byte cannot reach a clean boundary in a 2 byte character
        ];
        for (text, byte_length, base_char, expected_char) in cases {
            assert_eq!(
                char_index_from_utf8_offset(text, *byte_length, *base_char),
                *expected_char,
                "{text:?} {byte_length} {base_char}"
            );
        }
    }

    #[test]
    fn char_index_from_utf8_offset_does_not_split_multi_byte() {
        // From start, +2 bytes: only the 'a' (1 byte) fits cleanly before the 2 byte
        // character, so we should land at char 1, not partway through it.
        assert_eq!(char_index_from_utf8_offset(MIXED, 2, 0), 1);
        // +1 byte from char 1 should consume nothing since the character alone is 2 bytes.
        assert_eq!(char_index_from_utf8_offset(MIXED, 1, 1), 1);
    }

    #[test]
    fn char_index_from_utf8_offset_does_not_split_surrogate_pair() {
        // From start, +9 bytes = three characters (6) + 3 bytes into the pair (4): should
        // NOT return half a surrogate pair. Should clamp at 3 chars.
        assert_eq!(char_index_from_utf8_offset(MIXED, 9, 0), 3);
    }

    #[test]
    fn utf8_offset_from_char_index_cases() {
        let cases: &[(&str, i32, i32, i32)] = &[
            ("hello", 0, 0, 0),
            ("hello", 5, 0, 5),
            ("hello", 2, 5, -3),
            (MIXED, 0, 0, 0),
            (MIXED, 1, 0, 1),
            (MIXED, 2, 0, 3),
            (MIXED, 3, 0, 6),
            (MIXED, 5, 0, 10), // through the surrogate pair
            (MIXED, 6, 0, 11),
            (MIXED, 5, 6, -1), // backward
            (MIXED, 3, 6, -5), // backward through surrogate pair
        ];
        for (text, char_index, base_char, expected_bytes) in cases {
            assert_eq!(
                utf8_offset_from_char_index(text, *char_index, *base_char),
                *expected_bytes,
                "{text:?} {char_index} {base_char}"
            );
        }
    }

    #[test]
    fn utf8_offset_from_char_index_clamps_out_of_range() {
        assert_eq!(utf8_offset_from_char_index(MIXED, 100, 0), 11);
        assert_eq!(utf8_offset_from_char_index(MIXED, -10, 6), -11);
    }

    #[test]
    fn round_trip_all_char_boundaries() {
        // For every char index, char→byte→char should be the identity (modulo
        // surrogate-pair midpoints, which are skipped).
        let units: Vec<u16> = MIXED.encode_utf16().collect();
        for i in 0..=units.len() {
            if i > 0 && i < units.len() && (0xDC00..0xE000).contains(&units[i]) {
                continue;
            }

            let bytes = utf8_byte_index_from_char_index(MIXED, i as i32);
            let round_trip = char_index_from_utf8_offset(MIXED, bytes, 0);
            assert_eq!(round_trip, i as i32);
        }
    }

    #[test]
    fn a_part_of_a_text_is_taken_by_utf16_indices() {
        assert_eq!(utf16_length(MIXED), 6);
        assert_eq!(utf16_substring(MIXED, 1, 3), "\u{439}\u{65e5}");
        assert_eq!(utf16_substring(MIXED, 3, 5), "\u{1F30D}");
        assert_eq!(utf16_substring(MIXED, 0, 100), MIXED);
        assert_eq!(utf16_substring(MIXED, 5, 2), "");
    }
}
