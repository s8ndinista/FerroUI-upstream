use super::{
    ITextProvider, ITextRangeProvider, IValueProvider, SupportedTextSelection, TextAttribute,
    TextAttributeValue, TextPatternRangeEndpoint as Endpoint, TextProviderError, TextUnit,
};
use crate::automation::peers::ControlAutomationPeer;
use crate::documents::{
    Bold, CharacterFormat, Hyperlink, InlineUIContainer, Paragraph, RichTextDocument, Run, Span,
    UnderlineStyle,
};
use crate::test_support::{TestRoot, test_scope};
use crate::{Border, RichEditBox, RichTextBlock};
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::{Brushes, FontWeight, TextAlignment};
use ferroui_base::{Point, Rect, Ref, Size, Thickness};
use std::cell::Cell;
use std::rc::Rc;

fn provider(view: &RichTextBlock) -> Rc<dyn ITextProvider> {
    ControlAutomationPeer::create_peer_for_element(view)
        .get_provider::<dyn ITextProvider>()
        .expect("Rich text exposes the Text pattern")
}

fn rich(text: &str) -> Ref<RichTextBlock> {
    let view = RichTextBlock::new();
    view.set_document(RichTextDocument::from_text(text));
    view.set_is_text_selection_enabled(true);
    view
}

fn found(provider: &dyn ITextProvider, text: &str) -> Rc<dyn ITextRangeProvider> {
    provider
        .document_range()
        .unwrap()
        .find_text(text, false, false)
        .unwrap()
        .unwrap()
}

#[test]
fn rich_automation_document_text_preserves_utf16_boundaries_and_read_only_semantics() {
    let _scope = test_scope();
    let view = rich("a😀b\nnext");
    let text = provider(&view);
    let range = text.document_range().unwrap();
    assert_eq!(range.get_text(None).unwrap(), "a😀b\nnext");
    assert_eq!(range.get_text(Some(0)).unwrap(), "");
    assert_eq!(range.get_text(Some(2)).unwrap(), "a");
    assert_eq!(range.get_text(Some(3)).unwrap(), "a😀");
    assert_eq!(
        range
            .get_attribute_value(TextAttribute::IsReadOnly)
            .unwrap(),
        TextAttributeValue::Bool(true)
    );
    assert_eq!(
        text.supported_text_selection().unwrap(),
        SupportedTextSelection::Single
    );
    let emoji = found(&*text, "😀");
    assert_eq!(
        emoji
            .compare_endpoints(Endpoint::End, &*emoji, Endpoint::Start)
            .unwrap(),
        2
    );
}

#[test]
fn rich_automation_editor_exposes_text_and_value_over_the_same_undoable_document() {
    let _scope = test_scope();
    let editor = RichEditBox::new();
    editor.set_text("before");
    let peer = ControlAutomationPeer::create_peer_for_element(&editor);
    let value = peer.get_provider::<dyn IValueProvider>().unwrap();
    let text = peer.get_provider::<dyn ITextProvider>().unwrap();
    assert_eq!(
        text.document_range()
            .unwrap()
            .get_attribute_value(TextAttribute::IsReadOnly)
            .unwrap(),
        TextAttributeValue::Bool(false)
    );
    value.set_value(Some("after")).unwrap();
    assert_eq!(
        text.document_range().unwrap().get_text(None).unwrap(),
        "after"
    );
    assert!(editor.undo().unwrap());
    assert_eq!(
        text.document_range().unwrap().get_text(None).unwrap(),
        "before"
    );
    editor.set_is_read_only(true);
    assert_eq!(
        text.document_range()
            .unwrap()
            .get_attribute_value(TextAttribute::IsReadOnly)
            .unwrap(),
        TextAttributeValue::Bool(true)
    );
    assert!(value.set_value(Some("forbidden")).is_err());
    found(&*text, "before").select().unwrap();
    assert_eq!(editor.selected_text(), "before");
}

