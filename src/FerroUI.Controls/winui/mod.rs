//! Opt-in WinUI contracts. Existing controls and their type identities stay unchanged.

mod auto_suggest_box;
mod frame;
mod font_icon;
mod icon_source;
mod info_badge;
mod info_bar;
mod info_bar_panel;
mod navigation;
mod navigation_view;
mod navigation_view_automation_peer;
mod navigation_view_events;
mod navigation_view_item;
mod navigation_view_item_automation_peer;
mod number_box;
mod number_box_automation_peer;
mod number_box_expression;
mod page;
mod progress_ring;
mod progress_ring_automation_peer;
mod radio_buttons;
mod rating_control;
mod rating_item_info;
mod register_types;
mod symbol_icon;

pub use frame::{Frame, PageFactory};
pub use auto_suggest_box::{AutoSuggestBox, AutoSuggestBoxAutomationPeer, AutoSuggestBoxQuerySubmittedEventArgs, AutoSuggestBoxSuggestionChosenEventArgs, AutoSuggestBoxTextChangedEventArgs, AutoSuggestionBoxTextChangeReason};
pub use font_icon::FontIcon;
pub use icon_source::{FontIconSource, IconSource, IconSourceImpl, IconSourceImplExt, IconSourceVTable, PathIconSource, SymbolIconSource};
pub use info_badge::{InfoBadge, InfoBadgeTemplateSettings};
pub use info_bar::{InfoBar, InfoBarAutomationPeer, InfoBarClosedEventArgs, InfoBarCloseReason, InfoBarClosingEventArgs, InfoBarOpenedEventArgs, InfoBarSeverity, InfoBarTemplateSettings};
pub use info_bar_panel::InfoBarPanel;
pub use radio_buttons::{RadioButtons, RadioButtonsAutomationPeer};
pub use rating_control::{RatingControl, RatingControlAutomationPeer};
pub use rating_item_info::{RatingItemInfo, RatingItemFontInfo, RatingItemImageInfo};
pub use symbol_icon::{Symbol, SymbolIcon};
pub use navigation::{
    FrameNavigationOptions, NavigatingCancelEventArgs, NavigationCacheMode, NavigationError,
    NavigationEventArgs, NavigationFailedEventArgs, NavigationMode, NavigationStack, PageStackEntry,
};
pub use navigation_view::NavigationView;
pub use navigation_view_automation_peer::NavigationViewAutomationPeer;
pub use navigation_view_events::{
    NavigationViewBackButtonVisible, NavigationViewBackRequestedEventArgs,
    NavigationViewDisplayMode, NavigationViewDisplayModeChangedEventArgs,
    NavigationViewItemCollapsedEventArgs, NavigationViewItemExpandingEventArgs,
    NavigationViewItemInvokedEventArgs, NavigationViewPaneClosingEventArgs,
    NavigationViewPaneDisplayMode, NavigationViewSelectionChangedEventArgs,
    NavigationViewSelectionFollowsFocus,
};
pub use navigation_view_item::{
    NavigationViewItem, NavigationViewItemBase, NavigationViewItemHeader,
    NavigationViewItemSeparator,
};
pub use navigation_view_item_automation_peer::NavigationViewItemAutomationPeer;
pub use page::{Page, PageImpl, PageImplExt, PageVTable};
pub use progress_ring::{ProgressRing, ProgressRingTemplateSettings};
pub use progress_ring_automation_peer::ProgressRingAutomationPeer;
pub use number_box::{
    INumberBoxNumberFormatter, InvariantNumberBoxNumberFormatter, NumberBox,
    NumberBoxSpinButtonPlacementMode, NumberBoxValidationMode, NumberBoxValueChangedEventArgs,
};
pub use number_box_automation_peer::NumberBoxAutomationPeer;
pub use number_box_expression::{evaluate_number_expression, NumberBoxExpressionError};
pub(crate) use register_types::register_markup_types;
