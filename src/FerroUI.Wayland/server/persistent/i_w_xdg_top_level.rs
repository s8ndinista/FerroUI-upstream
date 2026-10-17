//! The commands of a shell surface and of a top-level (the port of
//! `IWXdgTopLevel.cs`) and the proxies the UI thread holds
//! (`WXdgShellSurfaceProxy`, `WXdgTopLevelProxy`, `WXdgPopupProxy`, which the
//! reference generates).
//!
//! `export_toplevel` is the one member with a result: the generated proxy of
//! the reference wraps it into a task; here the call is posted out of band and
//! its result is awaited (`CrossThreadResult`), and the handle of the UI thread
//! is made from what the worker gave.

use super::i_w_surface::WSurfaceProxy;
use super::i_w_surface_event_sink::PlatformInputEventCookie;
use super::w_surface::WSurfaceId;
use super::xdg_popup_positioner_params::XdgPopupPositionerParams;
use crate::i_wayland_xdg_top_level_export::IWaylandXdgTopLevelExport;
use crate::server::cross_thread_result::cross_thread_result;
use crate::server::transient::xdg_toplevel_export::UiThreadHandle;
use ferroui_base::input::LocalBoxFuture;
use std::rc::Rc;
use crate::server::wayland_dispatch_priority::WaylandDispatchPriority;
use crate::server::wayland_worker::WorkerMarshaller;
use ferroui_base::{Size, Thickness};
use std::ops::Deref;
use wayland_protocols::xdg::shell::client::xdg_toplevel::ResizeEdge;

/// Worker-side XDG shell surface commands posted from UI thread.
pub trait IWXdgShellSurface {
    /// Sets shadow extents (CSD shadows) so the worker can compute
    /// `xdg_surface.set_window_geometry` excluding shadow margins.
    fn set_shadow_extents(&self, extents: Thickness);
    fn set_pending_ack_serial(&self, serial: u32);
}

/// Worker-side XDG toplevel commands posted from UI thread.
pub trait IWXdgTopLevel: IWXdgShellSurface {
    fn set_maximized(&self);
    fn unset_maximized(&self);
    fn set_fullscreen(&self);
    fn unset_fullscreen(&self);
    fn set_minimized(&self);
    fn set_parent(&self, parent: Option<&WXdgTopLevelProxy>);
    fn move_(&self, platform_cookie: Option<PlatformInputEventCookie>, priority: WaylandDispatchPriority);
    fn resize(&self, platform_cookie: Option<PlatformInputEventCookie>, edge: ResizeEdge, priority: WaylandDispatchPriority);

    /// Sets the toplevel's minimum and maximum size constraints atomically.
    /// Either argument may be `None` to mean "no constraint on this side".
    /// Values are in surface-local logical pixels and exclude any shadow
    /// extents. Double-buffered state: cached and re-applied to the next commit.
    fn set_min_max_size(&self, min_size: Option<Size>, max_size: Option<Size>);

    /// Sets the toplevel's title (xdg_toplevel.set_title). The compositor uses it for
    /// the server-side titlebar, taskbar, and alt-tab. `None` is treated as empty.
    /// Cached on the worker so it is re-applied on reconnect.
    fn set_title(&self, title: Option<&str>);

    /// Sets the toplevel's application ID (xdg_toplevel.set_app_id). The compositor
    /// may use this to group windows, match desktop files, display window icons, etc.
    /// A `None` or empty value clears the app ID. The worker caches the value so it
    /// survives compositor reconnects.
    fn set_app_id(&self, app_id: Option<&str>);

    /// Destroys the decoration object and disables its creation completely.
    fn destroy_decoration(&self);

    /// Creates an xdg-foreign-v2 export of this toplevel on the worker; the returned
    /// façade's handle task completes asynchronously when the compositor delivers the
    /// handle event. Resolves to `None` if the exporter global isn't bound or the toplevel
    /// isn't currently connected.
    fn export_toplevel(&self) -> LocalBoxFuture<Option<Rc<dyn IWaylandXdgTopLevelExport>>>;
}

