//! The rendering benchmarks.

use crate::harness::Registry;

pub mod band_replay;
pub mod shape_rendering;

pub fn register(registry: &mut Registry) {
    band_replay::register(registry);
    shape_rendering::register(registry);
}
