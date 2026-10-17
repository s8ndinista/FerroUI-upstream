use super::{
    AutomationControlType, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl,
};
use crate::RichTextBlock;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};

/// The rich document's accessible name and text control identity. Text range
/// geometry remains in document coordinates through the owning view.
#[repr(C)]
pub struct RichTextBlockAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(RichTextBlockAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(RichTextBlockAutomationPeer {});
impl FerroObjectImpl for RichTextBlockAutomationPeer {}
impl ControlAutomationPeerImpl for RichTextBlockAutomationPeer {}
impl AutomationPeerImpl for RichTextBlockAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Text
    }
    fn get_name_core(this: &Self) -> Option<String> {
        Some(this.owner().document().text())
    }
}
impl RichTextBlockAutomationPeer {
    pub fn construct(owner: &RichTextBlock) -> Self {
        Self {
            base: ControlAutomationPeer::construct(owner),
        }
    }
    pub fn new(owner: &RichTextBlock) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
    pub fn owner(&self) -> Ref<RichTextBlock> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("The owner is a rich text view.")
    }
}
