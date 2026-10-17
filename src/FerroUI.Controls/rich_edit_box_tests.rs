use crate::documents::{
    CharacterFormat, DocumentSnapshot, InlineSnapshot, InlineSnapshotKind, ParagraphFormat,
    ParagraphSnapshot, RichTextDocument,
};
use crate::rich_text::{
    EditError, EditKind, EditorSelection, RichEditClipboardFormat, RichEditTextDocument,
    RichTextEditor, RichTextTransfer, RtfError, RtfLimits, RtfReader, RtfWriter, TextFindOptions,
    TextFormatValue, TextRangeUnit,
};
use crate::{RichEditBox, RichEditCharacterCasing};
use ferroui_base::input::text_input::{TextInputOptions, TextSelection};
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::{FontFamily, FontStyle, FontWeight, TextAlignment};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{Rect, Size};
use std::future::Future;
use std::task::{Context, Poll, Waker};

fn run(text: &str, format: CharacterFormat) -> InlineSnapshot {
    InlineSnapshot {
        format,
        kind: InlineSnapshotKind::Run(text.to_owned()),
    }
}

fn rich_document() -> RichTextDocument {
    let document = RichTextDocument::new();
    document
        .restore(&DocumentSnapshot {
            paragraphs: vec![ParagraphSnapshot {
                format: ParagraphFormat {
                    text_alignment: Some(TextAlignment::Center),
                    ..ParagraphFormat::default()
                },
                inlines: vec![
                    InlineSnapshot {
                        format: CharacterFormat::default(),
                        kind: InlineSnapshotKind::Bold(vec![run(
                            "bold",
                            CharacterFormat::default(),
                        )]),
                    },
                    run(
                        " tail",
                        CharacterFormat {
                            font_style: Some(FontStyle::Italic),
                            ..CharacterFormat::default()
                        },
                    ),
                ],
            }],
        })
        .unwrap();
    document
}

fn ready<T>(future: impl Future<Output = T>) -> T {
    let mut future = std::pin::pin!(future);
    match future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("The test operation is synchronous."),
    }
}

#[test]
fn rich_edit_undo_restores_nested_inline_structure_and_paragraph_format() {
    let _scope = Dispatcher::unit_test_scope();
    let document = rich_document();
    let original = document.snapshot();
    let editor = RichTextEditor::new(document.clone());
    editor.set_selection(EditorSelection::new(1, 3)).unwrap();
    editor.insert_text("XYZ", EditKind::Typing).unwrap();
    assert_eq!(document.text(), "bXYZd tail");
    assert_eq!(
        document.character_format_at(7).unwrap().font_style,
        Some(FontStyle::Italic)
    );
    assert!(editor.undo().unwrap());
    assert_eq!(document.snapshot(), original);
    assert_eq!(editor.selection(), EditorSelection::new(1, 3));
    assert!(editor.redo().unwrap());
    assert_eq!(document.text(), "bXYZd tail");
}

#[test]
fn rich_edit_typing_coalesces_until_an_explicit_caret_change() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("");
    let editor = RichTextEditor::new(document.clone());
    editor.insert_text("a", EditKind::Typing).unwrap();
    editor.insert_text("b", EditKind::Typing).unwrap();
    editor.insert_text("c", EditKind::Typing).unwrap();
    assert!(editor.undo().unwrap());
    assert_eq!(document.text(), "");
    editor.redo().unwrap();
    editor.set_selection(EditorSelection::caret(1)).unwrap();
    editor.insert_text("!", EditKind::Typing).unwrap();
    editor.undo().unwrap();
    assert_eq!(document.text(), "abc");
    editor.undo().unwrap();
    assert_eq!(document.text(), "");
}

#[test]
fn rich_edit_deletion_uses_graphemes_for_combining_sequences_and_emoji() {
    let _scope = Dispatcher::unit_test_scope();
    for text in ["a\u{301}", "👨‍👩‍👧‍👦", "🇵🇱", "👍🏽"] {
        let document = RichTextDocument::from_text(&format!("x{text}y"));
        let editor = RichTextEditor::new(document.clone());
        editor
            .set_selection(EditorSelection::caret(1 + text.encode_utf16().count()))
            .unwrap();
        editor.delete_backward(false).unwrap();
        assert_eq!(document.text(), "xy", "{text}");
        editor.undo().unwrap();
        editor.set_selection(EditorSelection::caret(1)).unwrap();
        editor.delete_forward(false).unwrap();
        assert_eq!(document.text(), "xy", "{text}");
    }
}

