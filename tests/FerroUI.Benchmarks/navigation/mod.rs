//! The navigation benchmarks.

use crate::harness::Registry;

pub mod carousel_page_benchmark;
pub mod drawer_page_benchmark;
pub mod navigation_page_benchmark;
pub mod tabbed_page_benchmark;

pub fn register(registry: &mut Registry) {
    navigation_page_benchmark::register(registry);
    drawer_page_benchmark::register(registry);
    carousel_page_benchmark::register(registry);
    tabbed_page_benchmark::register(registry);
}
