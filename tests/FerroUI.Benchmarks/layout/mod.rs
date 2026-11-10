//! The layout benchmarks.

use crate::harness::Registry;

pub mod compute_apply_spike;
pub mod controls_benchmark;
pub mod element_construction_benchmark;
pub mod measure;

pub fn register(registry: &mut Registry) {
    compute_apply_spike::register(registry);
    controls_benchmark::register(registry);
    element_construction_benchmark::register(registry);
    measure::register(registry);
}
