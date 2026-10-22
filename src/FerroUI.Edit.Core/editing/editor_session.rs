use super::{
    CaretMovement, IReadOnlySectionProvider, NoReadOnlySections, Selection, SelectionSegment,
    next_caret_position,
};
use crate::document::{
    AnchorMovementType, DocumentError, DocumentSubscription, SimpleSegment, TextDocument,
};
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::fmt;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorOptions {
    pub indentation_size: usize,
    pub convert_tabs_to_spaces: bool,
    pub enable_virtual_space: bool,
    pub allow_scroll_below_document: bool,
    pub show_spaces: bool,
    pub show_tabs: bool,
    pub show_end_of_line: bool,
    pub highlight_current_line: bool,
    pub newline: String,
}
impl Default for EditorOptions {
    fn default() -> Self {
        Self {
            indentation_size: 4,
            convert_tabs_to_spaces: false,
            enable_virtual_space: false,
            allow_scroll_below_document: false,
            show_spaces: false,
            show_tabs: false,
            show_end_of_line: false,
            highlight_current_line: false,
            newline: "\n".into(),
        }
    }
}
impl EditorOptions {
    pub fn indentation_string(&self) -> String {
        if self.convert_tabs_to_spaces {
            " ".repeat(self.indentation_size.clamp(1, 256))
        } else {
            "\t".into()
        }
    }
}

#[derive(Debug)]
pub enum EditError {
    Document(DocumentError),
    ReadOnly,
    InvalidSelection,
    ReplacementCount,
}
impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Document(e) => e.fmt(f),
            Self::ReadOnly => f.write_str("The edit intersects a read-only section"),
            Self::InvalidSelection => f.write_str("The selection lies outside the document"),
            Self::ReplacementCount => {
                f.write_str("Supply one replacement or one replacement per selection")
            }
        }
    }
}
impl std::error::Error for EditError {}
impl From<DocumentError> for EditError {
    fn from(value: DocumentError) -> Self {
        Self::Document(value)
    }
}

struct SessionInner {
    document: TextDocument,
    selection: Rc<RefCell<Selection>>,
    options: RefCell<EditorOptions>,
    read_only: Cell<bool>,
    read_only_sections: RefCell<Rc<dyn IReadOnlySectionProvider>>,
    subscription: DocumentSubscription,
}
impl Drop for SessionInner {
    fn drop(&mut self) {
        self.subscription.dispose();
    }
}

/// The editing state shared by a text area and its commands. All positions use
/// UTF-16. A session can be cloned without copying the document or selections.
#[derive(Clone)]
pub struct EditorSession {
    inner: Rc<SessionInner>,
}
impl EditorSession {
    pub fn new(document: TextDocument) -> Self {
        let selection = Rc::new(RefCell::new(Selection::default()));
        let weak = Rc::downgrade(&selection);
        let subscription = document.changed(move |change| {
            if let Some(selection) = weak.upgrade() {
                let updated = selection.borrow().update_on_document_change(change);
                *selection.borrow_mut() = updated;
            }
        });
        Self {
            inner: Rc::new(SessionInner {
                document,
                selection,
                options: RefCell::new(EditorOptions::default()),
                read_only: Cell::new(false),
                read_only_sections: RefCell::new(Rc::new(NoReadOnlySections)),
                subscription,
            }),
        }
    }
    pub fn document(&self) -> &TextDocument {
        &self.inner.document
    }
    pub fn selection(&self) -> Selection {
        self.inner.selection.borrow().clone()
    }
    pub fn options(&self) -> EditorOptions {
        self.inner.options.borrow().clone()
    }
    pub fn set_options(&self, mut options: EditorOptions) {
        options.indentation_size = options.indentation_size.clamp(1, 256);
        if !matches!(options.newline.as_str(), "\n" | "\r" | "\r\n") {
            options.newline = "\n".into();
        }
        *self.inner.options.borrow_mut() = options;
    }
    pub fn is_read_only(&self) -> bool {
        self.inner.read_only.get()
    }
    pub fn set_is_read_only(&self, value: bool) {
        self.inner.read_only.set(value);
    }
    pub fn set_read_only_section_provider(&self, provider: Rc<dyn IReadOnlySectionProvider>) {
        *self.inner.read_only_sections.borrow_mut() = provider;
    }
    pub fn set_selection(&self, selection: Selection) -> Result<(), EditError> {
        if selection
            .segments()
            .iter()
            .any(|s| s.end_offset() > self.document().text_length())
        {
            return Err(EditError::InvalidSelection);
        }
        *self.inner.selection.borrow_mut() = selection;
        Ok(())
    }
    pub fn select(&self, start: usize, length: usize) -> Result<(), EditError> {
        self.set_selection(Selection::simple(
            start,
            start
                .checked_add(length)
                .ok_or(EditError::InvalidSelection)?,
        ))
    }
    pub fn select_all(&self) {
        *self.inner.selection.borrow_mut() = Selection::simple(0, self.document().text_length());
    }
    pub fn caret_offset(&self) -> usize {
        self.selection().primary().caret
    }
    pub fn set_caret_offset(&self, offset: usize) -> Result<(), EditError> {
        self.set_selection(Selection::empty(offset))
    }
    pub fn selected_text(&self) -> Result<String, EditError> {
        let mut text = String::new();
        for (index, segment) in self.selection().segments().iter().enumerate() {
            if index != 0 {
                text.push_str(&self.options().newline);
            }
            text.push_str(
                &self
                    .document()
                    .get_text(segment.start_offset(), segment.length())?,
            );
        }
        Ok(text)
    }

