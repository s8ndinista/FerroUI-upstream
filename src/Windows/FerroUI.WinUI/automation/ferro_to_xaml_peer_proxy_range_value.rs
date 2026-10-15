#[cfg(windows)]
mod imp {
    use super::super::ferro_to_xaml_peer_proxy::FerroToXamlPeerProxy;
    use crate::interop::win_rt::to_abi;
    use crate::interop::IRangeValueProviderImpl;
    use ferroui_controls::automation::provider::IRangeValueProvider as FerroRange;
    use ferroui_microcom::HResult;

    impl IRangeValueProviderImpl for FerroToXamlPeerProxy {
        fn is_read_only(&self) -> Result<u8, HResult> {
            Ok(to_abi(self.peer.get_provider::<dyn FerroRange>().is_none_or(|p| p.is_read_only())))
        }

        fn minimum(&self) -> Result<f64, HResult> {
            Ok(self.peer.get_provider::<dyn FerroRange>().map_or(0.0, |p| p.minimum()))
        }

        fn maximum(&self) -> Result<f64, HResult> {
            Ok(self.peer.get_provider::<dyn FerroRange>().map_or(0.0, |p| p.maximum()))
        }

        fn value(&self) -> Result<f64, HResult> {
            Ok(self.peer.get_provider::<dyn FerroRange>().map_or(0.0, |p| p.value()))
        }

        fn large_change(&self) -> Result<f64, HResult> {
            Ok(self.peer.get_provider::<dyn FerroRange>().map_or(0.0, |p| p.large_change()))
        }

        fn small_change(&self) -> Result<f64, HResult> {
            Ok(self.peer.get_provider::<dyn FerroRange>().map_or(0.0, |p| p.small_change()))
        }

        fn set_value(&self, value: f64) -> Result<(), HResult> {
            if let Some(provider) = self.peer.get_provider::<dyn FerroRange>() {
                super::super::enabled(provider.set_value(value))?;
            }
            Ok(())
        }
    }
}