/// A shell surface of the worker as the UI thread holds it.
#[derive(Clone)]
pub struct WXdgShellSurfaceProxy {
    surface: WSurfaceProxy,
}

impl WXdgShellSurfaceProxy {
    pub fn new(id: WSurfaceId, marshaller: WorkerMarshaller) -> Self {
        Self { surface: WSurfaceProxy::new(id, marshaller) }
    }
}

impl Deref for WXdgShellSurfaceProxy {
    type Target = WSurfaceProxy;

    fn deref(&self) -> &WSurfaceProxy {
        &self.surface
    }
}

impl IWXdgShellSurface for WXdgShellSurfaceProxy {
    fn set_shadow_extents(&self, extents: Thickness) {
        self.post(move |worker, id| {
            if let Some(shell) = worker.state.shell_mut(id) {
                shell.set_shadow_extents(extents);
            }
        });
    }

    fn set_pending_ack_serial(&self, serial: u32) {
        self.post(move |worker, id| {
            if let Some(shell) = worker.state.shell_mut(id) {
                shell.set_pending_ack_serial(serial);
            }
        });
    }
}

/// A top-level of the worker as the UI thread holds it.
#[derive(Clone)]
pub struct WXdgTopLevelProxy {
    shell: WXdgShellSurfaceProxy,
}

impl WXdgTopLevelProxy {
    pub fn new(id: WSurfaceId, marshaller: WorkerMarshaller) -> Self {
        Self { shell: WXdgShellSurfaceProxy::new(id, marshaller) }
    }

    /// The proxy as the proxy of its shell surface (the base interface).
    pub fn as_shell_surface(&self) -> &WXdgShellSurfaceProxy {
        &self.shell
    }
}

impl Deref for WXdgTopLevelProxy {
    type Target = WXdgShellSurfaceProxy;

    fn deref(&self) -> &WXdgShellSurfaceProxy {
        &self.shell
    }
}

impl IWXdgShellSurface for WXdgTopLevelProxy {
    fn set_shadow_extents(&self, extents: Thickness) {
        self.shell.set_shadow_extents(extents);
    }

    fn set_pending_ack_serial(&self, serial: u32) {
        self.shell.set_pending_ack_serial(serial);
    }
}

impl IWXdgTopLevel for WXdgTopLevelProxy {
    fn set_maximized(&self) {
        self.post(|worker, id| {
            if let Some(top_level) = worker.state.top_levels.get(&id) {
                top_level.set_maximized();
            }
        });
    }

    fn unset_maximized(&self) {
        self.post(|worker, id| {
            if let Some(top_level) = worker.state.top_levels.get(&id) {
                top_level.unset_maximized();
            }
        });
    }

    fn set_fullscreen(&self) {
        self.post(|worker, id| {
            if let Some(top_level) = worker.state.top_levels.get(&id) {
                top_level.set_fullscreen();
            }
        });
    }

    fn unset_fullscreen(&self) {
        self.post(|worker, id| {
            if let Some(top_level) = worker.state.top_levels.get(&id) {
                top_level.unset_fullscreen();
            }
        });
    }

    fn set_minimized(&self) {
        self.post(|worker, id| {
            if let Some(top_level) = worker.state.top_levels.get(&id) {
                top_level.set_minimized();
            }
        });
    }

    fn set_parent(&self, parent: Option<&WXdgTopLevelProxy>) {
        let parent = parent.map(|parent| parent.id());
        self.post(move |worker, id| {
            let parent = parent
                .and_then(|parent| worker.state.top_levels.get(&parent))
                .and_then(|parent| parent.xdg_top_level().cloned());
            if let Some(top_level) = worker.state.top_levels.get(&id) {
                top_level.set_parent(parent.as_ref());
            }
        });
    }

    fn move_(&self, platform_cookie: Option<PlatformInputEventCookie>, priority: WaylandDispatchPriority) {
        self.post_with_priority(
            move |worker, id| {
                if let Some(top_level) = worker.state.top_levels.get(&id) {
                    top_level.move_(platform_cookie.as_ref());
                }
            },
            priority,
        );
    }

