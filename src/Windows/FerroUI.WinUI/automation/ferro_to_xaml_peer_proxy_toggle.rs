use crate::interop::ToggleState;
use ferroui_controls::automation::provider::ToggleState as FerroToggleState;

/// The state of the toggle pattern of XAML for the state of the framework's
/// provider; indeterminate without a provider.
fn to_xaml(state: Option<FerroToggleState>) -> ToggleState {
    match state {
        Some(FerroToggleState::Off) => ToggleState::Off,
        Some(FerroToggleState::On) => ToggleState::On,
        _ => ToggleState::Indeterminate,
    }
}

#[cfg(windows)]
mod imp {
    use super::super::ferro_to_xaml_peer_proxy::FerroToXamlPeerProxy;
    use super::to_xaml;
    use crate::interop::{IToggleProviderImpl, ToggleState};
    use ferroui_controls::automation::provider::IToggleProvider as FerroToggle;
    use ferroui_microcom::HResult;

    impl IToggleProviderImpl for FerroToXamlPeerProxy {
        fn toggle_state(&self) -> Result<ToggleState, HResult> {
            Ok(to_xaml(self.peer.get_provider::<dyn FerroToggle>().map(|provider| provider.toggle_state())))
        }

        fn toggle(&self) -> Result<(), HResult> {
            if let Some(provider) = self.peer.get_provider::<dyn FerroToggle>() {
                super::super::enabled(provider.toggle())?;
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
    fn the_toggle_states_of_xaml() {
        assert_eq!(ToggleState::Off, to_xaml(Some(FerroToggleState::Off)));
        assert_eq!(ToggleState::On, to_xaml(Some(FerroToggleState::On)));
        assert_eq!(ToggleState::Indeterminate, to_xaml(Some(FerroToggleState::Indeterminate)));
        assert_eq!(ToggleState::Indeterminate, to_xaml(None));
    }
}
