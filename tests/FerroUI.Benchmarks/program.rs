//! The entry points of the benchmarks: what the main function of the
//! upstream benchmark project does, as functions and as ignored tests.
//!
//! Upstream collects every benchmark class and hands the command line to
//! the switcher of its benchmark library; here every file registers its
//! benchmarks ([`crate::register`]) and the environment selects among them
//! (see [`crate::harness`]):
//!
//! ```sh
//! cargo test -p ferroui-benchmarks --release --lib run_benchmarks -- --ignored --nocapture --test-threads=1
//! FERROUI_BENCH_FILTER=styling::,ControlsBenchmark cargo test -p ferroui-benchmarks --release --lib run_benchmarks -- --ignored --nocapture --test-threads=1
//! ```
//!
//! The profiling entry of upstream (`--profile-textlayout`) is the ignored
//! test `profile_text_layout`: it builds text layouts for a fixed time
//! without the harness, for a sampling profiler to attach to.

use crate::harness;

/// Measures the benchmarks the environment selects and prints the table of
/// the results.
pub fn run() {
    harness::run(crate::register);
}

/// Builds text layouts for a fixed time; see
/// [`crate::text::text_layout_profile`].
pub fn profile_text_layout() {
    crate::text::text_layout_profile::profile_text_layout();
}

#[cfg(test)]
mod tests {
    use crate::harness::Registry;
    use std::collections::HashSet;

    #[test]
    #[ignore = "measures every benchmark the environment selects; run it alone in an optimised build"]
    fn run_benchmarks() {
        super::run();
    }

    #[test]
    #[ignore = "builds text layouts for eighteen seconds, for a profiler"]
    fn profile_text_layout() {
        super::profile_text_layout();
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
