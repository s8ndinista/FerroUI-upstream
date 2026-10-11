//! An offer of data of another client (the port of `WaylandDataOffer.cs`).
//!
//! The listener class of the reference (`WaylandDataOfferListener`) is the
//! `Dispatch` implementation of this file. An offer is created by an event
//! of the data device, so its user data cannot name the seat: the events of
//! an offer look for it among the offers of every data device.

use crate::server::interop::unsafe_native_methods::pipe2_cloexec;
use crate::server::wayland_worker::WaylandWorkerState;
use std::os::fd::{AsFd, OwnedFd};
use wayland_client::backend::ObjectId;
use wayland_client::protocol::wl_data_device_manager::DndAction;
use wayland_client::protocol::wl_data_offer::{self, WlDataOffer};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, WEnum};

/// Wraps a `wl_data_offer` received from the compositor.
/// Collects offered MIME types as they arrive and provides access to the offer's data.
/// Lives on the Wayland thread.
pub struct WaylandDataOffer {
    offer: WlDataOffer,
    mime_types: Vec<String>,
    /// Source-advertised DnD actions (set by `source_actions` event).
    pub source_actions: DndAction,
    /// Compositor-negotiated DnD action (set by `action` event).
    pub negotiated_action: DndAction,
}

/// Calls `wl_data_offer.receive(mimeType, fd)` using a pipe.
/// Returns the read-end file descriptor. The caller must close it after reading.
/// Returns `None` on failure.
pub(crate) fn receive(offer: &WlDataOffer, mime_type: &str) -> Option<OwnedFd> {
    let (read, write) = pipe2_cloexec().ok()?;
    offer.receive(mime_type.to_string(), write.as_fd());
    // The library copied the descriptor into the request: the writing end of this process
    // is closed, so that the read ends when the source closes its own.
    drop(write);
    Some(read)
}

impl WaylandDataOffer {
    pub fn new(offer: WlDataOffer) -> Self {
        Self { offer, mime_types: Vec::new(), source_actions: DndAction::None, negotiated_action: DndAction::None }
    }

    pub fn wl_offer(&self) -> &WlDataOffer {
        &self.offer
    }

    pub fn id(&self) -> ObjectId {
        self.offer.id()
    }

    pub fn mime_types(&self) -> &[String] {
        &self.mime_types
    }

    pub(crate) fn add_mime_type(&mut self, mime_type: String) {
        self.mime_types.push(mime_type);
    }

    /// Checks whether the offer contains the given MIME type.
    pub fn has_mime_type(&self, mime_type: &str) -> bool {
        self.mime_types.iter().any(|m| m == mime_type)
    }

    /// Calls `wl_data_offer.receive(mimeType, fd)` using a pipe; see [`receive`].
    pub fn receive(&self, mime_type: &str) -> Option<OwnedFd> {
        receive(&self.offer, mime_type)
    }

    /// Sets accepted DnD actions on this offer.
    pub fn set_actions(&self, dnd_actions: DndAction, preferred_action: DndAction) {
        self.offer.set_actions(dnd_actions, preferred_action);
    }

    /// Accepts the given MIME type for DnD.
    pub fn accept(&self, serial: u32, mime_type: Option<&str>) {
        self.offer.accept(serial, mime_type.map(str::to_string));
    }

    /// Signals that the DnD operation is complete.
    pub fn finish(&self) {
        self.offer.finish();
    }

    pub fn dispose(self) {
        self.offer.destroy();
    }
}

/// Listener for `wl_data_offer` events.
/// Collects MIME types and forwards DnD action changes.
impl Dispatch<WlDataOffer, ()> for WaylandWorkerState {
    fn event(
        state: &mut Self,
        proxy: &WlDataOffer,
        event: wl_data_offer::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        let Some(globals) = &mut state.globals else {
            return;
        };
        let Some(wrapper) = globals.input_dispatcher.find_data_offer_mut(&proxy.id()) else {
            return;
        };
        match event {
            wl_data_offer::Event::Offer { mime_type } => wrapper.add_mime_type(mime_type),
            wl_data_offer::Event::SourceActions { source_actions } => {
                wrapper.source_actions = actions_of(source_actions);
            }
            wl_data_offer::Event::Action { dnd_action } => {
                wrapper.negotiated_action = actions_of(dnd_action);
            }
            _ => {}
        }
    }
}

/// The actions of an event: bits this version of the protocol does not name are dropped.
pub(crate) fn actions_of(actions: WEnum<DndAction>) -> DndAction {
    match actions {
        WEnum::Value(actions) => actions,
        WEnum::Unknown(bits) => DndAction::from_bits_truncate(bits),
    }
}
