//! ferroui-treedatagrid-benchmarks
//!
//! The benchmarks of the TreeDataGrid control (the counterpart of the benchmark project of the
//! reference's TreeDataGrid): the geometry and the width commits of the column list, the recycle
//! pool of the element factory, the recycling of template cells, the sources and their
//! presentation without a control, the first layout of a grid, and the layout of a virtualizing
//! grid while it scrolls, while its items change and while it is detached and reattached.
//!
//! One file per reference file, in snake case. Each benchmark class is a struct whose constructor
//! is the constructor and the global setup of the class and whose methods are its benchmarks, and
//! each file registers its benchmarks with the harness of the framework's benchmarks
//! (`ferroui_benchmarks::harness`) under the snake case of the reference's names, with one test
//! per class that runs every benchmark of the class once. The two comparison runners of the
//! reference ([`paired_scroll_comparison`], [`revision_scroll_comparison`]) run the workloads of
//! `VirtualizationBenchmarks` outside the harness.
//!
//! See `README.md` for how to run the benchmarks and what differs from the reference.

pub mod column_geometry_benchmarks;
pub mod column_layout_benchmarks;
pub mod column_width_batch_benchmarks;
pub mod element_factory_benchmarks;
pub mod first_layout_benchmarks;
pub mod neutral_source_benchmarks;
pub mod template_cell_recycling_benchmarks;
pub mod virtualization_benchmarks;

pub mod paired_scroll_comparison;
pub mod revision_scroll_comparison;

mod program;

pub use program::{compare_scroll_revisions, paired_vertical_scroll, run};

use ferroui_benchmarks::harness::Registry;

/// Registers every benchmark of the crate, class by class.
pub fn register(registry: &mut Registry) {
    column_geometry_benchmarks::register(registry);
    column_layout_benchmarks::register(registry);
    column_width_batch_benchmarks::register(registry);
    element_factory_benchmarks::register(registry);
    first_layout_benchmarks::register(registry);
    neutral_source_benchmarks::register(registry);
    template_cell_recycling_benchmarks::register(registry);
    virtualization_benchmarks::register(registry);
}
