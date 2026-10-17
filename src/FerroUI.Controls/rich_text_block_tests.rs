use crate::documents::{
    Hyperlink, InlineCollection, InlineUIContainer, Paragraph, RichTextDocument, Run, Span,
    TextGravity,
};
use crate::test_support::test_scope;
use crate::{Border, RichTextBlock, RichTextBlockOverflow};
use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_base::{Point, Rect, Ref, Size};
use std::cell::Cell;
use std::rc::Rc;

fn rich(text: &str) -> Ref<RichTextBlock> {
    let view = RichTextBlock::new();
    view.blocks().add(Paragraph::with_text(text));
    view
}
fn measure(view: &RichTextBlock, width: f64, height: f64) {
    view.measure(Size::new(width, height));
    view.arrange(Rect::new(0.0, 0.0, width, height));
}

#[test]
fn rich_document_defaults_are_instance_local() {
    let _scope = test_scope();
    let first = rich("first");
    let second = RichTextBlock::new();
    assert_eq!(first.text(), "first");
    assert_eq!(second.text(), "");
    assert_ne!(first.document(), second.document());
}

#[test]
fn rich_paragraphs_measure_and_report_document_offsets() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich("one");
    view.blocks().add(Paragraph::with_text("two"));
    measure(&view, 200.0, 100.0);
    assert_eq!(view.text(), "one\ntwo");
    let metrics = view.line_metrics();
    assert_eq!(metrics.len(), 2);
    assert_eq!(metrics[0].start, 0);
    assert_eq!(metrics[1].start, 4);
    assert!(metrics[1].bounds.y > metrics[0].bounds.y);
    assert_eq!(view.content_end(), 7);
}

#[test]
fn rich_empty_document_has_a_caret_line() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = RichTextBlock::new();
    measure(&view, 200.0, 100.0);
    assert_eq!(view.line_metrics().len(), 1);
    assert!(view.cursor_rectangle().height > 0.0);
    assert_eq!(view.hit_test_text_position(Point::new(50.0, 5.0)), 0);
}

#[test]
fn rich_selection_preserves_direction_and_scalar_boundaries() {
    let _scope = test_scope();
    let view = rich("a😀b");
    view.select(4, 2);
    assert_eq!(view.selection_start(), 4);
    assert_eq!(view.selection_end(), 1);
    assert_eq!(view.selected_text(), "😀b");
    view.select(0, usize::MAX);
    assert_eq!(view.selected_text(), "a😀b");
}

#[test]
fn rich_inherited_fonts_and_span_replacement_invalidate_layout() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = RichTextBlock::new();
    let paragraph = Paragraph::new();
    let span = Span::new();
    span.add_child_text("short");
    paragraph.add_child(span.clone());
    view.blocks().add(paragraph.clone());
    view.set_font_size(24.0);
    measure(&view, 500.0, 100.0);
    assert_eq!(paragraph.font_size(), 24.0);
    let width = view.line_metrics()[0].bounds.width;
    let replacement = InlineCollection::new();
    replacement.add_text("much longer content");
    span.set_inlines(replacement);
    measure(&view, 500.0, 100.0);
    assert_eq!(view.text(), "much longer content");
    assert!(view.line_metrics()[0].bounds.width > width);
}

#[test]
fn rich_wrapping_reuses_shaping_and_selection_geometry() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich("alpha beta gamma delta");
    measure(&view, 65.0, 300.0);
    assert!(view.line_metrics().len() > 1);
    view.select(0, view.document().text_length());
    assert!(view.selection_rects().len() > 1);
    let rect = view.get_rect_from_character_index(2);
    let hit = view.hit_test_text_position(Point::new(rect.x, rect.y + rect.height / 2.0));
    assert_eq!(hit, 2);
}

