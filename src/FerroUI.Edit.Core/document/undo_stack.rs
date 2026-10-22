use super::{DocumentChangeEventArgs, DocumentError, TextDocument, UpdateGuard};

#[derive(Clone, Debug)]
pub(crate) struct UndoGroup {
    pub changes: Vec<DocumentChangeEventArgs>,
    pub before: u64,
    pub after: u64,
}

pub(crate) struct UndoHistory {
    pub undo: Vec<UndoGroup>,
    pub redo: Vec<UndoGroup>,
    pub pending: Option<UndoGroup>,
    pub size_limit: usize,
    pub current_state: u64,
    pub original_state: Option<u64>,
    pub next_state: u64,
    pub replaying: bool,
    pub allow_continue: bool,
    pub continued: bool,
}

impl Default for UndoHistory {
    fn default() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            pending: None,
            size_limit: usize::MAX,
            current_state: 0,
            original_state: Some(0),
            next_state: 1,
            replaying: false,
            allow_continue: false,
            continued: false,
        }
    }
}

impl UndoHistory {
    pub fn begin(&mut self, continued: bool) {
        if self.replaying {
            return;
        }
        self.continued = continued && self.allow_continue && self.redo.is_empty();
        self.pending = Some(UndoGroup {
            changes: Vec::new(),
            before: self.current_state,
            after: self.current_state,
        });
    }

    pub fn record(&mut self, change: DocumentChangeEventArgs) {
        if self.replaying {
            return;
        }
        if let Some(group) = &mut self.pending {
            group.changes.push(change);
            group.after = self.next_state;
            self.current_state = self.next_state;
            self.next_state = self.next_state.wrapping_add(1);
            self.redo.clear();
        }
    }

    pub fn finish(&mut self) {
        if self.replaying {
            return;
        }
        if let Some(mut pending) = self.pending.take() {
            if pending.changes.is_empty() {
                if !self.continued {
                    self.allow_continue = false;
                }
                return;
            }
            if self.continued {
                if let Some(previous) = self.undo.pop() {
                    let mut changes = previous.changes;
                    changes.append(&mut pending.changes);
                    pending.changes = changes;
                    pending.before = previous.before;
                }
            }
            self.undo.push(pending);
            self.allow_continue = true;
            self.enforce_limit();
        }
    }

    pub fn enforce_limit(&mut self) {
        if self.undo.len() > self.size_limit {
            self.undo.drain(..self.undo.len() - self.size_limit);
        }
        if self.redo.len() > self.size_limit {
            self.redo.drain(..self.redo.len() - self.size_limit);
        }
    }
}

/// Undo history attached to a document. Changes are retained as persistent rope
/// fragments; grouping does not duplicate complete document strings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UndoStack(pub(crate) TextDocument);

impl UndoStack {
    pub fn can_undo(&self) -> bool {
        self.0.can_undo()
    }
    pub fn can_redo(&self) -> bool {
        self.0.can_redo()
    }
    pub fn undo(&self) -> Result<bool, DocumentError> {
        self.0.undo()
    }
    pub fn redo(&self) -> Result<bool, DocumentError> {
        self.0.redo()
    }
    pub fn clear_all(&self) -> Result<(), DocumentError> {
        self.0.clear_undo_history()
    }
    pub fn size_limit(&self) -> usize {
        self.0.undo_size_limit()
    }
    pub fn set_size_limit(&self, value: usize) {
        self.0.set_undo_size_limit(value);
    }
    pub fn is_original_file(&self) -> bool {
        !self.0.is_modified()
    }
    pub fn mark_as_original_file(&self) {
        self.0.mark_as_original_file();
    }
    pub fn discard_original_file_marker(&self) {
        self.0.discard_original_file_marker();
    }
    pub fn accept_changes(&self) -> bool {
        self.0.accept_changes()
    }
    pub fn undo_count(&self) -> usize {
        self.0.undo_count()
    }
    pub fn redo_count(&self) -> usize {
        self.0.redo_count()
    }
    pub fn start_undo_group(&self) -> Result<UpdateGuard, DocumentError> {
        self.0.begin_update()
    }
    pub fn start_continued_undo_group(&self) -> Result<UpdateGuard, DocumentError> {
        self.0.begin_continued_update()
    }
}
