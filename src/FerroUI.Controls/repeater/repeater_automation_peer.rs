// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md for license information.

use super::ItemsRepeater;
use crate::automation::peers::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref};

/// Exposes realized child peers in data-source order, including peers whose
/// accessible control is a descendant of the generated element.
#[repr(C)]
pub struct RepeaterAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(RepeaterAutomationPeer: ControlAutomationPeer);
ferro_class_info!(RepeaterAutomationPeer {
    markup: {
        constructors: [(Ref<ItemsRepeater>) => |owner: Ref<ItemsRepeater>| RepeaterAutomationPeer::new(&owner)],
    },
});
impl FerroObjectImpl for RepeaterAutomationPeer {}
impl AutomationPeerImpl for RepeaterAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Group
    }
}
impl ControlAutomationPeerImpl for RepeaterAutomationPeer {
    fn get_children_core(this: &Self) -> Option<Vec<Ref<AutomationPeer>>> {
        let owner = this.owner();
        fn append(peer: Ref<AutomationPeer>, result: &mut Vec<Ref<AutomationPeer>>) {
            if peer.is_control_element() {
                result.push(peer);
            } else {
                // Raw containers contribute their accessible descendants.
                // Their own peer is not part of the control-view tree.
                for child in peer.get_children().iter() {
                    append(child.clone(), result);
                }
            }
        }
        let mut children = Vec::new();
        for element in owner.realized_elements() {
            if ItemsRepeater::get_virtualization_info(&element).is_held_by_layout()
                && element.is_visible()
            {
                append(this.get_or_create(&element), &mut children);
            }
        }
        Some(children)
    }
}
impl RepeaterAutomationPeer {
    pub fn construct(owner: &ItemsRepeater) -> Self {
        Self {
            base: ControlAutomationPeer::construct(owner),
        }
    }
    pub fn new(owner: &ItemsRepeater) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
    pub fn owner(&self) -> Ref<ItemsRepeater> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("The automation peer belongs to an ItemsRepeater.")
    }
}
