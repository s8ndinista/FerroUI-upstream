//! Opt-in WinUI contracts. Existing controls and their type identities stay unchanged.

mod frame;
mod font_icon;
mod navigation;
mod number_box;
mod number_box_automation_peer;
mod number_box_expression;
mod page;
mod radio_buttons;
mod symbol_icon;

pub use frame::{Frame, PageFactory};
pub use font_icon::FontIcon;
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
pub(crate) use navigation::register_markup_types;
