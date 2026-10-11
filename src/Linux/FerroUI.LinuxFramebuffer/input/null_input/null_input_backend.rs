//! The input backend without input (the port of `NullInputBackend.cs`).

use crate::input::{IInputBackend, IScreenInfoProvider};
use crate::raw_event_grouping::RawEvent;
use ferroui_base::input::IInputRoot;
use std::rc::Rc;

#[derive(Default)]
pub struct NullInputBackend;

impl NullInputBackend {
    pub fn new() -> Self {
        Self
    }
}

impl IInputBackend for NullInputBackend {
    fn initialize(&self, _screen: Rc<dyn IScreenInfoProvider>, _on_input: Rc<dyn Fn(RawEvent)>) {}

    fn set_input_root(&self, _root: Rc<dyn IInputRoot>) {}
}
