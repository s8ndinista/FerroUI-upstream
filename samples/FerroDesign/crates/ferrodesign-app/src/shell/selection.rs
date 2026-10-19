//! A native Select Layer menu built from the same indexed, mask-aware picking
//! query used by canvas selection. Choosing a row only changes editor focus.
use super::*;

impl EditorShell {
    pub(super) fn select_layer_dialog(self: &Rc<Self>) {
        if !self.flush_inspector_drafts() {
            return;
        }
        let candidates = self.canvas.selection_candidates(true);
        self.modal("Select layer", "Layers under the last canvas pointer, from front to back. Locked layers can be selected here for inspection.");
        let host = self.control::<StackPanel>("ModalContent");
        let editor = self.editor.borrow();
        let page = editor.active_page;
        if candidates.is_empty() {
            let label = TextBlock::new();
            label.set_text(Some(
                "Move the pointer over an object, then open Select layer.",
            ));
            label.set_text_wrapping(TextWrapping::Wrap);
            host.children().add(label);
        }
        for id in candidates {
            let Some(node) = editor.document.nodes.get(&id) else {
                continue;
            };
            let locked = editor.document.effective_locked(id);
            let row = Button::new();
            row.set_name(Some(format!("SelectLayer{id}")));
            row.set_horizontal_content_alignment(ferroui_base::layout::HorizontalAlignment::Left);
            row.set_content(Some(Rc::new(format!(
                "{}  ·  {}{}",
                node.name,
                node.kind.label(),
                if locked { "  ·  Locked" } else { "" }
            ))));
            row.classes().set("active", editor.selection.contains(&id));
            let weak = Rc::downgrade(self);
            row.click(move |_, _| {
                let Some(shell) = weak.upgrade() else {
                    return;
                };
                shell.canvas.cancel_active_edit();
                let mut editor = shell.editor.borrow_mut();
                if editor.active_page != page
                    || editor.has_active_transaction()
                    || !editor.document.nodes.contains_key(&id)
                    || editor.document.page_of(id) != Some(page)
                    || !editor.document.effective_visible(id)
                {
                    return;
                }
                editor.select(id, false);
                drop(editor);
                shell.canvas.leave_vector_edit();
                shell.close_modal();
                shell.refresh_selection();
                shell.canvas.invalidate_visual();
            });
            host.children().add(row);
        }
        drop(editor);
        self.modal_button("Done", "close-modal");
    }
}