#[test]
fn rich_overflow_columns_continue_without_dropping_text_on_reflow() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich("one two three four five six seven eight nine ten");
    let overflow = RichTextBlockOverflow::new();
    view.set_overflow_content_target(Some(overflow.clone()));
    measure(&view, 65.0, 28.0);
    measure(&overflow, 90.0, 300.0);
    assert!(view.has_overflow_content());
    assert_eq!(overflow.content_start(), view.content_end());
    assert_eq!(overflow.content_end(), view.document().text_length());
    let original = overflow.line_metrics();
    measure(&overflow, 80.0, 300.0);
    measure(&overflow, 90.0, 300.0);
    assert_eq!(overflow.line_metrics(), original);
}

#[test]
fn rich_overflow_rejects_cycles_and_second_owners() {
    let _scope = test_scope();
    let first = RichTextBlockOverflow::new();
    let second = RichTextBlockOverflow::new();
    let third = RichTextBlock::new();
    assert!(first.try_set_overflow_content_target(Some(second.clone())));
    assert!(!second.try_set_overflow_content_target(Some(first.clone())));
    assert!(!third.try_set_overflow_content_target(Some(second)));
}

#[test]
fn rich_embedded_controls_keep_text_offsets_and_receive_layout() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = RichTextBlock::new();
    let paragraph = Paragraph::with_text("a");
    paragraph.add_child(InlineUIContainer::new());
    let child = Border::new();
    child.set_width(30.0);
    child.set_height(20.0);
    paragraph.add_child(InlineUIContainer::with_child(child.clone()));
    paragraph.add_child(Run::with_text(Some("b")));
    view.blocks().add(paragraph);
    measure(&view, 200.0, 100.0);
    assert_eq!(view.text(), "a\u{fffc}b");
    assert_eq!(child.bounds().width, 30.0);
    assert!(child.bounds().x > 0.0);
    assert_eq!(view.content_end(), 3);
}

#[test]
fn rich_formatting_edit_rebuilds_runs_without_losing_text() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich("format me");
    measure(&view, 200.0, 100.0);
    let mut format = crate::documents::CharacterFormat::default();
    format.font_weight = Some(FontWeight::Bold);
    view.document().apply_format(0..6, &format).unwrap();
    measure(&view, 200.0, 100.0);
    assert_eq!(view.text(), "format me");
    assert_eq!(view.content_end(), 9);
}

#[test]
fn rich_max_lines_produces_a_stable_continuation() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich("one two three four five");
    view.set_text_wrapping(TextWrapping::Wrap);
    view.set_max_lines(1);
    measure(&view, 40.0, 200.0);
    assert_eq!(view.line_metrics().len(), 1);
    assert!(view.has_overflow_content());
    assert!(view.content_end() < view.document().text_length());
}

#[test]
fn rich_selection_tracks_external_edits_and_is_shared_across_columns() {
    let _scope = test_scope();
    let view = rich("alpha beta gamma");
    let overflow = RichTextBlockOverflow::new();
    view.set_overflow_content_target(Some(overflow.clone()));
    overflow.select(6, 10);
    let changes = Rc::new(Cell::new(0));
    let observed = changes.clone();
    let _subscription = overflow.selection_changed(move |_| observed.set(observed.get() + 1));
    view.document().replace_text(0..0, "😀 ").unwrap();
    assert_eq!(view.selected_text(), "beta");
    assert_eq!(overflow.selected_text(), "beta");
    assert_eq!(
        (overflow.selection_start(), overflow.selection_end()),
        (9, 13)
    );
    assert_eq!(changes.get(), 1);
}

#[test]
fn rich_document_view_ownership_is_checked_before_detaching_content() {
    let _scope = test_scope();
    let first = rich("first");
    let second = rich("second");
    let shared = first.document();
    assert!(!second.try_set_document(shared.clone()));
    assert_eq!(first.text(), "first");
    assert_eq!(second.text(), "second");
    assert!(first.try_set_document(RichTextDocument::new()));
    assert!(second.try_set_document(shared));
    assert_eq!(second.text(), "first");
    let empty = first.document();
    assert!(!second.try_set_document(empty));
}