#[test]
fn rich_automation_selection_matches_the_view_and_rejects_disjoint_selection() {
    let _scope = test_scope();
    let view = rich("alpha beta gamma");
    let text = provider(&view);
    let alpha = found(&*text, "alpha");
    let beta = found(&*text, "beta");
    let gamma = found(&*text, "gamma");
    beta.select().unwrap();
    assert_eq!((view.selection_start(), view.selection_end()), (6, 10));
    assert_eq!(
        text.get_selection().unwrap()[0].get_text(None).unwrap(),
        "beta"
    );
    assert_eq!(
        alpha
            .add_to_selection()
            .unwrap_err()
            .downcast_ref::<TextProviderError>(),
        Some(&TextProviderError::MultipleSelectionNotSupported)
    );
    text.document_range().unwrap().select().unwrap();
    assert!(beta.remove_from_selection().is_err());
    alpha.remove_from_selection().unwrap();
    assert_eq!(view.selected_text(), " beta gamma");
    gamma.remove_from_selection().unwrap();
    assert_eq!(view.selected_text(), " beta ");
    view.set_is_text_selection_enabled(false);
    assert_eq!(
        text.supported_text_selection().unwrap(),
        SupportedTextSelection::None
    );
    assert!(text.get_selection().unwrap().is_empty());
    assert_eq!(
        beta.select()
            .unwrap_err()
            .downcast_ref::<TextProviderError>(),
        Some(&TextProviderError::SelectionNotSupported)
    );
}

#[test]
fn rich_automation_live_clones_follow_edits_and_ranges_do_not_change_selection() {
    let _scope = test_scope();
    let view = rich("alpha beta");
    view.select(1, 3);
    let text = provider(&view);
    let beta = found(&*text, "beta");
    let clone = beta.clone_range().unwrap();
    clone
        .move_endpoint_by_unit(Endpoint::End, TextUnit::Character, -1)
        .unwrap();
    assert_eq!(clone.get_text(None).unwrap(), "bet");
    assert_eq!(beta.get_text(None).unwrap(), "beta");
    assert!(!clone.compare(&*beta).unwrap());
    assert_eq!((view.selection_start(), view.selection_end()), (1, 3));
    view.document().replace_text(0..0, "x").unwrap();
    assert_eq!(beta.get_text(None).unwrap(), "beta");
    assert_eq!(
        beta.compare_endpoints(
            Endpoint::Start,
            &*text.document_range().unwrap(),
            Endpoint::Start
        )
        .unwrap(),
        7
    );
    assert_eq!(clone.get_text(None).unwrap(), "bet");
}

#[test]
fn rich_automation_movement_uses_uia_normalization_and_grapheme_units() {
    let _scope = test_scope();
    let view = rich("one two a\u{301}😀\nlast");
    let text = provider(&view);
    let range = found(&*text, "ne two");
    range.expand_to_enclosing_unit(TextUnit::Word).unwrap();
    assert_eq!(range.get_text(None).unwrap(), "one");
    assert_eq!(range.move_by(TextUnit::Word, 2).unwrap(), 2);
    assert_eq!(range.get_text(None).unwrap(), "two");
    assert_eq!(range.move_by(TextUnit::Word, 0).unwrap(), 0);
    assert_eq!(range.get_text(None).unwrap(), "two");
    let accent = found(&*text, "a\u{301}");
    assert_eq!(accent.move_by(TextUnit::Character, 1).unwrap(), 1);
    assert_eq!(accent.get_text(None).unwrap(), "😀");
    accent
        .move_endpoint_by_range(
            Endpoint::End,
            &*accent.clone_range().unwrap(),
            Endpoint::Start,
        )
        .unwrap();
    assert_eq!(accent.move_by(TextUnit::Character, 1).unwrap(), 1);
    assert_eq!(
        accent
            .compare_endpoints(Endpoint::Start, &*found(&*text, "\n"), Endpoint::Start)
            .unwrap(),
        0
    );
    let last = found(&*text, "last");
    assert_eq!(last.move_by(TextUnit::Word, i32::MAX).unwrap(), 0);
    assert_eq!(last.get_text(None).unwrap(), "last");
}

