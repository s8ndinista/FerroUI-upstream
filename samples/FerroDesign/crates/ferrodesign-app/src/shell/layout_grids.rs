//! Native per-frame grid settings, sharing the inspector draft/lifecycle boundary.
use super::*;
use ferrodesign_core::{LayoutGrid, LayoutGridEdit};

impl EditorShell {
    pub(in crate::shell) fn layout_grid_dialog(self: &Rc<Self>) {
        if !self.flush_inspector_drafts() { return; }
        let snapshot = {
            let editor = self.editor.borrow();
            let selection = editor.selection.clone();
            let source = selection.iter().filter_map(|id| editor.document.nodes.get(id))
                .find(|node| LayoutGrid::supports(node.kind) && !editor.document.effective_locked(node.id));
            source.map(|source| (selection.clone(), source.name.clone(), source.layout_grid,
                selection.iter().filter_map(|id| editor.document.nodes.get(id))
                    .filter(|node| LayoutGrid::supports(node.kind) && !editor.document.effective_locked(node.id))
                    .any(|node| node.layout_grid != source.layout_grid)))
        };
        let Some((selection, name, grid, mixed)) = snapshot else {
            self.toast("Select an unlocked frame, component or instance to edit its layout grid.");
            return;
        };
        self.updating.set(true);
        self.modal("Layout grid", &format!("{name} · Square grid guides stay inside the frame and are excluded from artwork exports and presentations."));
        self.updating.set(false);
        let host = self.control::<StackPanel>("ModalContent");
        host.set_spacing(10.0);
        if mixed {
            host.children().add(style_label("Mixed frame grids · values show the first unlocked frame. Each setting updates existing grids on unlocked selected frames."));
        }
        if let Some(grid) = grid {
            host.children().add(self.layout_grid_field(&selection, "Spacing", "LayoutGridSize", &grid.size.to_string(), |text| {
                let value = style_number(text)?;
                LayoutGrid { size: value, ..LayoutGrid::default() }.validate().map_err(|error| error.to_string())?;
                Ok(LayoutGridEdit::Size(value))
            }));
            host.children().add(self.layout_grid_field(&selection, "Color · RGBA hex", "LayoutGridColor", &grid.color.hex(), |text| {
                Ok(LayoutGridEdit::Color(style_color(text)?))
            }));
            let visible = CheckBox::new();
            visible.set_name(Some("LayoutGridVisible".into()));
            visible.set_content(Some(Rc::new("Show grid on this frame".to_string())));
            visible.set_is_checked(Some(grid.visible));
            let weak = Rc::downgrade(self);
            let weak_visible = visible.downgrade();
            let selected = selection.clone();
            visible.is_checked_changed(move |_, _| {
                let (Some(shell), Some(toggle)) = (weak.upgrade(), weak_visible.upgrade()) else { return; };
                if shell.updating.get() || !toggle.is_attached_to_visual_tree() { return; }
                if !shell.flush_inspector_drafts() {
                    shell.updating.set(true);
                    toggle.set_is_checked(Some(grid.visible));
                    shell.updating.set(false);
                    return;
                }
                match shell.apply_layout_grid_edit(&selected, LayoutGridEdit::Visible(toggle.is_checked() == Some(true))) {
                    Ok(_) => shell.layout_grid_dialog(),
                    Err(error) => {
                        shell.updating.set(true);
                        toggle.set_is_checked(Some(grid.visible));
                        shell.updating.set(false);
                        shell.toast(&error);
                    }
                }
            });
            host.children().add(visible);
            host.children().add(style_label("Changing spacing or color detaches the corresponding local grid style. Visibility stays local and keeps the shared style link."));
        } else {
            host.children().add(style_label("This frame has no layout grid. Add a square grid or apply a local grid style."));
        }
        let row = style_row();
        let selected = selection.clone();
        row.children().add(self.style_button("Add missing grids", "LayoutGridAdd", true, move |shell| {
            if !shell.flush_inspector_drafts() { return; }
            match shell.apply_layout_grid_edit(&selected, LayoutGridEdit::Add) {
                Ok(_) => shell.layout_grid_dialog(),
                Err(error) => shell.toast(&error),
            }
        }));
        let selected = selection.clone();
        row.children().add(self.style_button("Remove grids", "LayoutGridRemove", grid.is_some() || mixed, move |shell| {
            if !shell.flush_inspector_drafts() { return; }
            match shell.apply_layout_grid_edit(&selected, LayoutGridEdit::Remove) {
                Ok(_) => shell.layout_grid_dialog(),
                Err(error) => shell.toast(&error),
            }
        }));
        host.children().add(row);
        let actions = self.control::<StackPanel>("ModalActions");
        actions.children().add(self.style_button("Create grid style", "LayoutGridCreateStyle", true,
            |shell| shell.create_style("create-grid-style")));
        actions.children().add(self.style_button("Local styles", "LayoutGridStyles", true, |shell| shell.styles_dialog()));
        let done = self.style_button("Done", "LayoutGridDone", true, |shell| shell.close_modal());
        done.classes().add("primary");
        actions.children().add(done);
    }