#[test]
fn rich_edit_rejects_a_split_surrogate_selection_without_mutating_content() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("a😀b");
    let editor = RichTextEditor::new(document.clone());
    assert!(editor.set_selection(EditorSelection::new(0, 2)).is_err());
    assert_eq!(editor.selection(), EditorSelection::default());
    assert_eq!(document.text(), "a😀b");
}

#[test]
fn rich_edit_read_only_rejects_all_mutations_and_preserves_selection() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("read me");
    let editor = RichTextEditor::new(document.clone());
    editor.set_selection(EditorSelection::caret(3)).unwrap();
    editor.set_is_read_only(true);
    assert!(matches!(
        editor.insert_text("x", EditKind::Typing),
        Err(EditError::ReadOnly)
    ));
    assert!(matches!(
        editor.delete_backward(false),
        Err(EditError::ReadOnly)
    ));
    assert!(matches!(
        editor.apply_format(&CharacterFormat::default()),
        Err(EditError::ReadOnly)
    ));
    assert!(matches!(
        editor.set_preedit("x", None),
        Err(EditError::ReadOnly)
    ));
    assert_eq!(editor.selection(), EditorSelection::caret(3));
    assert_eq!(document.text(), "read me");
}

#[test]
fn rich_edit_max_length_counts_utf16_and_never_splits_a_scalar() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("ab");
    let editor = RichTextEditor::new(document.clone());
    editor.set_selection(EditorSelection::caret(2)).unwrap();
    editor.set_max_length(5);
    editor.insert_text("😀😀q", EditKind::Typing).unwrap();
    assert_eq!(document.text(), "ab😀");
    assert_eq!(editor.selection(), EditorSelection::caret(4));
    editor.set_selection(EditorSelection::new(0, 2)).unwrap();
    editor.insert_text("XYZ", EditKind::Typing).unwrap();
    assert_eq!(document.text(), "XYZ😀");
}

#[test]
fn rich_edit_rejected_length_limited_input_does_not_remove_a_selection() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("abcd");
    let editor = RichTextEditor::new(document.clone());
    editor.set_max_length(2);
    editor.set_selection(EditorSelection::new(0, 1)).unwrap();
    assert!(!editor.insert_text("😀", EditKind::Typing).unwrap());
    assert_eq!(document.text(), "abcd");
    assert_eq!(editor.selection(), EditorSelection::new(0, 1));
}

#[test]
fn rich_edit_compound_transaction_rolls_back_on_error_and_is_one_undo_unit() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("abc");
    let editor = RichTextEditor::new(document.clone());
    let failure = editor.edit_group(|editor| {
        editor.insert_text("!", EditKind::Typing)?;
        editor.set_selection(EditorSelection::caret(99))?;
        Ok(())
    });
    assert!(failure.is_err());
    assert_eq!(document.text(), "abc");
    assert!(!editor.can_undo());
    editor
        .edit_group(|editor| {
            editor.insert_text("!", EditKind::Typing)?;
            editor.set_selection(EditorSelection::new(1, 4))?;
            editor.apply_format(&CharacterFormat {
                font_weight: Some(FontWeight::Bold),
                ..CharacterFormat::default()
            })
        })
        .unwrap();
    assert_eq!(document.text(), "!abc");
    assert!(editor.undo().unwrap());
    assert_eq!(document.text(), "abc");
    assert!(!editor.can_undo());
}

#[test]
fn rich_edit_external_document_changes_invalidate_stale_undo_units() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("abc");
    let editor = RichTextEditor::new(document.clone());
    editor.insert_text("!", EditKind::Typing).unwrap();
    document.replace_text(0..1, "external").unwrap();
    assert!(!editor.can_undo());
    assert!(!editor.undo().unwrap());
    assert_eq!(document.text(), "externalabc");
}

#[test]
fn rich_edit_ime_repeated_preedit_cancel_restores_rich_content_exactly() {
    let _scope = Dispatcher::unit_test_scope();
    let document = rich_document();
    let original = document.snapshot();
    let editor = RichTextEditor::new(document.clone());
    editor.set_selection(EditorSelection::new(1, 3)).unwrap();
    editor.set_preedit("に", Some(1)).unwrap();
    assert_eq!(document.text(), "bにd tail");
    assert!(!editor.can_undo());
    editor.set_preedit("日本", Some(2)).unwrap();
    assert_eq!(document.text(), "b日本d tail");
    editor.cancel_composition().unwrap();
    assert_eq!(document.snapshot(), original);
    assert_eq!(editor.selection(), EditorSelection::new(1, 3));
    assert!(!editor.can_undo());
}

