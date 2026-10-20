//! CoreText Range/Text/NewSelection regression contracts through the actual
//! input manager, rich control, document and raw/routed acknowledgement path.
use super::*;
use ferroui_base::input::text_input::{TextInputMethodEdit, TextSelection};

fn edit(start: i32, end: i32, text: &str, anchor: i32, active: i32) -> TextInputMethodEdit {
    TextInputMethodEdit::new(
        TextSelection::new(start, end),
        text,
        TextSelection::new(anchor, active),
    )
}

#[test]
fn rich_exact_input_replaces_requested_range_independently_of_selection_and_undoes_once() {
    let f = Fixture::new(true);
    f.owner.set_text("a😀bcd").unwrap();
    f.owner.select(5, 6);
    f.owner.editor().clear_history();
    let before = f.owner.document().snapshot();
    assert!(f
        .connection()
        .apply_edit(&edit(1, 3, "XY", 3, 1), 1)
        .unwrap());
    assert_eq!(f.owner.text(), "aXYbcd");
    assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (3, 1));
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.document().snapshot(), before);
    assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (5, 6));
    assert!(!f.owner.undo().unwrap());
    assert!(f.owner.redo().unwrap());
    assert_eq!(
        f.connection().edit_state().unwrap().selection,
        TextSelection::new(3, 1)
    );
}

#[test]
fn rich_exact_input_empty_replacement_deletes_real_range() {
    let f = Fixture::new(true);
    assert!(f.connection().apply_edit(&edit(0, 4, "", 0, 0), 0).unwrap());
    assert_eq!(f.owner.text(), "");
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.text(), "abcd");
}

#[test]
fn rich_exact_input_invalid_utf16_and_policy_changes_leave_text_selection_and_history_intact() {
    let f = Fixture::new(true);
    f.owner.set_text("a😀b").unwrap();
    f.owner.select(1, 3);
    f.owner.editor().clear_history();
    let before = f.owner.document().snapshot();
    let version = f.owner.document().revision();
    for request in [
        edit(-1, 0, "x", 0, 0),
        edit(1, 2, "x", 1, 1),
        edit(0, 0, "", 2, 2),
        edit(0, 10, "", 0, 0),
    ] {
        assert_eq!(
            f.connection().apply_edit(&request, 0),
            Err(TextInputMethodError::InvalidRange)
        );
    }
    f.owner.set_accepts_return(false);
    assert_eq!(
        f.connection().apply_edit(&edit(1, 3, "x\ny", 3, 3), 0),
        Err(TextInputMethodError::Rejected)
    );
    f.owner.set_accepts_return(true);
    assert_eq!(
        f.connection().apply_edit(&edit(1, 3, "\r\n", 3, 3), 0),
        Err(TextInputMethodError::Rejected)
    );
    f.owner.set_max_length(4);
    assert_eq!(
        f.connection().apply_edit(&edit(1, 3, "long", 5, 5), 0),
        Err(TextInputMethodError::Rejected)
    );
    assert_eq!(f.owner.document().snapshot(), before);
    assert_eq!(f.owner.document().revision(), version);
    assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (1, 3));
    assert!(!f.owner.can_undo());
}

#[test]
fn rich_exact_composition_uses_current_utf16_deltas_and_retains_suffix_when_caret_moves_inside() {
    let f = Fixture::new(true);
    let before = f.owner.document().snapshot();
    let session = f.connection().begin_composition().unwrap();
    assert_eq!(f.owner.document().snapshot(), before);
    session.apply_edit(&edit(1, 3, "😀XYZ", 3, 3)).unwrap();
    assert_eq!(f.owner.text(), "a😀XYZd");
    assert_eq!(f.owner.editor().active_composition_range(), Some(1..6));
    assert_eq!(f.connection().edit_state().unwrap().text, "a😀XYZd");
    // Replaces an interior suffix range in visible UTF-16 coordinates. The
    // resulting caret is deliberately before the end of the composition.
    session.apply_edit(&edit(4, 5, "q", 3, 3)).unwrap();
    assert_eq!(f.owner.text(), "a😀XqZd");
    assert!(session.is_current());
    assert!(session.complete(2).unwrap());
    assert_eq!(f.owner.text(), "a😀XqZd");
    assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (3, 3));
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.document().snapshot(), before);
    assert!(!f.owner.undo().unwrap());
    assert!(f.owner.redo().unwrap());
    assert_eq!(f.owner.text(), "a😀XqZd");
}

