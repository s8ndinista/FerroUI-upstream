//! The native text pattern of a node. The provider methods follow WinUI's
//! `CUIATextProviderWrapper` at `fe244ca6ac4d482712b234a9de3cf771cc6481de`
//! (Microsoft Corporation, MIT), using the existing FerroUI automation node's
//! dispatcher and identity instead of a parallel native document.

use super::automation_node::{AutomationNode, ELEMENT_NOT_AVAILABLE};
use super::interop::{
    IRawElementProviderSimple, ITextProviderImpl, ITextRangeProvider, SupportedTextSelection,
    UiaCoreProviderApi, UiaEventId, UiaPoint,
};
use super::marshalling::{SafeArrayMarshaller, SafeArrayRef};
use super::root_automation_node::RootAutomationNode;
use super::text_pattern_marshalling::{identity_of, provider_error};
use super::text_range_node::{SendTextRange, TextRangeNode};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{PixelPoint, PixelRect, Point, Rect, Ref};
use ferroui_controls::automation::peers::{AutomationPeer, ControlAutomationPeer};
use ferroui_controls::automation::provider::ITextProvider;
use ferroui_microcom::{ComPtr, E_FAIL, E_INVALIDARG, HResult};
use std::rc::Rc;

impl AutomationNode {
    /// The subscriptions use only the node identifier. They do not retain the
    /// peer, its control, or the provider adapter.
    pub(crate) fn text_subscriptions(
        peer: &Ref<AutomationPeer>,
        id: i32,
    ) -> Vec<Rc<dyn IDisposable>> {
        let Some(provider) = peer.get_provider::<dyn ITextProvider>() else {
            return Vec::new();
        };
        let mut subscriptions = Vec::new();
        if let Ok(subscription) = provider.text_changed(Rc::new(move || {
            Self::raise_text_event(id, UiaEventId::Text_TextChanged)
        })) {
            subscriptions.push(subscription);
        }
        if let Ok(subscription) = provider.text_selection_changed(Rc::new(move || {
            Self::raise_text_event(id, UiaEventId::Text_TextSelectionChanged)
        })) {
            subscriptions.push(subscription);
        }
        subscriptions
    }

    fn raise_text_event(id: i32, event: UiaEventId) {
        // Events run on the UI thread, like the existing property and selection
        // events. A released/disconnected peer cannot raise a new native event.
        if Self::peer_of(id).is_none() {
            return;
        }
        if let Some(node) = Self::provider_of(id) {
            UiaCoreProviderApi::uia_raise_automation_event(Some(&node), event.0);
        }
    }
}

impl ITextProviderImpl for AutomationNode {
    fn get_selection(&self) -> Result<SafeArrayRef, HResult> {
        let owner = self.id();
        let ranges = self.invoke_sync_provider(move |provider: &Rc<dyn ITextProvider>| {
            provider
                .get_selection()
                .map(|ranges| {
                    ranges
                        .into_iter()
                        .map(|range| TextRangeNode::new(owner, range))
                        .collect::<Vec<_>>()
                })
                .map_err(provider_error)
        })?;
        range_array(ranges)
    }

    fn get_visible_ranges(&self) -> Result<SafeArrayRef, HResult> {
        let owner = self.id();
        let ranges = self.invoke_sync_provider(move |provider: &Rc<dyn ITextProvider>| {
            provider
                .get_visible_ranges()
                .map(|ranges| {
                    ranges
                        .into_iter()
                        .map(|range| TextRangeNode::new(owner, range))
                        .collect::<Vec<_>>()
                })
                .map_err(provider_error)
        })?;
        range_array(ranges)
    }

    fn range_from_child(
        &self,
        child_element: Option<&IRawElementProviderSimple>,
    ) -> Result<Option<ComPtr<ITextRangeProvider>>, HResult> {
        let identity = identity_of(child_element.ok_or(HResult(E_INVALIDARG))?)?;
        let key = identity.as_ptr() as usize;
        let owner = self.id();
        let range = self.invoke_sync_provider(move |provider: &Rc<dyn ITextProvider>| {
            // Foreign native objects cannot be cast to a Rust node. Resolve only
            // identities registered by this backend and let the provider check
            // that the peer belongs to its own text document.
            let child = Self::peer_from_identity(key).ok_or(HResult(E_INVALIDARG))?;
            provider
                .range_from_child(&child)
                .map(|range| TextRangeNode::new(owner, range))
                .map_err(provider_error)
        })?;
        Ok(Some(range.into_inner()))
    }

