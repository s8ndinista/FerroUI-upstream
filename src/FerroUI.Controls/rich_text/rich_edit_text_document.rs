use super::{EditError, EditKind, EditorSelection, RichTextEditor};
use crate::RichTextBlock;
use crate::documents::{
    CharacterFormat, DocumentError, DocumentSnapshot, InlineSnapshot, InlineSnapshotKind,
    ParagraphFormat, RichTextDocument, TextGravity, TextPointer,
};
use ferroui_base::media::text_formatting::unicode::{
    GraphemeEnumerator, SentenceBreakEnumerator, WordBreakEnumerator,
};
use ferroui_base::{Point, Rect};
use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

/// Units whose boundaries are defined by the document and Unicode text services.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextRangeUnit {
    Character,
    Word,
    Sentence,
    Paragraph,
    Document,
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct TextFindOptions: u32 {
        const MATCH_CASE = 1;
        const WHOLE_WORD = 2;
        const BACKWARDS = 4;
    }
}

/// A formatting query distinguishes uniform values from mixed selections.
#[derive(Clone, Debug, PartialEq)]
pub enum TextFormatValue<T> {
    Uniform(T),
    Mixed,
}

/// Editing services over a shared FerroUI document. Range objects and selection commands
/// use the same transaction history as the visual editor, and can also be used headlessly.
#[derive(Clone)]
pub struct RichEditTextDocument {
    editor: Rc<RichTextEditor>,
}

impl RichEditTextDocument {
    pub fn new(document: RichTextDocument) -> Self {
        Self::from_editor(Rc::new(RichTextEditor::new(document)))
    }
    pub fn from_editor(editor: Rc<RichTextEditor>) -> Self {
        Self { editor }
    }
    pub fn document(&self) -> RichTextDocument {
        self.editor.document()
    }
    pub fn editor(&self) -> Rc<RichTextEditor> {
        self.editor.clone()
    }
    pub fn text(&self) -> String {
        self.document().text()
    }
    pub fn text_length(&self) -> usize {
        self.document().text_length()
    }
    pub fn selection(&self) -> RichEditTextSelection {
        RichEditTextSelection {
            editor: self.editor.clone(),
        }
    }
    pub fn get_range(&self, start: usize, end: usize) -> Result<RichEditTextRange, EditError> {
        RichEditTextRange::new(self.editor.clone(), start.min(end)..start.max(end))
    }
    pub fn document_range(&self) -> Result<RichEditTextRange, EditError> {
        self.get_range(0, self.text_length())
    }
    /// Hit tests the host's measured, shaped layout in local control coordinates.
    pub fn get_range_from_point(
        &self,
        view: &RichTextBlock,
        point: Point,
    ) -> Result<RichEditTextRange, EditError> {
        ensure_view_document(&self.document(), view)?;
        let position = view.hit_test_text_position(point);
        self.get_range(position, position)
    }
    pub fn set_text(&self, text: &str) -> Result<(), EditError> {
        self.document_range()?.set_text(text)
    }
    pub fn can_undo(&self) -> bool {
        self.editor.can_undo()
    }
    pub fn can_redo(&self) -> bool {
        self.editor.can_redo()
    }
    pub fn undo(&self) -> Result<bool, EditError> {
        self.editor.undo()
    }
    pub fn redo(&self) -> Result<bool, EditError> {
        self.editor.redo()
    }
    pub fn edit_group<T>(
        &self,
        edit: impl FnOnce(&Self) -> Result<T, EditError>,
    ) -> Result<T, EditError> {
        self.editor.edit_group(|_| edit(self))
    }
}

struct RangeData {
    start: TextPointer,
    end: TextPointer,
}

/// A live, independent range. Its endpoints follow document edits using explicit gravity.
/// Cloning shares the range object; [`get_clone`](Self::get_clone) creates independent endpoints.
#[derive(Clone)]
pub struct RichEditTextRange {
    editor: Rc<RichTextEditor>,
    data: Rc<RefCell<RangeData>>,
}

