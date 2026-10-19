use crate::documents::{
    CharacterFormat, DocumentSnapshot, InlineSnapshot, InlineSnapshotKind, ParagraphFormat,
    TextElement,
};
use crate::presenters::ContentPresenter;
use crate::rich_edit_box_text_input_method_client::RichEditBoxTextInputMethodClient;
use crate::rich_text::{
    EditError, EditKind, EditorSelection, RichEditClipboardFormat, RichEditTextDocument,
    RichTextEditor, RichTextTransfer, RtfError, RtfReader, RtfWriter,
};
use crate::rich_text_block::RichTextSelectionChangedEventArgs;
use crate::templates::IDataTemplate;
use crate::utils::ClipboardHelper;
use crate::TextBlock;
use crate::{
    Application, ControlImpl, RichTextBlock, RichTextBlockImpl, RichTextBlockImplExt, TextBox,
    TopLevel,
};
use ferroui_base::animation::TimeSpan;
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
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::media::{
    DrawingContext, FontStyle, FontWeight, IBrush, TextDecorationLocation, TextDecorations,
    TextWrapping,
};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_routed_event, ferro_routed_event_args, instantiate,
    BoxedValue, DirectProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Rect, Ref, Size, StyledElementImpl, StyledProperty,
    StyledPropertyOptions, Thickness, Visual, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs,
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
        SelectionStart: i32 { get: |args: &RichEditCompositionEventArgs| args.selection.anchor.min(i32::MAX as usize) as i32 },
        SelectionEnd: i32 { get: |args: &RichEditCompositionEventArgs| args.selection.active.min(i32::MAX as usize) as i32 },
        Committed: bool { get: |args: &RichEditCompositionEventArgs| args.committed },
        Handled: bool { get: |args: &RichEditCompositionEventArgs| args.handled(), set: |args: &RichEditCompositionEventArgs, value: bool| args.set_handled(value) },
    ],
});

/// Metadata carrier for the closed composition-event routed-event type.
#[doc(hidden)]
pub struct RichEditCompositionRoutedEvent;
ferroui_base::ferro_markup_type!(class RichEditCompositionRoutedEvent as "RoutedEvent`1" {
    namespace: "FerroUI.Interactivity",
    handles: [RoutedEvent<RichEditCompositionEventArgs>, Option<RoutedEvent<RichEditCompositionEventArgs>>],
    base: RoutedEvent,
    generic: "RoutedEvent`1" [RichEditCompositionEventArgs],
});

/// The proposed UTF-16 selection before its change is published.
///
/// A handler may cancel the proposal or select a different range. An explicitly
/// selected replacement wins over cancellation, as in the WinUI control contract.
#[derive(Clone, Default)]
pub struct RichEditBoxSelectionChangingEventArgs {
    base: RoutedEventArgs,
    selection_start: usize,
    selection_length: usize,
    cancel: Rc<Cell<bool>>,
}
ferro_routed_event_args!(RichEditBoxSelectionChangingEventArgs: RoutedEventArgs);

impl RichEditBoxSelectionChangingEventArgs {
    pub fn selection_start(&self) -> usize {
        self.selection_start
    }
    pub fn selection_length(&self) -> usize {
        self.selection_length
    }
    pub fn cancel(&self) -> bool {
        self.cancel.get()
    }
    pub fn set_cancel(&self, value: bool) {
        self.cancel.set(value);
    }
}
ferroui_base::ferro_markup_type!(class RichEditBoxSelectionChangingEventArgs {
    namespace: "FerroUI.Controls",
    handles: [RichEditBoxSelectionChangingEventArgs],
    properties: [
        SelectionStart: i32 { get: |args: &RichEditBoxSelectionChangingEventArgs| args.selection_start().min(i32::MAX as usize) as i32 },
        SelectionLength: i32 { get: |args: &RichEditBoxSelectionChangingEventArgs| args.selection_length().min(i32::MAX as usize) as i32 },
        Cancel: bool { get: RichEditBoxSelectionChangingEventArgs::cancel, set: RichEditBoxSelectionChangingEventArgs::set_cancel },
    ],
});

