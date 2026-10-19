use crate::documents::{
    CharacterFormat, DocumentError, DocumentSnapshot, ParagraphFormat, RichTextDocument,
    TextGravity,
};
use ferroui_base::media::TextDecorations;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{Ref, WeakRef};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::ops::Range;
use std::rc::Rc;

/// A directed selection. Offsets count UTF-16 code units and always lie on scalar boundaries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EditorSelection {
    pub anchor: usize,
    pub active: usize,
}

impl EditorSelection {
    pub const fn new(anchor: usize, active: usize) -> Self {
        Self { anchor, active }
    }
    pub const fn caret(offset: usize) -> Self {
        Self::new(offset, offset)
    }
    pub fn range(self) -> Range<usize> {
        self.anchor.min(self.active)..self.anchor.max(self.active)
    }
    pub fn is_empty(self) -> bool {
        self.anchor == self.active
    }
}

/// The semantic boundary used when grouping adjacent keyboard edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditKind {
    Typing,
    Delete,
    Paste,
    Formatting,
    Composition,
    Programmatic,
    Group,
}

/// A failed editing operation leaves the document and its undo history unchanged.
#[derive(Debug)]
pub enum EditError {
    ReadOnly,
    ReentrantEdit,
    Document(DocumentError),
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadOnly => f.write_str("The rich text document is read-only."),
            Self::ReentrantEdit => f.write_str("An edit is already being published."),
            Self::Document(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for EditError {}
impl From<DocumentError> for EditError {
    fn from(error: DocumentError) -> Self {
        Self::Document(error)
    }
}

#[derive(Clone)]
struct EditorSnapshot {
    document: DocumentSnapshot,
    selection: EditorSelection,
    typing_format: CharacterFormat,
    default_character_format: CharacterFormat,
    default_paragraph_format: ParagraphFormat,
}

#[derive(Clone)]
struct UndoUnit {
    before: EditorSnapshot,
    after: EditorSnapshot,
    kind: EditKind,
    merge_generation: u64,
}

struct Composition {
    before: EditorSnapshot,
    preview_revision: u64,
}

#[derive(Default)]
struct History {
    undo: Vec<UndoUnit>,
    redo: Vec<UndoUnit>,
    composition: Option<Composition>,
}

/// Portable rich-text transactions shared by input, commands, clipboard, and automation.
///
/// The document remains the authoritative structured object model. Undo snapshots retain
/// spans, hyperlinks, paragraph properties, and inline controls; no plain-text reconstruction
/// is involved. An external mutation invalidates history before the next edit, so undo cannot
/// silently overwrite an application's subsequent document changes.
pub struct RichTextEditor {
    document: RichTextDocument,
    text_view: RefCell<Option<WeakRef<crate::RichTextBlock>>>,
    selection: Cell<EditorSelection>,
    typing_format: RefCell<CharacterFormat>,
    default_character_format: RefCell<CharacterFormat>,
    default_paragraph_format: RefCell<ParagraphFormat>,
    selection_projection: RefCell<Option<Rc<dyn Fn(EditorSelection) -> EditorSelection>>>,
    projecting_selection: Cell<bool>,
    history: RefCell<History>,
    known_revision: Cell<u64>,
    merge_generation: Cell<u64>,
    is_read_only: Cell<bool>,
    max_length: Cell<usize>,
    undo_limit: Cell<usize>,
    in_operation: Cell<bool>,
    group_depth: Cell<usize>,
    changed: Rc<HandlerList<dyn Fn()>>,
}

struct Operation<'a>(&'a Cell<bool>);
impl Drop for Operation<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

struct SelectionProjection<'a> {
    flag: &'a Cell<bool>,
    previous: bool,
}
impl Drop for SelectionProjection<'_> {
    fn drop(&mut self) {
        self.flag.set(self.previous);
    }
}