#[test]
fn rich_edit_ime_commit_creates_one_undo_unit_and_restores_original_selection() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("before");
    let editor = RichTextEditor::new(document.clone());
    editor.set_selection(EditorSelection::new(0, 6)).unwrap();
    editor.set_preedit("に", None).unwrap();
    editor.set_preedit("日本", None).unwrap();
    editor.commit_composition("日本語").unwrap();
    assert_eq!(document.text(), "日本語");
    assert!(!editor.is_composing());
    assert!(editor.undo().unwrap());
    assert_eq!(document.text(), "before");
    assert_eq!(editor.selection(), EditorSelection::new(0, 6));
    assert!(!editor.can_undo());
}

#[test]
fn rich_edit_ime_cancel_does_not_overwrite_an_external_edit() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("abc");
    let editor = RichTextEditor::new(document.clone());
    editor.set_preedit("日", None).unwrap();
    document.replace_text(0..1, "external").unwrap();
    editor.cancel_composition().unwrap();
    assert_eq!(document.text(), "externalabc");
}

#[test]
fn rich_edit_undo_limit_and_redo_branching_are_bounded() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("");
    let editor = RichTextEditor::new(document.clone());
    editor.set_undo_limit(2);
    for text in ["a", "b", "c"] {
        editor.checkpoint();
        editor.insert_text(text, EditKind::Typing).unwrap();
    }
    assert!(editor.undo().unwrap());
    assert!(editor.undo().unwrap());
    assert!(!editor.undo().unwrap());
    assert_eq!(document.text(), "a");
    editor.insert_text("X", EditKind::Typing).unwrap();
    assert!(!editor.can_redo());
    editor.set_undo_limit(0);
    assert!(!editor.can_undo());
}

#[test]
fn rich_edit_rtf_unicode_groups_fallback_and_cp1252_are_decoded() {
    let _scope = Dispatcher::unit_test_scope();
    let rtf = br"{\rtf1\ansi\ansicpg1252\uc1 A {\b bold} \u55357?\u56832? \'80{\*\unknown should not appear}}";
    let document = RtfReader::read(rtf).unwrap();
    assert_eq!(document.text(), "A bold 😀 €");
    let model = RichTextDocument::new();
    model.restore(&document).unwrap();
    assert_eq!(
        model.character_format_at(3).unwrap().font_weight,
        Some(FontWeight::Bold)
    );
    assert_ne!(
        model.character_format_at(7).unwrap().font_weight,
        Some(FontWeight::Bold)
    );
}

#[test]
fn rich_edit_rtf_fonts_colors_paragraph_metrics_and_hyperlinks_are_retained() {
    let _scope = Dispatcher::unit_test_scope();
    let rtf = br#"{\rtf1\ansi{\fonttbl{\f0\fnil Example Sans;}}{\colortbl;\red240\green20\blue30;}\f0\fs30\cf1\qc\fi300 hello {\field{\*\fldinst HYPERLINK "https://example.org"}{\fldrslt site}}\par\ql next}"#;
    let document = RtfReader::read(rtf).unwrap();
    assert_eq!(document.text(), "hello site\nnext");
    assert_eq!(
        document.paragraphs[0].format.text_alignment,
        Some(TextAlignment::Center)
    );
    assert_eq!(document.paragraphs[0].format.text_indent, Some(20.0));
    assert!(document.paragraphs[0].inlines.iter().any(|inline| matches!(&inline.kind, InlineSnapshotKind::Hyperlink { navigate_uri: Some(uri), .. } if uri == "https://example.org")));
    let model = RichTextDocument::new();
    model.restore(&document).unwrap();
    let format = model.character_format_at(0).unwrap();
    assert_eq!(format.font_family, Some(FontFamily::new("Example Sans")));
    assert_eq!(format.font_size, Some(20.0));
    assert_eq!(
        format
            .foreground
            .unwrap()
            .unwrap()
            .as_solid_color_brush()
            .unwrap()
            .color()
            .r,
        240
    );
}

