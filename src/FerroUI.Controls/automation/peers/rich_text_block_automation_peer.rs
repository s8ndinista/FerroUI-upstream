use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};
use crate::RichTextBlock;
use crate::automation::provider::{ITextProvider, ProviderAdapter, RichTextAutomationPeer};
use ferroui_base::{FerroObjectImpl, Ref, ferro_class, instantiate};
use std::rc::Rc;

/// Accessibility projection of the rich view, with live document ranges,
/// selection, effective formatting, embedded children, and shaped geometry.
#[repr(C)]
pub struct RichTextBlockAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(RichTextBlockAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(RichTextBlockAutomationPeer {
    interfaces: [Rc<dyn ITextProvider> => ProviderAdapter::as_text_provider]
});
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
impl RichTextAutomationPeer for RichTextBlockAutomationPeer {
    fn text_owner(&self) -> Option<Ref<RichTextBlock>> {
        ControlAutomationPeer::try_owner(self).and_then(|owner| owner.cast())
    }
    fn text_peer(&self) -> Ref<AutomationPeer> {
        self.to_ref().upcast()
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
