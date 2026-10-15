#[cfg(windows)]
pub(crate) use imp::WinUIDragSource;

#[cfg(windows)]
mod imp {
    use crate::ferro_swap_chain_panel::FerroSwapChainPanel;
    use ferroui_base::input::platform::IPlatformDragSource;
    use ferroui_base::input::{DragDropEffects, IDataTransfer, LocalBoxFuture, PointerPressedEventArgs};
    use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
    use ferroui_base::Visual;
    use ferroui_controls::TopLevel;
    use std::any::Any;
    use std::rc::Rc;

    /// The `IPlatformDragSource` implementation that initiates native WinUI
    /// drags (via `UIElement.StartDragAsync`) for content hosted inside a
    /// [`FerroSwapChainPanel`]. WinUI only permits starting a drag with a
    /// `PointerPoint` captured during an active pointer interaction; the
    /// panel caches the most recent pointer point (regardless of pointer
    /// id) and uses it to start the drag.
    pub(crate) struct WinUIDragSource;

    impl IPlatformDragSource for WinUIDragSource {
        fn do_drag_drop_async(
            &self,
            trigger_event: &PointerPressedEventArgs,
            data_transfer: Rc<dyn IDataTransfer>,
            allowed_effects: DragDropEffects,
        ) -> LocalBoxFuture<DragDropEffects> {
            // Resolve the panel that hosts the visual that started the drag.
            let source = trigger_event.source().and_then(|source| source.cast::<Visual>());
            let top_level = TopLevel::get_top_level(source.as_deref());
            let platform_impl = top_level.as_ref().and_then(|top_level| top_level.platform_impl());
            let panel = platform_impl.as_ref().and_then(FerroSwapChainPanel::get_panel_for);

            let Some(panel) = panel else {
                if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::WINUI_PLATFORM) {
                    logger.log(
                        Some(self as &dyn Any),
                        &format!(
                            "Could not resolve FerroSwapChainPanel for drag source. TopLevel={}, PlatformImpl={}.",
                            if top_level.is_some() { "TopLevel" } else { "null" },
                            if platform_impl.is_some() { "ITopLevelImpl" } else { "null" },
                        ),
                    );
                }
                return Box::pin(std::future::ready(DragDropEffects::NONE));
            };

            Box::pin(async move { panel.start_outgoing_drag_async(data_transfer, allowed_effects).await })
        }
    }
}
