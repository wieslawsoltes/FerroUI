//! The animation benchmarks.

use crate::harness::Registry;

pub mod transition_benchmark;

pub fn register(registry: &mut Registry) {
    transition_benchmark::register(registry);
}
