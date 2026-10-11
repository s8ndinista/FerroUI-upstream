//! The source of a drag that starts in a window of the application (the
//! port of `WaylandDragSource.cs`).

use super::wayland_outgoing_transfer::WaylandOutgoingTransfer;
use crate::server::persistent::wayland_input_event_cookie::WaylandInputEventCookie;
use crate::window_impl::worker_cookie;
use ferroui_base::input::platform::IPlatformDragSource;
use ferroui_base::input::{DragDropEffects, IDataTransfer, LocalBoxFuture, PointerPressedEventArgs};
use std::future::ready;
use std::rc::Rc;

/// Implements the platform drag source for the Wayland backend.
/// Delegates to [`WaylandOutgoingTransfer::set_as_drag`] to create a
/// `wl_data_source` and call `wl_data_device.start_drag`.
#[derive(Default)]
pub struct WaylandDragSource;

impl WaylandDragSource {
    pub fn new() -> Self {
        Self
    }
}

impl IPlatformDragSource for WaylandDragSource {
    fn do_drag_drop_async(
        &self,
        trigger_event: &PointerPressedEventArgs,
        data_transfer: Rc<dyn IDataTransfer>,
        allowed_effects: DragDropEffects,
    ) -> LocalBoxFuture<DragDropEffects> {
        trigger_event.pointer().capture(None);

        let Some(async_transfer) = data_transfer.clone().as_async_data_transfer() else {
            return Box::pin(ready(DragDropEffects::NONE));
        };

        let input_cookie = worker_cookie(trigger_event.platform_input_event_cookie())
            .and_then(|cookie| cookie.downcast::<WaylandInputEventCookie>().ok());
        let Some(input_cookie) = input_cookie else {
            return Box::pin(ready(DragDropEffects::NONE));
        };

        let outgoing = WaylandOutgoingTransfer::new(async_transfer);
        outgoing.set_as_drag(data_transfer, input_cookie, allowed_effects)
    }
}
