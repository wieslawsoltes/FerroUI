//! The rendering benchmarks.

use crate::harness::Registry;

pub mod shape_rendering;
pub mod skia_drawing;

pub fn register(registry: &mut Registry) {
    shape_rendering::register(registry);
    skia_drawing::register(registry);
}