    pub fn replace_selection_with_text(&self, text: &str) -> Result<(), EditError> {
        self.replace_selections(&[text.to_owned()])
    }
    pub fn replace_selections(&self, replacements: &[String]) -> Result<(), EditError> {
        let selection = self.selection();
        if replacements.len() != 1 && replacements.len() != selection.segments().len() {
            return Err(EditError::ReplacementCount);
        }
        let edits: Vec<_> = selection
            .segments()
            .iter()
            .enumerate()
            .map(|(i, s)| {
                (
                    s.segment(),
                    replacements[if replacements.len() == 1 { 0 } else { i }].clone(),
                )
            })
            .collect();
        self.validate_edits(&edits)?;
        let _update = self.document().begin_update()?;
        let mut carets = Vec::with_capacity(edits.len());
        for (segment, replacement) in edits.iter().rev() {
            self.document()
                .replace(segment.offset, segment.length, replacement)?;
            let anchor = self
                .document()
                .create_anchor(segment.offset + replacement.encode_utf16().count())?;
            anchor.set_movement_type(AnchorMovementType::AfterInsertion);
            anchor.set_survive_deletion(true);
            carets.push(anchor);
        }
        carets.reverse();
        let segments: Result<Vec<_>, _> = carets
            .iter()
            .map(|anchor| anchor.offset().map(SelectionSegment::empty))
            .collect();
        self.set_selection(Selection::from_segments(
            segments?,
            selection.primary_index(),
        ))
    }

    fn validate_edits(&self, edits: &[(SimpleSegment, String)]) -> Result<(), EditError> {
        self.validate_ranges(
            edits
                .iter()
                .map(|(segment, text)| (*segment, !text.is_empty())),
        )
    }
    fn validate_ranges(
        &self,
        edits: impl IntoIterator<Item = (SimpleSegment, bool)>,
    ) -> Result<(), EditError> {
        if self.is_read_only() {
            return Err(EditError::ReadOnly);
        }
        let provider = self.inner.read_only_sections.borrow().clone();
        let length = self.document().text_length();
        for (segment, inserts_text) in edits {
            if segment.offset > length || segment.length > length - segment.offset {
                return Err(EditError::InvalidSelection);
            }
            if inserts_text && !provider.can_insert(segment.offset) {
                return Err(EditError::ReadOnly);
            }
            let writable: usize = provider
                .get_deletable_segments(segment)
                .iter()
                .map(|s| s.length)
                .sum();
            if writable != segment.length {
                return Err(EditError::ReadOnly);
            }
        }
        Ok(())
    }
    fn apply_edits(&self, edits: &[(SimpleSegment, String)]) -> Result<(), EditError> {
        self.validate_edits(edits)?;
        let _update = self.document().begin_update()?;
        for (segment, text) in edits.iter().rev() {
            self.document()
                .replace(segment.offset, segment.length, text)?;
        }
        Ok(())
    }

