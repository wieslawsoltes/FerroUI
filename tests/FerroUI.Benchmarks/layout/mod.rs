//! The layout benchmarks.
//!
//! The upstream benchmark of measuring a tree again (`Measure.Remeasure`)
//! is not here: it marks every control of the tree as not measured and not
//! arranged through the private setters of the two flags of a layoutable,
//! so that the time of invalidating is not measured, and a layoutable of
//! the framework has no member that sets those flags.

use crate::harness::Registry;

pub mod controls_benchmark;

pub fn register(registry: &mut Registry) {
    controls_benchmark::register(registry);
}