#[test]
fn rich_exact_composition_extends_across_original_selection_and_preserves_cancel_snapshot() {
    let f = Fixture::new(true);
    let before = f.owner.document().snapshot();
    let session = f.connection().begin_composition().unwrap();
    session.apply_edit(&edit(1, 2, "X", 2, 2)).unwrap();
    session.apply_edit(&edit(0, 1, "YZ", 3, 3)).unwrap();
    session.apply_edit(&edit(4, 5, "Q", 4, 4)).unwrap();
    assert_eq!(f.owner.text(), "YZXcQ");
    assert_eq!(f.owner.editor().active_composition_range(), Some(0..5));
    assert!(!f.owner.can_undo());
    assert!(session.cancel().unwrap());
    assert_eq!(f.owner.document().snapshot(), before);
    assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (1, 3));
    assert!(!f.owner.can_undo());
}

#[test]
fn rich_exact_composition_empty_deletion_commits_without_resurrecting_original_selection() {
    let f = Fixture::new(true);
    let session = f.connection().begin_composition().unwrap();
    session.apply_edit(&edit(1, 3, "", 1, 1)).unwrap();
    assert_eq!(f.owner.text(), "ad");
    assert!(session.complete(0).unwrap());
    assert_eq!(f.owner.text(), "ad");
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.text(), "abcd");
    assert!(f.owner.redo().unwrap());
    assert_eq!(f.owner.text(), "ad");
}

#[test]
fn rich_exact_composition_selection_update_keeps_full_preview_and_does_not_cancel() {
    let f = Fixture::new(true);
    let session = f.connection().begin_composition().unwrap();
    session.apply_edit(&edit(1, 3, "hello", 6, 6)).unwrap();
    session.select_exact(TextSelection::new(5, 2)).unwrap();
    assert!(session.is_current());
    assert_eq!(f.owner.text(), "ahellod");
    assert!(f.owner.editor().is_composing());
    assert!(session.complete(0).unwrap());
    assert_eq!(f.owner.text(), "ahellod");
    assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (5, 2));
}

#[test]
fn rich_exact_composition_intercepted_completion_preserves_preview_for_retry() {
    let f = Fixture::new(true);
    let session = f.connection().begin_composition().unwrap();
    session.apply_edit(&edit(1, 3, "preview", 3, 3)).unwrap();
    let preview = f.owner.document().snapshot();
    let token = f.owner.add_handler_with(
        InputElement::text_input_event(),
        |_, event: &TextInputEventArgs| event.set_handled(true),
        RoutingStrategies::TUNNEL,
        false,
    );
    assert_eq!(session.complete(1), Err(TextInputMethodError::Intercepted));
    assert_eq!(f.owner.document().snapshot(), preview);
    assert!(session.is_current());
    f.owner
        .remove_handler(InputElement::text_input_event(), token);
    assert!(session.complete(2).unwrap());
    assert_eq!(f.owner.text(), "apreviewd");
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.text(), "abcd");
}

#[test]
fn rich_exact_input_selection_cancellation_rolls_back_structured_edit_without_history() {
    let f = Fixture::new(true);
    let before = f.owner.document().snapshot();
    f.owner.selection_changing(|_, args| args.set_cancel(true));
    assert_eq!(
        f.connection().apply_edit(&edit(0, 1, "XY", 5, 5), 0),
        Err(TextInputMethodError::Rejected)
    );
    assert_eq!(f.owner.document().snapshot(), before);
    assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (1, 3));
    assert!(!f.owner.can_undo());
}

#[test]
fn rich_exact_composition_reentrant_callback_keeps_application_edit_and_rejects_native_success() {
    let f = Fixture::new(true);
    let session = f.connection().begin_composition().unwrap();
    let owner = f.owner.downgrade();
    f.owner.add_handler(
        RichEditBox::text_composition_changed_event(),
        move |_, _| {
            owner.upgrade().unwrap().set_text("external").unwrap();
        },
    );
    assert_eq!(
        session.apply_edit(&edit(1, 3, "preview", 3, 3)),
        Err(TextInputMethodError::ReentrantEdit)
    );
    assert_eq!(f.owner.text(), "external");
    assert!(!session.is_current());
    assert!(session.complete(0).is_err());
    assert_eq!(f.owner.text(), "external");
}

#[test]
fn rich_exact_input_identical_text_updates_selection_without_destroying_redo_or_formatting() {
    let f = Fixture::new(true);
    f.owner.insert_text("x").unwrap();
    f.owner.undo().unwrap();
    let before = f.owner.document().snapshot();
    assert!(!f
        .connection()
        .apply_edit(&edit(1, 3, "bc", 0, 0), 0)
        .unwrap());
    assert_eq!(f.owner.document().snapshot(), before);
    assert!(f.owner.can_redo());
}
