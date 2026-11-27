//! The rendering benchmarks.

use crate::harness::Registry;

pub mod shape_rendering;

pub fn register(registry: &mut Registry) {
    shape_rendering::register(registry);
}
