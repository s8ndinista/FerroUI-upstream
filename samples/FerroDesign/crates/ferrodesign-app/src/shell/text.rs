//! A native FerroUI TextBox supplies the caret, text selection, IME and clipboard.
use super::*;
use ferroui_base::input::KeyModifiers;
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::media::{FontFamily, FontStyle, MatrixTransform, TextAlignment};
use ferroui_base::{Matrix, RelativePoint};

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
            drop(editor);
            if changed {
                platform::set_document_draft_dirty(true);
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
        self.inline_updating.set(true);
        self.inspector_tab.set(0);
        self.refresh_selection();
        let field = self.field("InlineTextBox");
        field.set_text(Some(&node.text));
        field.set_font_family(FontFamily::new(&node.font_family));
        field.set_font_size(node.font_size);
        field.set_font_weight(FontWeight(node.font_weight as i32));
        field.set_font_style(if node.italic {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        });
        field.set_line_height(node.font_size * node.line_height);
        field.set_text_alignment(match node.text_align {
            TextAlign::Left => TextAlignment::Left,
            TextAlign::Center => TextAlignment::Center,
            TextAlign::Right => TextAlignment::Right,
            TextAlign::Justify => TextAlignment::Justify,
        });
        field.set_text_wrapping(
            if node.text_auto_resize == core::TextAutoResize::WidthAndHeight {
                TextWrapping::NoWrap
            } else {
                TextWrapping::Wrap
            },
        );
        let color = node
            .fills
            .iter()
            .find(|fill| fill.visible)
            .and_then(|fill| match &fill.paint {
                Paint::Solid(color) => Some(*color),
                _ => None,
            })
            .unwrap_or(Color::rgb(32, 32, 32));
        field.set_foreground(Some(icons::brush(&xaml_color(color))));
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
        field.set_width(node.width.max(20.0));
        field.set_height(node.height.max(node.font_size * node.line_height + 4.0));
        field.set_render_transform(Some(
            MatrixTransform::with_matrix(Matrix::new(
                t.a * zoom,
                t.b * zoom,
                t.c * zoom,
                t.d * zoom,
                t.e * zoom + viewport.pan.x,
                t.f * zoom + viewport.pan.y,
            ))
            .into(),
        ));
    }

    pub(super) fn finish_inline_text(self: &Rc<Self>, commit: bool) {
        if self.inline_text.take().is_none() {
            return;
        }
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
        platform::set_document_draft_dirty(false);
    }
}
