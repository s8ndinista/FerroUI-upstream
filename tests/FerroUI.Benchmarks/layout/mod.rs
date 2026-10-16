//! The layout benchmarks.

use crate::harness::Registry;

pub mod controls_benchmark;
pub mod measure;

pub fn register(registry: &mut Registry) {
    controls_benchmark::register(registry);
    measure::register(registry);
}