#[test]
fn rich_automation_endpoint_crossing_collapses_instead_of_reversing() {
    let _scope = test_scope();
    let view = rich("abcdef");
    let text = provider(&view);
    let range = found(&*text, "bc");
    assert_eq!(
        range
            .move_endpoint_by_unit(Endpoint::Start, TextUnit::Character, 4)
            .unwrap(),
        4
    );
    assert_eq!(range.get_text(None).unwrap(), "");
    assert_eq!(
        range
            .compare_endpoints(Endpoint::Start, &*range, Endpoint::End)
            .unwrap(),
        0
    );
    assert_eq!(
        range
            .move_endpoint_by_unit(Endpoint::End, TextUnit::Character, i32::MIN)
            .unwrap(),
        -5
    );
    assert_eq!(
        range
            .compare_endpoints(
                Endpoint::End,
                &*text.document_range().unwrap(),
                Endpoint::Start
            )
            .unwrap(),
        0
    );
}

#[test]
fn rich_automation_attributes_distinguish_uniform_mixed_and_unsupported_values() {
    let _scope = test_scope();
    let view = RichTextBlock::new();
    view.set_font_size(24.0);
    let paragraph = Paragraph::with_text("plain ");
    paragraph.set_text_alignment(TextAlignment::Center);
    paragraph.set_text_indent(12.0);
    let bold = Bold::new();
    bold.add_child_text("bold");
    paragraph.add_child(bold);
    paragraph.add_child_text(" tail");
    view.blocks().add(paragraph);
    let text = provider(&view);
    let all = text.document_range().unwrap();
    assert_eq!(
        all.get_attribute_value(TextAttribute::FontWeight).unwrap(),
        TextAttributeValue::Mixed
    );
    assert_eq!(
        all.get_attribute_value(TextAttribute::FontSize).unwrap(),
        TextAttributeValue::Number(18.0)
    );
    assert_eq!(
        all.get_attribute_value(TextAttribute::HorizontalTextAlignment)
            .unwrap(),
        TextAttributeValue::Integer(1)
    );
    assert_eq!(
        all.get_attribute_value(TextAttribute::IndentationFirstLine)
            .unwrap(),
        TextAttributeValue::Number(9.0)
    );
    let matched = all
        .find_attribute(
            TextAttribute::FontWeight,
            &TextAttributeValue::Integer(FontWeight::Bold.0),
            false,
        )
        .unwrap()
        .unwrap();
    assert_eq!(matched.get_text(None).unwrap(), "bold");
    assert_eq!(
        all.get_attribute_value(TextAttribute::Unknown(59999))
            .unwrap(),
        TextAttributeValue::NotSupported
    );
    assert!(
        all.find_attribute(
            TextAttribute::Unknown(59999),
            &TextAttributeValue::Bool(true),
            false
        )
        .is_err()
    );
}

#[test]
fn rich_automation_format_units_merge_equal_runs_and_stop_at_hyperlinks() {
    let _scope = test_scope();
    let view = RichTextBlock::new();
    view.set_foreground(Some(Brushes::black()));
    let paragraph = Paragraph::new();
    for text in ["ab", "cd"] {
        let span = Span::new();
        span.add_child_text(text);
        paragraph.add_child(span);
    }
    let link = Hyperlink::new();
    link.add_child_text("ef");
    link.set_underline_style(UnderlineStyle::None);
    link.set_foreground(Some(Brushes::black()));
    paragraph.add_child(link);
    paragraph.add_child_text("gh");
    view.blocks().add(paragraph);
    let text = provider(&view);
    let range = found(&*text, "b");
    range.expand_to_enclosing_unit(TextUnit::Format).unwrap();
    assert_eq!(range.get_text(None).unwrap(), "abcd");
    assert_eq!(range.move_by(TextUnit::Format, 1).unwrap(), 1);
    assert_eq!(range.get_text(None).unwrap(), "ef");
    assert_eq!(range.move_by(TextUnit::Format, 1).unwrap(), 1);
    assert_eq!(range.get_text(None).unwrap(), "gh");
}

