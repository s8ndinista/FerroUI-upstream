//! The expand and collapse pattern of a node (the port of
//! `AutomationNode.ExpandCollapse.cs`).

use super::automation_node::AutomationNode;
use super::interop::IExpandCollapseProviderImpl;
use ferroui_controls::automation::provider::IExpandCollapseProvider;
use ferroui_controls::automation::ExpandCollapseState;
use ferroui_microcom::HResult;
use std::rc::Rc;

impl IExpandCollapseProviderImpl for AutomationNode {
    fn expand(&self) -> Result<(), HResult> {
        self.invoke_sync_provider(|x: &Rc<dyn IExpandCollapseProvider>| Self::enabled(x.expand()))
    }

    fn collapse(&self) -> Result<(), HResult> {
        self.invoke_sync_provider(|x: &Rc<dyn IExpandCollapseProvider>| Self::enabled(x.collapse()))
    }

    fn get_expand_collapse_state(&self) -> Result<i32, HResult> {
        Ok(Self::to_uia_expand_collapse_state(
            self.invoke_sync_provider(|x: &Rc<dyn IExpandCollapseProvider>| Ok(x.expand_collapse_state()))?,
        ))
    }
}

impl AutomationNode {
    // The framework declares PartiallyExpanded and LeafNode in the opposite order
    // from the UIA wire values.
    pub(crate) fn to_uia_expand_collapse_state(state: ExpandCollapseState) -> i32 {
        match state {
            ExpandCollapseState::Collapsed => 0,
            ExpandCollapseState::Expanded => 1,
            ExpandCollapseState::PartiallyExpanded => 2,
            ExpandCollapseState::LeafNode => 3,
        }
    }
}
