//! `Program.cs`: the entry points of the benchmarks, as functions and as ignored tests.
//!
//! The reference hands its command line to the switcher of its benchmark library, or, with
//! `--paired-vertical-scroll` or `--compare-scroll-revisions <paths>`, runs one of its two
//! comparison runners instead. Here every file registers its benchmarks ([`crate::register`])
//! and the environment selects among them (see `ferroui_benchmarks::harness`); the runners are
//! ignored tests of their own:
//!
//! ```sh
//! cargo test -p ferroui-treedatagrid-benchmarks --release --lib run_benchmarks -- --ignored --nocapture --test-threads=1
//! FERROUI_BENCH_FILTER=VirtualizationBenchmarks.vertical_row_scrolls \
//!     cargo test -p ferroui-treedatagrid-benchmarks --release --lib run_benchmarks -- --ignored --nocapture --test-threads=1
//! cargo test -p ferroui-treedatagrid-benchmarks --release --lib paired_vertical_scroll -- --ignored --nocapture --test-threads=1
//! cargo test -p ferroui-treedatagrid-benchmarks --release --lib compare_scroll_revisions -- --ignored --nocapture --test-threads=1
//! ```

use ferroui_benchmarks::harness;

/// Measures the benchmarks the environment selects (`FERROUI_BENCH_FILTER` and the other
/// variables of the harness) and prints the table of the results.
pub fn run() {
    harness::run(crate::register);
}

/// `--paired-vertical-scroll`: see [`crate::paired_scroll_comparison`].
pub fn paired_vertical_scroll() {
    crate::paired_scroll_comparison::run();
}

/// `--compare-scroll-revisions`: see [`crate::revision_scroll_comparison`].
pub fn compare_scroll_revisions() {
    crate::revision_scroll_comparison::run();
}

#[cfg(test)]
mod tests {
    use ferroui_benchmarks::harness::Registry;
    use std::collections::HashSet;

    #[test]
    #[ignore = "measures every benchmark the environment selects; run it alone in an optimised build"]
    fn run_benchmarks() {
        super::run();
    }

    #[test]
    #[ignore = "scrolls the grid for the classic and the neutral columns in turn for fifteen rounds; run it alone in an optimised build"]
    fn paired_vertical_scroll() {
        super::paired_vertical_scroll();
    }

    #[test]
    #[ignore = "runs four scrolling workloads for twelve rounds; run it alone in an optimised build"]
    fn compare_scroll_revisions() {
        super::compare_scroll_revisions();
    }

    #[test]
    fn every_benchmark_has_a_name_of_its_own() {
        let mut registry = Registry::new();
        crate::register(&mut registry);
        assert!(!registry.is_empty(), "the crate registers benchmarks");

        let mut names = HashSet::new();
        for descriptor in registry.descriptors() {
            assert!(descriptor.operations_per_invoke > 0);
            assert!(names.insert(descriptor.full_name()), "{} is registered twice", descriptor.full_name());
        }
    }
}