#[test]
fn rich_automation_text_search_retains_unicode_offsets_and_backward_matches() {
    let _scope = test_scope();
    let view = rich("😀İ one ONE");
    let text = provider(&view);
    let all = text.document_range().unwrap();
    let matched = all.find_text("one", true, true).unwrap().unwrap();
    assert_eq!(matched.get_text(None).unwrap(), "ONE");
    let mapped = all.find_text("i\u{307}", false, true).unwrap().unwrap();
    assert_eq!(mapped.get_text(None).unwrap(), "İ");
    assert_eq!(
        mapped
            .compare_endpoints(Endpoint::Start, &*all, Endpoint::Start)
            .unwrap(),
        2
    );
    assert!(all.find_text("", false, false).is_err());
}

#[test]
fn rich_automation_embedded_objects_are_enumerable_and_have_real_ranges() {
    let _scope = test_scope();
    let view = RichTextBlock::new();
    let paragraph = Paragraph::with_text("a");
    let child = Border::new();
    paragraph.add_child(InlineUIContainer::with_child(child.clone()));
    paragraph.add_child_text("b");
    view.blocks().add(paragraph);
    let text = provider(&view);
    let all = text.document_range().unwrap();
    let children = all.get_children().unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(
        children[0],
        ControlAutomationPeer::create_peer_for_element(&child)
    );
    let range = text.range_from_child(&children[0]).unwrap();
    assert_eq!(range.get_text(None).unwrap(), "\u{fffc}");
    assert_eq!(range.get_enclosing_element().unwrap(), children[0]);
    range.expand_to_enclosing_unit(TextUnit::Format).unwrap();
    assert_eq!(range.get_text(None).unwrap(), "\u{fffc}");
    assert_eq!(all.get_enclosing_element().unwrap(), text.peer());
    assert!(
        text.range_from_child(&ControlAutomationPeer::create_peer_for_element(
            &Border::new()
        ))
        .is_err()
    );
}

#[test]
fn rich_automation_replaced_documents_and_foreign_ranges_fail_explicitly() {
    let _scope = test_scope();
    let view = rich("first");
    let other = rich("second");
    let text = provider(&view);
    let range = text.document_range().unwrap();
    let foreign = provider(&other).document_range().unwrap();
    assert_eq!(
        range
            .compare(&*foreign)
            .unwrap_err()
            .downcast_ref::<TextProviderError>(),
        Some(&TextProviderError::IncompatibleRange)
    );
    view.set_document(RichTextDocument::from_text("replacement"));
    assert_eq!(
        range
            .get_text(None)
            .unwrap_err()
            .downcast_ref::<TextProviderError>(),
        Some(&TextProviderError::ElementNotAvailable)
    );
    assert_eq!(
        text.document_range().unwrap().get_text(None).unwrap(),
        "replacement"
    );
}

#[test]
fn rich_automation_retained_ranges_do_not_keep_controls_alive() {
    let _scope = test_scope();
    let view = rich("temporary");
    let text = provider(&view);
    let range = text.document_range().unwrap();
    let weak = view.downgrade();
    drop(view);
    assert!(weak.upgrade().is_none());
    assert_eq!(
        range
            .get_text(None)
            .unwrap_err()
            .downcast_ref::<TextProviderError>(),
        Some(&TextProviderError::ElementNotAvailable)
    );
    assert!(text.document_range().is_err());
}