impl RichTextEditor {
    pub fn new(document: RichTextDocument) -> Self {
        Self {
            known_revision: Cell::new(document.revision()),
            document,
            text_view: RefCell::new(None),
            selection: Cell::new(EditorSelection::default()),
            typing_format: RefCell::new(CharacterFormat::default()),
            default_character_format: RefCell::new(CharacterFormat::default()),
            default_paragraph_format: RefCell::new(ParagraphFormat::default()),
            selection_projection: RefCell::new(None),
            projecting_selection: Cell::new(false),
            history: RefCell::new(History::default()),
            merge_generation: Cell::new(0),
            is_read_only: Cell::new(false),
            max_length: Cell::new(0),
            undo_limit: Cell::new(100),
            in_operation: Cell::new(false),
            group_depth: Cell::new(0),
            changed: Rc::new(HandlerList::new()),
        }
    }

    pub fn document(&self) -> RichTextDocument {
        self.document.clone()
    }

    /// Associates the live text layout used by display-dependent TOM units. The
    /// weak association does not keep a control alive or participate in history.
    pub fn set_text_view(&self, view: Option<Ref<crate::RichTextBlock>>) -> Result<(), EditError> {
        if view
            .as_ref()
            .is_some_and(|view| view.document() != self.document)
        {
            return Err(DocumentError::InvalidFormat(
                "The text view must display this editor's document.",
            )
            .into());
        }
        let previous = self.text_view.replace(view.as_ref().map(Ref::downgrade));
        drop(previous);
        Ok(())
    }

    pub(super) fn text_view(&self) -> Option<Ref<crate::RichTextBlock>> {
        self.text_view.borrow().as_ref().and_then(WeakRef::upgrade)
    }
    pub fn selection(&self) -> EditorSelection {
        self.selection.get()
    }
    pub fn is_read_only(&self) -> bool {
        self.is_read_only.get()
    }
    pub fn set_is_read_only(&self, value: bool) {
        self.is_read_only.set(value);
    }
    /// Zero means unlimited. The limit is measured in UTF-16 code units.
    pub fn max_length(&self) -> usize {
        self.max_length.get()
    }
    pub fn set_max_length(&self, value: usize) {
        self.max_length.set(value);
    }
    pub fn undo_limit(&self) -> usize {
        self.undo_limit.get()
    }
    pub fn set_undo_limit(&self, value: usize) {
        self.undo_limit.set(value);
        let mut history = self.history.borrow_mut();
        let remove = history.undo.len().saturating_sub(value);
        history.undo.drain(..remove);
        if value == 0 {
            history.redo.clear();
        }
    }
    pub fn can_undo(&self) -> bool {
        self.document.revision() == self.known_revision.get()
            && !self.history.borrow().undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        self.document.revision() == self.known_revision.get()
            && !self.history.borrow().redo.is_empty()
    }
    pub fn is_composing(&self) -> bool {
        self.history.borrow().composition.is_some()
    }
    /// Absolute UTF-16 extent of the current IME preview. A later external model
    /// mutation invalidates this extent instead of exposing stale composition text.
    pub fn active_composition_range(&self) -> Option<Range<usize>> {
        let (revision, range, retained) = {
            let history = self.history.borrow();
            let composition = history.composition.as_ref()?;
            let range = composition.before.selection.range();
            let retained = composition
                .before
                .document
                .text_length()
                .checked_sub(range.len())?;
            (composition.preview_revision, range, retained)
        };
        if self.document.revision() != revision {
            return None;
        }
        let length = self.document.text_length().checked_sub(retained)?;
        let end = range.start.checked_add(length)?;
        let result = range.start..end;
        if self.document.revision() != revision
            || self.document.validate_range(result.clone()).is_err()
        {
            return None;
        }
        Some(result)
    }
    pub fn typing_format(&self) -> CharacterFormat {
        self.typing_format.borrow().clone()
    }

    /// Assigned document defaults. Unassigned fields continue to inherit the host's
    /// FerroUI text properties; callers receive an independent value snapshot.
    pub fn default_character_format(&self) -> CharacterFormat {
        self.default_character_format.borrow().clone()
    }

    pub fn default_paragraph_format(&self) -> ParagraphFormat {
        self.default_paragraph_format.borrow().clone()
    }

