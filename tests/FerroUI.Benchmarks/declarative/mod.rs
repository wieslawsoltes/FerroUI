//! The benchmarks of the declarative API (`ferroui-view`): an addition, with
//! no upstream counterpart (`docs/declarative-ui.md`, section 12, phase 5).

pub mod view_benchmark;

use crate::harness::Registry;

pub fn register(registry: &mut Registry) {
    view_benchmark::register(registry);
}
