//! The benchmarks of visuals.

use crate::harness::Registry;

pub mod matrix_benchmarks;
pub mod media;
pub mod visual_affects_render_benchmarks;

pub fn register(registry: &mut Registry) {
    matrix_benchmarks::register(registry);
    visual_affects_render_benchmarks::register(registry);
    media::register(registry);
}