/// Rich text content-change information; formatting alone leaves this flag false.
#[derive(Clone, Default)]
pub struct RichEditBoxTextChangingEventArgs {
    base: RoutedEventArgs,
    is_content_changing: bool,
}
ferro_routed_event_args!(RichEditBoxTextChangingEventArgs: RoutedEventArgs);

impl RichEditBoxTextChangingEventArgs {
    pub fn is_content_changing(&self) -> bool {
        self.is_content_changing
    }
}
ferroui_base::ferro_markup_type!(class RichEditBoxTextChangingEventArgs {
    namespace: "FerroUI.Controls",
    handles: [RichEditBoxTextChangingEventArgs],
    properties: [
        IsContentChanging: bool { get: RichEditBoxTextChangingEventArgs::is_content_changing },
    ],
});

#[doc(hidden)]
pub struct RichEditBoxSelectionChangingRoutedEvent;
ferroui_base::ferro_markup_type!(class RichEditBoxSelectionChangingRoutedEvent as "RoutedEvent`1" {
    namespace: "FerroUI.Interactivity",
    handles: [RoutedEvent<RichEditBoxSelectionChangingEventArgs>, Option<RoutedEvent<RichEditBoxSelectionChangingEventArgs>>],
    base: RoutedEvent,
    generic: "RoutedEvent`1" [RichEditBoxSelectionChangingEventArgs],
});

