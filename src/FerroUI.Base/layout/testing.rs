//! The members of layout the benchmarks of the framework reach: upstream's
//! layout benchmark sets the flags of a layoutable through accessors of their
//! private setters (`UnsafeAccessor`), so that the time of invalidating a
//! control is not measured.

use super::Layoutable;

/// The private setter of `Layoutable.IsMeasureValid`: sets the flag alone,
/// without invalidating anything.
pub fn set_is_measure_valid(layoutable: &Layoutable, value: bool) {
    layoutable.set_is_measure_valid(value);
}

/// The private setter of `Layoutable.IsArrangeValid`: sets the flag alone,
/// without invalidating anything.
pub fn set_is_arrange_valid(layoutable: &Layoutable, value: bool) {
    layoutable.set_is_arrange_valid(value);
}