impl RichEditTextRange {
    fn new(editor: Rc<RichTextEditor>, range: Range<usize>) -> Result<Self, EditError> {
        let document = editor.document();
        document.validate_range(range.clone())?;
        let start_gravity = if range.is_empty() {
            TextGravity::Forward
        } else {
            TextGravity::Backward
        };
        Ok(Self {
            data: Rc::new(RefCell::new(RangeData {
                start: document.create_pointer(range.start, start_gravity)?,
                end: document.create_pointer(range.end, TextGravity::Forward)?,
            })),
            editor,
        })
    }
    pub fn document(&self) -> RichTextDocument {
        self.editor.document()
    }
    pub fn start_position(&self) -> usize {
        self.range().start
    }
    pub fn end_position(&self) -> usize {
        self.range().end
    }
    pub fn range(&self) -> Range<usize> {
        let data = self.data.borrow();
        data.start.offset().min(data.end.offset())..data.start.offset().max(data.end.offset())
    }
    pub fn length(&self) -> usize {
        self.range().len()
    }
    pub fn is_degenerate(&self) -> bool {
        self.range().is_empty()
    }
    pub fn text(&self) -> Result<String, EditError> {
        Ok(self.document().get_text(self.range())?)
    }
    pub fn snapshot(&self) -> Result<DocumentSnapshot, EditError> {
        Ok(self.document().slice(self.range())?)
    }
    /// Returns one rectangle for each visual fragment, including bidirectional and wrapped text.
    pub fn get_rectangles(&self, view: &RichTextBlock) -> Result<Vec<Rect>, EditError> {
        ensure_view_document(&self.document(), view)?;
        Ok(view.get_rects_for_range(self.range()))
    }
    pub fn get_clone(&self) -> Result<Self, EditError> {
        Self::new(self.editor.clone(), self.range())
    }
    pub fn set_range(&self, start: usize, end: usize) -> Result<(), EditError> {
        let range = start.min(end)..start.max(end);
        self.document().validate_range(range.clone())?;
        let replacement = RangeData {
            start: self.document().create_pointer(
                range.start,
                if range.is_empty() {
                    TextGravity::Forward
                } else {
                    TextGravity::Backward
                },
            )?,
            end: self
                .document()
                .create_pointer(range.end, TextGravity::Forward)?,
        };
        let old = self.data.replace(replacement);
        drop(old);
        Ok(())
    }
    pub fn collapse(&self, to_start: bool) -> Result<(), EditError> {
        let position = if to_start {
            self.start_position()
        } else {
            self.end_position()
        };
        self.set_range(position, position)
    }
    pub fn select(&self) -> Result<(), EditError> {
        let range = self.range();
        self.editor
            .set_selection(EditorSelection::new(range.start, range.end))
    }
    pub fn in_range(&self, other: &Self) -> bool {
        self.document() == other.document()
            && self.start_position() >= other.start_position()
            && self.end_position() <= other.end_position()
    }
    pub fn is_equal(&self, other: &Self) -> bool {
        self.document() == other.document() && self.range() == other.range()
    }

    /// Replaces the range while preserving and rebasing the editor's independent selection.
    /// The change, formatting, and selection are one undoable transaction.
    pub fn set_text(&self, text: &str) -> Result<(), EditError> {
        let range = self.range();
        let before_length = self.document().text_length();
        let selection = self.editor.selection();
        let typing_format = self.editor.typing_format();
        self.editor.edit_group(|editor| {
            editor.set_selection(EditorSelection::new(range.start, range.end))?;
            editor.insert_text(text, EditKind::Programmatic)?;
            let inserted = self
                .document()
                .text_length()
                .saturating_sub(before_length - range.len());
            let updated = rebase_selection(selection, range.clone(), inserted);
            editor.set_selection(updated)?;
            if updated.is_empty() {
                editor.apply_format(&typing_format)?;
            }
            Ok(())
        })?;
        let inserted = self
            .document()
            .text_length()
            .saturating_sub(before_length - range.len());
        self.set_range(range.start, range.start + inserted)
    }

    pub fn set_snapshot(&self, snapshot: &DocumentSnapshot) -> Result<(), EditError> {
        if snapshot.text_length() == 0 {
            return self.set_text("");
        }
        let range = self.range();
        let before_length = self.document().text_length();
        let selection = self.editor.selection();
        let typing_format = self.editor.typing_format();
        self.editor.edit_group(|editor| {
            editor.set_selection(EditorSelection::new(range.start, range.end))?;
            editor.insert_fragment(snapshot, EditKind::Paste)?;
            let inserted = self
                .document()
                .text_length()
                .saturating_sub(before_length - range.len());
            let updated = rebase_selection(selection, range.clone(), inserted);
            editor.set_selection(updated)?;
            if updated.is_empty() {
                editor.apply_format(&typing_format)?;
            }
            Ok(())
        })?;
        let inserted = self
            .document()
            .text_length()
            .saturating_sub(before_length - range.len());
        self.set_range(range.start, range.start + inserted)
    }