    fn resize(&self, platform_cookie: Option<PlatformInputEventCookie>, edge: ResizeEdge, priority: WaylandDispatchPriority) {
        self.post_with_priority(
            move |worker, id| {
                if let Some(top_level) = worker.state.top_levels.get(&id) {
                    top_level.resize(platform_cookie.as_ref(), edge);
                }
            },
            priority,
        );
    }

    fn set_min_max_size(&self, min_size: Option<Size>, max_size: Option<Size>) {
        self.post(move |worker, id| {
            if let Some(top_level) = worker.state.top_levels.get_mut(&id) {
                top_level.set_min_max_size(min_size, max_size);
            }
        });
    }

    fn set_title(&self, title: Option<&str>) {
        let title = title.map(str::to_string);
        self.post(move |worker, id| {
            if let Some(top_level) = worker.state.top_levels.get_mut(&id) {
                top_level.set_title(title);
            }
        });
    }

    fn set_app_id(&self, app_id: Option<&str>) {
        let app_id = app_id.map(str::to_string);
        self.post(move |worker, id| {
            let default_app_id = worker.state.globals.as_ref().and_then(|globals| globals.app_id.clone());
            if let Some(top_level) = worker.state.top_levels.get_mut(&id) {
                top_level.set_app_id(app_id, default_app_id);
            }
        });
    }

    fn destroy_decoration(&self) {
        self.post(|worker, id| {
            if let Some(top_level) = worker.state.top_levels.get_mut(&id) {
                top_level.destroy_decoration();
            }
        });
    }

    fn export_toplevel(&self) -> LocalBoxFuture<Option<Rc<dyn IWaylandXdgTopLevelExport>>> {
        let (completion, result) = cross_thread_result();
        let owner = self.id();
        self.post_with_priority(
            move |worker, id| {
                let globals = worker.state.globals.as_ref();
                let exported = worker.state.top_levels.get_mut(&id).and_then(|top_level| top_level.export_toplevel(globals));
                let worker_handle = worker.state.worker.clone();
                completion.complete(exported.map(|exported| (worker_handle, exported)));
            },
            WaylandDispatchPriority::Oob,
        );
        Box::pin(async move {
            let (worker, exported) = result.await.flatten()?;
            let handle: Rc<dyn IWaylandXdgTopLevelExport> = Rc::new(UiThreadHandle::new(worker, owner, exported));
            Some(handle)
        })
    }
}

/// Worker-side xdg_popup commands posted from the UI thread. The worker
/// caches the most recent positioner parameters and rebuilds
/// the protocol-level `xdg_positioner` from these on every connect.
pub trait IWXdgPopup: IWXdgShellSurface {
    /// Updates the cached positioner parameters used to (re-)create the
    /// xdg_popup on connect. Called from the UI thread before the popup is
    /// shown for the first time, and when reposition is invoked.
    fn update_positioner(&self, positioner: XdgPopupPositionerParams);
}

/// A popup of the worker as the UI thread holds it.
#[derive(Clone)]
pub struct WXdgPopupProxy {
    shell: WXdgShellSurfaceProxy,
}

impl WXdgPopupProxy {
    pub fn new(id: WSurfaceId, marshaller: WorkerMarshaller) -> Self {
        Self { shell: WXdgShellSurfaceProxy::new(id, marshaller) }
    }

    /// The proxy as the proxy of its shell surface (the base interface).
    pub fn as_shell_surface(&self) -> &WXdgShellSurfaceProxy {
        &self.shell
    }
}

impl Deref for WXdgPopupProxy {
    type Target = WXdgShellSurfaceProxy;

    fn deref(&self) -> &WXdgShellSurfaceProxy {
        &self.shell
    }
}

impl IWXdgShellSurface for WXdgPopupProxy {
    fn set_shadow_extents(&self, extents: Thickness) {
        self.shell.set_shadow_extents(extents);
    }

    fn set_pending_ack_serial(&self, serial: u32) {
        self.shell.set_pending_ack_serial(serial);
    }
}

impl IWXdgPopup for WXdgPopupProxy {
    fn update_positioner(&self, positioner: XdgPopupPositionerParams) {
        self.post(move |worker, id| worker.state.update_popup_positioner(id, positioner));
    }
}
