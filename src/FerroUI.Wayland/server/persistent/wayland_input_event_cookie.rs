//! The cookie of an input event (the port of `WaylandInputEventCookie.cs`).

use crate::server::wayland_worker::{WaylandWorker, WaylandWorkerThread};
use std::sync::{Arc, Mutex, PoisonError};
use wayland_client::protocol::wl_seat::WlSeat;

/// Opaque platform-specific cookie attached to raw input events.
/// Carries the Wayland seat and serial needed for requests like xdg_toplevel.move/resize.
/// Must NOT be accessed from the UI thread — only passed back to the worker.
///
/// The cookie remembers the connection it was created on by its number, so that stale
/// cookies from a previous connection are rejected at consume time.
///
/// Also captures the worker from the moment the input event was triggered, providing
/// [`post_oob`](Self::post_oob) for reconnect-safe access to transient state (e.g. initiating
/// DnD from a pointer press).
pub struct WaylandInputEventCookie {
    seat: Mutex<Option<WlSeat>>,
    serial: u32,
    connection_id: u64,
    worker: Arc<WaylandWorker>,
}

impl WaylandInputEventCookie {
    pub fn new(seat: WlSeat, serial: u32, connection_id: u64, worker: Arc<WaylandWorker>) -> Self {
        Self { seat: Mutex::new(Some(seat)), serial, connection_id, worker }
    }

    /// Posts a callback to the wayland thread that runs with the globals of the connection
    /// the cookie was made on. If a compositor reconnect occurred since then (i.e. the
    /// current globals are those of another connection), the callback is not invoked.
    pub fn post_oob(&self, callback: impl FnOnce(&mut WaylandWorkerThread) + Send + 'static) {
        let connection_id = self.connection_id;
        self.worker.post_oob(Box::new(move |worker| {
            if worker.state.globals.as_ref().is_some_and(|globals| globals.connection_id == connection_id) {
                callback(worker);
            }
        }));
    }

    /// Consumes the cookie, returning the seat and serial. Can only be consumed once.
    /// Returns `None` if already consumed or if the cookie belongs to a different
    /// connection (e.g. after a reconnect).
    pub fn try_consume(&self, current_connection_id: u64) -> Option<(WlSeat, u32)> {
        let seat = self.seat.lock().unwrap_or_else(PoisonError::into_inner).take()?;
        if self.connection_id == current_connection_id {
            Some((seat, self.serial))
        } else {
            None
        }
    }
}