#[test]
fn rich_edit_rtf_writer_roundtrip_preserves_text_and_formatting_without_extra_spaces() {
    let _scope = Dispatcher::unit_test_scope();
    let document = rich_document();
    document.replace_text(9..9, " 😀 {\\}").unwrap();
    let encoded = RtfWriter::write(&document.snapshot()).unwrap();
    assert!(encoded.is_ascii());
    let decoded = RtfReader::read_str(&encoded).unwrap();
    assert_eq!(decoded.text(), document.text());
    let model = RichTextDocument::new();
    model.restore(&decoded).unwrap();
    assert_eq!(
        model.character_format_at(1).unwrap().font_weight,
        Some(FontWeight::Bold)
    );
    assert_eq!(
        model.character_format_at(6).unwrap().font_style,
        Some(FontStyle::Italic)
    );
    assert_eq!(
        decoded.paragraphs[0].format.text_alignment,
        Some(TextAlignment::Center)
    );
}

#[test]
fn rich_edit_rtf_rejects_malformed_and_unsupported_content_without_panicking() {
    let _scope = Dispatcher::unit_test_scope();
    for bytes in [
        b"{\\rtf1".as_slice(),
        b"{\\rtf1 \\'xz}",
        b"{\\rtf1 \\u-55357?}",
        b"{\\rtf1 \\fs-2 x}",
        b"{\\rtf1 \\b9999999999999999999999 x}",
        b"{\\rtf1}garbage",
    ] {
        assert!(
            RtfReader::read(bytes).is_err(),
            "{}",
            String::from_utf8_lossy(bytes)
        );
    }
    assert!(matches!(
        RtfReader::read(br"{\rtf1\ansicpg932 x}"),
        Err(RtfError::UnsupportedCodePage(932))
    ));
    assert!(matches!(
        RtfReader::read(br"{\rtf1{\object\bin2 {}}}"),
        Err(RtfError::UnsupportedFeature(_))
    ));
    assert!(matches!(
        RtfReader::read(br"{\rtf1\trowd a\cell\row}"),
        Err(RtfError::UnsupportedFeature(_))
    ));
}

#[test]
fn rich_edit_rtf_enforces_input_nesting_tokens_and_output_limits() {
    let _scope = Dispatcher::unit_test_scope();
    for (bytes, limits) in [
        (
            br"{\rtf1 abc}".as_slice(),
            RtfLimits {
                max_bytes: 4,
                ..RtfLimits::default()
            },
        ),
        (
            br"{\rtf1{{{x}}}}",
            RtfLimits {
                max_depth: 2,
                ..RtfLimits::default()
            },
        ),
        (
            br"{\rtf1\b a\i b}",
            RtfLimits {
                max_tokens: 2,
                ..RtfLimits::default()
            },
        ),
        (
            br"{\rtf1 abcd}",
            RtfLimits {
                max_text_length: 3,
                ..RtfLimits::default()
            },
        ),
        (
            br"{\rtf1\line\line}",
            RtfLimits {
                max_text_length: 1,
                ..RtfLimits::default()
            },
        ),
    ] {
        assert!(matches!(
            RtfReader::read_with_limits(bytes, limits),
            Err(RtfError::LimitExceeded(_))
        ));
    }
}

#[test]
fn rich_edit_framework_clipboard_preserves_exact_in_process_snapshot() {
    let _scope = Dispatcher::unit_test_scope();
    let original = rich_document().snapshot();
    let transfer = RichTextTransfer::create(&original, RichEditClipboardFormat::AllFormats);
    let rich = ready(RichTextTransfer::read(transfer.as_ref(), false))
        .unwrap()
        .unwrap();
    assert_eq!(rich, original);
    let plain = ready(RichTextTransfer::read(transfer.as_ref(), true))
        .unwrap()
        .unwrap();
    assert_eq!(plain.text(), original.text());
    assert_eq!(plain.paragraphs[0].format, ParagraphFormat::default());
}

#[test]
fn rich_edit_control_uses_document_layout_and_native_input_method_projection() {
    let _scope = TextTestScope::new();
    let editor = RichEditBox::new();
    editor.set_text("hello\nworld").unwrap();
    editor.measure(Size::new(200.0, 100.0));
    editor.arrange(Rect::new(0.0, 0.0, 200.0, 100.0));
    editor.select(1, 4);
    let input = editor.text_input_method_client();
    assert_eq!(input.surrounding_text(), "hello\nworld");
    assert_eq!(input.selection(), TextSelection::new(1, 4));
    assert!(input.cursor_rectangle().height > 0.0);
    assert!(input.text_view_visual().is_some());
    input.set_preedit_text_with_cursor(Some("日"), Some(1));
    assert_eq!(editor.text(), "h日o\nworld");
    input.set_preedit_text(None);
    assert_eq!(editor.text(), "hello\nworld");
    assert_eq!(input.selection(), TextSelection::new(1, 4));
}

