//! Worker-thread handle to a single `zxdg_exported_v2` export of a wayland
//! toplevel (the port of `XdgToplevelExport.cs`), and the handle of it the UI
//! thread holds.
//!
//! The reference returns the worker's object through the cross-thread proxy,
//! wrapped in a small class whose task the worker completes when the
//! compositor delivers the handle event, and whose `Dispose` hops back to the
//! worker thread for the actual destroy. Here the worker's object stays in the
//! list of its top-level; what crosses is the number of the export with the
//! result of the handle ([`ExportedTopLevel`]), and the handle of the UI
//! thread ([`UiThreadHandle`]) names the export by the two numbers.

use crate::i_wayland_xdg_top_level_export::IWaylandXdgTopLevelExport;
use crate::server::cross_thread_result::{cross_thread_result, CrossThreadCompletion, CrossThreadResult};
use crate::server::persistent::w_surface::WSurfaceId;
use crate::server::wayland_worker::{WaylandWorker, WaylandWorkerState};
use ferroui_base::reactive::IDisposable;
use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols::xdg::foreign::zv2::client::zxdg_exported_v2::{self, ZxdgExportedV2};
use wayland_protocols::xdg::foreign::zv2::client::zxdg_exporter_v2::ZxdgExporterV2;

/// The number of an export: the user data of its protocol object.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct XdgToplevelExportId(u64);

impl XdgToplevelExportId {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

/// What the worker gives the UI thread of a new export.
pub struct ExportedTopLevel {
    pub id: XdgToplevelExportId,
    pub handle: CrossThreadResult<String>,
}

pub struct XdgToplevelExport {
    id: XdgToplevelExportId,
    /// Completed when the compositor delivers the handle event; dropped (which resolves
    /// the result to `None`) when the export is disposed before that.
    handle_completion: Option<CrossThreadCompletion<String>>,
    exported: Option<ZxdgExportedV2>,
    alive: bool,
}

impl XdgToplevelExport {
    /// Exports the top-level `owner`, whose surface `surface` is. The second value is what
    /// the UI thread gets.
    pub(crate) fn new(
        owner: WSurfaceId,
        exporter: &ZxdgExporterV2,
        surface: &WlSurface,
        queue_handle: &QueueHandle<WaylandWorkerState>,
    ) -> (Self, ExportedTopLevel) {
        let id = XdgToplevelExportId::new();
        let (completion, handle) = cross_thread_result();
        let exported = exporter.export_toplevel(surface, queue_handle, (owner, id));
        (Self { id, handle_completion: Some(completion), exported: Some(exported), alive: true }, ExportedTopLevel { id, handle })
    }

    pub fn id(&self) -> XdgToplevelExportId {
        self.id
    }

    /// Destroys the export. The top-level removes it from its list (`RemoveExport`).
    pub fn dispose(&mut self) {
        if !self.alive {
            return;
        }
        self.alive = false;
        if let Some(exported) = self.exported.take() {
            exported.destroy();
        }
        // Unblock any awaiter that's still waiting on a handle event we'll never get.
        self.handle_completion = None;
    }

    pub(crate) fn on_handle(&mut self, handle: String) {
        if !self.alive {
            return;
        }
        if let Some(completion) = self.handle_completion.take() {
            completion.complete(handle);
        }
    }
}

/// The export as the UI thread holds it.
pub struct UiThreadHandle {
    worker: Arc<WaylandWorker>,
    owner: WSurfaceId,
    id: XdgToplevelExportId,
    handle: RefCell<Option<CrossThreadResult<String>>>,
}

impl UiThreadHandle {
    pub(crate) fn new(worker: Arc<WaylandWorker>, owner: WSurfaceId, exported: ExportedTopLevel) -> Self {
        Self { worker, owner, id: exported.id, handle: RefCell::new(Some(exported.handle)) }
    }
}

impl IWaylandXdgTopLevelExport for UiThreadHandle {
    fn handle_task(&self) -> CrossThreadResult<String> {
        self.handle.borrow_mut().take().unwrap_or_else(|| CrossThreadResult::ready(None))
    }
}

impl IDisposable for UiThreadHandle {
    fn dispose(&self) {
        let (owner, id) = (self.owner, self.id);
        self.worker.post_oob(Box::new(move |worker| {
            if let Some(top_level) = worker.state.top_levels.get_mut(&owner) {
                top_level.dispose_export(id);
            }
        }));
    }
}

impl Dispatch<ZxdgExporterV2, ()> for WaylandWorkerState {
    fn event(_: &mut Self, _: &ZxdgExporterV2, _: <ZxdgExporterV2 as Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
    }
}

/// The listener of an export (`ExportListener` of the reference): the user data is the
/// number of the top-level and the number of the export.
impl Dispatch<ZxdgExportedV2, (WSurfaceId, XdgToplevelExportId)> for WaylandWorkerState {
    fn event(
        state: &mut Self,
        _: &ZxdgExportedV2,
        event: zxdg_exported_v2::Event,
        (owner, id): &(WSurfaceId, XdgToplevelExportId),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zxdg_exported_v2::Event::Handle { handle } = event {
            if let Some(top_level) = state.top_levels.get_mut(owner) {
                top_level.on_export_handle(*id, handle);
            }
        }
    }
}
