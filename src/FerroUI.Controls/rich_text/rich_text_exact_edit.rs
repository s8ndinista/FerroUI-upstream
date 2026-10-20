//! Exact CoreText-style replacements. Native requests address the current
//! visible story; they are not insertions at the current editor selection.
use super::*;
use ferroui_base::input::text_input::{TextInputMethodEdit, TextSelection};

fn rejected() -> EditError {
    DocumentError::InvalidFormat("The exact input-method request was not accepted.").into()
}

impl RichTextEditor {
    pub(crate) fn apply_exact_input_edit(
        &self,
        edit: &TextInputMethodEdit,
        composing: bool,
    ) -> Result<bool, EditError> {
        // Policy is checked against the original request before any mutation or
        // composition/history transition. CoreText receives Failed for a request
        // that would be normalized, truncated or split at a surrogate interior.
        let expected_text = edit
            .apply_to(&self.document.text())
            .map_err(|_| rejected())?;
        let range = edit.range.start as usize..edit.range.end as usize;
        if self.permitted_text(&edit.text, range.clone()) != edit.text {
            return Err(rejected());
        }
        let _operation = self.enter(true)?;
        self.synchronize();
        let previous = self.history.borrow().composition.clone();
        if previous.is_some() && (!composing || previous.as_ref().unwrap().exact.is_none()) {
            return Err(rejected());
        }
        let visible = self.capture();
        let before = previous
            .as_ref()
            .map_or_else(|| visible.clone(), |old| old.before.clone());
        let requested_selection =
            EditorSelection::new(edit.selection.start as usize, edit.selection.end as usize);
        let text_changed = self.document.get_text(range.clone())? != edit.text;
        let batch = self.document.begin_edit();
        let result = (|| -> Result<(), EditError> {
            if let Some(exact) = previous.as_ref().and_then(|old| old.exact.as_ref()) {
                self.document.restore(&exact.document)?;
            }
            if text_changed {
                self.document.replace_text(range.clone(), &edit.text)?;
                let end = range.start + edit.text.encode_utf16().count();
                if end > range.start {
                    self.document
                        .apply_format(range.start..end, &self.typing_format())?;
                }
                if visible.document.text_length() == 0 || self.document.text_length() == 0 {
                    self.document.apply_paragraph_format(
                        0..self.document.text_length(),
                        &self.insertion_defaults(),
                    )?;
                }
            }
            self.selection.set(requested_selection);
            if composing {
                let inserted = edit.text.encode_utf16().count();
                let active_range = previous
                    .as_ref()
                    .and_then(|old| old.exact.as_ref())
                    .map_or_else(
                        || range.start..range.start + inserted,
                        |old| {
                            let end = if old.range.end >= range.end {
                                old.range.end - (range.end - range.start) + inserted
                            } else {
                                range.start + inserted
                            };
                            old.range.start.min(range.start)..end.max(range.start + inserted)
                        },
                    );
                let preview_text = self.document.get_text(active_range.clone())?;
                let clean = self.document.snapshot();
                if !active_range.is_empty() {
                    self.document.apply_format(
                        active_range.clone(),
                        &CharacterFormat {
                            text_decorations: Some(Some(TextDecorations::underline())),
                            ..CharacterFormat::default()
                        },
                    )?;
                }
                self.history.borrow_mut().composition = Some(Composition {
                    before: before.clone(),
                    preview_version: self.document.mutation_version(),
                    preview_text,
                    exact: Some(ExactComposition {
                        document: clean,
                        range: active_range,
                    }),
                });
            }
            let version = self.document.mutation_version();
            self.project_selection();
            if self.document.mutation_version() != version {
                return Err(EditError::ReentrantEdit);
            }
            if self.selection() != requested_selection {
                return Err(rejected());
            }
            Ok(())
        })();
        if let Err(error) = result {
            // Application edits made from a routed selection callback own their
            // new state. Never restore a stale snapshot over those edits.
            if !matches!(error, EditError::ReentrantEdit) {
                self.restore_composition_state(&visible, previous)?;
            } else {
                self.clear_history();
                self.clamp_selection();
            }
            drop(batch);
            self.known_revision.set(self.document.revision());
            self.publish_changed();
            return Err(error);
        }
        if composing && text_changed {
            self.queue_composition_change(RichTextCompositionChange::Preview {
                changed_text: edit.text.clone(),
            });
        }
        let accepted_version = self.document.mutation_version();
        drop(batch);
        self.known_revision.set(self.document.revision());
        if self.document.mutation_version() != accepted_version {
            self.synchronize();
            return Err(EditError::ReentrantEdit);
        }
        if composing {
            self.publish_changed();
        } else {
            self.checkpoint();
            self.record(before, EditKind::Programmatic);
        }
        if self.document.mutation_version() != accepted_version
            || self.document.text() != expected_text
            || self.selection() != requested_selection
        {
            return Err(EditError::ReentrantEdit);
        }
        Ok(text_changed)
    }

    pub(crate) fn complete_exact_composition(&self) -> Result<bool, EditError> {
        let _operation = self.enter(true)?;
        let Some(composition) = self.history.borrow().composition.clone() else {
            return Ok(false);
        };
        let Some(exact) = &composition.exact else {
            return Err(rejected());
        };
        if composition.preview_version != self.document.mutation_version() {
            return Err(EditError::ReentrantEdit);
        }
        let changed = composition.before.document != exact.document;
        let batch = self.document.begin_edit();
        self.document.restore(&exact.document)?;
        self.history.borrow_mut().composition = None;
        self.queue_composition_change(RichTextCompositionChange::Finalized {
            text: composition.preview_text,
        });
        let version = self.document.mutation_version();
        drop(batch);
        if version != self.document.mutation_version() {
            self.synchronize();
            return Err(EditError::ReentrantEdit);
        }
        self.known_revision.set(self.document.revision());
        self.checkpoint();
        self.record(composition.before, EditKind::Composition);
        Ok(changed)
    }

    pub(crate) fn select_exact_input(&self, selection: TextSelection) -> Result<(), EditError> {
        let edit = TextInputMethodEdit::new(TextSelection::new(0, 0), "", selection);
        edit.apply_to(&self.document.text())
            .map_err(|_| rejected())?;
        // A selection-only update never cancels the composition or inserts text.
        let expected = EditorSelection::new(selection.start as usize, selection.end as usize);
        let version = self.document.mutation_version();
        self.set_selection(expected)?;
        if self.document.mutation_version() != version {
            Err(EditError::ReentrantEdit)
        } else if self.selection() == expected {
            Ok(())
        } else {
            Err(rejected())
        }
    }
}