#[test]
fn rich_edit_control_input_policy_maps_to_platform_options_and_enforces_casing() {
    let _scope = Dispatcher::unit_test_scope();
    let editor = RichEditBox::new();
    editor.set_accepts_return(false);
    editor.set_character_casing(RichEditCharacterCasing::Upper);
    editor.set_max_length(6);
    editor.insert_text("ab\nßc").unwrap();
    assert_eq!(editor.text(), "ABSSC");
    let options = TextInputOptions::from_styled_element(&editor);
    assert!(!options.multiline);
    assert!(options.uppercase);
    assert!(!options.lowercase);
}

#[test]
fn rich_edit_control_clipboard_copy_cut_and_paste_use_the_same_rich_editor() {
    let _scope = Dispatcher::unit_test_scope();
    let clipboard = crate::testing::TestClipboardImpl::new();
    let source = RichEditBox::new();
    source.set_document(rich_document());
    source.select_all();
    ready(source.copy_to_clipboard_async(clipboard.clipboard(), false)).unwrap();
    let target = RichEditBox::new();
    ready(target.paste_from_clipboard_async(clipboard.clipboard(), false)).unwrap();
    assert_eq!(target.text(), "bold tail");
    assert_eq!(
        target
            .document()
            .character_format_at(1)
            .unwrap()
            .font_weight,
        Some(FontWeight::Bold)
    );
    assert!(target.undo().unwrap());
    assert_eq!(target.text(), "");
    ready(source.copy_to_clipboard_async(clipboard.clipboard(), true)).unwrap();
    assert_eq!(source.text(), "");
    source.undo().unwrap();
    assert_eq!(source.text(), "bold tail");
}

#[test]
fn rich_edit_automation_enforces_enabled_read_only_and_undoable_value_changes() {
    let _scope = Dispatcher::unit_test_scope();
    let editor = RichEditBox::new();
    editor.set_text("initial").unwrap();
    let peer = crate::automation::peers::RichEditBoxAutomationPeer::new(&editor);
    editor.set_is_read_only(true);
    assert!(peer.set_value("forbidden").is_err());
    assert_eq!(editor.text(), "initial");
    editor.set_is_read_only(false);
    editor.set_is_enabled(false);
    assert!(peer.set_value("forbidden").is_err());
    editor.set_is_enabled(true);
    peer.set_value("allowed").unwrap();
    assert_eq!(editor.text(), "allowed");
    editor.undo().unwrap();
    assert_eq!(editor.text(), "initial");
}

#[test]
fn rich_edit_undo_deletion_restores_the_original_collapsed_caret() {
    let _scope = Dispatcher::unit_test_scope();
    let editor = RichTextEditor::new(RichTextDocument::from_text("ab😀cd"));
    editor.set_selection(EditorSelection::caret(4)).unwrap();
    editor.delete_backward(false).unwrap();
    assert_eq!(editor.document().text(), "abcd");
    editor.undo().unwrap();
    assert_eq!(editor.selection(), EditorSelection::caret(4));
    editor.delete_forward(false).unwrap();
    assert_eq!(editor.document().text(), "ab😀d");
    editor.undo().unwrap();
    assert_eq!(editor.selection(), EditorSelection::caret(4));
}

#[test]
fn rich_edit_live_ranges_follow_edits_and_support_independent_clones() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("a😀bc");
    let services = RichEditTextDocument::new(document.clone());
    let range = services.get_range(1, 3).unwrap();
    let shared = range.clone();
    let independent = range.get_clone().unwrap();
    document.replace_text(0..0, "XY").unwrap();
    assert_eq!(range.range(), 3..5);
    assert_eq!(range.text().unwrap(), "😀");
    range.collapse(false).unwrap();
    assert_eq!(shared.range(), 5..5);
    assert_eq!(independent.range(), 3..5);
    assert!(range.in_range(&independent));
    assert!(!range.is_equal(&independent));
    assert!(services.get_range(3, 4).is_err());
}