    fn apply_to_selection(
        &self,
        action: impl FnOnce(&RichTextEditor) -> Result<(), EditError>,
    ) -> Result<(), EditError> {
        let range = self.range();
        let selection = self.editor.selection();
        let typing_format = self.editor.typing_format();
        self.editor.edit_group(|editor| {
            editor.set_selection(EditorSelection::new(range.start, range.end))?;
            action(editor)?;
            editor.set_selection(selection)?;
            if selection.is_empty() {
                editor.apply_format(&typing_format)?;
            }
            Ok(())
        })
    }
    pub fn set_character_format(&self, format: &CharacterFormat) -> Result<(), EditError> {
        self.apply_to_selection(|editor| editor.apply_format(format))
    }
    pub fn set_paragraph_format(&self, format: &ParagraphFormat) -> Result<(), EditError> {
        self.apply_to_selection(|editor| editor.apply_paragraph_format(format))
    }

    /// Queries effective formats at run boundaries, so a long uniform run is inspected once.
    pub fn character_format(&self) -> Result<TextFormatValue<CharacterFormat>, EditError> {
        let range = self.range();
        let document = self.document();
        let format_at = |offset| {
            document
                .element_at(
                    offset,
                    if range.is_empty() {
                        TextGravity::Backward
                    } else {
                        TextGravity::Forward
                    },
                )
                .map(|element| CharacterFormat::effective(&element))
                .unwrap_or_default()
        };
        let first = format_at(range.start);
        if range.is_empty() {
            return Ok(TextFormatValue::Uniform(first));
        }
        let mut boundaries = Vec::new();
        let mut offset = 0;
        let snapshot = document.snapshot();
        for (index, paragraph) in snapshot.paragraphs.iter().enumerate() {
            if index > 0 {
                boundaries.push(offset);
                offset += 1;
            }
            inline_boundaries(&paragraph.inlines, &mut offset, &mut boundaries);
        }
        for offset in boundaries
            .into_iter()
            .filter(|offset| *offset > range.start && *offset < range.end)
        {
            if format_at(offset) != first {
                return Ok(TextFormatValue::Mixed);
            }
        }
        Ok(TextFormatValue::Uniform(first))
    }
    pub fn paragraph_format(&self) -> Result<TextFormatValue<ParagraphFormat>, EditError> {
        let snapshot = self.document().snapshot();
        let range = self.range();
        let mut offset = 0;
        let mut selected = None;
        for paragraph in snapshot.paragraphs {
            let end = offset + paragraph.inlines.iter().map(inline_length).sum::<usize>();
            if range.start <= end
                && (range.end > offset || range.is_empty() && range.start == offset)
            {
                if let Some(first) = &selected {
                    if first != &paragraph.format {
                        return Ok(TextFormatValue::Mixed);
                    }
                } else {
                    selected = Some(paragraph.format);
                }
            }
            offset = end + 1;
        }
        Ok(TextFormatValue::Uniform(selected.unwrap_or_default()))
    }

    /// Expands to the smallest whole units containing the range. Character units are
    /// extended grapheme clusters; word and sentence units use the framework's UAX #29 data.
    pub fn expand(&self, unit: TextRangeUnit) -> Result<(), EditError> {
        let units = unit_ranges(&self.document().text(), unit);
        let range = self.range();
        let start = units
            .iter()
            .find(|unit| unit.start <= range.start && (range.start < unit.end || unit.is_empty()))
            .map_or(range.start, |unit| unit.start);
        let active = if range.is_empty() {
            range.end
        } else {
            range.end.saturating_sub(1)
        };
        let end = units
            .iter()
            .find(|unit| unit.start <= active && (active < unit.end || unit.is_empty()))
            .map_or(range.end, |unit| unit.end);
        self.set_range(start, end)
    }

    /// Moves a collapsed range by whole units. A non-degenerate range collapses toward
    /// the movement direction before advancing. Returns the number of units traversed.
    pub fn move_by(&self, unit: TextRangeUnit, count: i32) -> Result<i32, EditError> {
        if count == 0 {
            return Ok(0);
        }
        let start = if count < 0 {
            self.start_position()
        } else {
            self.end_position()
        };
        let (position, moved) = move_position(&self.document().text(), start, unit, count);
        self.set_range(position, position)?;
        Ok(moved)
    }
    pub fn move_start(&self, unit: TextRangeUnit, count: i32) -> Result<i32, EditError> {
        let (position, moved) =
            move_position(&self.document().text(), self.start_position(), unit, count);
        self.set_range(position, self.end_position().max(position))?;
        Ok(moved)
    }
    pub fn move_end(&self, unit: TextRangeUnit, count: i32) -> Result<i32, EditError> {
        let (position, moved) =
            move_position(&self.document().text(), self.end_position(), unit, count);
        self.set_range(self.start_position().min(position), position)?;
        Ok(moved)
    }

