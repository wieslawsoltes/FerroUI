//! The text benchmarks.

use crate::harness::Registry;

pub mod codepoint_benchmark;
pub mod huge_text_layout;
pub mod shaped_buffer_ops;
pub mod text_layout_profile;
pub mod text_run_cache_benchmark;
pub mod unicode_break_enumerator_benchmark;
pub mod unicode_segmentation_scaling_benchmark;

mod random;

pub fn register(registry: &mut Registry) {
    text_run_cache_benchmark::register(registry);
    unicode_segmentation_scaling_benchmark::register(registry);
    unicode_break_enumerator_benchmark::register(registry);
    huge_text_layout::register(registry);
    shaped_buffer_ops::register(registry);
    text_layout_profile::register(registry);
    codepoint_benchmark::register(registry);
}