    /// Updates the defaults for empty content and document replacement without
    /// restyling existing text. The change participates in grouped undo/rollback.
    pub fn set_default_character_format(&self, format: &CharacterFormat) -> Result<(), EditError> {
        format.validate()?;
        self.prepare_edit()?;
        let _operation = self.enter(true)?;
        self.synchronize();
        let before = self.capture();
        let merged = self.default_character_format().overlay(format).clone();
        let previous = self.default_character_format.replace(merged);
        if self.document.text_length() == 0 {
            if let Err(error) = self
                .document
                .apply_paragraph_format(0..0, &self.insertion_defaults())
            {
                let failed = self.default_character_format.replace(previous);
                drop(failed);
                return Err(error.into());
            }
        }
        drop(previous);
        self.checkpoint();
        self.record(before, EditKind::Formatting);
        Ok(())
    }

    pub fn set_default_paragraph_format(&self, format: &ParagraphFormat) -> Result<(), EditError> {
        format.validate()?;
        self.prepare_edit()?;
        let _operation = self.enter(true)?;
        self.synchronize();
        let before = self.capture();
        let merged = self.default_paragraph_format().overlay(format).clone();
        let previous = self.default_paragraph_format.replace(merged);
        if self.document.text_length() == 0 {
            if let Err(error) = self
                .document
                .apply_paragraph_format(0..0, &self.insertion_defaults())
            {
                let failed = self.default_paragraph_format.replace(previous);
                drop(failed);
                return Err(error.into());
            }
        }
        drop(previous);
        self.checkpoint();
        self.record(before, EditKind::Formatting);
        Ok(())
    }

    pub(super) fn insertion_defaults(&self) -> ParagraphFormat {
        let mut format = self.default_paragraph_format();
        format.character = format.character.overlay(&self.default_character_format());
        format
    }

    pub(crate) fn is_in_operation(&self) -> bool {
        self.in_operation.get()
    }

    /// The control projects proposed selection through its cancellable routed event.
    /// This callback owns no control strongly and runs without an interior borrow.
    pub(crate) fn set_selection_projection(
        &self,
        projection: Option<Rc<dyn Fn(EditorSelection) -> EditorSelection>>,
    ) {
        let old = self.selection_projection.replace(projection);
        drop(old);
    }

    fn project_selection(&self) {
        if self.projecting_selection.get() || self.group_depth.get() > 0 {
            return;
        }
        self.invoke_selection_projection();
    }

    fn invoke_selection_projection(&self) {
        if self.group_depth.get() > 0 {
            return;
        }
        let projection = self.selection_projection.borrow().clone();
        let Some(projection) = projection else {
            return;
        };
        let _projection = SelectionProjection {
            flag: &self.projecting_selection,
            previous: self.projecting_selection.replace(true),
        };
        let candidate = self.selection();
        let accepted = projection(candidate);
        // The owner hook is internal, but never let a stale/reentrant result put a
        // surrogate interior or an out-of-document offset in the editor state.
        let accepted = if self.document.validate_range(accepted.range()).is_ok() {
            accepted
        } else {
            let length = self.document.text_length();
            EditorSelection::new(
                self.scalar_boundary(candidate.anchor.min(length)),
                self.scalar_boundary(candidate.active.min(length)),
            )
        };
        self.selection.set(accepted);
    }

    /// Effective formatting at the selection start or insertion point, including the
    /// current collapsed-selection format overrides.
    pub fn current_character_format(&self) -> Result<CharacterFormat, EditError> {
        let selection = self.selection();
        let offset = if selection.is_empty() {
            selection.active
        } else {
            selection.range().start
        };
        let gravity = if selection.is_empty() {
            TextGravity::Backward
        } else {
            TextGravity::Forward
        };
        let format = self
            .document
            .element_at(offset, gravity)
            .map(|element| CharacterFormat::effective(&element))
            .unwrap_or_else(|| self.insertion_defaults().character);
        Ok(format.overlay(&self.typing_format()))
    }

