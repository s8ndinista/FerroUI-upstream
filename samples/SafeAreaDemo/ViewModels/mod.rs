//! The view models of the sample (namespace `SafeAreaDemo.ViewModels`): one module per upstream
//! file.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{from_markup_value, MarkupType, MarkupTyped};
use ferroui_base::{BoxedValue, TypeInfo};
use std::rc::Rc;

mod main_view_model;

pub use main_view_model::MainViewModel;

/// The classes of this namespace (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[];

/// The types of this namespace declared with `ferro_markup_type!` (`<X as MarkupTyped>::MARKUP`).
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[<MainViewModel as MarkupTyped>::MARKUP];

/// What the untyped value conversions must know about the types of this namespace.
pub(crate) fn register_value_types() {
    ValueTypes::register_reference::<MainViewModel>();
}

/// `data is ViewModelBase`: whether the value is a view model of the sample. A view model of
/// the port embeds `ViewModelBase` instead of deriving from it, so the view models that do are
/// listed here: the one of the sample, [`MainViewModel`].
pub(crate) fn is_view_model_base(data: &BoxedValue) -> bool {
    from_markup_value::<Rc<MainViewModel>>(&Some(data.clone())).is_some()
}
