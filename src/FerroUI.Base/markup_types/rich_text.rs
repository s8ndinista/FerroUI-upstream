//! Registration of the rich paragraph formatter's declared markup types.

use crate::data::core::ValueTypes;
use crate::media::text_formatting::{TabAlignment, TabLeader, TextTabStops};
use crate::metadata::{MarkupType, MarkupTyped};

pub(super) const TYPES: &[&MarkupType] = &[
    <TabAlignment as MarkupTyped>::MARKUP,
    <TabLeader as MarkupTyped>::MARKUP,
    <TextTabStops as MarkupTyped>::MARKUP,
];

pub(super) fn register_value_types() {
    ValueTypes::register_nullable::<TabAlignment>();
    ValueTypes::register_nullable::<TabLeader>();
    ValueTypes::register_nullable::<TextTabStops>();
}