#[test]
fn rich_automation_notifications_follow_document_replacement_and_dispose_cleanly() {
    let _scope = test_scope();
    let view = rich("initial");
    let original = view.document();
    let text = provider(&view);
    let changes = Rc::new(Cell::new(0));
    let selections = Rc::new(Cell::new(0));
    let sink = changes.clone();
    let changed = text
        .text_changed(Rc::new(move || sink.set(sink.get() + 1)))
        .unwrap();
    let sink = selections.clone();
    let selected = text
        .text_selection_changed(Rc::new(move || sink.set(sink.get() + 1)))
        .unwrap();
    view.select(1, 2);
    assert_eq!(selections.get(), 1);
    original.replace_text(0..0, "x").unwrap();
    assert_eq!(changes.get(), 1);
    view.set_document(RichTextDocument::from_text("new"));
    assert_eq!(changes.get(), 2);
    original.replace_text(0..0, "old").unwrap();
    assert_eq!(changes.get(), 2);
    view.document()
        .apply_format(
            0..1,
            &CharacterFormat {
                font_weight: Some(FontWeight::Bold),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(changes.get(), 3);
    changed.dispose();
    selected.dispose();
    let selections_before = selections.get();
    view.document().replace_text(0..0, "again").unwrap();
    assert_eq!(changes.get(), 3);
    assert_eq!(selections.get(), selections_before);
}

#[test]
fn rich_automation_geometry_uses_root_coordinates_and_excludes_hidden_text() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich("alpha beta gamma delta");
    view.set_margin(Thickness::new(20.0, 10.0, 0.0, 0.0));
    let root = TestRoot::with_child(view.clone());
    root.measure(Size::new(180.0, 80.0));
    root.arrange(Rect::new(0.0, 0.0, 180.0, 80.0));
    let text = provider(&view);
    let all = text.document_range().unwrap();
    let rectangles = all.get_bounding_rectangles().unwrap();
    assert!(!rectangles.is_empty());
    assert!(rectangles[0].x >= 20.0);
    assert!(rectangles[0].y >= 10.0);
    let first = rectangles[0];
    let point = Point::new(first.x + 0.01, first.y + first.height / 2.0);
    let hit = text.range_from_point(point).unwrap();
    hit.expand_to_enclosing_unit(TextUnit::Character).unwrap();
    assert_eq!(hit.get_text(None).unwrap(), "a");
    assert!(!text.get_visible_ranges().unwrap().is_empty());
    let caret = text.get_selection().unwrap().remove(0);
    assert!(caret.get_bounding_rectangles().unwrap().is_empty());
    view.set_is_visible(false);
    assert!(all.get_bounding_rectangles().unwrap().is_empty());
    assert_eq!(
        text.get_visible_ranges().unwrap()[0]
            .get_text(None)
            .unwrap(),
        ""
    );
}

#[test]
fn rich_automation_line_units_follow_wrapped_layout_and_paragraph_units_follow_breaks() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich("alpha beta gamma delta\nnext");
    view.measure(Size::new(70.0, 500.0));
    view.arrange(Rect::new(0.0, 0.0, 70.0, 500.0));
    let text = provider(&view);
    let range = found(&*text, "gamma");
    range.expand_to_enclosing_unit(TextUnit::Line).unwrap();
    let line = view
        .line_metrics()
        .into_iter()
        .find(|line| line.start <= 11 && 11 < line.start + line.length)
        .unwrap();
    assert_eq!(
        range.get_text(None).unwrap(),
        view.document()
            .get_text(line.start..line.start + line.length)
            .unwrap()
    );
    range.expand_to_enclosing_unit(TextUnit::Paragraph).unwrap();
    assert_eq!(range.get_text(None).unwrap(), "alpha beta gamma delta\n");
    range.expand_to_enclosing_unit(TextUnit::Page).unwrap();
    assert_eq!(range.get_text(None).unwrap(), view.document().text());
}
