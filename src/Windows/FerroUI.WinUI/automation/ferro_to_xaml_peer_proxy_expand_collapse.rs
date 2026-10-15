use crate::interop::ExpandCollapseState;
use ferroui_controls::automation::ExpandCollapseState as FerroExpandState;

/// The state of the expand and collapse pattern of XAML for the state of the
/// framework's provider; a leaf without a provider.
fn to_xaml(state: Option<FerroExpandState>) -> ExpandCollapseState {
    match state {
        Some(FerroExpandState::Collapsed) => ExpandCollapseState::Collapsed,
        Some(FerroExpandState::Expanded) => ExpandCollapseState::Expanded,
        Some(FerroExpandState::PartiallyExpanded) => ExpandCollapseState::PartiallyExpanded,
        _ => ExpandCollapseState::LeafNode,
    }
}

#[cfg(windows)]
mod imp {
    use super::super::ferro_to_xaml_peer_proxy::FerroToXamlPeerProxy;
    use super::to_xaml;
    use crate::interop::{ExpandCollapseState, IExpandCollapseProviderImpl};
    use ferroui_controls::automation::provider::IExpandCollapseProvider as FerroExpand;
    use ferroui_microcom::HResult;

    impl IExpandCollapseProviderImpl for FerroToXamlPeerProxy {
        fn expand_collapse_state(&self) -> Result<ExpandCollapseState, HResult> {
            Ok(to_xaml(self.peer.get_provider::<dyn FerroExpand>().map(|provider| provider.expand_collapse_state())))
        }

        fn expand(&self) -> Result<(), HResult> {
            if let Some(provider) = self.peer.get_provider::<dyn FerroExpand>() {
                super::super::enabled(provider.expand())?;
            }
            Ok(())
        }

        fn collapse(&self) -> Result<(), HResult> {
            if let Some(provider) = self.peer.get_provider::<dyn FerroExpand>() {
                super::super::enabled(provider.collapse())?;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the reference has no tests of the integration.
    use super::*;

    #[test]
    fn the_expand_collapse_states_of_xaml() {
        assert_eq!(ExpandCollapseState::Collapsed, to_xaml(Some(FerroExpandState::Collapsed)));
        assert_eq!(ExpandCollapseState::Expanded, to_xaml(Some(FerroExpandState::Expanded)));
        assert_eq!(ExpandCollapseState::PartiallyExpanded, to_xaml(Some(FerroExpandState::PartiallyExpanded)));
        assert_eq!(ExpandCollapseState::LeafNode, to_xaml(Some(FerroExpandState::LeafNode)));
        assert_eq!(ExpandCollapseState::LeafNode, to_xaml(None));
    }
}