#[test]
fn rich_edit_range_replacement_rebases_independent_selection_and_undoes_once() {
    let _scope = Dispatcher::unit_test_scope();
    let services = RichEditTextDocument::new(RichTextDocument::from_text("one two three"));
    services.selection().set_range(13, 8).unwrap();
    let range = services.get_range(4, 7).unwrap();
    range.set_text("2").unwrap();
    assert_eq!(services.text(), "one 2 three");
    assert_eq!(
        services.selection().selection(),
        EditorSelection::new(11, 6)
    );
    assert_eq!(range.range(), 4..5);
    services.undo().unwrap();
    assert_eq!(services.text(), "one two three");
    assert_eq!(
        services.selection().selection(),
        EditorSelection::new(13, 8)
    );
    assert!(!services.can_undo());
    services
        .get_range(4, 7)
        .unwrap()
        .set_snapshot(&DocumentSnapshot::default())
        .unwrap();
    assert_eq!(services.text(), "one  three");
}

#[test]
fn rich_edit_text_unit_movement_preserves_graphemes_and_directed_selection() {
    let _scope = Dispatcher::unit_test_scope();
    let services = RichEditTextDocument::new(RichTextDocument::from_text("A😀e\u{301} Z"));
    let range = services.get_range(1, 3).unwrap();
    assert_eq!(range.move_by(TextRangeUnit::Character, 0).unwrap(), 0);
    assert_eq!(range.range(), 1..3);
    assert_eq!(range.move_by(TextRangeUnit::Character, 1).unwrap(), 1);
    assert_eq!(range.range(), 5..5);
    range.move_start(TextRangeUnit::Character, -1).unwrap();
    assert_eq!(range.text().unwrap(), "e\u{301}");
    services.selection().set_range(5, 5).unwrap();
    services
        .selection()
        .move_by(TextRangeUnit::Character, -2, true)
        .unwrap();
    assert_eq!(services.selection().selection(), EditorSelection::new(5, 1));
    assert_eq!(services.selection().text().unwrap(), "😀e\u{301}");
}

#[test]
fn rich_edit_range_search_keeps_utf16_offsets_and_reverse_overlapping_matches() {
    let _scope = Dispatcher::unit_test_scope();
    let services = RichEditTextDocument::new(RichTextDocument::from_text(
        "İstanbul 😀 aaa alpha alphabet",
    ));
    let all = services.document_range().unwrap();
    let unicode = all
        .find("i\u{307}stanbul", TextFindOptions::empty())
        .unwrap()
        .unwrap();
    assert_eq!(unicode.range(), 0..8);
    let overlap = all.find("aa", TextFindOptions::BACKWARDS).unwrap().unwrap();
    assert_eq!(overlap.range(), 13..15);
    let word = all
        .find(
            "alpha",
            TextFindOptions::WHOLE_WORD | TextFindOptions::BACKWARDS,
        )
        .unwrap()
        .unwrap();
    assert_eq!(word.range(), 16..21);
    assert!(
        all.find("istanbul", TextFindOptions::MATCH_CASE)
            .unwrap()
            .is_none()
    );
    assert!(!all.find_text("missing", TextFindOptions::empty()).unwrap());
    assert_eq!(all.range(), 0..services.text_length());
}

#[test]
fn rich_edit_format_queries_distinguish_run_boundaries_and_mixed_paragraphs() {
    let _scope = Dispatcher::unit_test_scope();
    let services = RichEditTextDocument::new(rich_document());
    assert!(matches!(
        services
            .document_range()
            .unwrap()
            .character_format()
            .unwrap(),
        TextFormatValue::Mixed
    ));
    let TextFormatValue::Uniform(format) = services
        .get_range(4, 9)
        .unwrap()
        .character_format()
        .unwrap()
    else {
        panic!("The tail has one effective format.");
    };
    assert_eq!(format.font_style, Some(FontStyle::Italic));
    let other = RichEditTextDocument::new(RichTextDocument::from_text("one\ntwo"));
    other
        .get_range(4, 7)
        .unwrap()
        .set_paragraph_format(&ParagraphFormat {
            text_alignment: Some(TextAlignment::Right),
            ..ParagraphFormat::default()
        })
        .unwrap();
    assert!(matches!(
        other.document_range().unwrap().paragraph_format().unwrap(),
        TextFormatValue::Mixed
    ));
    other.undo().unwrap();
    assert!(matches!(
        other.document_range().unwrap().paragraph_format().unwrap(),
        TextFormatValue::Uniform(_)
    ));
}