    /// Applies a precomputed edit plan using original document coordinates.
    /// All protected ranges are checked before the document validates overlaps
    /// and commits the plan as one undo unit.
    pub fn apply_document_edits(
        &self,
        edits: &[crate::document::DocumentEdit],
    ) -> Result<(), EditError> {
        self.validate_document_edits(edits)?;
        self.document().apply_edits(edits)?;
        Ok(())
    }
    /// Checks the host's read-only policy without changing the document. Other
    /// editing services can use this before applying their own undo grouping.
    pub fn validate_document_edits(
        &self,
        edits: &[crate::document::DocumentEdit],
    ) -> Result<(), EditError> {
        self.validate_ranges(edits.iter().map(|edit| {
            (
                SimpleSegment::new(edit.offset, edit.length),
                !edit.text.is_empty(),
            )
        }))
    }
    pub fn move_caret(&self, movement: CaretMovement, extend_selection: bool) {
        let selection = self.selection();
        let segments = selection.segments().iter().map(|s| {
            let target =
                if !extend_selection && !s.is_empty() && movement == CaretMovement::CharacterLeft {
                    s.start_offset()
                } else if !extend_selection
                    && !s.is_empty()
                    && movement == CaretMovement::CharacterRight
                {
                    s.end_offset()
                } else {
                    self.move_position(s.caret, movement)
                };
            if extend_selection {
                SelectionSegment::new(s.anchor, target)
            } else {
                SelectionSegment::empty(target)
            }
        });
        *self.inner.selection.borrow_mut() =
            Selection::from_segments(segments, selection.primary_index());
    }
    fn move_position(&self, offset: usize, movement: CaretMovement) -> usize {
        let document = self.document();
        if movement == CaretMovement::DocumentStart {
            return 0;
        }
        if movement == CaretMovement::DocumentEnd {
            return document.text_length();
        }
        let Ok(line) = document.get_line_by_offset(offset) else {
            return offset.min(document.text_length());
        };
        match movement {
            CaretMovement::LineStart => return line.offset,
            CaretMovement::LineEnd => return line.offset + line.length,
            CaretMovement::LineUp | CaretMovement::LineDown => {
                let number = if movement == CaretMovement::LineUp {
                    line.line_number.saturating_sub(1)
                } else {
                    line.line_number + 1
                };
                let Ok(target) = document.get_line_by_number(number) else {
                    return if movement == CaretMovement::LineUp {
                        0
                    } else {
                        document.text_length()
                    };
                };
                let column = offset - line.offset;
                let position = target.offset + column.min(target.length);
                if column >= target.length {
                    return target.offset + target.length;
                }
                // Snap a UTF-16 column that would split a surrogate or grapheme.
                let Ok(text) = document.get_text(target.offset, target.length) else {
                    return position;
                };
                if column == 0 {
                    return target.offset;
                }
                return target.offset
                    + next_caret_position(
                        &text,
                        column.min(target.length) + 1,
                        CaretMovement::CharacterLeft,
                    );
            }
            CaretMovement::CharacterLeft | CaretMovement::WordLeft if offset == line.offset => {
                return document
                    .get_line_by_number(line.line_number.saturating_sub(1))
                    .map_or(0, |l| l.offset + l.length);
            }
            CaretMovement::CharacterRight | CaretMovement::WordRight
                if offset >= line.offset + line.length =>
            {
                return (line.offset + line.length + line.delimiter_length)
                    .min(document.text_length());
            }
            _ => {}
        }
        let Ok(text) = document.get_text(line.offset, line.length) else {
            return offset;
        };
        line.offset + next_caret_position(&text, offset - line.offset, movement)
    }
    pub fn delete(&self, backward: bool, by_word: bool) -> Result<(), EditError> {
        let original = self.selection();
        let movement = match (backward, by_word) {
            (true, false) => CaretMovement::CharacterLeft,
            (false, false) => CaretMovement::CharacterRight,
            (true, true) => CaretMovement::WordLeft,
            (false, true) => CaretMovement::WordRight,
        };
        let selection = Selection::from_segments(
            original.segments().iter().map(|s| {
                if s.is_empty() {
                    SelectionSegment::new(s.caret, self.move_position(s.caret, movement))
                } else {
                    *s
                }
            }),
            original.primary_index(),
        );
        self.set_selection(selection)?;
        let result = self.replace_selection_with_text("");
        if result.is_err() {
            self.set_selection(original)?;
        }
        result
    }
    pub fn insert_new_line(&self) -> Result<(), EditError> {
        let options = self.options();
        let replacements: Result<Vec<_>, DocumentError> = self
            .selection()
            .segments()
            .iter()
            .map(|s| {
                let line = self.document().get_line_by_offset(s.start_offset())?;
                let prefix = self.document().get_text(
                    line.offset,
                    (s.start_offset() - line.offset).min(line.length),
                )?;
                let indent: String = prefix
                    .chars()
                    .take_while(|c| *c == ' ' || *c == '\t')
                    .collect();
                Ok(format!("{}{}", options.newline, indent))
            })
            .collect();
        self.replace_selections(&replacements?)
    }
    pub fn insert_tab(&self) -> Result<(), EditError> {
        if !self.selection().is_empty() {
            return self.indent_selection(false);
        }
        let options = self.options();
        let replacements: Result<Vec<_>, DocumentError> = self
            .selection()
            .segments()
            .iter()
            .map(|s| {
                if !options.convert_tabs_to_spaces {
                    return Ok("\t".into());
                }
                let line = self.document().get_line_by_offset(s.caret)?;
                let prefix = self
                    .document()
                    .get_text(line.offset, s.caret - line.offset)?;
                let mut visual = 0;
                for c in prefix.chars() {
                    visual += if c == '\t' {
                        options.indentation_size - visual % options.indentation_size
                    } else {
                        1
                    };
                }
                Ok(" ".repeat(options.indentation_size - visual % options.indentation_size))
            })
            .collect();
        self.replace_selections(&replacements?)
    }
    pub fn indent_selection(&self, unindent: bool) -> Result<(), EditError> {
        let mut selected_lines = BTreeSet::new();
        for segment in self.selection().segments() {
            let first = self
                .document()
                .get_line_by_offset(segment.start_offset())?
                .line_number;
            let end = if segment.is_empty() {
                segment.end_offset()
            } else {
                segment.end_offset() - 1
            };
            let last = self.document().get_line_by_offset(end)?.line_number;
            selected_lines.extend(first..=last);
        }
        let options = self.options();
        let mut edits = Vec::new();
        for index in selected_lines {
            let line = self.document().get_line_by_number(index)?;
            if !unindent {
                edits.push((
                    SimpleSegment::new(line.offset, 0),
                    options.indentation_string(),
                ));
                continue;
            }
            let prefix = self
                .document()
                .get_text(line.offset, line.length.min(options.indentation_size))?;
            let length = if prefix.starts_with('\t') {
                1
            } else {
                prefix.chars().take_while(|c| *c == ' ').count()
            };
            if length != 0 {
                edits.push((SimpleSegment::new(line.offset, length), String::new()));
            }
        }
        self.apply_edits(&edits)
    }
    pub fn undo(&self) -> Result<bool, EditError> {
        if self.is_read_only() {
            return Err(EditError::ReadOnly);
        }
        Ok(self.document().undo()?)
    }
    pub fn redo(&self) -> Result<bool, EditError> {
        if self.is_read_only() {
            return Err(EditError::ReadOnly);
        }
        Ok(self.document().redo()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editing::TextSegmentReadOnlySectionProvider;
    #[test]
    fn multicaret_edit_is_single_undo_and_survives_external_insert() {
        let document = TextDocument::from_text("abc def ghi");
        let editor = EditorSession::new(document.clone());
        editor
            .set_selection(Selection::from_segments(
                [SelectionSegment::new(0, 3), SelectionSegment::new(8, 11)],
                0,
            ))
            .unwrap();
        editor.replace_selection_with_text("😀").unwrap();
        assert_eq!(document.text(), "😀 def 😀");
        assert_eq!(
            editor.selection().segments(),
            &[SelectionSegment::empty(2), SelectionSegment::empty(9)]
        );
        assert!(editor.undo().unwrap());
        assert_eq!(document.text(), "abc def ghi");
        document.insert(0, "x").unwrap();
        assert_eq!(editor.selection().segments()[0].caret, 4);
    }
    #[test]
    fn protected_multicaret_edit_is_preflighted_before_mutation() {
        let document = TextDocument::from_text("abc def");
        let editor = EditorSession::new(document.clone());
        let protected = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
        protected.add(0, 3).unwrap();
        editor.set_read_only_section_provider(protected);
        editor
            .set_selection(Selection::from_segments(
                [SelectionSegment::empty(1), SelectionSegment::empty(6)],
                0,
            ))
            .unwrap();
        assert!(matches!(
            editor.replace_selection_with_text("X"),
            Err(EditError::ReadOnly)
        ));
        assert_eq!(document.text(), "abc def");
    }
    #[test]
    fn indent_excludes_line_at_selection_end_and_newline_preserves_indentation() {
        let document = TextDocument::from_text("a\nb\n");
        let editor = EditorSession::new(document.clone());
        editor.select(0, 2).unwrap();
        editor.indent_selection(false).unwrap();
        assert_eq!(document.text(), "\ta\nb\n");
        editor.set_caret_offset(2).unwrap();
        editor.insert_new_line().unwrap();
        assert_eq!(document.text(), "\ta\n\t\nb\n");
    }
}
