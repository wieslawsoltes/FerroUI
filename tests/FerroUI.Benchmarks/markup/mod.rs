//! The benchmarks of markup.

use crate::harness::Registry;

pub mod parsing;

pub fn register(registry: &mut Registry) {
    parsing::register(registry);
}
