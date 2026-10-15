//! Port of the `Utility` directory of RemoteViewing: the helpers the server
//! session uses, and the reentrant lock that stands for the monitors of
//! .NET.

mod auto_clear;
mod periodic_thread;
mod reentrant_lock;

pub(crate) use auto_clear::AutoClear;
pub(crate) use periodic_thread::PeriodicThread;
pub use reentrant_lock::{ReentrantLock, ReentrantLockGuard};
