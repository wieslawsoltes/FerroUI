//! The benchmarks of the themes.

use crate::harness::Registry;

pub mod fluent_benchmark;
pub mod theme_benchmark;

pub fn register(registry: &mut Registry) {
    fluent_benchmark::register(registry);
    theme_benchmark::register(registry);
}
