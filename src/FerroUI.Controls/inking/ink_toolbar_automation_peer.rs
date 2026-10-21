//! Accessibility adapters: reuse the framework providers and add inking-specific patterns.
use super::{
    InkToolbar, InkToolbarFlyoutItem, InkToolbarFlyoutItemKind, InkToolbarMenuButton,
    InkToolbarStrokeWidthSlider, InkToolbarToolButton,
};
use crate::automation::peers::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer,
    ControlAutomationPeerImpl, RadioButtonAutomationPeer, RadioButtonAutomationPeerImpl,
    RangeBaseAutomationPeerImpl, SliderAutomationPeer, ToggleButtonAutomationPeer,
};
use crate::automation::provider::{
    IExpandCollapseProvider, ISelectionItemProvider, ISelectionProvider, IValueProvider,
    ProviderAdapter, ProviderError,
};
use crate::automation::{
    ElementNotEnabledException, ExpandCollapsePatternIdentifiers, ExpandCollapseState,
};
use crate::Button;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl,
    FerroObjectImplExt, Ref,
};
use std::rc::Rc;

fn button_state(owner: &Button) -> ExpandCollapseState {
    match owner.flyout() {
        None => ExpandCollapseState::LeafNode,
        Some(f) if f.is_open() => ExpandCollapseState::Expanded,
        _ => ExpandCollapseState::Collapsed,
    }
}
fn expand(owner: &Button) -> Result<(), ElementNotEnabledException> {
    if !owner.is_effectively_enabled() {
        return Err(ElementNotEnabledException::new());
    }
    if let Some(flyout) = owner.flyout() {
        flyout.show_at(owner);
    }
    Ok(())
}
fn collapse(owner: &Button) -> Result<(), ElementNotEnabledException> {
    if !owner.is_effectively_enabled() {
        return Err(ElementNotEnabledException::new());
    }
    if let Some(flyout) = owner.flyout() {
        flyout.hide();
    }
    Ok(())
}
fn observe_flyout(peer: &Ref<AutomationPeer>, owner: &Button) {
    if let Some(flyout) = owner.flyout() {
        let weak_peer = peer.downgrade();
        let weak_owner = owner.to_ref().downgrade();
        let weak_flyout = flyout.downgrade();
        flyout.opened(move || {
            if let (Some(peer), Some(owner), Some(flyout)) = (
                weak_peer.upgrade(),
                weak_owner.upgrade(),
                weak_flyout.upgrade(),
            ) {
                if owner.flyout().as_ref() == Some(&flyout) {
                    peer.raise_property_changed_event(
                        ExpandCollapsePatternIdentifiers::expand_collapse_state_property(),
                        Some(Rc::new(ExpandCollapseState::Collapsed)),
                        Some(Rc::new(ExpandCollapseState::Expanded)),
                    );
                }
            }
        });
        let weak_peer = peer.downgrade();
        let weak_owner = owner.to_ref().downgrade();
        let weak_flyout = flyout.downgrade();
        flyout.closed(move || {
            if let (Some(peer), Some(owner), Some(flyout)) = (
                weak_peer.upgrade(),
                weak_owner.upgrade(),
                weak_flyout.upgrade(),
            ) {
                if owner.flyout().as_ref() == Some(&flyout) {
                    peer.raise_property_changed_event(
                        ExpandCollapsePatternIdentifiers::expand_collapse_state_property(),
                        Some(Rc::new(ExpandCollapseState::Expanded)),
                        Some(Rc::new(ExpandCollapseState::Collapsed)),
                    );
                }
            }
        });
    }
}
fn subscribe_flyout(peer: &Ref<AutomationPeer>, owner: &Button) {
    observe_flyout(peer, owner);
    let weak_peer = peer.downgrade();
    let weak_owner = owner.to_ref().downgrade();
    owner.property_changed(move |change| {
        if change.property() == Button::flyout_property().as_property() {
            if let (Some(peer), Some(owner)) = (weak_peer.upgrade(), weak_owner.upgrade()) {
                observe_flyout(&peer, &owner);
            }
        }
    });
}

#[repr(C)]
pub struct InkToolbarAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(InkToolbarAutomationPeer:ControlAutomationPeer);
ferro_class_info!(InkToolbarAutomationPeer {});
ferro_impl_classes!(InkToolbarAutomationPeer:FerroObjectImpl,ControlAutomationPeerImpl);
impl AutomationPeerImpl for InkToolbarAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "InkToolbar".to_owned()
    }
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::ToolBar
    }
}
impl InkToolbarAutomationPeer {
    pub fn construct(owner: &InkToolbar) -> Self {
        Self {
            base: ControlAutomationPeer::construct(owner),
        }
    }
    pub fn new(owner: &InkToolbar) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
}

