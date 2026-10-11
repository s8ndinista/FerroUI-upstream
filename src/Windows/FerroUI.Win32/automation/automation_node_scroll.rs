//! The scroll pattern and the scroll item pattern of a node (the port of
//! `AutomationNode.Scroll.cs`).

use super::automation_node::AutomationNode;
use super::interop::{IScrollItemProviderImpl, IScrollProviderImpl};
use ferroui_controls::automation::provider::{IScrollProvider, ScrollAmount};
use ferroui_microcom::{HResult, E_INVALIDARG};
use std::rc::Rc;

/// An amount of the system as the amount of the automation contracts; the
/// two have the same numbers.
fn scroll_amount(amount: i32) -> Result<ScrollAmount, HResult> {
    Ok(match amount {
        0 => ScrollAmount::LargeDecrement,
        1 => ScrollAmount::SmallDecrement,
        2 => ScrollAmount::NoAmount,
        3 => ScrollAmount::LargeIncrement,
        4 => ScrollAmount::SmallIncrement,
        _ => return Err(HResult(E_INVALIDARG)),
    })
}

impl IScrollProviderImpl for AutomationNode {
    fn scroll(&self, horizontal_amount: i32, vertical_amount: i32) -> Result<(), HResult> {
        let (horizontal_amount, vertical_amount) = (scroll_amount(horizontal_amount)?, scroll_amount(vertical_amount)?);
        self.invoke_sync_provider(move |x: &Rc<dyn IScrollProvider>| {
            Self::enabled(x.scroll(horizontal_amount, vertical_amount))
        })
    }

    fn set_scroll_percent(&self, horizontal_percent: f64, vertical_percent: f64) -> Result<(), HResult> {
        self.invoke_sync_provider(move |x: &Rc<dyn IScrollProvider>| {
            Self::enabled(x.set_scroll_percent(horizontal_percent, vertical_percent))
        })
    }

    fn get_horizontal_scroll_percent(&self) -> Result<f64, HResult> {
        self.invoke_sync_provider(|x: &Rc<dyn IScrollProvider>| Ok(x.horizontal_scroll_percent()))
    }

    fn get_vertical_scroll_percent(&self) -> Result<f64, HResult> {
        self.invoke_sync_provider(|x: &Rc<dyn IScrollProvider>| Ok(x.vertical_scroll_percent()))
    }

    fn get_horizontal_view_size(&self) -> Result<f64, HResult> {
        self.invoke_sync_provider(|x: &Rc<dyn IScrollProvider>| Ok(x.horizontal_view_size()))
    }

    fn get_vertical_view_size(&self) -> Result<f64, HResult> {
        self.invoke_sync_provider(|x: &Rc<dyn IScrollProvider>| Ok(x.vertical_view_size()))
    }

    fn get_horizontally_scrollable(&self) -> Result<i32, HResult> {
        self.invoke_sync_provider(|x: &Rc<dyn IScrollProvider>| Ok(i32::from(x.horizontally_scrollable())))
    }

    fn get_vertically_scrollable(&self) -> Result<i32, HResult> {
        self.invoke_sync_provider(|x: &Rc<dyn IScrollProvider>| Ok(i32::from(x.vertically_scrollable())))
    }
}

impl IScrollItemProviderImpl for AutomationNode {
    fn scroll_into_view(&self) -> Result<(), HResult> {
        self.invoke_sync(|peer| peer.bring_into_view())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Not from the reference.
    #[test]
    fn the_amounts_of_the_system_are_the_amounts_of_the_contracts() {
        for amount in 0..5 {
            assert_eq!(amount, scroll_amount(amount).unwrap() as i32);
        }
        assert_eq!(Err(HResult(E_INVALIDARG)), scroll_amount(5));
        assert_eq!(Err(HResult(E_INVALIDARG)), scroll_amount(-1));
    }
}
