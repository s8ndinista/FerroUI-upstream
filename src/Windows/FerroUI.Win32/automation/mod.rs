//! UI Automation: the provider side of the accessibility of Windows (the
//! port of the reference's automation project of the Windows backend,
//! which is internal to the backend there too).
//!
//! A window answers `WM_GETOBJECT` with the provider of its root; a tree of
//! nodes over the automation peers of the controls answers what a client
//! (a screen reader, a test tool) asks, and raises the events of the peers.
//! The design is in `docs/porting/win32-platform.md`, section 13.

pub mod interop;
pub mod marshalling;