    fn apply_layout_grid_edit(self: &Rc<Self>, selection: &[NodeId], edit: LayoutGridEdit) -> Result<bool, String> {
        let result = {
            let mut editor = self.editor.borrow_mut();
            if editor.selection != selection { return Err("The selection changed. Reopen layout grid settings before editing.".into()); }
            editor.edit_layout_grids(selection, edit).map_err(|error| error.to_string())?
        };
        if result != 0 { self.changed(); }
        Ok(result != 0)
    }

    fn layout_grid_field(
        self: &Rc<Self>, selection: &[NodeId], caption: &str, name: &str, value: &str,
        parse: impl Fn(&str) -> Result<LayoutGridEdit, String> + 'static,
    ) -> Ref<StackPanel> {
        let panel = StackPanel::new();
        panel.set_spacing(4.0);
        panel.children().add(style_label(caption));
        let field = TextBox::new();
        field.set_name(Some(name.into()));
        field.set_text(Some(value));
        panel.children().add(field.clone());
        let error = style_label("");
        error.set_name(Some(format!("{name}Error")));
        error.set_foreground(Some(icons::brush("#C8372B")));
        error.set_is_visible(false);
        panel.children().add(error.clone());
        let committed = Rc::new(RefCell::new(value.to_string()));
        let weak = Rc::downgrade(self);
        let weak_field = field.downgrade();
        let weak_error = error.downgrade();
        let previous = committed.clone();
        let selection = selection.to_vec();
        let commit: Rc<dyn Fn()> = Rc::new(move || {
            let (Some(shell), Some(field), Some(error)) = (weak.upgrade(), weak_field.upgrade(), weak_error.upgrade()) else { return; };
            if shell.updating.get() || field.is_composing_text() || !field.is_attached_to_visual_tree() || !field.is_effectively_visible() { return; }
            let text = field.text().unwrap_or_default();
            if text == *previous.borrow() { return; }
            match parse(&text).and_then(|edit| shell.apply_layout_grid_edit(&selection, edit)) {
                Ok(_) => {
                    *previous.borrow_mut() = text;
                    error.set_is_visible(false);
                    field.set_border_brush(Some(icons::brush("#E5E5E5")));
                }
                Err(message) => {
                    error.set_text(Some(&message));
                    error.set_is_visible(true);
                    field.set_border_brush(Some(icons::brush("#C8372B")));
                }
            }
            shell.sync_document_draft_dirty();
        });
        let blur = commit.clone();
        field.add_handler(InputElement::lost_focus_event(), move |_, _| blur());
        let enter = commit.clone();
        let weak_field = field.downgrade();
        field.add_handler_with(InputElement::key_down_event(), move |_, event| {
            if event.key == Key::Enter && !weak_field.upgrade().is_some_and(|field| field.is_composing_text()) {
                enter();
                event.set_handled(true);
            }
        }, RoutingStrategies::TUNNEL, false);
        self.bind_inspector_draft(&field, committed, commit, Some(&error));
        panel
    }
}
