//! Cross-paragraph RTF field results project to the framework's paragraph-owned
//! inline hyperlinks, preserving text, target and formatting on every segment.

use super::{RtfError, RtfLimits, RtfReader, RtfWriter};
use crate::documents::{DocumentSnapshot, InlineSnapshot, InlineSnapshotKind};
use crate::rich_text::{TextSetOptions, TextStreamOptions};
use crate::RichEditBox;
use ferroui_base::media::{FontWeight, TextAlignment};
use ferroui_base::threading::Dispatcher;

fn link(inline: &InlineSnapshot) -> (&[InlineSnapshot], &str, Option<&str>) {
    match &inline.kind {
        InlineSnapshotKind::Hyperlink {
            inlines,
            navigate_uri,
            target_name,
            ..
        } => (
            inlines,
            navigate_uri.as_deref().unwrap(),
            target_name.as_deref(),
        ),
        _ => panic!("expected an inline hyperlink"),
    }
}

fn links(document: &DocumentSnapshot) -> usize {
    document
        .paragraphs
        .iter()
        .flat_map(|paragraph| &paragraph.inlines)
        .filter(|inline| matches!(inline.kind, InlineSnapshotKind::Hyperlink { .. }))
        .count()
}

#[test]
fn rich_rtf_field_across_paragraphs_retains_targets_metrics_and_surrounding_text() {
    let _scope = Dispatcher::unit_test_scope();
    let source = concat!(
        r#"{\rtf1 prefix {\field{\*\fldinst HYPERLINK "https://example.test" \\t "_blank"}"#,
        r"{\fldrslt {\b first}\par\qc second\par\par tail}} suffix}"
    );
    let document = RtfReader::read_str(source).unwrap();
    assert_eq!(document.text(), "prefix first\nsecond\n\ntail suffix");
    assert_eq!(document.paragraphs.len(), 4);
    assert_eq!(links(&document), 3);
    assert_eq!(document.paragraphs[0].inlines[0].text(), "prefix ");
    assert_eq!(document.paragraphs[3].inlines[1].text(), " suffix");
    let first = link(&document.paragraphs[0].inlines[1]);
    assert_eq!(first.0[0].format.font_weight, Some(FontWeight::Bold));
    for (paragraph, index) in [(0, 1), (1, 0), (3, 0)] {
        let (_, uri, target) = link(&document.paragraphs[paragraph].inlines[index]);
        assert_eq!(uri, "https://example.test");
        assert_eq!(target, Some("_blank"));
    }
    assert_eq!(
        document.paragraphs[1].format.text_alignment,
        Some(TextAlignment::Center)
    );
    assert_eq!(
        document.paragraphs[3].format.text_alignment,
        Some(TextAlignment::Center)
    );
    assert!(document.paragraphs[2].inlines.is_empty());
    assert!(document.validate().is_ok());
}

#[test]
fn rich_rtf_field_empty_edges_do_not_create_invisible_hyperlinks() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RtfReader::read_str(concat!(
        r#"{\rtf1{\field{\*\fldinst HYPERLINK "https://example.test"}"#,
        r"{\fldrslt\par middle\par}}after}"
    ))
    .unwrap();
    assert_eq!(document.text(), "\nmiddle\nafter");
    assert_eq!(links(&document), 1);
    assert!(document.paragraphs[0].inlines.is_empty());
    assert_eq!(
        link(&document.paragraphs[1].inlines[0]).1,
        "https://example.test"
    );
    assert!(matches!(
        document.paragraphs[2].inlines[0].kind,
        InlineSnapshotKind::Run(_)
    ));
}

#[test]
fn rich_rtf_field_paragraph_separators_count_toward_the_text_limit() {
    let _scope = Dispatcher::unit_test_scope();
    let source = concat!(
        r#"{\rtf1{\field{\*\fldinst HYPERLINK "x"}"#,
        r"{\fldrslt abcdefghijklmnopqrstuvwxyz12345\par x}}}"
    );
    assert_eq!(
        RtfReader::read_with_limits(
            source.as_bytes(),
            RtfLimits {
                max_text_length: 32,
                ..Default::default()
            }
        ),
        Err(RtfError::LimitExceeded("text length"))
    );
    let document = RtfReader::read_with_limits(
        source.as_bytes(),
        RtfLimits {
            max_text_length: 33,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(document.text_length(), 33);
    assert_eq!(links(&document), 2);
}

#[test]
fn rich_rtf_field_paragraph_segments_round_trip_through_canonical_rtf() {
    let _scope = Dispatcher::unit_test_scope();
    let original = RtfReader::read_str(concat!(
        r#"{\rtf1{\field{\*\fldinst HYPERLINK "https://example.test"}"#,
        r"{\fldrslt\b first\par\qr\i second}}}"
    ))
    .unwrap();
    let encoded = RtfWriter::write(&original).unwrap();
    let restored = RtfReader::read_str(&encoded).unwrap();
    assert_eq!(restored.text(), original.text());
    assert_eq!(links(&restored), 2);
    assert_eq!(
        restored.paragraphs[1].format.text_alignment,
        Some(TextAlignment::Right)
    );
    for paragraph in &restored.paragraphs {
        assert_eq!(link(&paragraph.inlines[0]).1, "https://example.test");
    }
    assert_eq!(
        RtfReader::read_str(&RtfWriter::write(&restored).unwrap()).unwrap(),
        restored
    );
}

#[test]
fn rich_rtf_field_metadata_expansion_is_bounded_across_the_whole_document() {
    let _scope = Dispatcher::unit_test_scope();
    let field = format!(
        r#"{{\field{{\*\fldinst HYPERLINK "{}"}}{{\fldrslt a\par b\par c}}}}"#,
        "x".repeat(128)
    );
    let source = format!(r"{{\rtf1{field}{field}}}");
    assert!(source.len() < 512);
    assert_eq!(
        RtfReader::read_with_limits(
            source.as_bytes(),
            RtfLimits {
                max_bytes: 512,
                ..Default::default()
            }
        ),
        Err(RtfError::LimitExceeded("field metadata expansion"))
    );
    let accepted = RtfReader::read_with_limits(
        source.as_bytes(),
        RtfLimits {
            max_bytes: 768,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(links(&accepted), 6);
    assert_eq!(accepted.text(), "a\nb\nca\nb\nc");

    // The stream adapter passes its byte policy into the codec and parses the
    // whole snapshot before cancelling an existing preview or editing history.
    let control = RichEditBox::new();
    control.set_text("before").unwrap();
    control.editor().set_preedit("preview", None).unwrap();
    let snapshot = control.document().snapshot();
    let selection = control.editor().selection();
    let revision = control.document().revision();
    assert!(control
        .text_document()
        .load_from_stream(
            TextSetOptions::FORMAT_RTF,
            &mut source.as_bytes(),
            TextStreamOptions {
                max_bytes: 512,
                ..Default::default()
            }
        )
        .is_err());
    assert_eq!(control.document().snapshot(), snapshot);
    assert_eq!(control.document().revision(), revision);
    assert_eq!(control.editor().selection(), selection);
    assert!(control.editor().is_composing());
}
