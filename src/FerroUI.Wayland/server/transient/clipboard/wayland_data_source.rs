//! A source of data of this client (the port of `WaylandDataSource.cs`).
//!
//! The listener class of the reference (`WaylandDataSourceListener`) is the
//! `Dispatch` implementation of this file. The sources of a connection are
//! kept by its globals (`WaylandGlobals::data_sources`) by the identity of
//! their protocol object, which is where an event finds its source; the
//! reference keeps a source alive through its listener. A callback receives
//! the state of the worker, because what the reference captures in its
//! closures (the data device, the globals) is reached through it here.

use crate::server::wayland_worker::WaylandWorkerState;
use std::os::fd::OwnedFd;
use wayland_client::backend::ObjectId;
use wayland_client::protocol::wl_data_device_manager::DndAction;
use wayland_client::protocol::wl_data_source::{self, WlDataSource};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};

use super::wayland_data_offer::actions_of;

/// A callback of a source, run on the Wayland thread.
pub type DataSourceCallback<A> = Box<dyn FnMut(&mut WaylandWorkerState, A)>;

/// Wraps a `wl_data_source` created by us (for clipboard set or DnD start).
/// Handles `send` (compositor requests data) and `cancelled` (we lost ownership) events.
/// Lives on the Wayland thread.
pub struct WaylandDataSource {
    source: WlDataSource,
    destroyed: bool,
    /// Called on the Wayland thread when the compositor requests data for a MIME type.
    /// The callback receives (mimeType, fd) and must write data and close the fd.
    pub on_send: Option<DataSourceCallback<(String, OwnedFd)>>,
    /// Called on the Wayland thread when this source has been cancelled
    /// (another client took clipboard ownership, or DnD was cancelled).
    pub on_cancelled: Option<DataSourceCallback<()>>,
    /// Called when DnD drop has been performed by the user.
    pub on_dnd_drop_performed: Option<DataSourceCallback<()>>,
    /// Called when the DnD operation has been completed by the destination.
    pub on_dnd_finished: Option<DataSourceCallback<()>>,
    /// Called when the compositor negotiates a DnD action.
    pub on_action: Option<DataSourceCallback<DndAction>>,
}

impl WaylandDataSource {
    pub fn new(source: WlDataSource) -> Self {
        Self {
            source,
            destroyed: false,
            on_send: None,
            on_cancelled: None,
            on_dnd_drop_performed: None,
            on_dnd_finished: None,
            on_action: None,
        }
    }

    /// Advertises a MIME type that this source can provide.
    /// Must be called before `set_selection` or `start_drag`.
    pub fn offer(&self, mime_type: &str) {
        self.source.offer(mime_type.to_string());
    }

    /// Sets the DnD actions supported by this source.
    pub fn set_actions(&self, actions: DndAction) {
        self.source.set_actions(actions);
    }

    pub fn wl_source(&self) -> &WlDataSource {
        &self.source
    }

    pub fn id(&self) -> ObjectId {
        self.source.id()
    }

    pub fn dispose(&mut self) {
        if !self.destroyed {
            self.destroyed = true;
            self.source.destroy();
        }
    }
}

/// Runs a callback of the source `id` with the state of the worker. The callback is taken
/// out of the source for the call, and put back when the source is still there afterwards
/// and has not been given another one.
fn invoke<A>(
    state: &mut WaylandWorkerState,
    id: &ObjectId,
    slot: fn(&mut WaylandDataSource) -> &mut Option<DataSourceCallback<A>>,
    argument: A,
) {
    let callback = state.globals.as_mut().and_then(|globals| globals.data_sources.get_mut(id)).and_then(|source| slot(source).take());
    let Some(mut callback) = callback else {
        return;
    };
    callback(state, argument);
    if let Some(source) = state.globals.as_mut().and_then(|globals| globals.data_sources.get_mut(id)) {
        let slot = slot(source);
        if slot.is_none() {
            *slot = Some(callback);
        }
    }
}

impl Dispatch<WlDataSource, ()> for WaylandWorkerState {
    fn event(
        state: &mut Self,
        proxy: &WlDataSource,
        event: wl_data_source::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        let id = proxy.id();
        match event {
            wl_data_source::Event::Send { mime_type, fd } => invoke(state, &id, |source| &mut source.on_send, (mime_type, fd)),
            wl_data_source::Event::Cancelled => invoke(state, &id, |source| &mut source.on_cancelled, ()),
            wl_data_source::Event::Target { .. } => {
                // DnD target notification — not needed for clipboard
            }
            wl_data_source::Event::DndDropPerformed => invoke(state, &id, |source| &mut source.on_dnd_drop_performed, ()),
            wl_data_source::Event::DndFinished => invoke(state, &id, |source| &mut source.on_dnd_finished, ()),
            wl_data_source::Event::Action { dnd_action } => {
                invoke(state, &id, |source| &mut source.on_action, actions_of(dnd_action));
            }
            _ => {}
        }
    }
}