#[test]
fn rich_edit_collapsed_typing_format_is_used_until_the_caret_moves() {
    let _scope = Dispatcher::unit_test_scope();
    let services = RichEditTextDocument::new(RichTextDocument::from_text("ab"));
    services.selection().set_range(2, 2).unwrap();
    services
        .selection()
        .set_character_format(&CharacterFormat {
            font_weight: Some(FontWeight::Bold),
            ..CharacterFormat::default()
        })
        .unwrap();
    assert_eq!(
        services.selection().character_format().unwrap().font_weight,
        Some(FontWeight::Bold)
    );
    services.selection().type_text("C").unwrap();
    assert_eq!(
        services
            .document()
            .character_format_at(3)
            .unwrap()
            .font_weight,
        Some(FontWeight::Bold)
    );
    services.selection().set_range(0, 0).unwrap();
    assert_ne!(
        services.selection().character_format().unwrap().font_weight,
        Some(FontWeight::Bold)
    );
    services.selection().type_text("x").unwrap();
    assert_eq!(services.text(), "xabC");
}

#[test]
fn rich_edit_control_text_services_share_selection_history_and_visual_geometry() {
    let _scope = TextTestScope::new();
    let control = RichEditBox::new();
    control.set_text("hello world").unwrap();
    let services = control.text_document();
    services.selection().set_range(1, 4).unwrap();
    assert_eq!((control.selection_start(), control.selection_end()), (1, 4));
    services.selection().type_text("i").unwrap();
    assert_eq!(control.text(), "hio world");
    assert_eq!(control.caret_index(), 2);
    control.measure(Size::new(200.0, 100.0));
    control.arrange(Rect::new(0.0, 0.0, 200.0, 100.0));
    let range = services.get_range(0, 2).unwrap();
    assert!(
        range
            .get_rectangles(&control)
            .unwrap()
            .iter()
            .any(|rect| rect.width > 0.0 && rect.height > 0.0)
    );
    let position = services
        .get_range_from_point(&control, control.cursor_rectangle().top_left())
        .unwrap();
    assert!(position.start_position() <= services.text_length());
    let unrelated = crate::RichTextBlock::new();
    assert!(range.get_rectangles(&unrelated).is_err());
    control.undo().unwrap();
    assert_eq!(control.text(), "hello world");
}

#[test]
fn rich_edit_native_surrounding_text_excludes_preview_and_selection_cancels_it() {
    let _scope = Dispatcher::unit_test_scope();
    let control = RichEditBox::new();
    control.set_text("a😀bc").unwrap();
    control.select(1, 3);
    let input = control.text_input_method_client();
    input.set_preedit_text_with_cursor(Some("候補"), Some(1));
    assert_eq!(control.text(), "a候補bc");
    assert_eq!(input.surrounding_text(), "a😀bc");
    assert_eq!(input.selection(), TextSelection::new(1, 3));
    input.set_selection(TextSelection::new(4, 5));
    assert_eq!(control.text(), "a😀bc");
    assert!(!control.editor().is_composing());
    assert_eq!(input.selection(), TextSelection::new(4, 5));
}

#[test]
fn rich_edit_clipboard_captures_copy_before_poll_and_cancels_stale_paste() {
    let _scope = Dispatcher::unit_test_scope();
    let clipboard = crate::testing::TestClipboardImpl::new();
    let control = RichEditBox::new();
    control.set_text("original").unwrap();
    control.select_all();
    let cut = control.copy_to_clipboard_async(clipboard.clipboard(), true);
    control.set_text("replacement").unwrap();
    ready(cut).unwrap();
    assert_eq!(clipboard.text().as_deref(), Some("original"));
    assert_eq!(control.text(), "replacement");
    control.select_all();
    let paste = control.paste_from_clipboard_async(clipboard.clipboard(), false);
    control.set_text("later").unwrap();
    assert!(ready(paste).is_err());
    assert_eq!(control.text(), "later");
}

#[test]
fn rich_edit_clipboard_casing_and_single_line_policies_preserve_rich_runs() {
    let _scope = Dispatcher::unit_test_scope();
    let clipboard = crate::testing::TestClipboardImpl::new();
    let source = RichEditBox::new();
    source.set_document(rich_document());
    source.select_all();
    ready(source.copy_to_clipboard_async(clipboard.clipboard(), false)).unwrap();
    let target = RichEditBox::new();
    target.set_accepts_return(false);
    target.set_character_casing(RichEditCharacterCasing::Upper);
    ready(target.paste_from_clipboard_async(clipboard.clipboard(), false)).unwrap();
    assert_eq!(target.text(), "BOLD TAIL");
    assert_eq!(
        target
            .document()
            .character_format_at(1)
            .unwrap()
            .font_weight,
        Some(FontWeight::Bold)
    );
    assert_eq!(
        target.document().character_format_at(7).unwrap().font_style,
        Some(FontStyle::Italic)
    );
}

