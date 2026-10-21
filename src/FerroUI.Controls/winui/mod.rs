//! Opt-in WinUI contracts. Existing controls and their type identities stay unchanged.

mod auto_suggest_box;
mod frame;
mod font_icon;
mod icon_source;
mod info_bar;
mod info_bar_panel;
mod navigation;
mod number_box;
mod number_box_automation_peer;
mod number_box_expression;
mod page;
mod radio_buttons;
mod register_types;
mod symbol_icon;

pub use frame::{Frame, PageFactory};
pub use auto_suggest_box::{AutoSuggestBox, AutoSuggestBoxAutomationPeer, AutoSuggestBoxQuerySubmittedEventArgs, AutoSuggestBoxSuggestionChosenEventArgs, AutoSuggestBoxTextChangedEventArgs, AutoSuggestionBoxTextChangeReason};
pub use font_icon::FontIcon;
pub use icon_source::{FontIconSource, IconSource, IconSourceImpl, IconSourceImplExt, IconSourceVTable, PathIconSource, SymbolIconSource};
pub use info_bar::{InfoBar, InfoBarAutomationPeer, InfoBarClosedEventArgs, InfoBarCloseReason, InfoBarClosingEventArgs, InfoBarOpenedEventArgs, InfoBarSeverity, InfoBarTemplateSettings};
pub use info_bar_panel::InfoBarPanel;
pub use radio_buttons::{RadioButtons, RadioButtonsAutomationPeer};
pub use symbol_icon::{Symbol, SymbolIcon};
pub use navigation::{
    FrameNavigationOptions, NavigatingCancelEventArgs, NavigationCacheMode, NavigationError,
    NavigationEventArgs, NavigationFailedEventArgs, NavigationMode, NavigationStack, PageStackEntry,
};
pub use page::{Page, PageImpl, PageImplExt, PageVTable};
pub use number_box::{
    INumberBoxNumberFormatter, InvariantNumberBoxNumberFormatter, NumberBox,
    NumberBoxSpinButtonPlacementMode, NumberBoxValidationMode, NumberBoxValueChangedEventArgs,
};
pub use number_box_automation_peer::NumberBoxAutomationPeer;
pub use number_box_expression::{evaluate_number_expression, NumberBoxExpressionError};
pub(crate) use register_types::register_markup_types;
