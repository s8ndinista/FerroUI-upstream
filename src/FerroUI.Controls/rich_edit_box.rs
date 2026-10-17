use crate::documents::{
    CharacterFormat, DocumentSnapshot, InlineSnapshot, InlineSnapshotKind, ParagraphFormat,
    TextElement,
};
use crate::rich_edit_box_text_input_method_client::RichEditBoxTextInputMethodClient;
use crate::rich_text::{
    EditError, EditKind, EditorSelection, RichEditClipboardFormat, RichEditTextDocument,
    RichTextEditor, RichTextTransfer, RtfError, RtfReader, RtfWriter,
};
use crate::utils::ClipboardHelper;
use crate::{
    Application, ControlImpl, RichTextBlock, RichTextBlockImpl, RichTextBlockImplExt,
    TextChangedEventArgs, TextChangingEventArgs, TopLevel,
};
use ferroui_base::input::platform::{
    ClipboardError, ClipboardErrorKind, IClipboard, PlatformHotkeyConfiguration,
};
use ferroui_base::input::text_input::{
    TextInputContentType, TextInputMethodClient, TextInputOptions,
};
use ferroui_base::input::{
    FocusChangedEventArgs, InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs,
    KeyGesture, LocalBoxFuture, TextInputEventArgs,
};
use ferroui_base::interactivity::{
    Interactive, InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken,
    RoutingStrategies,
};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::text_formatting::{TextLayout, TextLayoutOptions};
use ferroui_base::media::{
    DrawingContext, FontStyle, FontWeight, IBrush, TextDecorationLocation, TextDecorations,
    Typeface,
};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    DirectProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Point, Ref, StyledElementImpl, StyledProperty, VisualImpl,
    ferro_class, ferro_impl_classes, ferro_routed_event, ferro_routed_event_args, instantiate,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Character casing applied to newly entered text, including clipboard and IME commits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum RichEditCharacterCasing {
    #[default]
    Normal = 0,
    Lower = 1,
    Upper = 2,
}
ferroui_base::ferro_markup_enum!(RichEditCharacterCasing { Normal, Lower, Upper }, { namespace: "FerroUI.Controls" });

bitflags::bitflags! {
    /// Built-in formatting accelerators applications can individually suppress.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct DisabledFormattingAccelerators: u32 {
        const NONE = 0;
        const BOLD = 1;
        const ITALIC = 2;
        const UNDERLINE = 4;
        const ALL = u32::MAX;
    }
}
ferroui_base::ferro_markup_enum!(flags DisabledFormattingAccelerators {
    None = DisabledFormattingAccelerators::NONE,
    Bold = DisabledFormattingAccelerators::BOLD,
    Italic = DisabledFormattingAccelerators::ITALIC,
    Underline = DisabledFormattingAccelerators::UNDERLINE,
    All = DisabledFormattingAccelerators::ALL,
}, { namespace: "FerroUI.Controls" });

#[derive(Clone, Default)]
pub struct RichEditCompositionEventArgs {
    base: RoutedEventArgs,
    pub text: String,
    pub selection: EditorSelection,
    pub committed: bool,
}
ferro_routed_event_args!(RichEditCompositionEventArgs: RoutedEventArgs);
ferroui_base::ferro_markup_type!(class RichEditCompositionEventArgs {
    namespace: "FerroUI.Controls",
    handles: [RichEditCompositionEventArgs],
    properties: [
        Text: String { get: |args: &RichEditCompositionEventArgs| args.text.clone() },
        SelectionStart: usize { get: |args: &RichEditCompositionEventArgs| args.selection.anchor },
        SelectionEnd: usize { get: |args: &RichEditCompositionEventArgs| args.selection.active },
        Committed: bool { get: |args: &RichEditCompositionEventArgs| args.committed },
        Handled: bool { get: |args: &RichEditCompositionEventArgs| args.handled(), set: |args: &RichEditCompositionEventArgs, value: bool| args.set_handled(value) },
    ],
});

/// A structured rich-text editor rendered by FerroUI's text layout, drawing, and input APIs.
///
/// The inherited `Document` is shared with the view and remains editable independently of
/// this control. Read-only and length policies apply to user editing, clipboard, and IME;
/// applications can replace the document directly. Every user operation goes through the
/// same transaction engine, including formatting and accessible value changes.
#[repr(C)]
pub struct RichEditBox {
    base: RichTextBlock,
    editor: RefCell<Option<Rc<RichTextEditor>>>,
    im_client: Rc<RichEditBoxTextInputMethodClient>,
    document_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    selection_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    editor_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    last_text: RefCell<String>,
    can_undo: Cell<bool>,
    can_redo: Cell<bool>,
    can_copy: Cell<bool>,
    can_cut: Cell<bool>,
    can_paste: Cell<bool>,
    errors: Rc<HandlerList<dyn Fn(&EditError)>>,
    clipboard_errors: Rc<HandlerList<dyn Fn(&ClipboardError)>>,
    caret_scroll_pending: Cell<bool>,
}