    /// Searches this range. Case-insensitive matching uses Unicode lowercase mappings;
    /// UTF-16 offsets remain exact even when a character maps to several lowercase scalars.
    pub fn find(&self, text: &str, options: TextFindOptions) -> Result<Option<Self>, EditError> {
        if text.is_empty() {
            return Ok(None);
        }
        let source = self.text()?;
        let case_sensitive = options.contains(TextFindOptions::MATCH_CASE);
        let (haystack, boundaries) = searchable(&source, case_sensitive);
        let (needle, _) = searchable(text, case_sensitive);
        let words = if options.contains(TextFindOptions::WHOLE_WORD) {
            unit_ranges(&source, TextRangeUnit::Word)
        } else {
            Vec::new()
        };
        let matches: Box<dyn Iterator<Item = (usize, &str)> + '_> =
            if options.contains(TextFindOptions::BACKWARDS) {
                Box::new(haystack.rmatch_indices(needle.as_str()))
            } else {
                Box::new(haystack.match_indices(needle.as_str()))
            };
        for (byte_start, _) in matches {
            let byte_end = byte_start + needle.len();
            let start = boundaries
                .binary_search_by_key(&byte_start, |(byte, _)| *byte)
                .ok()
                .map(|index| boundaries[index].1);
            let end = boundaries
                .binary_search_by_key(&byte_end, |(byte, _)| *byte)
                .ok()
                .map(|index| boundaries[index].1);
            let (Some(start), Some(end)) = (start, end) else {
                continue;
            };
            if options.contains(TextFindOptions::WHOLE_WORD)
                && !words
                    .binary_search_by_key(&start, |word| word.start)
                    .ok()
                    .is_some_and(|index| words[index].end == end)
            {
                continue;
            }
            return Ok(Some(Self::new(
                self.editor.clone(),
                self.start_position() + start..self.start_position() + end,
            )?));
        }
        Ok(None)
    }
    pub fn find_text(&self, text: &str, options: TextFindOptions) -> Result<bool, EditError> {
        let Some(found) = self.find(text, options)? else {
            return Ok(false);
        };
        self.set_range(found.start_position(), found.end_position())?;
        Ok(true)
    }
}

/// Live directed editor selection. Extending preserves the anchor; moving without
/// extension collapses to the requested text-unit boundary.
#[derive(Clone)]
pub struct RichEditTextSelection {
    editor: Rc<RichTextEditor>,
}
impl RichEditTextSelection {
    pub fn selection(&self) -> EditorSelection {
        self.editor.selection()
    }
    pub fn set_range(&self, anchor: usize, active: usize) -> Result<(), EditError> {
        self.editor
            .set_selection(EditorSelection::new(anchor, active))
    }
    pub fn range(&self) -> Result<RichEditTextRange, EditError> {
        RichEditTextRange::new(self.editor.clone(), self.selection().range())
    }
    pub fn text(&self) -> Result<String, EditError> {
        self.editor.selected_text()
    }
    pub fn select_all(&self) -> Result<(), EditError> {
        self.set_range(0, self.editor.document().text_length())
    }
    pub fn type_text(&self, text: &str) -> Result<bool, EditError> {
        self.editor.insert_text(text, EditKind::Typing)
    }
    pub fn delete(&self) -> Result<bool, EditError> {
        self.editor.delete_selection()
    }
    pub fn character_format(&self) -> Result<CharacterFormat, EditError> {
        self.editor.current_character_format()
    }
    pub fn set_character_format(&self, format: &CharacterFormat) -> Result<(), EditError> {
        self.editor.apply_format(format)
    }
    pub fn set_paragraph_format(&self, format: &ParagraphFormat) -> Result<(), EditError> {
        self.editor.apply_paragraph_format(format)
    }
    pub fn move_by(&self, unit: TextRangeUnit, count: i32, extend: bool) -> Result<i32, EditError> {
        if count == 0 {
            return Ok(0);
        }
        let current = self.selection();
        let start = if extend || current.is_empty() {
            current.active
        } else if count < 0 {
            current.range().start
        } else {
            current.range().end
        };
        let (position, moved) = move_position(&self.editor.document().text(), start, unit, count);
        self.set_range(if extend { current.anchor } else { position }, position)?;
        Ok(moved)
    }
}

