//! An input backend (the port of `IInputBackend.cs`).

use super::IScreenInfoProvider;
use crate::raw_event_grouping::RawEvent;
use ferroui_base::input::IInputRoot;
use std::rc::Rc;

pub trait IInputBackend {
    /// Starts the backend. `on_input` is called on the UI thread, where
    /// raw input events live (the reference calls it on the thread of the
    /// backend and the top-level posts the event).
    fn initialize(&self, info: Rc<dyn IScreenInfoProvider>, on_input: Rc<dyn Fn(RawEvent)>);

    fn set_input_root(&self, root: Rc<dyn IInputRoot>);
}
