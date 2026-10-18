//! ferroui-system
//!
//! The parts of the base class library of .NET that FerroUI and its reusable libraries share, in
//! Rust: thread-local variables that work on every target, events, change notification and the
//! cancellation of work. Nothing here depends on the user interface; the base
//! library of FerroUI re-exports every item at the path it had there.
//!
//! The modules follow the namespaces of .NET where an item ports one of its types:
//!
//! - [`component_model`]: `System.ComponentModel` (`INotifyPropertyChanged`).
//! - [`collections::specialized`]: `System.Collections.Specialized` (`INotifyCollectionChanged` and
//!   the arguments of a change).
//! - [`threading`]: `System.Threading` (`CancellationToken` and its source).
//! - [`events`]: the multicast events and the lists of handlers that FerroUI's events are built on.
//! - [`ferro_thread_local!`]: thread-local variables, `std::thread_local!` on every target with
//!   thread-local statics and one table per thread on Android ([`mod@ferro_thread_local`]).

pub mod collections;
pub mod component_model;
pub mod events;
pub mod ferro_thread_local;
pub mod threading;

#[cfg(test)]
mod tests;
