//! The benchmarks of markup.

use crate::harness::Registry;

pub mod compiled_handlers;
pub mod parsing;

pub fn register(registry: &mut Registry) {
    compiled_handlers::register(registry);
    parsing::register(registry);
}
