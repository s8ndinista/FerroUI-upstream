//! Logical Home/End selection navigation over the shared model and text view.
//!
//! Contracts: pinned Microsoft.UI.Text.xml ITextSelection.HomeKey/EndKey and
//! https://learn.microsoft.com/windows/win32/api/tom/nf-tom-itextselection-homekey
//! https://learn.microsoft.com/windows/win32/api/tom/nf-tom-itextselection-endkey
use super::*;

impl RichEditTextSelection {
    /// Moves the active caret to the logical start of its Line or the Story.
    /// Extension retains the anchor; otherwise the selection collapses at the
    /// target. Other units are rejected before changing selection or composition.
    ///
    /// The result is the accepted signed UTF-16 displacement of the active end,
    /// including application changes made through cancellable selection events.
    /// Line uses complete untrimmed shaping when a view is associated, and the
    /// shared paragraph fallback otherwise. No editing/history unit is created.
    pub fn home_key(&self, unit: TextRangeUnit, extend: bool) -> Result<isize, EditError> {
        self.move_to_caret_boundary(unit, extend, false)
    }

    /// Moves the active caret to the logical end of its Line or the Story.
    /// A Line target excludes its actual line/paragraph terminator. Story ends at
    /// the model's final caret; no synthetic terminal CR is exposed.
    /// See [`home_key`](Self::home_key) for extension, layout and result policy.
    pub fn end_key(&self, unit: TextRangeUnit, extend: bool) -> Result<isize, EditError> {
        self.move_to_caret_boundary(unit, extend, true)
    }

    fn move_to_caret_boundary(
        &self,
        unit: TextRangeUnit,
        extend: bool,
        to_end: bool,
    ) -> Result<isize, EditError> {
        if !matches!(unit, TextRangeUnit::Line | TextRangeUnit::Story) {
            return Err(DocumentError::InvalidFormat(
                "HomeKey and EndKey support only Line and Story units.",
            )
            .into());
        }
        let units = TextUnits::resolve(&self.editor, unit)?;
        // Formatting can run callbacks that change the directed selection.
        // Capture the live active end after the validated layout is available.
        let before = self.selection();
        let bounds = units.caret_bounds(before.active);
        let target = if to_end { bounds.end } else { bounds.start };
        // Validate representability before a selection callback can run.
        units::signed_displacement(target, before.active)?;
        let document = self.editor.document();
        let version = document.mutation_version();
        let result = self.set_range(if extend { before.anchor } else { target }, target);
        if document.mutation_version() != version {
            // A selection or editor notification can publish an independent edit
            // even inside an application-owned batch. Never acknowledge stale
            // navigation or permit an enclosing group to restore that old model.
            self.editor.observe_external_document_change();
            return Err(EditError::ReentrantEdit);
        }
        result?;
        units::signed_displacement(self.selection().active, before.active)
    }
}

#[cfg(test)]
#[path = "rich_edit_text_selection_navigation_tests.rs"]
mod tests;
