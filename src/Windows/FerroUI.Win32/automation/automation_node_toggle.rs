//! The toggle pattern of a node (the port of `AutomationNode.Toggle.cs`).

use super::automation_node::AutomationNode;
use super::interop::IToggleProviderImpl;
use ferroui_controls::automation::provider::IToggleProvider;
use ferroui_microcom::HResult;
use std::rc::Rc;

impl IToggleProviderImpl for AutomationNode {
    fn toggle(&self) -> Result<(), HResult> {
        self.invoke_sync_provider(|x: &Rc<dyn IToggleProvider>| Self::enabled(x.toggle()))
    }

    /// The state of the automation contracts, whose numbers are the ones
    /// of the system.
    fn get_toggle_state(&self) -> Result<i32, HResult> {
        self.invoke_sync_provider(|x: &Rc<dyn IToggleProvider>| Ok(x.toggle_state() as i32))
    }
}