ferro_class!(RichEditBox: RichTextBlock);
ferroui_base::ferro_class_info!(RichEditBox {
    new: RichEditBox::new,
    markup: {
        methods: [
            fn Copy() => RichEditBox::copy,
            fn Cut() => RichEditBox::cut,
            fn Paste() => RichEditBox::paste,
            fn SelectAll() => RichEditBox::select_all,
            fn Undo() => |this: &Ref<RichEditBox>| { this.handle_edit(this.undo()); },
            fn Redo() => |this: &Ref<RichEditBox>| { this.handle_edit(this.redo()); },
        ],
    },
});
ferro_impl_classes!(RichEditBox: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl);

impl RichTextBlockImpl for RichEditBox {
    fn allow_hyperlink_navigation(
        this: &Self,
        modifiers: ferroui_base::input::KeyModifiers,
    ) -> bool {
        this.is_read_only()
            || modifiers.intersects(
                ferroui_base::input::KeyModifiers::CONTROL
                    | ferroui_base::input::KeyModifiers::META,
            )
    }
    fn caret_visible(this: &Self) -> bool {
        this.is_focused() && this.is_enabled() && !this.is_read_only()
    }

    fn render_text(this: &Self, context: &mut DrawingContext) {
        Self::parent_render_text(this, context);
        if this.document().text_length() == 0 && !this.placeholder_text().is_empty() {
            let padding = this.padding();
            let font = Typeface::with_style(
                TextElement::get_font_family(this),
                TextElement::get_font_style(this),
                TextElement::get_font_weight(this),
                TextElement::get_font_stretch(this),
            );
            let layout = TextLayout::new(
                &this.placeholder_text(),
                font,
                TextLayoutOptions {
                    font_size: TextElement::get_font_size(this),
                    foreground: this
                        .placeholder_foreground()
                        .or_else(|| TextElement::get_foreground(this)),
                    max_width: (this.bounds().width - padding.left - padding.right).max(0.0),
                    text_wrapping: this.text_wrapping(),
                    ..TextLayoutOptions::default()
                },
            );
            layout.draw(context, Point::new(padding.left, padding.top));
        }
    }
}

impl ControlImpl for RichEditBox {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::RichEditBoxAutomationPeer::new(this).upcast()
    }
}

impl FerroObjectImpl for RichEditBox {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.im_client.attach(Some(&this.to_ref()));
        this.attach_document();
        let weak = this.to_ref().downgrade();
        let selection_subscription = this.selection_changed(move |_| {
            if let Some(this) = weak.upgrade() {
                // Document notifications can rebase the view while an editor transaction
                // is publishing. The editor emits its final selection when that finishes.
                let _ = this.editor().set_selection(this.selection());
                this.im_client.raise_selection_changed();
                this.im_client.raise_cursor_rectangle_changed();
                this.update_command_states();
            }
        });
        *this.selection_subscription.borrow_mut() = Some(selection_subscription);
        this.update_input_options();
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == RichTextBlock::document_property().as_property() {
            this.attach_document();
        }
        if property == Self::is_read_only_property().as_property() {
            let editor = this.editor();
            if this.is_read_only() {
                this.handle_edit(editor.cancel_composition());
            }
            editor.set_is_read_only(this.is_read_only());
            this.im_client.request_reset();
            this.update_command_states();
            this.invalidate_visual();
        } else if property == Self::max_length_property().as_property() {
            this.editor().set_max_length(this.max_length());
        } else if property == Self::undo_limit_property().as_property()
            || property == Self::is_undo_enabled_property().as_property()
        {
            this.editor().set_undo_limit(if this.is_undo_enabled() {
                this.undo_limit()
            } else {
                0
            });
            this.update_command_states();
        }
        if property == Self::accepts_return_property().as_property()
            || property == Self::character_casing_property().as_property()
            || property == Self::is_text_prediction_enabled_property().as_property()
            || property == Self::input_scope_property().as_property()
        {
            this.update_input_options();
        }
        if property == Self::placeholder_text_property().as_property()
            || property == Self::placeholder_foreground_property().as_property()
        {
            this.invalidate_visual();
        }
    }
}

