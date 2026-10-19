use crate::RichEditBox;
use ferroui_base::input::text_input::{
    ContextMenuAction, TextInputMethodClient, TextInputMethodClientEvents, TextSelection,
};
use ferroui_base::{Rect, Ref, Visual, WeakRef};
use std::cell::RefCell;
use std::rc::Rc;

/// Platform input-method projection of the live rich document and layout geometry.
/// Its owner is weak, so retaining a platform client cannot retain a removed control.
pub struct RichEditBoxTextInputMethodClient {
    events: TextInputMethodClientEvents,
    owner: RefCell<Option<WeakRef<RichEditBox>>>,
}

impl RichEditBoxTextInputMethodClient {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self {
            events: TextInputMethodClientEvents::new(),
            owner: RefCell::new(None),
        })
    }
    pub(crate) fn attach(&self, owner: Option<&Ref<RichEditBox>>) {
        *self.owner.borrow_mut() = owner.map(Ref::downgrade);
        self.raise_text_view_visual_changed();
        self.raise_cursor_rectangle_changed();
    }
    fn owner(&self) -> Option<Ref<RichEditBox>> {
        self.owner.borrow().as_ref().and_then(WeakRef::upgrade)
    }
}

impl TextInputMethodClient for RichEditBoxTextInputMethodClient {
    fn events(&self) -> &TextInputMethodClientEvents {
        &self.events
    }
    fn text_view_visual(&self) -> Option<Ref<Visual>> {
        self.owner().map(|owner| owner.text_presenter().upcast())
    }
    fn supports_preedit(&self) -> bool {
        true
    }
    fn supports_surrounding_text(&self) -> bool {
        true
    }
    fn surrounding_text(&self) -> String {
        self.owner()
            .map_or_else(String::new, |owner| owner.editor().surrounding_text())
    }
    fn cursor_rectangle(&self) -> Rect {
        self.owner()
            .map_or_else(Rect::default, |owner| owner.cursor_rectangle())
    }
    fn selection(&self) -> TextSelection {
        self.owner().map_or_else(TextSelection::default, |owner| {
            let selection = if owner.editor().is_composing() {
                owner.editor().input_method_selection()
            } else {
                crate::rich_text::EditorSelection::new(
                    owner.selection_start(),
                    owner.selection_end(),
                )
            };
            TextSelection::new(
                selection.anchor.min(i32::MAX as usize) as i32,
                selection.active.min(i32::MAX as usize) as i32,
            )
        })
    }
    fn set_selection(&self, selection: TextSelection) {
        if let Some(owner) = self.owner() {
            // Native offsets refer to the committed surrounding text, so remove the
            // visible preview before applying the platform's new selection.
            if owner.editor().is_composing() {
                owner.handle_edit(owner.set_preedit_text(None, None));
            }
            owner.select(
                selection.start.max(0) as usize,
                selection.end.max(0) as usize,
            );
        }
    }
    fn set_preedit_text(&self, text: Option<&str>) {
        self.set_preedit_text_with_cursor(text, None);
    }
    fn set_preedit_text_with_cursor(&self, text: Option<&str>, cursor: Option<i32>) {
        if let Some(owner) = self.owner() {
            owner.handle_edit(
                owner.set_preedit_text(text, cursor.map(|value| value.max(0) as usize)),
            );
        }
    }
    fn execute_context_menu_action(&self, action: ContextMenuAction) {
        if let Some(owner) = self.owner() {
            match action {
                ContextMenuAction::Copy => owner.copy(),
                ContextMenuAction::Cut => owner.cut(),
                ContextMenuAction::Paste => owner.paste(),
                ContextMenuAction::SelectAll => owner.select_all(),
            }
        }
    }
}