#[test]
fn rich_edit_rtf_unicode_font_names_hyperlink_uris_and_targets_roundtrip() {
    let _scope = Dispatcher::unit_test_scope();
    let mut document = RtfReader::read(
        br#"{\rtf1{\field{\*\fldinst HYPERLINK "https://example.com"}{\fldrslt link}}}"#,
    )
    .unwrap();
    let hyperlink = &mut document.paragraphs[0].inlines[0];
    hyperlink.format.font_family = Some(FontFamily::new("日本語 Font"));
    let InlineSnapshotKind::Hyperlink {
        navigate_uri,
        target_name,
        ..
    } = &mut hyperlink.kind
    else {
        panic!("Expected a hyperlink field.");
    };
    *navigate_uri = Some("https://example.com/日本/😀".to_owned());
    *target_name = Some("詳細".to_owned());
    let rtf = RtfWriter::write(&document).unwrap();
    let decoded = RtfReader::read(rtf.as_bytes()).unwrap();
    let link = decoded.paragraphs[0]
        .inlines
        .iter()
        .find(|inline| matches!(inline.kind, InlineSnapshotKind::Hyperlink { .. }))
        .unwrap();
    let InlineSnapshotKind::Hyperlink {
        navigate_uri,
        target_name,
        ..
    } = &link.kind
    else {
        unreachable!();
    };
    assert_eq!(navigate_uri.as_deref(), Some("https://example.com/日本/😀"));
    assert_eq!(target_name.as_deref(), Some("詳細"));
    let model = RichTextDocument::new();
    model.restore(&decoded).unwrap();
    assert_eq!(
        model
            .character_format_at(1)
            .unwrap()
            .font_family
            .unwrap()
            .name(),
        "日本語 Font"
    );
    assert_eq!(model.text(), "link");
}

#[test]
fn rich_edit_rtf_hyperlink_bookmark_and_exact_line_spacing_are_preserved() {
    let _scope = Dispatcher::unit_test_scope();
    let snapshot = RtfReader::read(br#"{\rtf1\sl-300\slmult0{\field{\*\fldinst HYPERLINK \\l "section" \\t "frame"}{\fldrslt local}}}"#).unwrap();
    assert_eq!(snapshot.paragraphs[0].format.line_height, Some(20.0));
    assert_eq!(
        snapshot.paragraphs[0].format.line_stacking_strategy,
        Some(crate::documents::LineStackingStrategy::BlockLineHeight)
    );
    let InlineSnapshotKind::Hyperlink {
        navigate_uri,
        target_name,
        ..
    } = &snapshot.paragraphs[0].inlines[0].kind
    else {
        panic!("Expected a bookmark hyperlink.");
    };
    assert_eq!(navigate_uri.as_deref(), Some("#section"));
    assert_eq!(target_name.as_deref(), Some("frame"));
    let encoded = RtfWriter::write(&snapshot).unwrap();
    let roundtrip = RtfReader::read(encoded.as_bytes()).unwrap();
    assert_eq!(
        roundtrip.paragraphs[0].format.line_stacking_strategy,
        Some(crate::documents::LineStackingStrategy::BlockLineHeight)
    );
}

#[test]
fn rich_edit_ime_notifications_expose_committed_text_during_preview_publication() {
    let _scope = Dispatcher::unit_test_scope();
    let control = RichEditBox::new();
    control.set_text("original").unwrap();
    control.select(1, 4);
    let input = control.text_input_method_client();
    let observations = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let values = observations.clone();
    let weak = std::rc::Rc::downgrade(&input);
    let _subscription = input.surrounding_text_changed(std::rc::Rc::new(move || {
        if let Some(input) = weak.upgrade() {
            values
                .borrow_mut()
                .push((input.surrounding_text(), input.selection()));
        }
    }));
    input.set_preedit_text(Some("日"));
    input.set_preedit_text(Some("日本"));
    assert!(!observations.borrow().is_empty());
    assert!(
        observations
            .borrow()
            .iter()
            .all(|value| value == &("original".to_owned(), TextSelection::new(1, 4)))
    );
    control.commit_composition("日本").unwrap();
    assert_eq!(input.surrounding_text(), "o日本inal");
}
