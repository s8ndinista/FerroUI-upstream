//! An automation node which serves as the root of an embedded native
//! control automation tree (the port of `InteropAutomationNode.cs`).
//!
//! The class of the reference derives from the node and overrides most of
//! it. Here a node of the kind
//! [`NodeKind::Interop`](super::automation_node::NodeKind) carries the
//! handle of the native control, and what the class answers is in this
//! file.

use super::interop::{
    IRawElementProviderFragment, IRawElementProviderFragmentRoot, IRawElementProviderSimple, NavigateDirection,
    ProviderOptions, UiaCoreProviderApi,
};
use super::marshalling::VariantValue;
use ferroui_base::{Rect, Ref};
use ferroui_controls::automation::peers::InteropAutomationPeer;
use ferroui_microcom::{ComPtr, HResult, IUnknown};

/// What the node of an embedded native control answers.
pub(crate) struct InteropAutomationNode;

impl InteropAutomationNode {
    /// The constructor of the reference: the handle of the native control
    /// of the peer.
    pub fn new(peer: &Ref<InteropAutomationPeer>) -> isize {
        peer.native_control_handle().handle()
    }

    pub fn get_bounding_rectangle() -> Rect {
        Rect::default()
    }

    pub fn get_fragment_root() -> Option<ComPtr<IRawElementProviderFragmentRoot>> {
        None
    }

    pub fn get_provider_options() -> ProviderOptions {
        ProviderOptions::ServerSideProvider | ProviderOptions::OverrideProvider
    }

    pub fn get_pattern_provider(_pattern_id: i32) -> Option<ComPtr<IUnknown>> {
        None
    }

    pub fn get_property_value(_property_id: i32) -> Option<VariantValue> {
        None
    }

    /// The provider of the system for the window of the native control,
    /// whose tree is the tree of this node.
    pub fn get_host_raw_element_provider(handle: isize) -> Result<Option<ComPtr<IRawElementProviderSimple>>, HResult> {
        UiaCoreProviderApi::uia_host_provider_from_hwnd(handle)
    }

    /// `Navigate`: the node leads to its parent and nowhere else; whether
    /// the node answers for `direction`.
    pub fn navigates(direction: NavigateDirection) -> bool {
        direction == NavigateDirection::Parent
    }

    pub fn element_provider_from_point(_x: f64, _y: f64) -> Option<ComPtr<IRawElementProviderFragment>> {
        None
    }

    pub fn get_focus() -> Option<ComPtr<IRawElementProviderFragment>> {
        None
    }

    pub fn get_embedded_fragment_roots() -> Option<Vec<ComPtr<IRawElementProviderSimple>>> {
        None
    }
}