impl InputElementImpl for RichEditBox {
    fn on_got_focus(this: &Self, event: &FocusChangedEventArgs) {
        Self::parent_on_got_focus(this, event);
        this.im_client.raise_text_view_visual_changed();
        this.im_client.raise_cursor_rectangle_changed();
        this.invalidate_visual();
    }
    fn on_lost_focus(this: &Self, event: &FocusChangedEventArgs) {
        Self::parent_on_lost_focus(this, event);
        this.handle_edit(this.cancel_composition());
        this.editor().checkpoint();
        this.im_client.request_reset();
        this.invalidate_visual();
    }
    fn on_text_input(this: &Self, event: &TextInputEventArgs) {
        if event.handled() {
            return;
        }
        if !this.is_read_only() {
            if let Some(text) = &event.text {
                this.handle_edit(this.insert_text(text));
            }
        }
        event.set_handled(true);
    }
    fn on_key_down(this: &Self, event: &KeyEventArgs) {
        if event.handled() {
            return;
        }
        if this.editor().is_composing() {
            if event.key == Key::Escape {
                this.handle_edit(this.cancel_composition());
                event.set_handled(true);
            }
            return;
        }
        let keymap = Application::current()
            .and_then(|app| app.platform_settings())
            .map(|settings| settings.hotkey_configuration())
            .unwrap_or_else(|| Rc::new(PlatformHotkeyConfiguration::default()));
        let matches =
            |gestures: &[KeyGesture]| gestures.iter().any(|gesture| gesture.matches(Some(event)));
        if matches(&keymap.copy) {
            this.copy();
        } else if matches(&keymap.cut) {
            this.cut();
        } else if matches(&keymap.paste) {
            this.paste();
        } else if matches(&keymap.undo) {
            if !this.is_read_only() {
                this.handle_edit(this.undo());
            }
        } else if matches(&keymap.redo) {
            if !this.is_read_only() {
                this.handle_edit(this.redo());
            }
        } else if !this.is_read_only()
            && event.key_modifiers == keymap.command_modifiers
            && this.format_accelerator(event.key)
        {
        } else {
            let whole_word = event
                .key_modifiers
                .contains(keymap.whole_word_text_action_modifiers);
            match event.key {
                Key::Back => {
                    if !this.is_read_only() {
                        this.handle_edit(this.delete_backward(whole_word));
                    }
                }
                Key::Delete => {
                    if !this.is_read_only() {
                        this.handle_edit(this.delete_forward(whole_word));
                    }
                }
                Key::Enter if this.accepts_return() => {
                    if !this.is_read_only() {
                        this.handle_edit(this.insert_text("\n"));
                    }
                }
                Key::Tab if this.accepts_tab() => {
                    if !this.is_read_only() {
                        this.handle_edit(this.insert_text("\t"));
                    }
                }
                _ => {
                    Self::parent_on_key_down(this, event);
                    return;
                }
            }
        }
        event.set_handled(true);
    }
}

ferroui_base::ferro_properties! { impl RichEditBox {
    pub fn is_read_only_property() -> StyledProperty<bool> { FerroProperty::register::<RichEditBox, _>("IsReadOnly", false) }
    pub fn accepts_return_property() -> StyledProperty<bool> { FerroProperty::register::<RichEditBox, _>("AcceptsReturn", true) }
    pub fn accepts_tab_property() -> StyledProperty<bool> { FerroProperty::register::<RichEditBox, _>("AcceptsTab", false) }
    pub fn max_length_property() -> StyledProperty<usize> { FerroProperty::register::<RichEditBox, _>("MaxLength", 0) }
    pub fn clipboard_copy_format_property() -> StyledProperty<RichEditClipboardFormat> { FerroProperty::register::<RichEditBox, _>("ClipboardCopyFormat", RichEditClipboardFormat::AllFormats) }
    pub fn disabled_formatting_accelerators_property() -> StyledProperty<DisabledFormattingAccelerators> { FerroProperty::register::<RichEditBox, _>("DisabledFormattingAccelerators", DisabledFormattingAccelerators::NONE) }
    pub fn character_casing_property() -> StyledProperty<RichEditCharacterCasing> { FerroProperty::register::<RichEditBox, _>("CharacterCasing", RichEditCharacterCasing::Normal) }
    pub fn is_text_prediction_enabled_property() -> StyledProperty<bool> { FerroProperty::register::<RichEditBox, _>("IsTextPredictionEnabled", true) }
    pub fn input_scope_property() -> StyledProperty<TextInputContentType> { FerroProperty::register::<RichEditBox, _>("InputScope", TextInputContentType::Normal) }
    pub fn placeholder_text_property() -> StyledProperty<String> { FerroProperty::register::<RichEditBox, _>("PlaceholderText", String::new()) }
    pub fn placeholder_foreground_property() -> StyledProperty<Option<Rc<dyn IBrush>>> { FerroProperty::register::<RichEditBox, _>("PlaceholderForeground", None) }
    pub fn is_undo_enabled_property() -> StyledProperty<bool> { FerroProperty::register::<RichEditBox, _>("IsUndoEnabled", true) }
    pub fn undo_limit_property() -> StyledProperty<usize> { FerroProperty::register::<RichEditBox, _>("UndoLimit", 100) }
    pub fn can_undo_property() -> DirectProperty<RichEditBox, bool> { FerroProperty::register_direct::<RichEditBox, _>("CanUndo", |this| this.can_undo.get(), None, false) }
    pub fn can_redo_property() -> DirectProperty<RichEditBox, bool> { FerroProperty::register_direct::<RichEditBox, _>("CanRedo", |this| this.can_redo.get(), None, false) }
    pub fn can_copy_property() -> DirectProperty<RichEditBox, bool> { FerroProperty::register_direct::<RichEditBox, _>("CanCopy", |this| this.can_copy.get(), None, false) }
    pub fn can_cut_property() -> DirectProperty<RichEditBox, bool> { FerroProperty::register_direct::<RichEditBox, _>("CanCut", |this| this.can_cut.get(), None, false) }
    pub fn can_paste_property() -> DirectProperty<RichEditBox, bool> { FerroProperty::register_direct::<RichEditBox, _>("CanPaste", |this| this.can_paste.get(), None, false) }
    pub fn text_property() -> DirectProperty<RichEditBox, String> {
        FerroProperty::register_direct::<RichEditBox, String>("Text", |this| this.text(), Some(|this, value| { this.handle_edit(this.set_text(&value)); }), String::new())
    }
} }