#[repr(C)]
pub struct InkToolbarToolButtonAutomationPeer {
    base: RadioButtonAutomationPeer,
}
ferro_class!(InkToolbarToolButtonAutomationPeer:RadioButtonAutomationPeer);
ferro_class_info!(InkToolbarToolButtonAutomationPeer{interfaces:[Rc<dyn IExpandCollapseProvider> => ProviderAdapter::as_expand_collapse_provider,Rc<dyn ISelectionItemProvider> => ProviderAdapter::as_selection_item_provider]});
ferro_impl_classes!(InkToolbarToolButtonAutomationPeer:ControlAutomationPeerImpl,RadioButtonAutomationPeerImpl);
impl FerroObjectImpl for InkToolbarToolButtonAutomationPeer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        subscribe_flyout(&this.to_ref().upcast(), &this.owner_button());
    }
}
impl AutomationPeerImpl for InkToolbarToolButtonAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "InkToolbarToolButton".to_owned()
    }
}
impl InkToolbarToolButtonAutomationPeer {
    pub fn construct(owner: &InkToolbarToolButton) -> Self {
        Self {
            base: RadioButtonAutomationPeer::construct(owner),
        }
    }
    pub fn new(owner: &InkToolbarToolButton) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
    fn owner_button(&self) -> Ref<Button> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("tool button peer owner")
    }
}
impl IExpandCollapseProvider for ProviderAdapter<InkToolbarToolButtonAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn expand_collapse_state(&self) -> ExpandCollapseState {
        button_state(&self.0.owner_button())
    }
    fn shows_menu(&self) -> bool {
        true
    }
    fn expand(&self) -> Result<(), ElementNotEnabledException> {
        expand(&self.0.owner_button())
    }
    fn collapse(&self) -> Result<(), ElementNotEnabledException> {
        collapse(&self.0.owner_button())
    }
}
impl ISelectionItemProvider for ProviderAdapter<InkToolbarToolButtonAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn is_selected(&self) -> bool {
        self.0.is_selected()
    }
    fn selection_container(&self) -> Option<Rc<dyn ISelectionProvider>> {
        self.0.selection_container()
    }
    fn add_to_selection(&self) -> Result<(), ElementNotEnabledException> {
        self.0.add_to_selection()
    }
    fn remove_from_selection(&self) -> Result<(), ElementNotEnabledException> {
        self.0.remove_from_selection()
    }
    fn select(&self) -> Result<(), ElementNotEnabledException> {
        self.0.select()
    }
}

#[repr(C)]
pub struct InkToolbarMenuButtonAutomationPeer {
    base: ToggleButtonAutomationPeer,
}
ferro_class!(InkToolbarMenuButtonAutomationPeer:ToggleButtonAutomationPeer);
ferro_class_info!(InkToolbarMenuButtonAutomationPeer{interfaces:[Rc<dyn IExpandCollapseProvider> => ProviderAdapter::as_expand_collapse_provider]});
ferro_impl_classes!(InkToolbarMenuButtonAutomationPeer:ControlAutomationPeerImpl);
impl FerroObjectImpl for InkToolbarMenuButtonAutomationPeer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        subscribe_flyout(&this.to_ref().upcast(), &this.owner_button());
    }
}
impl AutomationPeerImpl for InkToolbarMenuButtonAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "InkToolbarMenuButton".to_owned()
    }
}
impl InkToolbarMenuButtonAutomationPeer {
    pub fn construct(owner: &InkToolbarMenuButton) -> Self {
        Self {
            base: ToggleButtonAutomationPeer::construct(owner),
        }
    }
    pub fn new(owner: &InkToolbarMenuButton) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
    fn owner_button(&self) -> Ref<Button> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("menu button peer owner")
    }
}
impl IExpandCollapseProvider for ProviderAdapter<InkToolbarMenuButtonAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn expand_collapse_state(&self) -> ExpandCollapseState {
        button_state(&self.0.owner_button())
    }
    fn shows_menu(&self) -> bool {
        true
    }
    fn expand(&self) -> Result<(), ElementNotEnabledException> {
        expand(&self.0.owner_button())
    }
    fn collapse(&self) -> Result<(), ElementNotEnabledException> {
        collapse(&self.0.owner_button())
    }
}

