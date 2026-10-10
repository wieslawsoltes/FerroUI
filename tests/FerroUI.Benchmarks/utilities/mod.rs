//! The benchmarks of the utilities.

use crate::harness::Registry;

pub mod ferro_property_dictionary_benchmarks;

pub fn register(registry: &mut Registry) {
    ferro_property_dictionary_benchmarks::register(registry);
}