#[doc(hidden)]
pub struct RichEditBoxTextChangingRoutedEvent;
ferroui_base::ferro_markup_type!(class RichEditBoxTextChangingRoutedEvent as "RoutedEvent`1" {
    namespace: "FerroUI.Interactivity",
    handles: [RoutedEvent<RichEditBoxTextChangingEventArgs>, Option<RoutedEvent<RichEditBoxTextChangingEventArgs>>],
    base: RoutedEvent,
    generic: "RoutedEvent`1" [RichEditBoxTextChangingEventArgs],
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
    projecting_selection: Cell<bool>,
    last_input_selection: Cell<EditorSelection>,
    last_text: RefCell<String>,
    document_generation: Cell<u64>,
    header_presenter: RefCell<Option<Ref<ContentPresenter>>>,
    description_presenter: RefCell<Option<Ref<ContentPresenter>>>,
    placeholder_presenter: RefCell<Option<Ref<TextBlock>>>,
    header_height: Cell<f64>,
    description_height: Cell<f64>,
    can_undo: Cell<bool>,
    can_redo: Cell<bool>,
    can_copy: Cell<bool>,
    can_cut: Cell<bool>,
    can_paste: Cell<bool>,
    errors: Rc<HandlerList<dyn Fn(&EditError)>>,
    clipboard_errors: Rc<HandlerList<dyn Fn(&ClipboardError)>>,
    caret_scroll_pending: Cell<bool>,
    caret_timer: RefCell<Option<Rc<DispatcherTimer>>>,
    caret_phase: Cell<bool>,
}

ferro_class!(RichEditBox: RichTextBlock);

impl Drop for RichEditBox {
    fn drop(&mut self) {
        if let Some(timer) = self.caret_timer.get_mut().take() {
            timer.stop();
        }
    }
}
ferroui_base::ferro_class_info!(RichEditBox {
    new: RichEditBox::new,
    markup: {
        fields: [
            CopyingToClipboardEvent: RoutedEvent<RoutedEventArgs> => || *RichEditBox::copying_to_clipboard_event(),
            CuttingToClipboardEvent: RoutedEvent<RoutedEventArgs> => || *RichEditBox::cutting_to_clipboard_event(),
            PastingFromClipboardEvent: RoutedEvent<RoutedEventArgs> => || *RichEditBox::pasting_from_clipboard_event(),
            TextChangedEvent: RoutedEvent<RoutedEventArgs> => || *RichEditBox::text_changed_event(),
            TextChangingEvent: RoutedEvent<RichEditBoxTextChangingEventArgs> => || *RichEditBox::text_changing_event(),
            SelectionChangingEvent: RoutedEvent<RichEditBoxSelectionChangingEventArgs> => || *RichEditBox::selection_changing_event(),
            TextCompositionStartedEvent: RoutedEvent<RichEditCompositionEventArgs> => || *RichEditBox::text_composition_started_event(),
            TextCompositionChangedEvent: RoutedEvent<RichEditCompositionEventArgs> => || *RichEditBox::text_composition_changed_event(),
            TextCompositionEndedEvent: RoutedEvent<RichEditCompositionEventArgs> => || *RichEditBox::text_composition_ended_event(),
        ],
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
ferro_impl_classes!(RichEditBox: StyledElementImpl, InteractiveImpl);

impl RichTextBlockImpl for RichEditBox {
    fn request_selection_change(this: &Self, start: usize, end: usize) -> bool {
        if this.projecting_selection.get() {
            return false;
        }
        let editor = this.editor();
        let selection = EditorSelection::new(start, end);
        if editor.is_in_operation() || editor.selection() == selection {
            return false;
        }
        this.handle_edit(editor.set_selection(selection));
        true
    }

    fn on_selection_changing(this: &Self, args: &RichTextSelectionChangedEventArgs) -> bool {
        let args = RichEditBoxSelectionChangingEventArgs {
            base: RoutedEventArgs::with_event(Self::selection_changing_event()),
            selection_start: args.start.min(args.end),
            selection_length: args.start.abs_diff(args.end),
            cancel: Rc::new(Cell::new(false)),
        };
        this.raise_event(&args);
        !args.cancel()
    }

    fn rebase_selection_on_document_change(this: &Self) -> bool {
        let editor = this.editor.borrow().clone();
        !editor.is_some_and(|editor| editor.is_in_operation())
    }

    fn text_insets(this: &Self) -> Thickness {
        let padding = this.padding();
        Thickness::new(
            padding.left,
            padding.top + this.header_height.get(),
            padding.right,
            padding.bottom + this.description_height.get(),
        )
    }

    fn text_background_bounds(this: &Self) -> Rect {
        Rect::new(
            0.0,
            this.header_height.get(),
            this.bounds().width,
            (this.bounds().height - this.header_height.get() - this.description_height.get())
                .max(0.0),
        )
    }

    fn additional_visual_children(this: &Self) -> Vec<Ref<Visual>> {
        let mut children = Vec::new();
        if let Some(header) = this.header_presenter() {
            children.push(header.upcast());
        }
        if let Some(description) = this.description_presenter() {
            children.push(description.upcast());
        }
        if let Some(placeholder) = this.placeholder_presenter() {
            children.push(placeholder.upcast());
        }
        children
    }

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
        this.caret_is_active() && this.caret_phase.get()
    }

    fn render_text(this: &Self, context: &mut DrawingContext) {
        Self::parent_render_text(this, context);
    }
}

impl LayoutableImpl for RichEditBox {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let auxiliary = Size::new(available_size.width, f64::INFINITY);
        let header = this.header_presenter();
        let description = this.description_presenter();
        for presenter in [&header, &description].into_iter().flatten() {
            presenter.measure(auxiliary);
        }
        let header_size = header.map_or(Size::default(), |header| header.desired_size());
        let description_size =
            description.map_or(Size::default(), |description| description.desired_size());
        this.header_height.set(header_size.height);
        this.description_height.set(description_size.height);
        let mut desired = Self::parent_measure_override(this, available_size);
        desired.width = desired
            .width
            .max(header_size.width)
            .max(description_size.width);
        if let Some(placeholder) = this.placeholder_presenter() {
            placeholder.measure(available_size.deflate(this.text_insets()));
            if placeholder.is_visible() {
                let placeholder_size = placeholder.desired_size().inflate(this.text_insets());
                desired.width = desired.width.max(placeholder_size.width);
                desired.height = desired.height.max(placeholder_size.height);
            }
        }
        desired
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let result = Self::parent_arrange_override(this, final_size);
        if let Some(header) = this.header_presenter() {
            header.arrange(Rect::new(
                0.0,
                0.0,
                final_size.width,
                this.header_height.get(),
            ));
        }
        if let Some(description) = this.description_presenter() {
            description.arrange(Rect::new(
                0.0,
                (final_size.height - this.description_height.get()).max(this.header_height.get()),
                final_size.width,
                this.description_height.get(),
            ));
        }
        if let Some(placeholder) = this.placeholder_presenter() {
            let insets = this.text_insets();
            let inner = final_size.deflate(insets);
            placeholder.arrange(Rect::new(
                insets.left,
                insets.top,
                inner.width,
                inner.height,
            ));
        }
        this.im_client.raise_cursor_rectangle_changed();
        result
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
        this.update_presenters();
        let weak = this.to_ref().downgrade();
        let selection_subscription = this.selection_changed(move |_| {
            if let Some(this) = weak.upgrade() {
                // Document notifications can rebase the view while an editor transaction
                // is publishing. The editor emits its final selection when that finishes.
                let _ = this.editor().set_selection(this.selection());
                this.notify_input_method_selection();
                this.im_client.raise_cursor_rectangle_changed();
                this.update_command_states();
                this.reset_caret_blink();
                this.ensure_caret_visible();
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
                this.handle_edit(this.cancel_composition());
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
            || property == Self::header_property().as_property()
            || property == Self::header_template_property().as_property()
            || property == Self::description_property().as_property()
            || property == RichTextBlock::text_alignment_property().as_property()
            || property == RichTextBlock::text_wrapping_property().as_property()
        {
            this.update_presenters();
            this.invalidate_measure();
        }
        if property == Self::caret_blink_interval_property().as_property()
            || property == Self::is_read_only_property().as_property()
            || property == InputElement::is_effectively_enabled_property().as_property()
            || property == Visual::is_visible_property().as_property()
        {
            this.reset_caret_blink();
        }
        if property == InputElement::is_effectively_enabled_property().as_property() {
            this.update_command_states();
        }
    }
}

impl VisualImpl for RichEditBox {
    fn on_attached_to_visual_tree(this: &Self, event: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, event);
        this.reset_caret_blink();
        this.ensure_caret_visible();
    }

    fn on_detached_from_visual_tree(this: &Self, event: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, event);
        this.stop_caret_blink();
    }
}

impl InputElementImpl for RichEditBox {
    fn on_got_focus(this: &Self, event: &FocusChangedEventArgs) {
        Self::parent_on_got_focus(this, event);
        this.im_client.raise_text_view_visual_changed();
        this.im_client.raise_cursor_rectangle_changed();
        this.reset_caret_blink();
        this.ensure_caret_visible();
        this.invalidate_visual();
    }
    fn on_lost_focus(this: &Self, event: &FocusChangedEventArgs) {
        Self::parent_on_lost_focus(this, event);
        this.handle_edit(this.cancel_composition());
        this.editor().checkpoint();
        this.im_client.request_reset();
        this.stop_caret_blink();
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
    pub fn caret_blink_interval_property() -> StyledProperty<TimeSpan> { TextBox::caret_blink_interval_property().add_owner::<RichEditBox>() }
    pub fn max_length_property() -> StyledProperty<i32> { FerroProperty::register_with::<RichEditBox, _>("MaxLength", StyledPropertyOptions::new(0).validate(|value| *value >= 0)) }
    pub fn clipboard_copy_format_property() -> StyledProperty<RichEditClipboardFormat> { FerroProperty::register::<RichEditBox, _>("ClipboardCopyFormat", RichEditClipboardFormat::AllFormats) }
    pub fn disabled_formatting_accelerators_property() -> StyledProperty<DisabledFormattingAccelerators> { FerroProperty::register::<RichEditBox, _>("DisabledFormattingAccelerators", DisabledFormattingAccelerators::NONE) }
    pub fn character_casing_property() -> StyledProperty<RichEditCharacterCasing> { FerroProperty::register::<RichEditBox, _>("CharacterCasing", RichEditCharacterCasing::Normal) }
    pub fn is_text_prediction_enabled_property() -> StyledProperty<bool> { FerroProperty::register::<RichEditBox, _>("IsTextPredictionEnabled", true) }
    pub fn input_scope_property() -> StyledProperty<TextInputContentType> { FerroProperty::register::<RichEditBox, _>("InputScope", TextInputContentType::Normal) }
    pub fn placeholder_text_property() -> StyledProperty<String> { FerroProperty::register::<RichEditBox, _>("PlaceholderText", String::new()) }
    pub fn placeholder_foreground_property() -> StyledProperty<Option<Rc<dyn IBrush>>> { FerroProperty::register::<RichEditBox, _>("PlaceholderForeground", None) }
    pub fn header_property() -> StyledProperty<Option<BoxedValue>> { FerroProperty::register::<RichEditBox, _>("Header", None) }
    pub fn header_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> { FerroProperty::register::<RichEditBox, _>("HeaderTemplate", None) }
    pub fn description_property() -> StyledProperty<Option<BoxedValue>> { FerroProperty::register::<RichEditBox, _>("Description", None) }
    pub fn is_undo_enabled_property() -> StyledProperty<bool> { FerroProperty::register::<RichEditBox, _>("IsUndoEnabled", true) }
    pub fn undo_limit_property() -> StyledProperty<i32> { FerroProperty::register_with::<RichEditBox, _>("UndoLimit", StyledPropertyOptions::new(100).validate(|value| *value >= 0)) }
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
            projecting_selection: Cell::new(false),
            last_input_selection: Cell::new(EditorSelection::default()),
            last_text: RefCell::new(String::new()),
            document_generation: Cell::new(0),
            header_presenter: RefCell::new(None),
            description_presenter: RefCell::new(None),
            placeholder_presenter: RefCell::new(None),
            header_height: Cell::new(0.0),
            description_height: Cell::new(0.0),
            can_undo: Cell::new(false),
            can_redo: Cell::new(false),
            can_copy: Cell::new(false),
            can_cut: Cell::new(false),
            can_paste: Cell::new(true),
            errors: Rc::new(HandlerList::new()),
            clipboard_errors: Rc::new(HandlerList::new()),
            caret_scroll_pending: Cell::new(false),
            caret_timer: RefCell::new(None),
            caret_phase: Cell::new(false),
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
    /// The framework caret period. Zero or a negative interval keeps the caret steady.
    pub fn caret_blink_interval(&self) -> TimeSpan {
        self.get_value(Self::caret_blink_interval_property())
    }
    pub fn set_caret_blink_interval(&self, value: TimeSpan) {
        self.set_value(Self::caret_blink_interval_property(), value);
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
        self.get_value(Self::max_length_property()) as usize
    }
    pub fn set_max_length(&self, value: usize) {
        self.set_value(
            Self::max_length_property(),
            value.min(i32::MAX as usize) as i32,
        );
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
    pub fn header(&self) -> Option<BoxedValue> {
        self.get_value(Self::header_property())
    }
    pub fn set_header(&self, value: Option<BoxedValue>) {
        self.set_value(Self::header_property(), value);
    }
    pub fn header_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::header_template_property())
    }
    pub fn set_header_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::header_template_property(), value);
    }
    pub fn description(&self) -> Option<BoxedValue> {
        self.get_value(Self::description_property())
    }
    pub fn set_description(&self, value: Option<BoxedValue>) {
        self.set_value(Self::description_property(), value);
    }
    /// The framework presenter for header data or a HeaderTemplate.
    pub fn header_presenter(&self) -> Option<Ref<ContentPresenter>> {
        self.header_presenter.borrow().clone()
    }
    pub fn description_presenter(&self) -> Option<Ref<ContentPresenter>> {
        self.description_presenter.borrow().clone()
    }
    pub fn placeholder_presenter(&self) -> Option<Ref<TextBlock>> {
        self.placeholder_presenter.borrow().clone()
    }

    fn update_presenters(&self) {
        let header = self.header();
        let header_template = self.header_template();
        let show_header = header.is_some() || header_template.is_some();
        let mut presenter = self.header_presenter();
        if presenter.is_none() && show_header {
            let created = ContentPresenter::new();
            created.set_name(Some("PART_HeaderPresenter".to_owned()));
            // RichEditBoxTopHeaderMargin in the pinned WinUI template.
            created.set_margin(Thickness::new(0.0, 0.0, 0.0, 8.0));
            created.set_text_wrapping(TextWrapping::Wrap);
            created.set_font_weight(FontWeight::Normal);
            *self.header_presenter.borrow_mut() = Some(created.clone());
            self.logical_children().add(created.clone().upcast());
            presenter = Some(created);
        }
        if let Some(presenter) = presenter {
            let changed =
                presenter.content() != header || presenter.content_template() != header_template;
            presenter.set_content(header);
            presenter.set_content_template(header_template);
            presenter.set_is_visible(show_header);
            if changed && !presenter.is_attached_to_logical_tree() {
                presenter.update_child();
            }
        }
        let description = self.description();
        let show_description = description.is_some();
        let mut presenter = self.description_presenter();
        if presenter.is_none() && show_description {
            let created = ContentPresenter::new();
            created.set_name(Some("PART_DescriptionPresenter".to_owned()));
            created.set_text_wrapping(TextWrapping::Wrap);
            *self.description_presenter.borrow_mut() = Some(created.clone());
            self.logical_children().add(created.clone().upcast());
            presenter = Some(created);
        }
        if let Some(presenter) = presenter {
            let changed = presenter.content() != description;
            presenter.set_content(description);
            presenter.set_is_visible(show_description);
            if changed && !presenter.is_attached_to_logical_tree() {
                presenter.update_child();
            }
        }
        let text = self.placeholder_text();
        let mut presenter = self.placeholder_presenter();
        if presenter.is_none() && !text.is_empty() {
            let created = TextBlock::new();
            created.set_name(Some("PART_PlaceholderTextPresenter".to_owned()));
            created.set_is_hit_test_visible(false);
            *self.placeholder_presenter.borrow_mut() = Some(created.clone());
            self.logical_children().add(created.clone().upcast());
            presenter = Some(created);
        }
        if let Some(presenter) = presenter {
            presenter.set_text(Some(&text));
            presenter.set_text_alignment(self.text_alignment());
            presenter.set_text_wrapping(self.text_wrapping());
            if let Some(foreground) = self.placeholder_foreground() {
                presenter.set_foreground(Some(foreground));
            } else {
                presenter.clear_value(TextElement::foreground_property());
            }
        }
        self.pseudo_classes().set(":header", show_header);
        self.pseudo_classes().set(":description", show_description);
        self.update_placeholder_visibility();
    }

    fn update_placeholder_visibility(&self) {
        let empty = self.document().text_length() == 0;
        self.pseudo_classes().set(":empty", empty);
        self.pseudo_classes()
            .set(":placeholder", empty && !self.placeholder_text().is_empty());
        if let Some(placeholder) = self.placeholder_presenter() {
            placeholder.set_is_visible(empty && !self.placeholder_text().is_empty());
        }
    }
    pub fn is_undo_enabled(&self) -> bool {
        self.get_value(Self::is_undo_enabled_property())
    }
    pub fn set_is_undo_enabled(&self, value: bool) {
        self.set_value(Self::is_undo_enabled_property(), value);
    }
    pub fn undo_limit(&self) -> usize {
        self.get_value(Self::undo_limit_property()) as usize
    }
    pub fn set_undo_limit(&self, value: usize) {
        self.set_value(
            Self::undo_limit_property(),
            value.min(i32::MAX as usize) as i32,
        );
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
        !self.is_read_only() && self.is_effectively_enabled()
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
        if let Some(editor) = existing {
            if editor.document() == self.document() {
                return editor;
            }
            editor.set_selection_projection(None);
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
        let weak = self.to_ref().downgrade();
        editor.set_selection_projection(Some(Rc::new(move |selection| {
            if let Some(this) = weak.upgrade() {
                struct Projection<'a>(&'a Cell<bool>, bool);
                impl Drop for Projection<'_> {
                    fn drop(&mut self) {
                        self.0.set(self.1);
                    }
                }
                let guard = Projection(
                    &this.projecting_selection,
                    this.projecting_selection.replace(true),
                );
                this.select(selection.anchor, selection.active);
                let accepted = this.selection();
                drop(guard);
                accepted
            } else {
                selection
            }
        })));
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
        self.document_generation
            .set(self.document_generation.get().wrapping_add(1));
        let old = self.document_subscription.borrow_mut().take();
        if let Some(old) = old {
            old.dispose();
        }
        let old_editor = self.editor.borrow_mut().take();
        if let Some(editor) = &old_editor {
            editor.set_selection_projection(None);
        }
        drop(old_editor);
        let weak = self.to_ref().downgrade();
        let subscription = self.document().changed(move |_| {
            if let Some(this) = weak.upgrade() {
                this.on_document_changed();
            }
        });
        *self.document_subscription.borrow_mut() = Some(subscription);
        if self.document_generation.get() > 1 {
            self.on_document_changed();
        } else {
            *self.last_text.borrow_mut() = self.text();
            self.update_command_states();
        }
    }

    fn on_document_changed(&self) {
        let text = self.text();
        let previous = self.last_text.replace(text.clone());
        if previous != text {
            self.raise_direct_property_changed(Self::text_property(), &previous, &text);
        }
        // CRichEditBox::OnContentChanged publishes TextChanging synchronously
        // after the backing store is updated. TextChanged is a deferred routed
        // event, allowing normal UI changes after the text has been rendered.
        let generation = self.document_generation.get();
        self.raise_event(&RichEditBoxTextChangingEventArgs {
            base: RoutedEventArgs::with_event(Self::text_changing_event()),
            is_content_changing: previous != text,
        });
        let weak = self.to_ref().downgrade();
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(this) = weak.upgrade() {
                    if this.document_generation.get() == generation {
                        this.raise_event(&RoutedEventArgs::with_event(Self::text_changed_event()));
                    }
                }
            },
            DispatcherPriority::LOADED,
        );
        self.im_client.raise_surrounding_text_changed();
        self.im_client.raise_cursor_rectangle_changed();
        self.update_placeholder_visibility();
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
        self.notify_input_method_selection();
        self.im_client.raise_cursor_rectangle_changed();
        self.update_command_states();
        self.reset_caret_blink();
        self.invalidate_visual();
        self.ensure_caret_visible();
    }

    fn notify_input_method_selection(&self) {
        let selection = self.editor().input_method_selection();
        if self.last_input_selection.replace(selection) != selection {
            self.im_client.raise_selection_changed();
        }
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
                DispatcherPriority::LOADED,
            );
        }
    }
    fn caret_is_active(&self) -> bool {
        self.is_focused()
            && self.is_effectively_enabled()
            && self.is_effectively_visible()
            && !self.is_read_only()
            && self.selection_start() == self.selection_end()
            && self.visual_parent().is_some()
    }

    fn stop_caret_blink(&self) {
        let timer = self.caret_timer.borrow().clone();
        if let Some(timer) = timer {
            timer.stop();
        }
        if self.caret_phase.replace(false) {
            self.invalidate_visual();
        }
    }

    fn reset_caret_blink(&self) {
        let timer = self.caret_timer.borrow().clone();
        if let Some(timer) = &timer {
            timer.stop();
        }
        let active = self.caret_is_active();
        self.caret_phase.set(active);
        self.invalidate_visual();
        let interval = self.caret_blink_interval();
        if !active || interval <= TimeSpan::ZERO || self.editor().is_composing() {
            return;
        }
        let timer = timer.unwrap_or_else(|| {
            let timer = DispatcherTimer::with_priority(DispatcherPriority::BACKGROUND);
            let weak = self.to_ref().downgrade();
            timer.tick(move |timer| {
                let Some(this) = weak.upgrade() else {
                    timer.stop();
                    return;
                };
                if this.caret_is_active() {
                    this.caret_phase.set(!this.caret_phase.get());
                    this.invalidate_visual();
                } else {
                    this.caret_phase.set(false);
                    timer.stop();
                }
            });
            *self.caret_timer.borrow_mut() = Some(timer.clone());
            timer
        });
        // The dispatcher uses signed millisecond due times. Keep submillisecond
        // values from spinning the UI queue and bound unusually long intervals.
        let duration = interval.to_duration().unwrap_or_default();
        timer.set_interval(duration.clamp(
            std::time::Duration::from_millis(1),
            std::time::Duration::from_millis(i32::MAX as u64),
        ));
        timer.start();
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
        if composing && result.is_ok() {
            self.raise_composition(Self::text_composition_ended_event(), &text, true);
        }
        result
    }
    pub fn replace_selection(&self, fragment: &DocumentSnapshot) -> Result<bool, EditError> {
        let editor = self.editor();
        self.synchronize_editor_selection(&editor)?;
        editor.insert_fragment(fragment, EditKind::Paste)
    }
    pub fn delete_backward(&self, whole_word: bool) -> Result<bool, EditError> {
        let editor = self.editor();
        self.synchronize_editor_selection(&editor)?;
        editor.delete_backward(whole_word)
    }
    pub fn delete_forward(&self, whole_word: bool) -> Result<bool, EditError> {
        let editor = self.editor();
        self.synchronize_editor_selection(&editor)?;
        editor.delete_forward(whole_word)
    }
    pub fn apply_format(&self, format: &CharacterFormat) -> Result<(), EditError> {
        let editor = self.editor();
        self.synchronize_editor_selection(&editor)?;
        editor.apply_format(format)
    }
    pub fn apply_paragraph_format(&self, format: &ParagraphFormat) -> Result<(), EditError> {
        let editor = self.editor();
        self.synchronize_editor_selection(&editor)?;
        editor.apply_paragraph_format(format)
    }
    pub fn undo(&self) -> Result<bool, EditError> {
        let editor = self.editor();
        editor.undo()
    }
    pub fn redo(&self) -> Result<bool, EditError> {
        let editor = self.editor();
        editor.redo()
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
        if editor.is_read_only() {
            return Err(EditError::ReadOnly);
        }
        if editor.is_in_operation() {
            return Err(EditError::ReentrantEdit);
        }
        self.synchronize_editor_selection(&editor)?;
        let started = !editor.is_composing();
        if started {
            self.raise_composition(Self::text_composition_started_event(), text, false);
        }
        let cursor = cursor.map(|offset| {
            self.filter_input(crate::rich_text::utf16_prefix(text, offset))
                .encode_utf16()
                .count()
        });
        let result = editor.set_preedit(&self.filter_input(text), cursor);
        if result.is_ok() {
            self.raise_composition(Self::text_composition_changed_event(), text, false);
        }
        result
    }
    pub fn cancel_composition(&self) -> Result<bool, EditError> {
        let editor = self.editor();
        let result = editor.cancel_composition();
        if matches!(result, Ok(true)) {
            self.raise_composition(Self::text_composition_ended_event(), "", false);
        }
        result
    }
    pub fn commit_composition(&self, text: &str) -> Result<bool, EditError> {
        let editor = self.editor();
        let composing = editor.is_composing();
        let result = editor.commit_composition(&self.filter_input(text));
        if composing && result.is_ok() {
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
        pub fn text_changed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<RichEditBox, _>("TextChanged", RoutingStrategies::BUBBLE)
        }
    );
    ferro_routed_event!(
        pub fn text_changing_event() -> RoutedEvent<RichEditBoxTextChangingEventArgs> {
            RoutedEvent::register::<RichEditBox, _>("TextChanging", RoutingStrategies::DIRECT)
        }
    );
    ferro_routed_event!(
        pub fn selection_changing_event() -> RoutedEvent<RichEditBoxSelectionChangingEventArgs> {
            RoutedEvent::register::<RichEditBox, _>("SelectionChanging", RoutingStrategies::DIRECT)
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
            RoutedEvent::register::<RichEditBox, _>(
                "PastingFromClipboard",
                RoutingStrategies::BUBBLE,
            )
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
        handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::text_changed_event(), handler)
    }
    pub fn text_changing(
        &self,
        handler: impl Fn(&Interactive, &RichEditBoxTextChangingEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::text_changing_event(), handler)
    }
    pub fn selection_changing(
        &self,
        handler: impl Fn(&Interactive, &RichEditBoxSelectionChangingEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::selection_changing_event(), handler)
    }
}
