//! A native FerroUI TextBox supplies the caret, text selection, IME and clipboard.
use super::*;
use ferroui_base::input::KeyModifiers;
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::media::MatrixTransform;
use ferroui_base::{Matrix, RelativePoint};
use crate::renderer::typography;

pub(super) struct InlineLayout {
    id: NodeId,
    revision: u64,
    origin: ferroui_base::Point,
    width: f64,
    height: f64,
}

impl EditorShell {
    pub(super) fn wire_inline_text(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.field("InlineTextBox").text_changing(move |_, _| {
            let Some(shell) = weak.upgrade() else {
                return;
            };
            if shell.inline_updating.get() {
                return;
            }
            let Some(id) = shell.inline_text.get() else {
                return;
            };
            let field = shell.field("InlineTextBox");
            // IME preedit can remove the native selection before it commits.
            // Keep confirmed text and rich-text runs in the document so a
            // lifecycle save during composition cannot persist that removal.
            if field.is_composing_text() {
                return;
            }
            let value = field.text().unwrap_or_default();
            let mut editor = shell.editor.borrow_mut();
            let changed = editor
                .document
                .nodes
                .get_mut(&id)
                .map(|node| node.replace_text_preserving_runs(value))
                .unwrap_or(false);
            editor.document.layout_all();
            editor.refresh();
            let node = editor.document.nodes.get(&id).cloned();
            drop(editor);
            if let Some(node) = node { typography::refresh_text_box_ranges(&field, &node); }
            if changed {
                shell.set_inline_draft_dirty(true);
            }
            shell.position_inline_text();
            shell.canvas.invalidate_visual();
        });
        let weak = Rc::downgrade(self);
        self.field("InlineTextBox")
            .add_handler(InputElement::lost_focus_event(), move |_, _| {
                if let Some(shell) = weak.upgrade() {
                    if !shell.inline_updating.get() {
                        shell.finish_inline_text(true);
                    }
                }
            });
        let weak = Rc::downgrade(self);
        self.field("InlineTextBox").add_handler_with(
            InputElement::key_down_event(),
            move |_, event| {
                if event.key == Key::Escape
                    || (event.key == Key::Enter
                        && event
                            .key_modifiers
                            .intersects(KeyModifiers::CONTROL | KeyModifiers::META))
                {
                    if let Some(shell) = weak.upgrade() {
                        shell.finish_inline_text(event.key != Key::Escape);
                        shell.canvas.focus();
                    }
                    event.set_handled(true);
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        );
    }

    pub(super) fn begin_inline_text(self: &Rc<Self>, id: NodeId) {
        if self.inline_text.get() == Some(id) {
            self.field("InlineTextBox").focus();
            return;
        }
        self.finish_inline_text(true);
        let node = {
            let editor = self.editor.borrow();
            let Some(node) = editor.document.nodes.get(&id) else {
                return;
            };
            if node.kind != NodeKind::Text || node.locked || !node.visible {
                return;
            }
            let mut parent = node.parent;
            while let Some(id) = parent {
                let Some(ancestor) = editor.document.nodes.get(&id) else {
                    break;
                };
                if ancestor.locked || !ancestor.visible {
                    return;
                }
                parent = ancestor.parent;
            }
            node.clone()
        };
        self.canvas.leave_vector_edit();
        {
            let mut editor = self.editor.borrow_mut();
            editor.selection = vec![id];
            editor.begin_transaction("Edit text");
        }
        self.inline_text.set(Some(id));
        self.inline_layout.borrow_mut().take();
        self.inline_updating.set(true);
        self.inspector_tab.set(0);
        self.refresh_selection();
        let field = self.field("InlineTextBox");
        field.set_text(Some(&node.text));
        typography::configure_text_box(&field, &node);
        field.set_opacity(node.opacity);
        field.set_render_transform_origin(RelativePoint::TOP_LEFT);
        self.position_inline_text();
        self.visible("InlineTextHost", true);
        self.canvas.set_editing_text(Some(id));
        field.focus();
        field.select_all();
        self.inline_updating.set(false);
    }

    pub(super) fn position_inline_text(&self) {
        let Some(id) = self.inline_text.get() else {
            return;
        };
        let editor = self.editor.borrow();
        let Some(node) = editor.document.nodes.get(&id) else {
            return;
        };
        let Some(t) = editor.document.world_transform(id) else {
            return;
        };
        let viewport = self.canvas.viewport.get();
        let zoom = viewport.zoom;
        let field = self.field("InlineTextBox");
        let mut cached = self.inline_layout.borrow_mut();
        if cached.as_ref().is_none_or(|cached| cached.id != id || cached.revision != node.revision) {
            let brush = field.foreground().unwrap_or_else(|| typography::editor_brush(node));
            let (origin, width, height) = typography::inline_geometry(node, brush);
            *cached = Some(InlineLayout { id, revision: node.revision, origin, width, height });
        }
        let cached = cached.as_ref().expect("inline layout");
        field.set_width(cached.width);
        field.set_height(cached.height);
        let origin = t.apply(Point::new(cached.origin.x, cached.origin.y));
        field.set_render_transform(Some(
            MatrixTransform::with_matrix(Matrix::new(
                t.a * zoom,
                t.b * zoom,
                t.c * zoom,
                t.d * zoom,
                origin.x * zoom + viewport.pan.x,
                origin.y * zoom + viewport.pan.y,
            ))
            .into(),
        ));
    }

    pub(super) fn finish_inline_text(self: &Rc<Self>, commit: bool) {
        if self.inline_text.take().is_none() {
            return;
        }
        self.inline_layout.borrow_mut().take();
        self.inline_updating.set(true);
        let changed = {
            let mut editor = self.editor.borrow_mut();
            if commit {
                editor.commit_transaction()
            } else {
                editor.cancel_transaction();
                false
            }
        };
        self.visible("InlineTextHost", false);
        self.canvas.set_editing_text(None);
        self.inline_updating.set(false);
        if changed {
            self.changed();
        } else {
            self.refresh_selection();
        }
        self.set_inline_draft_dirty(false);
    }
}
