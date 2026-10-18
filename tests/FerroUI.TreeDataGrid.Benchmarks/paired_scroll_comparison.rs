//! `PairedScrollComparison.cs`: the one-row vertical scrolls of `VirtualizationBenchmarks`
//! (`vertical_row_scrolls`) for the classic and the neutral columns in turn, in one warmed
//! process, to control the drift between the separate runs of the two parameter values.
//!
//! Alternate APIs within one warmed process to control drift between isolated runs. Keep exactly
//! the existing `vertical_row_scrolls` workload and exclude setup and cleanup from timing.
//!
//! The reference collects the garbage before each sample and reads the bytes the thread
//! allocated; the port has no collector, and reads the bytes from the counting allocator of the
//! harness, which the feature `count-allocations` turns on (without it the bytes are `null`).
//! The five warm-up rounds (-5 to -1) are not recorded; the runner prints the samples of rounds 0
//! to 9 as JSON.

use crate::virtualization_benchmarks::{VirtualizationBenchmarks, VERTICAL_SCROLL_OPERATIONS};
use ferroui_benchmarks::harness::allocations;
use std::fmt::Write as _;
use std::time::Instant;

/// The stack of the thread of the runner: the trees of the grid are deep (the harness gives its
/// benchmarks a stack of the same size).
const STACK_SIZE: usize = 64 * 1024 * 1024;

/// One recorded sample.
struct Sample {
    round: i32,
    native: bool,
    microseconds: f64,
    allocated_bytes: Option<f64>,
    realized_rows: usize,
}

/// Runs the comparison on a thread of its own (the application, the dispatcher and the property
/// system belong to a thread) and prints the samples.
pub fn run() {
    let thread = std::thread::Builder::new()
        .name("paired-scroll-comparison".to_string())
        .stack_size(STACK_SIZE)
        .spawn(run_on_this_thread)
        .expect("the thread of the comparison starts");
    let json = thread.join().expect("the comparison runs");
    println!("{json}");
}

/// The comparison; returns the samples as JSON.
fn run_on_this_thread() -> String {
    let mut results = Vec::new();
    let mut benchmark = VirtualizationBenchmarks::new(false);

    for round in -5..10 {
        for order in 0..2 {
            benchmark.neutral_source = ((round + order) & 1) == 0;
            benchmark.setup_vertical_row_scrolls();

            let allocation = allocations::snapshot();
            let timer = Instant::now();
            let count = benchmark.vertical_row_scrolls();
            let elapsed = timer.elapsed();
            let allocation = allocations::snapshot().since(&allocation);

            if round >= 0 {
                let operations = f64::from(VERTICAL_SCROLL_OPERATIONS);
                results.push(Sample {
                    round,
                    native: benchmark.neutral_source,
                    microseconds: elapsed.as_secs_f64() * 1e6 / operations,
                    allocated_bytes: allocations::ENABLED.then(|| allocation.bytes as f64 / operations),
                    realized_rows: count,
                });
            }

            benchmark.cleanup_vertical_row_scrolls();
        }
    }

    benchmark.global_cleanup();
    to_json(&results)
}

/// `JsonSerializer.Serialize(results, new JsonSerializerOptions { WriteIndented = true })`.
fn to_json(results: &[Sample]) -> String {
    let mut out = String::from("[");
    for (index, sample) in results.iter().enumerate() {
        let separator = if index == 0 { "" } else { "," };
        let allocated = sample.allocated_bytes.map_or("null".to_string(), |bytes| bytes.to_string());
        let _ = write!(
            out,
            "{separator}\n  {{\n    \"Round\": {},\n    \"Native\": {},\n    \"Microseconds\": {},\n    \"AllocatedBytes\": {},\n    \"RealizedRows\": {}\n  }}",
            sample.round, sample.native, sample.microseconds, allocated, sample.realized_rows
        );
    }
    out.push_str(if results.is_empty() { "]" } else { "\n]" });
    out
}
