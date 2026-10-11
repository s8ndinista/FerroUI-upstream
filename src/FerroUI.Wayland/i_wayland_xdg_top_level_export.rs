//! UI-thread façade over a worker-thread `zxdg_exported_v2` handle (the port
//! of `IWaylandXdgTopLevelExport.cs`). Restricts the UI side to the only two
//! operations it should care about: awaiting the compositor-issued handle
//! string and disposing the export. The underlying worker object is
//! intentionally not exposed.

use crate::server::cross_thread_result::CrossThreadResult;
use ferroui_base::reactive::IDisposable;

pub trait IWaylandXdgTopLevelExport: IDisposable {
    /// Resolves to the raw compositor-issued handle string (no protocol prefix), or
    /// `None` if the wayland connection died before the handle was delivered.
    ///
    /// The reference has a task that may be awaited any number of times; the result of the
    /// port is taken by its one reader, and a second call gives a result that is `None`.
    fn handle_task(&self) -> CrossThreadResult<String>;
}
