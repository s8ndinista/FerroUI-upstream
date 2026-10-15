//! The delegates the integration hands to WinUI and to the system: a COM
//! object of the delegate's interface over a Rust closure. The reference
//! subscribes C# methods and lambdas, which its runtime wraps the same way.
//!
//! The system calls a delegate through its own frames, which a panic must
//! not cross: a closure runs under the guard of the Windows backend
//! (`ferroui_win32::wnd_proc_guard`), which keeps a panic until the message
//! loop has returned (`xaml_application.rs`).
//!
//! A closure that the object of the event holds must not hold that object
//! (the reference's garbage collector breaks such a cycle; counted
//! references do not): the closures of the integration hold weak references
//! to the objects of the integration.

use super::*;
use ferroui_microcom::{make_com, make_com_with, ComPtr, HResult, InterfaceEntry};
use ferroui_win32::wnd_proc_guard::guard;

macro_rules! delegate {
    ($(#[$doc:meta])* $name:ident, $interface:ident, $implementation:ident, ($($arg:ident : $ty:ty),*)) => {
        $(#[$doc])*
        pub fn $name(handler: impl Fn($($ty),*) + 'static) -> ComPtr<$interface> {
            struct Delegate<F>(F);

            impl<F: Fn($($ty),*) + 'static> $implementation for Delegate<F> {
                fn invoke(&self, $($arg: $ty),*) -> Result<(), HResult> {
                    guard((), || (self.0)($($arg),*));
                    Ok(())
                }
            }

            make_com(Delegate(handler))
        }
    };
}

delegate!(
    /// `RoutedEventHandler`: `Loaded`, `Unloaded`, `GotFocus`, `LostFocus`.
    routed_event_handler, RoutedEventHandler, RoutedEventHandlerImpl,
    (sender: Option<&IInspectable>, e: Option<&IRoutedEventArgs>)
);

delegate!(
    /// `SizeChangedEventHandler`.
    size_changed_event_handler, SizeChangedEventHandler, SizeChangedEventHandlerImpl,
    (sender: Option<&IInspectable>, e: Option<&ISizeChangedEventArgs>)
);

delegate!(
    /// `PointerEventHandler`.
    pointer_event_handler, PointerEventHandler, PointerEventHandlerImpl,
    (sender: Option<&IInspectable>, e: Option<&IPointerRoutedEventArgs>)
);

delegate!(
    /// `KeyEventHandler`.
    key_event_handler, KeyEventHandler, KeyEventHandlerImpl,
    (sender: Option<&IInspectable>, e: Option<&IKeyRoutedEventArgs>)
);

delegate!(
    /// `DragEventHandler`.
    drag_event_handler, DragEventHandler, DragEventHandlerImpl,
    (sender: Option<&IInspectable>, e: Option<&IDragEventArgs>)
);

delegate!(
    /// `TypedEventHandler<SwapChainPanel, Object>`: `CompositionScaleChanged`.
    composition_scale_changed_handler, TypedEventHandlerSwapChainPanelObject, TypedEventHandlerSwapChainPanelObjectImpl,
    (sender: Option<&ISwapChainPanel>, args: Option<&IInspectable>)
);

delegate!(
    /// `TypedEventHandler<UIElement, CharacterReceivedRoutedEventArgs>`.
    character_received_handler, TypedEventHandlerUIElementCharacterReceived, TypedEventHandlerUIElementCharacterReceivedImpl,
    (sender: Option<&IUIElement>, args: Option<&ICharacterReceivedRoutedEventArgs>)
);

delegate!(
    /// `TypedEventHandler<UIElement, DragStartingEventArgs>`.
    drag_starting_handler, TypedEventHandlerUIElementDragStarting, TypedEventHandlerUIElementDragStartingImpl,
    (sender: Option<&IUIElement>, args: Option<&IDragStartingEventArgs>)
);

delegate!(
    /// `TypedEventHandler<FrameworkElement, Object>`: `FrameworkElement.Loading`.
    loading_handler, TypedEventHandlerFrameworkElementObject, TypedEventHandlerFrameworkElementObjectImpl,
    (sender: Option<&IFrameworkElement>, args: Option<&IInspectable>)
);

delegate!(
    /// `RangeBaseValueChangedEventHandler`: `RangeBase.ValueChanged` (a slider).
    range_base_value_changed_handler, RangeBaseValueChangedEventHandler, RangeBaseValueChangedEventHandlerImpl,
    (sender: Option<&IInspectable>, e: Option<&IRangeBaseValueChangedEventArgs>)
);

delegate!(
    /// `SelectionChangedEventHandler`: `Selector.SelectionChanged` (a combo box).
    selection_changed_handler, SelectionChangedEventHandler, SelectionChangedEventHandlerImpl,
    (sender: Option<&IInspectable>, e: Option<&ISelectionChangedEventArgs>)
);

delegate!(
    /// `TypedEventHandler<Object, WindowEventArgs>`: `Window.Closed`.
    window_closed_handler, TypedEventHandlerObjectWindowEventArgs, TypedEventHandlerObjectWindowEventArgsImpl,
    (sender: Option<&IInspectable>, args: Option<&IWindowEventArgs>)
);

delegate!(
    /// `ApplicationInitializationCallback`: what `Application.Start` calls
    /// on the thread of the application.
    application_initialization_callback, ApplicationInitializationCallback, ApplicationInitializationCallbackImpl,
    (p: Option<&IInspectable>)
);

delegate!(
    /// `CoreTextEditContext.TextRequested`.
    text_requested_handler, TypedEventHandlerTextRequested, TypedEventHandlerTextRequestedImpl,
    (sender: Option<&ICoreTextEditContext>, args: Option<&ICoreTextTextRequestedEventArgs>)
);

delegate!(
    /// `CoreTextEditContext.SelectionRequested`.
    selection_requested_handler, TypedEventHandlerSelectionRequested, TypedEventHandlerSelectionRequestedImpl,
    (sender: Option<&ICoreTextEditContext>, args: Option<&ICoreTextSelectionRequestedEventArgs>)
);

delegate!(
    /// `CoreTextEditContext.LayoutRequested`.
    layout_requested_handler, TypedEventHandlerLayoutRequested, TypedEventHandlerLayoutRequestedImpl,
    (sender: Option<&ICoreTextEditContext>, args: Option<&ICoreTextLayoutRequestedEventArgs>)
);

delegate!(
    /// `CoreTextEditContext.TextUpdating`.
    text_updating_handler, TypedEventHandlerTextUpdating, TypedEventHandlerTextUpdatingImpl,
    (sender: Option<&ICoreTextEditContext>, args: Option<&ICoreTextTextUpdatingEventArgs>)
);

delegate!(
    /// `CoreTextEditContext.SelectionUpdating`.
    selection_updating_handler, TypedEventHandlerSelectionUpdating, TypedEventHandlerSelectionUpdatingImpl,
    (sender: Option<&ICoreTextEditContext>, args: Option<&ICoreTextSelectionUpdatingEventArgs>)
);

delegate!(
    /// `CoreTextEditContext.FormatUpdating`.
    format_updating_handler, TypedEventHandlerFormatUpdating, TypedEventHandlerFormatUpdatingImpl,
    (sender: Option<&ICoreTextEditContext>, args: Option<&ICoreTextFormatUpdatingEventArgs>)
);

delegate!(
    /// `CoreTextEditContext.CompositionStarted`.
    composition_started_handler, TypedEventHandlerCompositionStarted, TypedEventHandlerCompositionStartedImpl,
    (sender: Option<&ICoreTextEditContext>, args: Option<&ICoreTextCompositionStartedEventArgs>)
);

delegate!(
    /// `CoreTextEditContext.CompositionCompleted`.
    composition_completed_handler, TypedEventHandlerCompositionCompleted, TypedEventHandlerCompositionCompletedImpl,
    (sender: Option<&ICoreTextEditContext>, args: Option<&ICoreTextCompositionCompletedEventArgs>)
);

delegate!(
    /// `CoreTextEditContext.FocusRemoved`.
    focus_removed_handler, TypedEventHandlerFocusRemoved, TypedEventHandlerFocusRemovedImpl,
    (sender: Option<&ICoreTextEditContext>, args: Option<&IInspectable>)
);

/// A delegate as an `object`, which `UIElement.AddHandler` takes: an
/// `IReference<KeyEventHandler>` whose value is the delegate (the box
/// C#/WinRT and C++/WinRT make of a delegate passed as an object). A query
/// of the box for the delegate's own interface is answered by the delegate.
pub(crate) fn box_key_event_handler(handler: ComPtr<KeyEventHandler>) -> Result<ComPtr<IInspectable>, HResult> {
    struct Box(ComPtr<KeyEventHandler>);

    inspectable!(Box, "Windows.Foundation.IReference`1<Microsoft.UI.Xaml.Input.KeyEventHandler>", [IReferenceKeyEventHandler]);

    impl IReferenceKeyEventHandlerImpl for Box {
        fn value(&self) -> Result<Option<ComPtr<KeyEventHandler>>, HResult> {
            Ok(Some(self.0.clone()))
        }
    }

    let boxed: ComPtr<IReferenceKeyEventHandler> = make_com(Box(handler.clone()));
    // SAFETY: an interface of the object just made with a `Box`; the
    // delegate's reference is the box's from now on.
    unsafe { ferroui_microcom::set_inner::<Box>(boxed.as_ptr().cast(), handler.into_raw().cast()) };
    boxed.cast::<IInspectable>()
}

/// As [`box_key_event_handler`], for the handler of `CharacterReceived`.
pub(crate) fn box_character_received_handler(
    handler: ComPtr<TypedEventHandlerUIElementCharacterReceived>,
) -> Result<ComPtr<IInspectable>, HResult> {
    struct Box(ComPtr<TypedEventHandlerUIElementCharacterReceived>);

    inspectable!(
        Box,
        "Windows.Foundation.IReference`1<Windows.Foundation.TypedEventHandler`2<Microsoft.UI.Xaml.UIElement, Microsoft.UI.Xaml.Input.CharacterReceivedRoutedEventArgs>>",
        [IReferenceCharacterReceivedHandler]
    );

    impl IReferenceCharacterReceivedHandlerImpl for Box {
        fn value(&self) -> Result<Option<ComPtr<TypedEventHandlerUIElementCharacterReceived>>, HResult> {
            Ok(Some(self.0.clone()))
        }
    }

    let boxed: ComPtr<IReferenceCharacterReceivedHandler> = make_com(Box(handler.clone()));
    // SAFETY: as above.
    unsafe { ferroui_microcom::set_inner::<Box>(boxed.as_ptr().cast(), handler.into_raw().cast()) };
    boxed.cast::<IInspectable>()
}

/// `DispatcherQueueHandler`: work given to the dispatcher queue of WinUI
/// from another thread. The object is agile (created on one thread, called
/// and released on the thread of the queue), so the work is `Send`.
pub(crate) fn dispatcher_queue_handler(work: impl FnOnce() + Send + 'static) -> ComPtr<DispatcherQueueHandler> {
    struct Work(std::sync::Mutex<Option<Box<dyn FnOnce() + Send>>>);

    impl DispatcherQueueHandlerImpl for Work {
        fn invoke(&self) -> Result<(), HResult> {
            let work = self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner).take();
            if let Some(work) = work {
                guard((), work);
            }
            Ok(())
        }
    }

    impl IAgileObjectImpl for Work {}

    make_com_with(Work(std::sync::Mutex::new(Some(Box::new(work)))), [InterfaceEntry::of::<IAgileObject>()])
}