impl RichEditBox {
    fn static_constructor() {
        InputElement::focusable_property().override_default_value::<RichEditBox>(true);
        InputElement::text_input_method_client_requested_event().add_class_handler::<RichEditBox>(
            |this, event| {
                if !this.is_read_only() {
                    event.set_client(Some(this.im_client.clone()));
                }
            },
        );
    }
    pub fn construct() -> Self {
        Self {
            base: RichTextBlock::construct(),
            editor: RefCell::new(None),
            im_client: RichEditBoxTextInputMethodClient::new(),
            document_subscription: RefCell::new(None),
            selection_subscription: RefCell::new(None),
            editor_subscription: RefCell::new(None),
            last_text: RefCell::new(String::new()),
            can_undo: Cell::new(false),
            can_redo: Cell::new(false),
            can_copy: Cell::new(false),
            can_cut: Cell::new(false),
            can_paste: Cell::new(true),
            errors: Rc::new(HandlerList::new()),
            clipboard_errors: Rc::new(HandlerList::new()),
            caret_scroll_pending: Cell::new(false),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn is_read_only(&self) -> bool {
        self.get_value(Self::is_read_only_property())
    }
    pub fn set_is_read_only(&self, value: bool) {
        self.set_value(Self::is_read_only_property(), value);
    }
    pub fn accepts_return(&self) -> bool {
        self.get_value(Self::accepts_return_property())
    }
    pub fn set_accepts_return(&self, value: bool) {
        self.set_value(Self::accepts_return_property(), value);
    }
    pub fn accepts_tab(&self) -> bool {
        self.get_value(Self::accepts_tab_property())
    }
    pub fn set_accepts_tab(&self, value: bool) {
        self.set_value(Self::accepts_tab_property(), value);
    }
    pub fn max_length(&self) -> usize {
        self.get_value(Self::max_length_property())
    }
    pub fn set_max_length(&self, value: usize) {
        self.set_value(Self::max_length_property(), value);
    }
    pub fn clipboard_copy_format(&self) -> RichEditClipboardFormat {
        self.get_value(Self::clipboard_copy_format_property())
    }
    pub fn set_clipboard_copy_format(&self, value: RichEditClipboardFormat) {
        self.set_value(Self::clipboard_copy_format_property(), value);
    }
    pub fn disabled_formatting_accelerators(&self) -> DisabledFormattingAccelerators {
        self.get_value(Self::disabled_formatting_accelerators_property())
    }
    pub fn set_disabled_formatting_accelerators(&self, value: DisabledFormattingAccelerators) {
        self.set_value(Self::disabled_formatting_accelerators_property(), value);
    }
    pub fn character_casing(&self) -> RichEditCharacterCasing {
        self.get_value(Self::character_casing_property())
    }
    pub fn set_character_casing(&self, value: RichEditCharacterCasing) {
        self.set_value(Self::character_casing_property(), value);
    }
    pub fn is_text_prediction_enabled(&self) -> bool {
        self.get_value(Self::is_text_prediction_enabled_property())
    }
    pub fn set_is_text_prediction_enabled(&self, value: bool) {
        self.set_value(Self::is_text_prediction_enabled_property(), value);
    }
    pub fn input_scope(&self) -> TextInputContentType {
        self.get_value(Self::input_scope_property())
    }
    pub fn set_input_scope(&self, value: TextInputContentType) {
        self.set_value(Self::input_scope_property(), value);
    }
    pub fn placeholder_text(&self) -> String {
        self.get_value(Self::placeholder_text_property())
    }
    pub fn set_placeholder_text(&self, value: impl Into<String>) {
        self.set_value(Self::placeholder_text_property(), value.into());
    }
    pub fn placeholder_foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::placeholder_foreground_property())
    }
    pub fn set_placeholder_foreground(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::placeholder_foreground_property(), value);
    }
    pub fn is_undo_enabled(&self) -> bool {
        self.get_value(Self::is_undo_enabled_property())
    }
    pub fn set_is_undo_enabled(&self, value: bool) {
        self.set_value(Self::is_undo_enabled_property(), value);
    }
    pub fn undo_limit(&self) -> usize {
        self.get_value(Self::undo_limit_property())
    }
    pub fn set_undo_limit(&self, value: usize) {
        self.set_value(Self::undo_limit_property(), value);
    }
    pub fn can_undo(&self) -> bool {
        self.editor().can_undo()
    }
    pub fn can_redo(&self) -> bool {
        self.editor().can_redo()
    }
    pub fn can_copy(&self) -> bool {
        self.selection_start() != self.selection_end()
    }
    pub fn can_cut(&self) -> bool {
        self.can_copy() && !self.is_read_only()
    }
    pub fn can_paste(&self) -> bool {
        !self.is_read_only() && self.is_enabled()
    }
    pub fn text(&self) -> String {
        self.document().text()
    }
    /// Text-object editing services sharing the view's document, selection, and undo history.
    pub fn text_document(&self) -> RichEditTextDocument {
        RichEditTextDocument::from_editor(self.editor())
    }

    /// Replaces all content programmatically. The public document is not limited by the UI's
    /// input policies. Selection and user undo history are reset after replacement.
    pub fn set_text(&self, text: &str) -> Result<(), EditError> {
        self.document()
            .replace_text(0..self.document().text_length(), text)?;
        self.editor().clear_history();
        self.select(0, 0);
        self.update_command_states();
        Ok(())
    }

    pub fn editor(&self) -> Rc<RichTextEditor> {
        let existing = self.editor.borrow().clone();
        if let Some(editor) = existing.filter(|editor| editor.document() == self.document()) {
            return editor;
        }
        let editor = Rc::new(RichTextEditor::new(self.document()));
        editor.set_is_read_only(self.is_read_only());
        editor.set_max_length(self.max_length());
        editor.set_undo_limit(if self.is_undo_enabled() {
            self.undo_limit()
        } else {
            0
        });
        *self.editor.borrow_mut() = Some(editor.clone());
        let old = self.editor_subscription.borrow_mut().take();
        if let Some(old) = old {
            old.dispose();
        }
        let weak = self.to_ref().downgrade();
        let subscription = editor.changed(move || {
            if let Some(this) = weak.upgrade() {
                this.after_edit(&this.editor());
            }
        });
        *self.editor_subscription.borrow_mut() = Some(subscription);
        editor
    }

    fn attach_document(&self) {
        let old = self.document_subscription.borrow_mut().take();
        if let Some(old) = old {
            old.dispose();
        }
        let old_editor = self.editor.borrow_mut().take();
        drop(old_editor);
        let weak = self.to_ref().downgrade();
        let subscription = self.document().changed(move |_| {
            if let Some(this) = weak.upgrade() {
                this.on_document_changed();
            }
        });
        *self.document_subscription.borrow_mut() = Some(subscription);
        self.on_document_changed();
    }

    fn on_document_changed(&self) {
        let text = self.text();
        let previous = self.last_text.replace(text.clone());
        if previous != text {
            self.raise_direct_property_changed(Self::text_property(), &previous, &text);
        }
        self.raise_event(&TextChangingEventArgs::with_event(
            Self::text_changing_event(),
        ));
        self.raise_event(&TextChangedEventArgs::with_event(Self::text_changed_event()));
        self.im_client.raise_surrounding_text_changed();
        self.im_client.raise_cursor_rectangle_changed();
        self.update_command_states();
    }

    fn update_input_options(&self) {
        TextInputOptions::set_multiline(self, self.accepts_return());
        TextInputOptions::set_lowercase(
            self,
            self.character_casing() == RichEditCharacterCasing::Lower,
        );
        TextInputOptions::set_uppercase(
            self,
            self.character_casing() == RichEditCharacterCasing::Upper,
        );
        TextInputOptions::set_show_suggestions(self, Some(self.is_text_prediction_enabled()));
        TextInputOptions::set_content_type(self, self.input_scope());
    }

    fn update_command_states(&self) {
        self.set_and_raise_cell(Self::can_undo_property(), &self.can_undo, self.can_undo());
        self.set_and_raise_cell(Self::can_redo_property(), &self.can_redo, self.can_redo());
        self.set_and_raise_cell(Self::can_copy_property(), &self.can_copy, self.can_copy());
        self.set_and_raise_cell(Self::can_cut_property(), &self.can_cut, self.can_cut());
        self.set_and_raise_cell(
            Self::can_paste_property(),
            &self.can_paste,
            self.can_paste(),
        );
    }

    fn selection(&self) -> EditorSelection {
        EditorSelection::new(self.selection_start(), self.selection_end())
    }
    fn synchronize_editor_selection(&self, editor: &RichTextEditor) -> Result<(), EditError> {
        editor.set_selection(self.selection())
    }
    fn after_edit(&self, editor: &RichTextEditor) {
        let selection = editor.selection();
        self.select(selection.anchor, selection.active);
        self.im_client.raise_selection_changed();
        self.im_client.raise_cursor_rectangle_changed();
        self.update_command_states();
        self.invalidate_visual();
        self.ensure_caret_visible();
    }
    fn ensure_caret_visible(&self) {
        if self.visual_parent().is_none() {
            return;
        }
        if self.is_measure_valid() && self.is_arrange_valid() {
            self.bring_into_view_rect(self.cursor_rectangle());
        } else if !self.caret_scroll_pending.replace(true) {
            let weak = self.to_ref().downgrade();
            Dispatcher::ui_thread().post_local(
                move || {
                    if let Some(this) = weak.upgrade() {
                        this.caret_scroll_pending.set(false);
                        if this.visual_parent().is_some() {
                            this.bring_into_view_rect(this.cursor_rectangle());
                        }
                    }
                },
                DispatcherPriority::from_value(DispatcherPriority::UI_THREAD_RENDER.value() + 1),
            );
        }
    }
    fn filter_input(&self, text: &str) -> String {
        let text: String = text
            .chars()
            .filter(|ch| {
                *ch != '\u{7f}'
                    && (*ch != '\r' && *ch != '\n' || self.accepts_return())
                    && (*ch != '\t' || self.accepts_tab())
            })
            .collect();
        match self.character_casing() {
            RichEditCharacterCasing::Normal => text,
            RichEditCharacterCasing::Lower => text.to_lowercase(),
            RichEditCharacterCasing::Upper => text.to_uppercase(),
        }
    }
    fn filter_fragment(&self, fragment: &DocumentSnapshot) -> DocumentSnapshot {
        fn filter_inlines(owner: &RichEditBox, inlines: &mut Vec<InlineSnapshot>) {
            inlines.retain_mut(|inline| {
                match &mut inline.kind {
                    InlineSnapshotKind::Run(text) => *text = owner.filter_input(text),
                    InlineSnapshotKind::LineBreak if !owner.accepts_return() => return false,
                    _ => {}
                }
                if let Some(children) = inline.children_mut() {
                    filter_inlines(owner, children);
                }
                true
            });
        }
        let mut fragment = fragment.clone();
        for paragraph in &mut fragment.paragraphs {
            filter_inlines(self, &mut paragraph.inlines);
        }
        if !self.accepts_return() && fragment.paragraphs.len() > 1 {
            let mut paragraphs = fragment.paragraphs.into_iter();
            let mut first = paragraphs
                .next()
                .expect("The fragment has multiple paragraphs.");
            for paragraph in paragraphs {
                first.inlines.push(InlineSnapshot {
                    format: paragraph.format.character,
                    kind: InlineSnapshotKind::Span(paragraph.inlines),
                });
            }
            fragment.paragraphs = vec![first];
        }
        fragment
    }
    pub fn insert_text(&self, text: &str) -> Result<bool, EditError> {
        let editor = self.editor();
        self.synchronize_editor_selection(&editor)?;
        let text = self.filter_input(text);
        let composing = editor.is_composing();
        let result = if composing {
            editor.commit_composition(&text)
        } else {
            editor.insert_text(&text, EditKind::Typing)
        };
        self.after_edit(&editor);
        if composing && result.is_ok() {
            self.raise_composition(Self::text_composition_ended_event(), &text, true);
        }
        result
    }
    pub fn replace_selection(&self, fragment: &DocumentSnapshot) -> Result<bool, EditError> {
        let editor = self.editor();
        self.synchronize_editor_selection(&editor)?;
        let result = editor.insert_fragment(fragment, EditKind::Paste);
        self.after_edit(&editor);
        result
    }
    pub fn delete_backward(&self, whole_word: bool) -> Result<bool, EditError> {
        let editor = self.editor();
        self.synchronize_editor_selection(&editor)?;
        let result = editor.delete_backward(whole_word);
        self.after_edit(&editor);
        result
    }
    pub fn delete_forward(&self, whole_word: bool) -> Result<bool, EditError> {
        let editor = self.editor();
        self.synchronize_editor_selection(&editor)?;
        let result = editor.delete_forward(whole_word);
        self.after_edit(&editor);
        result
    }
    pub fn apply_format(&self, format: &CharacterFormat) -> Result<(), EditError> {
        let editor = self.editor();
        self.synchronize_editor_selection(&editor)?;
        let result = editor.apply_format(format);
        self.after_edit(&editor);
        result
    }
    pub fn apply_paragraph_format(&self, format: &ParagraphFormat) -> Result<(), EditError> {
        let editor = self.editor();
        self.synchronize_editor_selection(&editor)?;
        let result = editor.apply_paragraph_format(format);
        self.after_edit(&editor);
        result
    }
    pub fn undo(&self) -> Result<bool, EditError> {
        let editor = self.editor();
        let result = editor.undo();
        self.after_edit(&editor);
        result
    }
    pub fn redo(&self) -> Result<bool, EditError> {
        let editor = self.editor();
        let result = editor.redo();
        self.after_edit(&editor);
        result
    }
    pub fn select_all(&self) {
        RichTextBlock::select_all(self);
    }

    fn format_accelerator(&self, key: Key) -> bool {
        let disabled = self.disabled_formatting_accelerators();
        let editor = self.editor();
        let _ = self.synchronize_editor_selection(&editor);
        let current = editor.current_character_format().unwrap_or_default();
        let format = match key {
            Key::B if !disabled.contains(DisabledFormattingAccelerators::BOLD) => CharacterFormat {
                font_weight: Some(if current.font_weight == Some(FontWeight::Bold) {
                    FontWeight::Normal
                } else {
                    FontWeight::Bold
                }),
                ..CharacterFormat::default()
            },
            Key::I if !disabled.contains(DisabledFormattingAccelerators::ITALIC) => {
                CharacterFormat {
                    font_style: Some(if current.font_style == Some(FontStyle::Italic) {
                        FontStyle::Normal
                    } else {
                        FontStyle::Italic
                    }),
                    ..CharacterFormat::default()
                }
            }
            Key::U if !disabled.contains(DisabledFormattingAccelerators::UNDERLINE) => {
                let underlined = current
                    .text_decorations
                    .as_ref()
                    .and_then(Option::as_ref)
                    .is_some_and(|decorations| {
                        (0..decorations.len()).any(|index| {
                            decorations.get(index).location() == TextDecorationLocation::Underline
                        })
                    });
                let mut decorations = current
                    .text_decorations
                    .as_ref()
                    .and_then(Option::as_ref)
                    .map_or_else(Vec::new, |items| {
                        (0..items.len())
                            .map(|index| items.get(index))
                            .filter(|item| item.location() != TextDecorationLocation::Underline)
                            .collect()
                    });
                if !underlined {
                    decorations.push(TextDecorations::underline().get(0));
                }
                CharacterFormat {
                    text_decorations: Some((!decorations.is_empty()).then(|| {
                        ferroui_base::media::TextDecorationCollection::from_items(decorations)
                    })),
                    ..CharacterFormat::default()
                }
            }
            _ => return false,
        };
        self.handle_edit(self.apply_format(&format));
        true
    }

    pub fn set_preedit_text(
        &self,
        text: Option<&str>,
        cursor: Option<usize>,
    ) -> Result<(), EditError> {
        let Some(text) = text else {
            self.cancel_composition()?;
            return Ok(());
        };
        let editor = self.editor();
        self.synchronize_editor_selection(&editor)?;
        let started = !editor.is_composing();
        if started {
            self.raise_composition(Self::text_composition_started_event(), text, false);
        }
        let result = editor.set_preedit(&self.filter_input(text), cursor);
        self.after_edit(&editor);
        if result.is_ok() {
            self.raise_composition(Self::text_composition_changed_event(), text, false);
        }
        result
    }
    pub fn cancel_composition(&self) -> Result<bool, EditError> {
        let editor = self.editor();
        let result = editor.cancel_composition();
        self.after_edit(&editor);
        if matches!(result, Ok(true)) {
            self.raise_composition(Self::text_composition_ended_event(), "", false);
        }
        result
    }
    pub fn commit_composition(&self, text: &str) -> Result<bool, EditError> {
        let editor = self.editor();
        let result = editor.commit_composition(&self.filter_input(text));
        self.after_edit(&editor);
        if result.is_ok() {
            self.raise_composition(Self::text_composition_ended_event(), text, true);
        }
        result
    }
    fn raise_composition(
        &self,
        event: &RoutedEvent<RichEditCompositionEventArgs>,
        text: &str,
        committed: bool,
    ) {
        self.raise_event(&RichEditCompositionEventArgs {
            base: RoutedEventArgs::with_event(event),
            text: text.to_owned(),
            selection: self.selection(),
            committed,
        });
    }

    pub fn export_rtf(&self) -> Result<String, RtfError> {
        RtfWriter::write(&self.document().snapshot())
    }
    pub fn import_rtf(&self, bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let document = RtfReader::read(bytes)?;
        self.document().restore(&document)?;
        self.editor().clear_history();
        self.select(0, 0);
        self.update_command_states();
        Ok(())
    }

    pub fn copy(&self) {
        self.start_copy(false);
    }
    pub fn cut(&self) {
        self.start_copy(true);
    }
    fn start_copy(&self, cut: bool) {
        if !self.can_copy() || cut && !self.can_cut() {
            return;
        }
        if let Some(clipboard) = TopLevel::get_top_level(Some(self)).and_then(|top| top.clipboard())
        {
            let operation = self.copy_to_clipboard_async(clipboard, cut);
            self.start_clipboard_operation(operation);
        }
    }
    pub fn paste(&self) {
        if !self.can_paste() {
            return;
        }
        if let Some(clipboard) = TopLevel::get_top_level(Some(self)).and_then(|top| top.clipboard())
        {
            let operation = self.paste_from_clipboard_async(clipboard, false);
            self.start_clipboard_operation(operation);
        }
    }
    fn start_clipboard_operation(&self, operation: LocalBoxFuture<Result<(), ClipboardError>>) {
        let weak = self.to_ref().downgrade();
        ClipboardHelper::start(async move {
            if let Err(error) = operation.await {
                if let Some(this) = weak.upgrade() {
                    for (_, callback) in this.clipboard_errors.snapshot().iter() {
                        callback(&error);
                    }
                }
            }
            Ok(())
        });
    }
    pub fn copy_to_clipboard_async(
        &self,
        clipboard: Rc<dyn IClipboard>,
        cut: bool,
    ) -> LocalBoxFuture<Result<(), ClipboardError>> {
        let this = self.to_ref();
        let document = self.document();
        let revision = document.revision();
        let selection = self.selection();
        let allowed = !selection.is_empty() && (!cut || !self.is_read_only());
        let event = RoutedEventArgs::with_event(if cut {
            Self::cutting_to_clipboard_event()
        } else {
            Self::copying_to_clipboard_event()
        });
        if allowed {
            self.raise_event(&event);
        }
        let format = self.clipboard_copy_format();
        let fragment = if allowed && !event.handled() {
            Some(
                document
                    .slice(selection.range())
                    .map_err(|error| ClipboardError::other(error.to_string())),
            )
        } else {
            None
        };
        Box::pin(async move {
            if !allowed || event.handled() {
                return Ok(());
            }
            let Some(fragment) = fragment else {
                return Ok(());
            };
            let fragment = fragment?;
            clipboard
                .set_data_async(Some(RichTextTransfer::create(&fragment, format)))
                .await?;
            // The async clipboard may complete after typing or moving the selection. In that
            // case it has still copied the requested fragment, but must not delete new content.
            if cut
                && this.document() == document
                && document.revision() == revision
                && this.selection() == selection
                && !this.is_read_only()
            {
                let editor = this.editor();
                this.synchronize_editor_selection(&editor)
                    .map_err(|error| ClipboardError::other(error.to_string()))?;
                editor
                    .delete_selection()
                    .map_err(|error| ClipboardError::other(error.to_string()))?;
                this.after_edit(&editor);
            }
            Ok(())
        })
    }
    pub fn paste_from_clipboard_async(
        &self,
        clipboard: Rc<dyn IClipboard>,
        plain_text_only: bool,
    ) -> LocalBoxFuture<Result<(), ClipboardError>> {
        let this = self.to_ref();
        let document = self.document();
        let revision = document.revision();
        let selection = self.selection();
        let event = RoutedEventArgs::with_event(Self::pasting_from_clipboard_event());
        if self.can_paste() {
            self.raise_event(&event);
        }
        let allowed = self.can_paste() && !event.handled();
        Box::pin(async move {
            if !allowed {
                return Ok(());
            }
            let Some(data) = clipboard.try_get_data_async().await? else {
                return Ok(());
            };
            let result = RichTextTransfer::read(data.as_ref(), plain_text_only).await;
            data.dispose();
            let Some(fragment) = result? else {
                return Ok(());
            };
            if this.document() != document
                || document.revision() != revision
                || this.selection() != selection
                || this.is_read_only()
            {
                return Err(ClipboardError::from_kind(ClipboardErrorKind::Canceled));
            }
            let fragment = this.filter_fragment(&fragment);
            this.replace_selection(&fragment)
                .map_err(|error| ClipboardError::other(error.to_string()))?;
            Ok(())
        })
    }

    pub(crate) fn handle_edit<T>(&self, result: Result<T, EditError>) {
        if let Err(error) = result {
            for (_, callback) in self.errors.snapshot().iter() {
                callback(&error);
            }
        }
    }
    pub fn edit_failed(&self, handler: impl Fn(&EditError) + 'static) -> Rc<dyn IDisposable> {
        let token = self.errors.add(Rc::new(handler));
        let events = self.errors.clone();
        Disposable::create(move || {
            events.remove(token);
        })
    }
    pub fn clipboard_failed(
        &self,
        handler: impl Fn(&ClipboardError) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.clipboard_errors.add(Rc::new(handler));
        let events = self.clipboard_errors.clone();
        Disposable::create(move || {
            events.remove(token);
        })
    }
    pub fn text_input_method_client(&self) -> Rc<dyn TextInputMethodClient> {
        self.im_client.clone()
    }

    ferro_routed_event!(
        pub fn text_changed_event() -> RoutedEvent<TextChangedEventArgs> {
            RoutedEvent::register::<RichEditBox, _>("TextChanged", RoutingStrategies::BUBBLE)
        }
    );
    ferro_routed_event!(
        pub fn text_changing_event() -> RoutedEvent<TextChangingEventArgs> {
            RoutedEvent::register::<RichEditBox, _>("TextChanging", RoutingStrategies::BUBBLE)
        }
    );
    ferro_routed_event!(
        pub fn copying_to_clipboard_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<RichEditBox, _>("CopyingToClipboard", RoutingStrategies::BUBBLE)
        }
    );
    ferro_routed_event!(
        pub fn cutting_to_clipboard_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<RichEditBox, _>("CuttingToClipboard", RoutingStrategies::BUBBLE)
        }
    );
    ferro_routed_event!(
        pub fn pasting_from_clipboard_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<RichEditBox, _>("Paste", RoutingStrategies::BUBBLE)
        }
    );
    ferro_routed_event!(
        pub fn text_composition_started_event() -> RoutedEvent<RichEditCompositionEventArgs> {
            RoutedEvent::register::<RichEditBox, _>(
                "TextCompositionStarted",
                RoutingStrategies::BUBBLE,
            )
        }
    );
    ferro_routed_event!(
        pub fn text_composition_changed_event() -> RoutedEvent<RichEditCompositionEventArgs> {
            RoutedEvent::register::<RichEditBox, _>(
                "TextCompositionChanged",
                RoutingStrategies::BUBBLE,
            )
        }
    );
    ferro_routed_event!(
        pub fn text_composition_ended_event() -> RoutedEvent<RichEditCompositionEventArgs> {
            RoutedEvent::register::<RichEditBox, _>(
                "TextCompositionEnded",
                RoutingStrategies::BUBBLE,
            )
        }
    );
    pub fn text_changed(
        &self,
        handler: impl Fn(&Interactive, &TextChangedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::text_changed_event(), handler)
    }
}