fn ensure_view_document(
    document: &RichTextDocument,
    view: &RichTextBlock,
) -> Result<(), EditError> {
    if *document == view.document() {
        Ok(())
    } else {
        Err(
            DocumentError::InvalidFormat("The text view must display this range's document.")
                .into(),
        )
    }
}

fn rebase_selection(
    selection: EditorSelection,
    range: Range<usize>,
    inserted: usize,
) -> EditorSelection {
    let map = |position: usize| {
        if position < range.start {
            position
        } else if position > range.end {
            position - range.len() + inserted
        } else {
            range.start + inserted
        }
    };
    EditorSelection::new(map(selection.anchor), map(selection.active))
}
fn unit_ranges(text: &str, unit: TextRangeUnit) -> Vec<Range<usize>> {
    let utf16: Vec<u16> = text.encode_utf16().collect();
    match unit {
        TextRangeUnit::Character => GraphemeEnumerator::new(&utf16)
            .map(|segment| segment.offset()..segment.offset() + segment.length())
            .collect(),
        TextRangeUnit::Word => WordBreakEnumerator::new(&utf16)
            .map(|segment| segment.offset()..segment.offset() + segment.length())
            .collect(),
        TextRangeUnit::Sentence => SentenceBreakEnumerator::new(&utf16)
            .map(|segment| segment.offset()..segment.offset() + segment.text().len())
            .collect(),
        TextRangeUnit::Paragraph => {
            let mut ranges = Vec::new();
            let mut start = 0;
            for (index, unit) in utf16.iter().enumerate() {
                if *unit == b'\n' as u16 {
                    ranges.push(start..index + 1);
                    start = index + 1;
                }
            }
            ranges.push(start..utf16.len());
            ranges
        }
        TextRangeUnit::Document => vec![0..utf16.len()],
    }
}
fn move_position(text: &str, position: usize, unit: TextRangeUnit, count: i32) -> (usize, i32) {
    let units = unit_ranges(text, unit);
    let mut boundaries: Vec<usize> = units
        .iter()
        .flat_map(|range| [range.start, range.end])
        .collect();
    boundaries.push(0);
    boundaries.push(text.encode_utf16().count());
    boundaries.sort_unstable();
    boundaries.dedup();
    let (mut position, mut moved) = (position, 0);
    for _ in 0..count.unsigned_abs().min(boundaries.len() as u32) {
        let next = if count < 0 {
            boundaries
                .iter()
                .rev()
                .find(|boundary| **boundary < position)
        } else {
            boundaries.iter().find(|boundary| **boundary > position)
        };
        let Some(next) = next else {
            break;
        };
        position = *next;
        moved += count.signum();
    }
    (position, moved)
}
fn searchable(text: &str, case_sensitive: bool) -> (String, Vec<(usize, usize)>) {
    let mut output = String::new();
    let mut boundaries = vec![(0, 0)];
    let mut offset = 0;
    for ch in text.chars() {
        if case_sensitive {
            output.push(ch);
        } else {
            output.extend(ch.to_lowercase());
        }
        offset += ch.len_utf16();
        boundaries.push((output.len(), offset));
    }
    (output, boundaries)
}
fn inline_length(inline: &InlineSnapshot) -> usize {
    match &inline.kind {
        InlineSnapshotKind::Run(text) => text.encode_utf16().count(),
        InlineSnapshotKind::LineBreak | InlineSnapshotKind::Embedded(_) => 1,
        InlineSnapshotKind::Span(inlines)
        | InlineSnapshotKind::Bold(inlines)
        | InlineSnapshotKind::Italic(inlines)
        | InlineSnapshotKind::Underline(inlines)
        | InlineSnapshotKind::Hyperlink { inlines, .. } => inlines.iter().map(inline_length).sum(),
    }
}
fn inline_boundaries(inlines: &[InlineSnapshot], offset: &mut usize, output: &mut Vec<usize>) {
    for inline in inlines {
        output.push(*offset);
        match &inline.kind {
            InlineSnapshotKind::Span(inlines)
            | InlineSnapshotKind::Bold(inlines)
            | InlineSnapshotKind::Italic(inlines)
            | InlineSnapshotKind::Underline(inlines)
            | InlineSnapshotKind::Hyperlink { inlines, .. } => {
                inline_boundaries(inlines, offset, output)
            }
            _ => *offset += inline_length(inline),
        }
    }
}