    /// Observes completed editing or selection changes. Handlers run after document
    /// notifications and outside all history borrows.
    pub fn changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.changed.add(Rc::new(handler));
        let weak = Rc::downgrade(&self.changed);
        Disposable::create(move || {
            if let Some(events) = weak.upgrade() {
                events.remove(token);
            }
        })
    }

    fn publish_changed(&self) {
        if self.group_depth.get() > 0 || self.projecting_selection.get() {
            return;
        }
        self.project_selection();
        for (_, handler) in self.changed.snapshot().iter() {
            handler();
        }
    }

    /// Committed text exposed to the platform while a separate preview is visible.
    pub fn surrounding_text(&self) -> String {
        let committed = self
            .history
            .borrow()
            .composition
            .as_ref()
            .map(|composition| composition.before.document.clone());
        committed.map_or_else(|| self.document.text(), |snapshot| snapshot.text())
    }

    /// Selection in the committed surrounding text, excluding the uncommitted preview.
    pub fn input_method_selection(&self) -> EditorSelection {
        self.history
            .borrow()
            .composition
            .as_ref()
            .map_or(self.selection(), |composition| composition.before.selection)
    }

    pub fn set_selection(&self, selection: EditorSelection) -> Result<(), EditError> {
        self.document.get_text(selection.range())?;
        if self.selection() == selection {
            return Ok(());
        }
        if self.in_operation.get() && !self.projecting_selection.get() {
            return Err(EditError::ReentrantEdit);
        }
        let previous = self.selection.replace(selection);
        if previous != selection {
            let previous_format = self.typing_format.replace(CharacterFormat::default());
            // A nested owner request must immediately update the view's proposal.
            // The view suppresses recursive SelectionChanging, while this guard
            // suppresses nested editor publication until the outer request settles.
            self.invoke_selection_projection();
            if self.selection() == previous {
                *self.typing_format.borrow_mut() = previous_format;
                return Ok(());
            }
            self.checkpoint();
            self.publish_changed();
        }
        Ok(())
    }

    pub fn selected_text(&self) -> Result<String, EditError> {
        Ok(self.document.get_text(self.selection().range())?)
    }
    pub fn selected_fragment(&self) -> Result<DocumentSnapshot, EditError> {
        Ok(self.document.slice(self.selection().range())?)
    }

    /// Separates the next typed characters from the preceding undo unit.
    pub fn checkpoint(&self) {
        self.merge_generation
            .set(self.merge_generation.get().wrapping_add(1));
    }
    pub fn clear_history(&self) {
        let old = std::mem::take(&mut *self.history.borrow_mut());
        drop(old);
        self.known_revision.set(self.document.revision());
        self.checkpoint();
    }

    fn enter(&self, require_writable: bool) -> Result<Operation<'_>, EditError> {
        if require_writable && self.is_read_only() {
            return Err(EditError::ReadOnly);
        }
        if self.in_operation.replace(true) {
            return Err(EditError::ReentrantEdit);
        }
        Ok(Operation(&self.in_operation))
    }

    fn synchronize(&self) {
        if self.document.revision() != self.known_revision.get() {
            self.clear_history();
            let length = self.document.text_length();
            let selection = self.selection();
            self.selection.set(EditorSelection::new(
                self.scalar_boundary(selection.anchor.min(length)),
                self.scalar_boundary(selection.active.min(length)),
            ));
        }
    }

    fn scalar_boundary(&self, offset: usize) -> usize {
        if self.document.get_text(offset..offset).is_ok() {
            offset
        } else {
            offset.saturating_sub(1)
        }
    }

    fn capture(&self) -> EditorSnapshot {
        EditorSnapshot {
            document: self.document.snapshot(),
            selection: self.selection(),
            typing_format: self.typing_format(),
            default_character_format: self.default_character_format(),
            default_paragraph_format: self.default_paragraph_format(),
        }
    }

    fn restore(&self, snapshot: &EditorSnapshot) -> Result<(), EditError> {
        self.document.restore(&snapshot.document)?;
        self.selection.set(snapshot.selection);
        *self.typing_format.borrow_mut() = snapshot.typing_format.clone();
        *self.default_character_format.borrow_mut() = snapshot.default_character_format.clone();
        *self.default_paragraph_format.borrow_mut() = snapshot.default_paragraph_format.clone();
        self.known_revision.set(self.document.revision());
        Ok(())
    }

    fn record(&self, before: EditorSnapshot, kind: EditKind) {
        self.known_revision.set(self.document.revision());
        if self.group_depth.get() > 0 {
            return;
        }
        self.project_selection();
        let after = self.capture();
        if before.document == after.document
            && before.typing_format == after.typing_format
            && before.default_character_format == after.default_character_format
            && before.default_paragraph_format == after.default_paragraph_format
        {
            self.publish_changed();
            return;
        }
        let mut history = self.history.borrow_mut();
        history.redo.clear();
        if self.undo_limit.get() == 0 {
            drop(history);
            self.publish_changed();
            return;
        }
        let generation = self.merge_generation.get();
        if kind == EditKind::Typing && before.selection.is_empty() {
            if let Some(last) = history.undo.last_mut().filter(|unit| {
                unit.kind == kind
                    && unit.merge_generation == generation
                    && unit.after.selection == before.selection
                    && unit.after.typing_format == before.typing_format
            }) {
                last.after = after;
                drop(history);
                self.publish_changed();
                return;
            }
        }
        history.undo.push(UndoUnit {
            before,
            after,
            kind,
            merge_generation: generation,
        });
        let remove = history.undo.len().saturating_sub(self.undo_limit.get());
        history.undo.drain(..remove);
        drop(history);
        self.publish_changed();
    }

    fn permitted_text(&self, text: &str, replacing: Range<usize>) -> String {
        let normalized = normalize_text(text);
        if self.max_length() == 0 {
            return normalized;
        }
        let retained = self
            .document
            .text_length()
            .saturating_sub(replacing.end - replacing.start);
        let mut remaining = self.max_length().saturating_sub(retained);
        normalized
            .chars()
            .take_while(|ch| {
                let length = ch.len_utf16();
                if length > remaining {
                    false
                } else {
                    remaining -= length;
                    true
                }
            })
            .collect()
    }

    /// Replaces the selection and applies the current insertion format in one undo unit.
    pub fn insert_text(&self, text: &str, kind: EditKind) -> Result<bool, EditError> {
        let composing = self.prepare_edit()?;
        let kind = if composing && kind == EditKind::Typing {
            EditKind::Composition
        } else {
            kind
        };
        let _operation = self.enter(true)?;
        self.synchronize();
        let range = self.selection().range();
        self.replace_text_core(text, kind, range)
    }

    fn replace_text_core(
        &self,
        text: &str,
        kind: EditKind,
        range: Range<usize>,
    ) -> Result<bool, EditError> {
        let text = self.permitted_text(text, range.clone());
        // Rejected input at a length limit must not delete a selected range.
        if text.is_empty() && (!text_is_deletion(kind) || range.is_empty()) {
            return Ok(false);
        }
        let before = self.capture();
        let length = text.encode_utf16().count();
        {
            let _batch = self.document.begin_edit();
            self.document.replace_text(range.clone(), &text)?;
            if before.document.text_length() == 0 || self.document.text_length() == 0 {
                self.document.apply_paragraph_format(
                    0..self.document.text_length(),
                    &self.insertion_defaults(),
                )?;
            }
            if length > 0 {
                self.document
                    .apply_format(range.start..range.start + length, &self.typing_format())?;
            }
            self.selection
                .set(EditorSelection::caret(range.start + length));
        }
        if text.contains('\n') {
            self.checkpoint();
        }
        self.record(before, kind);
        Ok(true)
    }

    pub(super) fn prepare_edit(&self) -> Result<bool, EditError> {
        if self.is_read_only() {
            return Err(EditError::ReadOnly);
        }
        if self.in_operation.get() || self.projecting_selection.get() {
            return Err(EditError::ReentrantEdit);
        }
        let composing = self.is_composing();
        if composing {
            self.cancel_composition()?;
            self.checkpoint();
        }
        Ok(composing)
    }

    /// Replaces the selection with a structured fragment, retaining all supported inline kinds.
    pub fn insert_fragment(
        &self,
        fragment: &DocumentSnapshot,
        kind: EditKind,
    ) -> Result<bool, EditError> {
        fragment.validate()?;
        self.prepare_edit()?;
        let _operation = self.enter(true)?;
        self.synchronize();
        let range = self.selection().range();
        let fragment = if self.max_length() > 0 {
            let retained = self
                .document
                .text_length()
                .saturating_sub(range.end - range.start);
            let limit = self.max_length().saturating_sub(retained);
            if limit < fragment.text_length() {
                let temp = RichTextDocument::new();
                temp.restore(fragment)?;
                let mut end = limit;
                if temp.get_text(end..end).is_err() {
                    end = end.saturating_sub(1);
                }
                temp.slice(0..end)?
            } else {
                fragment.clone()
            }
        } else {
            fragment.clone()
        };
        if fragment.text_length() == 0 {
            return Ok(false);
        }
        let before = self.capture();
        {
            let _batch = self.document.begin_edit();
            self.document.replace_range(range.clone(), &fragment)?;
            self.selection
                .set(EditorSelection::caret(range.start + fragment.text_length()));
        }
        self.checkpoint();
        self.record(before, kind);
        Ok(true)
    }

    pub fn delete_selection(&self) -> Result<bool, EditError> {
        self.prepare_edit()?;
        if self.selection().is_empty() {
            return Ok(false);
        }
        self.insert_text("", EditKind::Delete)
    }

    pub fn delete_backward(&self, whole_word: bool) -> Result<bool, EditError> {
        self.prepare_edit()?;
        let _operation = self.enter(true)?;
        self.synchronize();
        let range = if self.selection().is_empty() {
            let position = self.selection().active;
            let next = if whole_word {
                self.document.previous_word(position)
            } else {
                self.document.previous_grapheme(position)
            };
            next..position
        } else {
            self.selection().range()
        };
        self.replace_text_core("", EditKind::Delete, range)
    }

    pub fn delete_forward(&self, whole_word: bool) -> Result<bool, EditError> {
        self.prepare_edit()?;
        let _operation = self.enter(true)?;
        self.synchronize();
        let range = if self.selection().is_empty() {
            let position = self.selection().active;
            let next = if whole_word {
                self.document.next_word(position)
            } else {
                self.document.next_grapheme(position)
            };
            position..next
        } else {
            self.selection().range()
        };
        self.replace_text_core("", EditKind::Delete, range)
    }

    pub fn apply_format(&self, format: &CharacterFormat) -> Result<(), EditError> {
        format.validate()?;
        self.prepare_edit()?;
        let _operation = self.enter(true)?;
        self.synchronize();
        let before = self.capture();
        if self.selection().is_empty() {
            // Validate the format through the same model path as a nonempty selection.
            self.document
                .apply_format(self.selection().range(), format)?;
            let merged = merge_format(self.typing_format(), format).clone();
            *self.typing_format.borrow_mut() = merged;
        } else {
            self.document
                .apply_format(self.selection().range(), format)?;
        }
        self.checkpoint();
        self.record(before, EditKind::Formatting);
        Ok(())
    }

    pub fn apply_paragraph_format(&self, format: &ParagraphFormat) -> Result<(), EditError> {
        format.validate()?;
        self.prepare_edit()?;
        let _operation = self.enter(true)?;
        self.synchronize();
        let before = self.capture();
        self.document
            .apply_paragraph_format(self.selection().range(), format)?;
        self.checkpoint();
        self.record(before, EditKind::Formatting);
        Ok(())
    }

    /// Executes multiple edits as one undo unit. An error rolls the group back atomically.
    pub fn edit_group<T>(
        &self,
        action: impl FnOnce(&Self) -> Result<T, EditError>,
    ) -> Result<T, EditError> {
        self.prepare_edit()?;
        if self.is_read_only() {
            return Err(EditError::ReadOnly);
        }
        if self.in_operation.get() {
            return Err(EditError::ReentrantEdit);
        }
        self.synchronize();
        let before = self.capture();
        let before_limit = self.max_length();
        let depth = self.group_depth.get();
        self.group_depth.set(depth + 1);
        let batch = self.document.begin_edit();
        let result = action(self);
        let _operation = self.enter(false)?;
        if result.is_err() {
            // TOM SetText may grow MaxLength inside a nested group. Failed
            // transactions restore that configuration along with the story.
            self.set_max_length(before_limit);
            self.restore(&before)?;
        }
        drop(batch);
        self.group_depth.set(depth);
        self.known_revision.set(self.document.revision());
        if result.is_ok() {
            self.checkpoint();
            self.record(before, EditKind::Group);
        } else {
            self.publish_changed();
        }
        result
    }

    pub fn undo(&self) -> Result<bool, EditError> {
        self.prepare_edit()?;
        let _operation = self.enter(true)?;
        self.synchronize();
        let unit = self.history.borrow_mut().undo.pop();
        let Some(unit) = unit else {
            return Ok(false);
        };
        if let Err(error) = self.restore(&unit.before) {
            self.history.borrow_mut().undo.push(unit);
            return Err(error);
        }
        self.history.borrow_mut().redo.push(unit);
        self.checkpoint();
        self.publish_changed();
        Ok(true)
    }

    pub fn redo(&self) -> Result<bool, EditError> {
        self.prepare_edit()?;
        let _operation = self.enter(true)?;
        self.synchronize();
        let unit = self.history.borrow_mut().redo.pop();
        let Some(unit) = unit else {
            return Ok(false);
        };
        if let Err(error) = self.restore(&unit.after) {
            self.history.borrow_mut().redo.push(unit);
            return Err(error);
        }
        self.history.borrow_mut().undo.push(unit);
        self.checkpoint();
        self.publish_changed();
        Ok(true)
    }

    /// Updates visible IME text without adding an undo unit. The first update captures the
    /// entire pre-composition selection and structure; later updates replace that preview.
    pub fn set_preedit(&self, text: &str, cursor: Option<usize>) -> Result<(), EditError> {
        let _operation = self.enter(true)?;
        let cursor = cursor.map(|offset| {
            normalize_text(utf16_prefix(text, offset))
                .encode_utf16()
                .count()
        });
        self.synchronize();
        let composition = self.history.borrow_mut().composition.take();
        let before = composition.map_or_else(|| self.capture(), |composition| composition.before);
        // Keep the committed context available before publishing the model change:
        // platform callbacks can synchronously query surrounding text while it is raised.
        self.history.borrow_mut().composition = Some(Composition {
            before: before.clone(),
            preview_revision: self.document.revision(),
        });
        let batch = self.document.begin_edit();
        self.restore(&before)?;
        let range = before.selection.range();
        let text = self.permitted_text(text, range.clone());
        let length = text.encode_utf16().count();
        {
            self.document.replace_text(range.clone(), &text)?;
            if length > 0 {
                let preedit_format = before.typing_format.overlay(&CharacterFormat {
                    text_decorations: Some(Some(TextDecorations::underline())),
                    ..CharacterFormat::default()
                });
                self.document
                    .apply_format(range.start..range.start + length, &preedit_format)?;
            }
            let offset = range.start + cursor.unwrap_or(length).min(length);
            self.selection
                .set(EditorSelection::caret(self.scalar_boundary(offset)));
        }
        drop(batch);
        let revision = self.document.revision();
        self.known_revision.set(revision);
        self.history.borrow_mut().composition = Some(Composition {
            before,
            preview_revision: revision,
        });
        self.publish_changed();
        Ok(())
    }

    /// Cancels only our preview. An intervening external edit is preserved and invalidates history.
    pub fn cancel_composition(&self) -> Result<bool, EditError> {
        let _operation = self.enter(false)?;
        let composition = self.history.borrow_mut().composition.take();
        let Some(composition) = composition else {
            return Ok(false);
        };
        if self.document.revision() == composition.preview_revision {
            self.restore(&composition.before)?;
        } else {
            self.synchronize();
        }
        self.publish_changed();
        Ok(true)
    }

    pub fn commit_composition(&self, text: &str) -> Result<bool, EditError> {
        self.cancel_composition()?;
        if text.is_empty() {
            return Ok(false);
        }
        self.checkpoint();
        self.insert_text(text, EditKind::Composition)
    }
}

/// Gets the prefix at or before a UTF-16 position without splitting a scalar.
pub(crate) fn utf16_prefix(text: &str, mut length: usize) -> &str {
    for (index, ch) in text.char_indices() {
        if ch.len_utf16() > length {
            return &text[..index];
        }
        length -= ch.len_utf16();
    }
    text
}

fn normalize_text(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

fn text_is_deletion(kind: EditKind) -> bool {
    matches!(kind, EditKind::Delete | EditKind::Programmatic)
}

fn merge_format(mut current: CharacterFormat, update: &CharacterFormat) -> CharacterFormat {
    macro_rules! update { ($($field:ident),* $(,)?) => { $(if update.$field.is_some() { current.$field = update.$field.clone(); })* }; }
    update!(
        language,
        font_family,
        font_size,
        font_style,
        font_weight,
        font_stretch,
        font_features,
        foreground,
        background,
        text_decorations,
        baseline_alignment,
        letter_spacing
    );
    current
}
