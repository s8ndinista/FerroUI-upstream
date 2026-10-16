//! The compositor benchmarks.

use crate::harness::Registry;

pub mod composition_target_update;

pub fn register(registry: &mut Registry) {
    composition_target_update::register(registry);
}