#[test]
fn rich_range_geometry_does_not_mutate_selection_and_rejects_invalid_utf16() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich("a😀b c d e");
    measure(&view, 40.0, 200.0);
    view.select(3, 4);
    assert!(!view.get_rects_for_range(0..5).is_empty());
    assert!(view.get_rects_for_range(0..2).is_empty());
    assert!(view.get_rects_for_range(0..usize::MAX).is_empty());
    assert_eq!((view.selection_start(), view.selection_end()), (3, 4));
}

#[test]
fn rich_pointer_geometry_keeps_the_insertion_offset_and_finds_overflow_view() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich("one two three four five six seven eight");
    let pointer = view
        .document()
        .create_pointer(2, TextGravity::Forward)
        .unwrap();
    assert!(pointer.get_character_rect(TextGravity::Forward).is_none());
    let overflow = RichTextBlockOverflow::new();
    view.set_overflow_content_target(Some(overflow.clone()));
    measure(&view, 65.0, 28.0);
    measure(&overflow, 100.0, 300.0);
    let forward = pointer.get_character_rect(TextGravity::Forward).unwrap();
    let backward = pointer.get_character_rect(TextGravity::Backward).unwrap();
    assert_eq!(forward, backward);
    assert_eq!(forward.x, view.get_rect_from_character_index(2).x);
    assert_eq!(forward.width, 0.0);
    let later = view
        .document()
        .create_pointer(overflow.content_start() + 1, TextGravity::Forward)
        .unwrap();
    assert_eq!(later.visual_parent(), Some(overflow.clone().upcast()));
    let rect = later.get_character_rect(TextGravity::Forward).unwrap();
    assert_eq!(
        rect.y,
        overflow.get_rect_from_character_index(later.offset()).y
    );
}

fn key(view: &RichTextBlock, value: Key, release: bool) -> KeyEventArgs {
    let mut event = KeyEventArgs::new();
    event.key = value;
    event.key_modifiers = KeyModifiers::NONE;
    event.set_routed_event(Some(if release {
        InputElement::key_up_event()
    } else {
        InputElement::key_down_event()
    }));
    view.raise_event(&event);
    event
}

#[test]
fn rich_hyperlink_hit_testing_keyboard_focus_and_cancellation_use_the_host() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = RichTextBlock::new();
    let paragraph = Paragraph::with_text("open ");
    let link = Hyperlink::new();
    link.add_child_text("link");
    link.set_navigate_uri(Some("https://example.com".into()));
    paragraph.add_child(link.clone());
    view.blocks().add(paragraph);
    measure(&view, 200.0, 100.0);
    let rect = view.get_rect_from_character_index(6);
    assert_eq!(
        view.hyperlink_at(Point::new(rect.x + 1.0, rect.y + rect.height / 2.0)),
        Some(link.clone())
    );
    assert!(view.hyperlink_at(Point::new(190.0, 95.0)).is_none());
    let invocations = Rc::new(Cell::new(0));
    let observed = invocations.clone();
    let _subscription = link.click(move |event| {
        observed.set(observed.get() + 1);
        event.set_handled(true);
    });
    let revision = view.document().revision();
    assert!(key(&view, Key::Tab, false).handled());
    assert!(link.is_focused());
    assert!(key(&view, Key::Enter, false).handled());
    assert_eq!(invocations.get(), 1);
    assert_eq!(view.document().revision(), revision);
    assert!(!key(&view, Key::Tab, false).handled());
    assert!(!link.is_focused());
    assert!(key(&view, Key::Tab, false).handled());
    assert!(key(&view, Key::Space, false).handled());
    assert_eq!(invocations.get(), 1);
    assert!(key(&view, Key::Space, true).handled());
    assert_eq!(invocations.get(), 2);
}
