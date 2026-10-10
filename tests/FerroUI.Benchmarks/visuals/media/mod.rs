//! The benchmarks of the media of visuals.

use crate::harness::Registry;

pub mod path_markup_parser_tests;

pub fn register(registry: &mut Registry) {
    path_markup_parser_tests::register(registry);
}
