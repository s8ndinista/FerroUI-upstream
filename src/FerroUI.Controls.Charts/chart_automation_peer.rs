use crate::Chart;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl,
};

/// Uses normal framework name, label and description resolution. Peer queries
/// never force a chart layout or decode its data.
#[repr(C)]
pub struct ChartAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(ChartAutomationPeer: ControlAutomationPeer);
ferro_class_info!(ChartAutomationPeer {});
ferro_impl_classes!(ChartAutomationPeer: FerroObjectImpl, ControlAutomationPeerImpl);
impl AutomationPeerImpl for ChartAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "Chart".into()
    }
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Custom
    }
}
impl ChartAutomationPeer {
    pub fn construct(chart: &Chart) -> Self {
        Self {
            base: ControlAutomationPeer::construct(chart),
        }
    }
    pub fn new(chart: &Chart) -> Ref<Self> {
        instantiate(Self::construct(chart))
    }
}
