//! `RevisionScrollComparison.cs`: the scrolling workloads of `VirtualizationBenchmarks` for the
//! neutral columns (one-row vertical scrolls, one-pixel horizontal scrolls, column scrolls over
//! 200 columns and column scrolls near column 900 of 1 000), in rounds, with setup and cleanup
//! outside the timing.
//!
//! The reference loads several builds of its benchmark assembly, each with its dependencies, into
//! isolated load contexts of one process, and rotates the order of the builds from round to
//! round to reduce the drift of machine load and warm-up between them. A Rust process cannot load
//! another build of the crate it is linked with, so the port runs the rounds of the build it is:
//! the list of revisions holds this build alone (its executable as the path), and two revisions
//! are compared by running this in each build on the same machine and comparing the samples. The
//! reference also records the processor time of the process; the standard library has no clock
//! of processor time, so the port records the elapsed time only. The bytes come from the counting
//! allocator of the harness, which the feature `count-allocations` turns on (without it they are
//! `null`); the reference collects the garbage before each sample, which the port has no need of.

use crate::virtualization_benchmarks::{
    VirtualizationBenchmarks, HORIZONTAL_COLUMN_OPERATIONS, HORIZONTAL_SCROLL_OPERATIONS, VERTICAL_SCROLL_OPERATIONS,
};
use ferroui_benchmarks::harness::allocations;
use std::fmt::Write as _;
use std::time::Instant;

/// The stack of the thread of the runner: the trees of the grid are deep (the harness gives its
/// benchmarks a stack of the same size).
const STACK_SIZE: usize = 64 * 1024 * 1024;

/// The workloads, by the names of their methods, with the operations of an invocation.
const WORKLOADS: [(&str, u32); 4] = [
    ("VerticalRowScrolls", VERTICAL_SCROLL_OPERATIONS),
    ("HorizontalSmallScrolls", HORIZONTAL_SCROLL_OPERATIONS),
    ("HorizontalColumnScrolls", HORIZONTAL_COLUMN_OPERATIONS),
    ("HorizontalFarColumnScrolls", HORIZONTAL_COLUMN_OPERATIONS),
];

/// A revision: a build and the benchmark of its workloads, with the neutral columns.
struct Revision {
    path: String,
    benchmark: VirtualizationBenchmarks,
}

impl Revision {
    fn new(path: String) -> Self {
        // `NeutralSource = true`, then `GlobalSetup`.
        Self { path, benchmark: VirtualizationBenchmarks::new(true) }
    }

    /// `Invoke("Setup" + workload)`.
    fn setup(&mut self, workload: &str) {
        match workload {
            "VerticalRowScrolls" => self.benchmark.setup_vertical_row_scrolls(),
            "HorizontalSmallScrolls" => self.benchmark.setup_horizontal_small_scrolls(),
            "HorizontalColumnScrolls" => self.benchmark.setup_horizontal_column_scrolls(),
            "HorizontalFarColumnScrolls" => self.benchmark.setup_horizontal_far_column_scrolls(),
            _ => panic!("no workload {workload}"),
        }
    }

    /// `Invoke(workload)`: the realized rows.
    fn invoke(&self, workload: &str) -> usize {
        match workload {
            "VerticalRowScrolls" => self.benchmark.vertical_row_scrolls(),
            "HorizontalSmallScrolls" => self.benchmark.horizontal_small_scrolls(),
            "HorizontalColumnScrolls" => self.benchmark.horizontal_column_scrolls(),
            "HorizontalFarColumnScrolls" => self.benchmark.horizontal_far_column_scrolls(),
            _ => panic!("no workload {workload}"),
        }
    }

    /// `Invoke("Cleanup" + workload)`.
    fn cleanup(&mut self, workload: &str) {
        match workload {
            "VerticalRowScrolls" => self.benchmark.cleanup_vertical_row_scrolls(),
            "HorizontalSmallScrolls" => self.benchmark.cleanup_horizontal_small_scrolls(),
            "HorizontalColumnScrolls" => self.benchmark.cleanup_horizontal_column_scrolls(),
            "HorizontalFarColumnScrolls" => self.benchmark.cleanup_horizontal_far_column_scrolls(),
            _ => panic!("no workload {workload}"),
        }
    }
}

/// One recorded sample.
struct Sample {
    workload: &'static str,
    round: i32,
    revision: usize,
    path: String,
    microseconds: f64,
    allocated_bytes: Option<f64>,
    realized_rows: usize,
}

/// Runs the comparison on a thread of its own (the application, the dispatcher and the property
/// system belong to a thread) and prints the samples.
pub fn run() {
    let thread = std::thread::Builder::new()
        .name("revision-scroll-comparison".to_string())
        .stack_size(STACK_SIZE)
        .spawn(run_on_this_thread)
        .expect("the thread of the comparison starts");
    let json = thread.join().expect("the comparison runs");
    println!("{json}");
}

/// The comparison; returns the samples as JSON.
fn run_on_this_thread() -> String {
    let path = std::env::current_exe().map(|path| path.display().to_string()).unwrap_or_default();
    let mut revisions = vec![Revision::new(path)];
    let mut results = Vec::new();

    for (workload, operations) in WORKLOADS {
        for round in -3..9 {
            for order in 0..revisions.len() {
                let index = (round + 3 + order as i32) as usize % revisions.len();
                let revision = &mut revisions[index];
                revision.setup(workload);

                let allocated = allocations::snapshot();
                let timer = Instant::now();
                let rows = revision.invoke(workload);
                let elapsed = timer.elapsed();
                let allocated = allocations::snapshot().since(&allocated);

                if round >= 0 {
                    let operations = f64::from(operations);
                    results.push(Sample {
                        workload,
                        round,
                        revision: index,
                        path: revision.path.clone(),
                        microseconds: elapsed.as_secs_f64() * 1e6 / operations,
                        allocated_bytes: allocations::ENABLED.then(|| allocated.bytes as f64 / operations),
                        realized_rows: rows,
                    });
                }

                revision.cleanup(workload);
            }
        }
    }

    for revision in &mut revisions {
        revision.benchmark.global_cleanup();
    }

    to_json(&results)
}

/// A JSON string literal.
fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// `JsonSerializer.Serialize(results, new JsonSerializerOptions { WriteIndented = true })`.
fn to_json(results: &[Sample]) -> String {
    let mut out = String::from("[");
    for (index, sample) in results.iter().enumerate() {
        let separator = if index == 0 { "" } else { "," };
        let allocated = sample.allocated_bytes.map_or("null".to_string(), |bytes| bytes.to_string());
        let _ = write!(
            out,
            "{separator}\n  {{\n    \"Workload\": {},\n    \"Round\": {},\n    \"Revision\": {},\n    \"Path\": {},\n    \"Microseconds\": {},\n    \"AllocatedBytes\": {},\n    \"RealizedRows\": {}\n  }}",
            json_string(sample.workload),
            sample.round,
            sample.revision,
            json_string(&sample.path),
            sample.microseconds,
            allocated,
            sample.realized_rows
        );
    }
    out.push_str(if results.is_empty() { "]" } else { "\n]" });
    out
}
