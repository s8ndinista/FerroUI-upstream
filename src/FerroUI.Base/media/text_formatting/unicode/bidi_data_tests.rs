//! Port of upstream's `Media/TextFormatting/BiDiDataTests.cs`.

use super::*;

#[test]
fn explicit_formatting_should_be_resolved_after_reset() {
    let rows: [(&str, i8); 3] = [
        ("\u{202E}abc\u{202C}", 1), // RLO ... PDF
        ("\u{202B}abc\u{202C}", 2), // RLE ... PDF
        ("\u{2067}abc\u{2069}", 2), // RLI ... PDI
    ];

    for (text, expected_level) in rows {
        let mut bidi_data = BidiData::new();
        let mut bidi = BidiAlgorithm::new();

        let first: Vec<u16> = "abc".encode_utf16().collect();
        bidi_data.append(&first);
        bidi.process(&mut bidi_data);

        bidi_data.reset();
        bidi.reset();

        let text_utf16: Vec<u16> = text.encode_utf16().collect();
        bidi_data.append(&text_utf16);
        bidi.process(&mut bidi_data);

        assert_eq!(expected_level, bidi.resolved_levels()[1], "{text:?}");
    }
}
