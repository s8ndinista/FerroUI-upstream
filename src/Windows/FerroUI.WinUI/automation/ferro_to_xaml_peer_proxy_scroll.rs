use crate::interop::ScrollAmount;
use ferroui_controls::automation::provider::ScrollAmount as FerroScrollAmount;

fn to_ferro(amount: ScrollAmount) -> FerroScrollAmount {
    match amount {
        ScrollAmount::LargeDecrement => FerroScrollAmount::LargeDecrement,
        ScrollAmount::SmallDecrement => FerroScrollAmount::SmallDecrement,
        ScrollAmount::LargeIncrement => FerroScrollAmount::LargeIncrement,
        ScrollAmount::SmallIncrement => FerroScrollAmount::SmallIncrement,
        _ => FerroScrollAmount::NoAmount,
    }
}

#[cfg(windows)]
mod imp {
    use super::super::ferro_to_xaml_peer_proxy::FerroToXamlPeerProxy;
    use super::to_ferro;
    use crate::interop::win_rt::to_abi;
    use crate::interop::{IScrollItemProviderImpl, IScrollProviderImpl, ScrollAmount};
    use ferroui_controls::automation::provider::IScrollProvider as FerroScroll;
    use ferroui_microcom::HResult;

    impl IScrollProviderImpl for FerroToXamlPeerProxy {
        fn horizontally_scrollable(&self) -> Result<u8, HResult> {
            Ok(to_abi(self.peer.get_provider::<dyn FerroScroll>().is_some_and(|p| p.horizontally_scrollable())))
        }

        fn horizontal_scroll_percent(&self) -> Result<f64, HResult> {
            Ok(self.peer.get_provider::<dyn FerroScroll>().map_or(-1.0, |p| p.horizontal_scroll_percent()))
        }

        fn horizontal_view_size(&self) -> Result<f64, HResult> {
            Ok(self.peer.get_provider::<dyn FerroScroll>().map_or(100.0, |p| p.horizontal_view_size()))
        }

        fn vertically_scrollable(&self) -> Result<u8, HResult> {
            Ok(to_abi(self.peer.get_provider::<dyn FerroScroll>().is_some_and(|p| p.vertically_scrollable())))
        }

        fn vertical_scroll_percent(&self) -> Result<f64, HResult> {
            Ok(self.peer.get_provider::<dyn FerroScroll>().map_or(-1.0, |p| p.vertical_scroll_percent()))
        }

        fn vertical_view_size(&self) -> Result<f64, HResult> {
            Ok(self.peer.get_provider::<dyn FerroScroll>().map_or(100.0, |p| p.vertical_view_size()))
        }

        fn scroll(&self, horizontal_amount: ScrollAmount, vertical_amount: ScrollAmount) -> Result<(), HResult> {
            if let Some(provider) = self.peer.get_provider::<dyn FerroScroll>() {
                super::super::enabled(provider.scroll(to_ferro(horizontal_amount), to_ferro(vertical_amount)))?;
            }
            Ok(())
        }

        fn set_scroll_percent(&self, horizontal_percent: f64, vertical_percent: f64) -> Result<(), HResult> {
            if let Some(provider) = self.peer.get_provider::<dyn FerroScroll>() {
                super::super::enabled(provider.set_scroll_percent(horizontal_percent, vertical_percent))?;
            }
            Ok(())
        }
    }

    impl IScrollItemProviderImpl for FerroToXamlPeerProxy {
        fn scroll_into_view(&self) -> Result<(), HResult> {
            self.peer.bring_into_view();
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the reference has no tests of the integration.
    use super::*;

    #[test]
    fn the_scroll_amounts_of_the_framework() {
        assert_eq!(FerroScrollAmount::LargeDecrement, to_ferro(ScrollAmount::LargeDecrement));
        assert_eq!(FerroScrollAmount::SmallDecrement, to_ferro(ScrollAmount::SmallDecrement));
        assert_eq!(FerroScrollAmount::LargeIncrement, to_ferro(ScrollAmount::LargeIncrement));
        assert_eq!(FerroScrollAmount::SmallIncrement, to_ferro(ScrollAmount::SmallIncrement));
        assert_eq!(FerroScrollAmount::NoAmount, to_ferro(ScrollAmount::NoAmount));
    }
}
