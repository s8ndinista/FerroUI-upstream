use super::*;
use crate::{Document, TextRun};

fn text(value: &str) -> Node {
    let mut node = Node::new(NodeKind::Text, "Paragraphs", 10.0, 20.0, 240.0, 160.0);
    node.text = value.into();
    node
}
fn center() -> ParagraphStylePatch {
    ParagraphStylePatch {
        alignment: TextStyleChange::Set(TextAlign::Center),
        ..Default::default()
    }
}

#[test]
fn paragraphs_cover_crlf_unicode_breaks_and_trailing_empty_text_exactly() {
    let value = "A🦀\r\nשלום\u{2029}Ω\n";
    let ranges = text_paragraph_ranges(value);
    assert_eq!(
        ranges.iter().map(|r| &value[r.clone()]).collect::<Vec<_>>(),
        vec!["A🦀\r\n", "שלום\u{2029}", "Ω\n", ""]
    );
    assert_eq!(text_paragraph_ranges(""), vec![0..0]);
    let mut node = text(value);
    assert_eq!(node.apply_paragraph_style(1..5, &center()).unwrap(), 1);
    assert_eq!(
        (node.text_paragraphs[0].start, node.text_paragraphs[0].end),
        (0, 7)
    );
    assert_eq!(
        node.apply_paragraph_style(value.len()..value.len(), &center())
            .unwrap(),
        1
    );
    assert_eq!(node.text_paragraphs.last().unwrap().start, value.len());
    validate_text_paragraphs(&node).unwrap();
}

#[test]
fn paragraph_range_apply_inherit_and_boundary_selection_preserve_other_overrides() {
    let mut node = text("one\ntwo\nthree");
    let patch = ParagraphStylePatch {
        direction: TextStyleChange::Set(TextDirection::RightToLeft),
        line_height: TextStyleChange::Set(2.0),
        space_before: TextStyleChange::Set(8.0),
        space_after: TextStyleChange::Set(12.0),
        indent: TextStyleChange::Set(18.0),
        ..center()
    };
    assert_eq!(node.apply_paragraph_style(1..8, &patch).unwrap(), 2);
    assert_eq!(
        node.text_paragraphs
            .iter()
            .map(|p| p.start)
            .collect::<Vec<_>>(),
        vec![0, 4]
    );
    assert_eq!(node.apply_paragraph_style(4..4, &center()).unwrap(), 0);
    assert_eq!(
        node.apply_paragraph_style(
            4..8,
            &ParagraphStylePatch {
                alignment: TextStyleChange::Inherit,
                ..Default::default()
            }
        )
        .unwrap(),
        1
    );
    assert_eq!(node.text_paragraphs[1].alignment, None);
    assert_eq!(node.text_paragraphs[1].space_after, Some(12.0));
    assert_eq!(
        node.apply_paragraph_style(0..node.text.len(), &ParagraphStylePatch::inherit_all())
            .unwrap(),
        2
    );
    assert!(node.text_paragraphs.is_empty());
}

#[test]
fn paragraph_unicode_replacement_splits_inherits_merges_and_remaps_following_styles() {
    let mut node = text("A🦀B\r\nשלום\nlast");
    let original_ranges = text_paragraph_ranges(&node.text);
    node.apply_paragraph_style(0..1, &center()).unwrap();
    node.apply_paragraph_style(
        original_ranges[1].clone(),
        &ParagraphStylePatch {
            direction: TextStyleChange::Set(TextDirection::RightToLeft),
            ..Default::default()
        },
    )
    .unwrap();
    node.text_runs.push(TextRun {
        start: 1,
        end: 5,
        font_family: None,
        font_size: Some(28.0),
        font_weight: None,
        color: None,
        italic: None,
        decoration: None,
    });
    assert!(node.replace_text_preserving_runs("A🦀\nB\r\nשלום\nlast".into()));
    assert_eq!(node.text_paragraphs.len(), 3);
    assert_eq!(node.text_paragraphs[0].alignment, Some(TextAlign::Center));
    assert_eq!(node.text_paragraphs[1].alignment, Some(TextAlign::Center));
    assert_eq!(
        node.text_paragraphs[2].direction,
        Some(TextDirection::RightToLeft)
    );
    assert_eq!((node.text_runs[0].start, node.text_runs[0].end), (1, 5));
    validate_text_paragraphs(&node).unwrap();
    assert!(node.replace_text_preserving_runs("A🦀Bשלום\nlast".into()));
    assert_eq!(node.text_paragraphs.len(), 1);
    assert_eq!(node.text_paragraphs[0].alignment, Some(TextAlign::Center));
    assert_eq!(
        node.text_paragraphs[0].direction, None,
        "Deleting separators retains the first surviving paragraph style"
    );
    validate_text_paragraphs(&node).unwrap();
}

#[test]
fn paragraphs_validate_complete_ranges_values_and_keep_failed_commands_atomic() {
    let mut node = text("A🦀\nB");
    let before = node.clone();
    assert!(node.apply_paragraph_style(2..3, &center()).is_err());
    assert_eq!(node, before);
    for invalid in [f64::NAN, -1.0, f64::INFINITY] {
        assert!(node
            .apply_paragraph_style(
                0..0,
                &ParagraphStylePatch {
                    space_before: TextStyleChange::Set(invalid),
                    ..Default::default()
                }
            )
            .is_err());
        assert_eq!(node, before);
    }
    node.text_paragraphs.push(TextParagraph {
        start: 1,
        end: 5,
        alignment: Some(TextAlign::Center),
        ..Default::default()
    });
    assert!(validate_text_paragraphs(&node).is_err());
    node.text_paragraphs[0].start = 0;
    node.text_paragraphs[0].end = 6;
    validate_text_paragraphs(&node).unwrap();
    node.text_paragraphs.push(node.text_paragraphs[0].clone());
    assert!(validate_text_paragraphs(&node).is_err());
}

#[test]
fn paragraph_commands_are_one_history_edit_locked_aware_and_serialize_exactly() {
    let mut document = Document::new("Paragraph commands");
    let page = document.pages[0].id;
    let id = document.insert_node(page, text("one\ntwo\n")).unwrap();
    let mut editor = Editor::new(document);
    editor.selection = vec![id];
    let before = editor.document.to_json().unwrap();
    assert_eq!(
        editor.style_text_paragraphs(id, 0..8, &center()).unwrap(),
        2
    );
    let after = editor.document.to_json().unwrap();
    assert_eq!(editor.history_len(), 1);
    assert_eq!(
        Document::from_json(&after).unwrap().to_json().unwrap(),
        after
    );
    assert_eq!(
        editor.style_text_paragraphs(id, 0..8, &center()).unwrap(),
        0
    );
    assert_eq!(editor.history_len(), 1);
    assert!(editor.undo());
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert!(editor.redo());
    assert_eq!(editor.document.to_json().unwrap(), after);
    editor.document.nodes.get_mut(&id).unwrap().locked = true;
    let locked = editor.document.to_json().unwrap();
    assert_eq!(
        editor.style_text_paragraphs(id, 0..0, &ParagraphStylePatch::inherit_all()),
        Err(TextStyleError::Locked)
    );
    assert_eq!(editor.document.to_json().unwrap(), locked);
    assert_eq!(editor.history_len(), 1);
}
