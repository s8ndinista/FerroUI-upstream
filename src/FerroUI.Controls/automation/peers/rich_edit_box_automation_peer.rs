use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};
use crate::automation::provider::{
    ITextProvider, IValueProvider, ProviderAdapter, ProviderError, RichTextAutomationPeer,
    TextProviderError,
};
use crate::automation::{ElementNotEnabledException, ValuePatternIdentifiers};
use crate::{RichEditBox, RichTextBlock};
use ferroui_base::{
    BoxedValue, FerroObjectImpl, FerroObjectImplExt, Ref, ferro_class, instantiate,
};
use std::rc::Rc;

/// Accessibility projection of the rich editor using the framework's existing provider ABI.
#[repr(C)]
pub struct RichEditBoxAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(RichEditBoxAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(RichEditBoxAutomationPeer {
    interfaces: [
        Rc<dyn IValueProvider> => ProviderAdapter::as_value_provider,
        Rc<dyn ITextProvider> => ProviderAdapter::as_text_provider,
    ]
});
impl ControlAutomationPeerImpl for RichEditBoxAutomationPeer {}
impl AutomationPeerImpl for RichEditBoxAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Edit
    }
    fn get_placeholder_text_core(this: &Self) -> Option<String> {
        Some(this.owner().placeholder_text()).filter(|text| !text.is_empty())
    }
}
impl FerroObjectImpl for RichEditBoxAutomationPeer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let weak = this.to_ref().downgrade();
        this.owner().property_changed(move |event| {
            if event.property() == RichEditBox::text_property().as_property() {
                if let Some(this) = weak.upgrade() {
                    this.raise_property_changed_event(
                        ValuePatternIdentifiers::value_property(),
                        event
                            .get_old_value::<String>()
                            .map(|value| Rc::new(value) as BoxedValue),
                        Some(Rc::new(event.get_new_value::<String>()) as BoxedValue),
                    );
                }
            }
        });
    }
}
impl RichEditBoxAutomationPeer {
    pub fn construct(owner: &RichEditBox) -> Self {
        Self {
            base: ControlAutomationPeer::construct(owner),
        }
    }
    pub fn new(owner: &RichEditBox) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
    pub fn owner(&self) -> Ref<RichEditBox> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("The owner of the peer is a rich editor.")
    }
    pub fn is_read_only(&self) -> bool {
        self.owner().is_read_only()
    }
    pub fn value(&self) -> String {
        self.owner().text()
    }
    pub fn set_value(&self, value: &str) -> Result<(), ProviderError> {
        let owner = ControlAutomationPeer::try_owner(self)
            .and_then(|owner| owner.cast::<RichEditBox>())
            .ok_or(TextProviderError::ElementNotAvailable)?;
        if !owner.is_effectively_enabled() {
            return Err(Box::new(ElementNotEnabledException::new()));
        }
        if owner.is_read_only() {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "The rich text document is read-only.",
            )));
        }
        let editor = owner.editor();
        editor
            .set_selection(crate::rich_text::EditorSelection::new(
                0,
                owner.document().text_length(),
            ))
            .map_err(|error| Box::new(std::io::Error::other(error.to_string())) as ProviderError)?;
        editor
            .insert_text(value, crate::rich_text::EditKind::Programmatic)
            .map_err(|error| Box::new(std::io::Error::other(error.to_string())) as ProviderError)?;
        let selection = editor.selection();
        owner.select(selection.anchor, selection.active);
        Ok(())
    }
}
impl RichTextAutomationPeer for RichEditBoxAutomationPeer {
    fn text_owner(&self) -> Option<Ref<RichTextBlock>> {
        ControlAutomationPeer::try_owner(self).and_then(|owner| owner.cast())
    }
    fn text_peer(&self) -> Ref<AutomationPeer> {
        self.to_ref().upcast()
    }
}
impl IValueProvider for ProviderAdapter<RichEditBoxAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn is_read_only(&self) -> bool {
        self.0.is_read_only()
    }
    fn value(&self) -> Option<String> {
        Some(self.0.value())
    }
    fn set_value(&self, value: Option<&str>) -> Result<(), ProviderError> {
        self.0.set_value(value.unwrap_or_default())
    }
}
