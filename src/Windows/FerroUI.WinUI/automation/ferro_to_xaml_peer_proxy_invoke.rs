#[cfg(windows)]
mod imp {
    use super::super::ferro_to_xaml_peer_proxy::FerroToXamlPeerProxy;
    use crate::interop::IInvokeProviderImpl;
    use ferroui_controls::automation::provider::IInvokeProvider as FerroInvoke;
    use ferroui_microcom::HResult;

    impl IInvokeProviderImpl for FerroToXamlPeerProxy {
        fn invoke(&self) -> Result<(), HResult> {
            if let Some(provider) = self.peer.get_provider::<dyn FerroInvoke>() {
                super::super::enabled(provider.invoke())?;
            }
            Ok(())
        }
    }
}