#[repr(C)]
pub struct InkToolbarFlyoutItemAutomationPeer {
    base: ToggleButtonAutomationPeer,
}
ferro_class!(InkToolbarFlyoutItemAutomationPeer:ToggleButtonAutomationPeer);
ferro_class_info!(InkToolbarFlyoutItemAutomationPeer{interfaces:[Rc<dyn ISelectionItemProvider> => ProviderAdapter::as_selection_item_provider]});
ferro_impl_classes!(InkToolbarFlyoutItemAutomationPeer:FerroObjectImpl,ControlAutomationPeerImpl);
impl AutomationPeerImpl for InkToolbarFlyoutItemAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "InkToolbarFlyoutItem".to_owned()
    }
}
impl InkToolbarFlyoutItemAutomationPeer {
    pub fn construct(owner: &InkToolbarFlyoutItem) -> Self {
        Self {
            base: ToggleButtonAutomationPeer::construct(owner),
        }
    }
    pub fn new(owner: &InkToolbarFlyoutItem) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
    fn owner_item(&self) -> Ref<InkToolbarFlyoutItem> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("flyout item peer owner")
    }
    fn set_selected(&self, value: bool) -> Result<(), ElementNotEnabledException> {
        let owner = self.owner_item();
        if !owner.is_effectively_enabled() {
            return Err(ElementNotEnabledException::new());
        }
        if owner.kind() != InkToolbarFlyoutItemKind::Simple {
            owner.set_is_checked(Some(value));
        }
        Ok(())
    }
}
impl ISelectionItemProvider for ProviderAdapter<InkToolbarFlyoutItemAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn is_selected(&self) -> bool {
        self.0.owner_item().is_checked() == Some(true)
    }
    fn selection_container(&self) -> Option<Rc<dyn ISelectionProvider>> {
        None
    }
    fn add_to_selection(&self) -> Result<(), ElementNotEnabledException> {
        self.0.set_selected(true)
    }
    fn remove_from_selection(&self) -> Result<(), ElementNotEnabledException> {
        self.0.set_selected(false)
    }
    fn select(&self) -> Result<(), ElementNotEnabledException> {
        self.0.set_selected(true)
    }
}

#[repr(C)]
pub struct InkToolbarStrokeWidthSliderAutomationPeer {
    base: SliderAutomationPeer,
}
ferro_class!(InkToolbarStrokeWidthSliderAutomationPeer:SliderAutomationPeer);
ferro_class_info!(InkToolbarStrokeWidthSliderAutomationPeer{interfaces:[Rc<dyn IValueProvider> => ProviderAdapter::as_value_provider]});
ferro_impl_classes!(InkToolbarStrokeWidthSliderAutomationPeer:FerroObjectImpl,ControlAutomationPeerImpl,RangeBaseAutomationPeerImpl);
impl AutomationPeerImpl for InkToolbarStrokeWidthSliderAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "InkToolbarStrokeWidthSlider".to_owned()
    }
}
impl InkToolbarStrokeWidthSliderAutomationPeer {
    pub fn construct(owner: &InkToolbarStrokeWidthSlider) -> Self {
        Self {
            base: SliderAutomationPeer::construct(owner),
        }
    }
    pub fn new(owner: &InkToolbarStrokeWidthSlider) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
    fn owner_slider(&self) -> Ref<InkToolbarStrokeWidthSlider> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("stroke width peer owner")
    }
}
impl IValueProvider for ProviderAdapter<InkToolbarStrokeWidthSliderAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn is_read_only(&self) -> bool {
        !self.0.owner_slider().is_effectively_enabled()
    }
    fn value(&self) -> Option<String> {
        Some(self.0.owner_slider().value().to_string())
    }
    fn set_value(&self, value: Option<&str>) -> Result<(), ProviderError> {
        let owner = self.0.owner_slider();
        if !owner.is_effectively_enabled() {
            return Err(Box::new(ElementNotEnabledException::new()));
        }
        let value = value.unwrap_or("").parse::<f64>()?;
        if !value.is_finite() || value < owner.minimum() || value > owner.maximum() {
            return Err(Box::new(super::InkError::InvalidInput(
                "Stroke width must lie within the slider range",
            )));
        }
        owner.set_range_value(value);
        Ok(())
    }
}