    fn range_from_point(
        &self,
        point: UiaPoint,
    ) -> Result<Option<ComPtr<ITextRangeProvider>>, HResult> {
        if !point.x.is_finite()
            || !point.y.is_finite()
            || point.x < i32::MIN as f64
            || point.x > i32::MAX as f64
            || point.y < i32::MIN as f64
            || point.y > i32::MAX as f64
        {
            return Err(HResult(E_INVALIDARG));
        }
        let owner = self.id();
        let range = self.invoke_sync_provider(move |provider: &Rc<dyn ITextProvider>| {
            let point = from_screen(&provider.peer(), point)?;
            provider
                .range_from_point(point)
                .map(|range| TextRangeNode::new(owner, range))
                .map_err(provider_error)
        })?;
        Ok(Some(range.into_inner()))
    }

    fn get_document_range(&self) -> Result<Option<ComPtr<ITextRangeProvider>>, HResult> {
        let owner = self.id();
        let range = self.invoke_sync_provider(move |provider: &Rc<dyn ITextProvider>| {
            provider
                .document_range()
                .map(|range| TextRangeNode::new(owner, range))
                .map_err(provider_error)
        })?;
        Ok(Some(range.into_inner()))
    }

    fn get_supported_text_selection(&self) -> Result<SupportedTextSelection, HResult> {
        self.invoke_sync_provider(|provider: &Rc<dyn ITextProvider>| {
            provider
                .supported_text_selection()
                .map(|value| SupportedTextSelection(value as i32))
                .map_err(provider_error)
        })
    }
}

fn range_array(ranges: Vec<SendTextRange>) -> Result<SafeArrayRef, HResult> {
    let ranges: Vec<_> = ranges.into_iter().map(SendTextRange::into_inner).collect();
    SafeArrayMarshaller::convert_to_unmanaged(Some(&ranges)).map_err(|_| HResult(E_FAIL))
}

fn from_screen(peer: &Ref<AutomationPeer>, point: UiaPoint) -> Result<Point, HResult> {
    let pixel = PixelPoint::new(point.x.round() as i32, point.y.round() as i32);
    if let Some(control_peer) = peer.cast::<ControlAutomationPeer>() {
        // Use exactly the presentation root that ControlAutomationPeer uses
        // for `to_screen`, including popup and embedded presentation roots.
        let owner = control_peer.try_owner().ok_or(ELEMENT_NOT_AVAILABLE)?;
        let source = owner.presentation_source().ok_or(HResult(E_FAIL))?;
        source.root_visual().ok_or(HResult(E_FAIL))?;
        return source.point_to_client(pixel).ok_or(HResult(E_FAIL));
    }
    let root = peer.get_visual_root().ok_or(HResult(E_FAIL))?;
    let platform = RootAutomationNode::window_impl(&root).ok_or(HResult(E_FAIL))?;
    Ok(platform.point_to_client(pixel))
}

pub(crate) fn to_screen(peer: &Ref<AutomationPeer>, rect: Rect) -> Result<Rect, HResult> {
    if let Some(control_peer) = peer.cast::<ControlAutomationPeer>() {
        let owner = control_peer.try_owner().ok_or(ELEMENT_NOT_AVAILABLE)?;
        let source = owner.presentation_source().ok_or(HResult(E_FAIL))?;
        source.root_visual().ok_or(HResult(E_FAIL))?;
        let first = source
            .point_to_screen(rect.top_left())
            .ok_or(HResult(E_FAIL))?;
        let last = source
            .point_to_screen(rect.bottom_right())
            .ok_or(HResult(E_FAIL))?;
        return Ok(PixelRect::from_points(first, last).to_rect(1.0));
    }
    peer.to_screen(rect).ok_or(HResult(E_FAIL))
}
