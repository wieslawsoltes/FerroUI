//! The harness of the benchmarks: a registry of benchmarks (class, name,
//! parameters, setup, body) and a runner that measures them with the clock
//! of the standard library and prints a table.
//!
//! Not a port: the upstream project measures with a benchmark library of its
//! platform. What the attributes of that library state is kept in the
//! registry:
//!
//! | Upstream | Here |
//! |---|---|
//! | a benchmark class | [`Registry::class`], one struct per class |
//! | the constructor and `[GlobalSetup]` | the `setup` closure of a benchmark, which returns the state |
//! | `[GlobalSetup(Target = ..)]` | the `setup` closure of that benchmark alone |
//! | `[GlobalCleanup]`, `Dispose` | the state is dropped after the last iteration |
//! | `[IterationSetup]` | [`Class::benchmark_with_iteration_setup`] (one invocation an iteration, as upstream) |
//! | `[Params]`, `[Arguments]` | one registered benchmark per value, with the values in its parameters text |
//! | `[Benchmark]` | [`Class::benchmark`], named with the snake case of the upstream method |
//! | `[Benchmark(Baseline = true)]` | [`Descriptor::baseline`] (the table has a ratio column) |
//! | `[Benchmark(OperationsPerInvoke = n)]` | [`Descriptor::operations_per_invoke`] |
//! | `[MinIterationTime(ms)]` | [`Class::min_iteration_time`] |
//! | `[MemoryDiagnoser]` | the feature `count-allocations` (bytes and blocks an operation) |
//!
//! Every benchmark runs on a thread of its own: the property system, the
//! dispatcher and the services of the framework belong to a thread, so a
//! benchmark starts from the state a new process has, as upstream, where
//! each benchmark runs in a process of its own.
//!
//! # Measuring
//!
//! [`run`] reads the environment:
//!
//! - `FERROUI_BENCH_FILTER`: parts of benchmark names separated by commas; a
//!   benchmark runs when its full name (`category::Class.name[parameters]`)
//!   contains one of them. Without it every benchmark runs.
//! - `FERROUI_BENCH_LIST`: when set, the names are printed and nothing runs.
//! - `FERROUI_BENCH_ITERATIONS` (15): the measured iterations of a benchmark.
//! - `FERROUI_BENCH_WARMUP` (3): the iterations before them.
//! - `FERROUI_BENCH_MIN_ITERATION_MS`: the least time of an iteration, in
//!   place of what the class states (500 ms unless it states another).
//!
//! An iteration invokes the body as many times as a pilot run found to fill
//! the least time of an iteration; a benchmark with an iteration setup
//! invokes it once. The time of an operation is the time of the iteration
//! over its invocations and the operations of an invocation.
//!
//! # Smoke tests
//!
//! [`smoke_class`] runs the setup, the iteration setup and one invocation of
//! every benchmark of a class, for every parameter set; each file has one
//! test per class that calls it, so a benchmark that no longer builds its
//! scenario fails `cargo test`.

use std::fmt::Write as _;
use std::hint::black_box;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// A function that registers benchmarks: every file has one (`register`),
/// and the crate has one for all of them.
pub type RegisterFn = fn(&mut Registry);

/// The least time of an iteration when a class states none.
const DEFAULT_MIN_ITERATION_TIME: Duration = Duration::from_millis(500);

/// The stack of the thread of a benchmark: the trees of some scenarios are
/// hundreds of levels deep.
const STACK_SIZE: usize = 64 * 1024 * 1024;

/// What is known of a benchmark without running it.
#[derive(Clone, Debug)]
pub struct Descriptor {
    /// The folder of the file (`layout`, `styling`, ...).
    pub category: &'static str,
    /// The benchmark class.
    pub class: &'static str,
    /// The benchmark: the snake case of the upstream method.
    pub name: &'static str,
    /// The parameter values, as `Name=value` pairs separated by commas;
    /// empty for a benchmark without parameters.
    pub parameters: String,
    /// Whether the other benchmarks of the class are compared with this one.
    pub baseline: bool,
    /// The operations one invocation of the body performs.
    pub operations_per_invoke: u32,
    /// The least time of an iteration, when the class states one.
    pub min_iteration_time: Option<Duration>,
    /// Whether the benchmark has an iteration setup, and so invokes its body
    /// once an iteration.
    pub has_iteration_setup: bool,
}

impl Descriptor {
    /// Makes this benchmark the baseline of its class.
    pub fn baseline(&mut self) -> &mut Self {
        self.baseline = true;
        self
    }

    /// States how many operations one invocation of the body performs.
    pub fn operations_per_invoke(&mut self, operations: u32) -> &mut Self {
        assert!(operations > 0, "an invocation performs at least one operation");
        self.operations_per_invoke = operations;
        self
    }

    /// `category::Class.name`, with `[parameters]` when there are any.
    pub fn full_name(&self) -> String {
        if self.parameters.is_empty() {
            format!("{}::{}.{}", self.category, self.class, self.name)
        } else {
            format!("{}::{}.{}[{}]", self.category, self.class, self.name, self.parameters)
        }
    }
}

/// A benchmark whose state exists: what the runner invokes.
trait Instance {
    fn iteration_setup(&mut self);
    fn invoke(&mut self);
}

struct TypedInstance<S> {
    state: S,
    iteration_setup: Option<Rc<dyn Fn(&mut S)>>,
    body: Rc<dyn Fn(&mut S)>,
}

impl<S> Instance for TypedInstance<S> {
    fn iteration_setup(&mut self) {
        if let Some(iteration_setup) = &self.iteration_setup {
            iteration_setup(&mut self.state);
        }
    }

    fn invoke(&mut self) {
        (self.body)(&mut self.state);
    }
}

struct Entry {
    descriptor: Descriptor,
    create: Box<dyn Fn() -> Box<dyn Instance>>,
}

/// The benchmarks that were registered, in the order of registration.
#[derive(Default)]
pub struct Registry {
    entries: Vec<Entry>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts the registration of the benchmarks of a class.
    pub fn class(&mut self, category: &'static str, class: &'static str) -> Class<'_> {
        Class { registry: self, category, class, min_iteration_time: None }
    }

    /// What is known of every registered benchmark.
    pub fn descriptors(&self) -> impl Iterator<Item = &Descriptor> {
        self.entries.iter().map(|entry| &entry.descriptor)
    }

    /// The number of registered benchmarks.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no benchmark is registered.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// The registration of the benchmarks of one class.
pub struct Class<'a> {
    registry: &'a mut Registry,
    category: &'static str,
    class: &'static str,
    min_iteration_time: Option<Duration>,
}

impl Class<'_> {
    /// States the least time of an iteration of the benchmarks registered
    /// after this call, in milliseconds.
    pub fn min_iteration_time(mut self, milliseconds: u64) -> Self {
        self.min_iteration_time = Some(Duration::from_millis(milliseconds));
        self
    }

    /// Registers a benchmark.
    ///
    /// `setup` creates the state: what the constructor and the global setup
    /// of the class do. It runs once, outside the measurement, and the state
    /// is dropped after the last iteration. `body` is the benchmark; what it
    /// returns is kept from the optimiser and dropped.
    pub fn benchmark<S: 'static, R>(
        &mut self,
        name: &'static str,
        parameters: impl Into<String>,
        setup: impl Fn() -> S + 'static,
        body: impl Fn(&mut S) -> R + 'static,
    ) -> &mut Descriptor {
        self.add(name, parameters.into(), setup, None, body)
    }

    /// Registers a benchmark with an iteration setup, which runs before
    /// every invocation of the body, outside the measurement. Such a
    /// benchmark invokes its body once an iteration.
    pub fn benchmark_with_iteration_setup<S: 'static, R>(
        &mut self,
        name: &'static str,
        parameters: impl Into<String>,
        setup: impl Fn() -> S + 'static,
        iteration_setup: impl Fn(&mut S) + 'static,
        body: impl Fn(&mut S) -> R + 'static,
    ) -> &mut Descriptor {
        let iteration_setup: Rc<dyn Fn(&mut S)> = Rc::new(iteration_setup);
        self.add(name, parameters.into(), setup, Some(iteration_setup), body)
    }

    fn add<S: 'static, R>(
        &mut self,
        name: &'static str,
        parameters: String,
        setup: impl Fn() -> S + 'static,
        iteration_setup: Option<Rc<dyn Fn(&mut S)>>,
        body: impl Fn(&mut S) -> R + 'static,
    ) -> &mut Descriptor {
        let descriptor = Descriptor {
            category: self.category,
            class: self.class,
            name,
            parameters,
            baseline: false,
            operations_per_invoke: 1,
            min_iteration_time: self.min_iteration_time,
            has_iteration_setup: iteration_setup.is_some(),
        };
        let body: Rc<dyn Fn(&mut S)> = Rc::new(move |state: &mut S| {
            black_box(body(state));
        });
        let create: Box<dyn Fn() -> Box<dyn Instance>> = Box::new(move || -> Box<dyn Instance> {
            Box::new(TypedInstance { state: setup(), iteration_setup: iteration_setup.clone(), body: body.clone() })
        });
        self.registry.entries.push(Entry { descriptor, create });
        &mut self.registry.entries.last_mut().expect("an entry was pushed").descriptor
    }
}

/// How [`run`] measures; see the module documentation for the environment.
#[derive(Clone, Debug)]
pub struct Options {
    pub filter: Vec<String>,
    pub list: bool,
    pub iterations: usize,
    pub warmup: usize,
    pub min_iteration_time: Option<Duration>,
}

impl Options {
    /// The options the environment states.
    pub fn from_environment() -> Self {
        fn number(name: &str) -> Option<u64> {
            std::env::var(name).ok().and_then(|value| value.trim().parse::<u64>().ok())
        }

        let filter = std::env::var("FERROUI_BENCH_FILTER")
            .map(|value| {
                value.split(',').map(str::trim).filter(|part| !part.is_empty()).map(str::to_string).collect()
            })
            .unwrap_or_default();
        Self {
            filter,
            list: std::env::var_os("FERROUI_BENCH_LIST").is_some(),
            iterations: number("FERROUI_BENCH_ITERATIONS").map_or(15, |value| value.max(1) as usize),
            warmup: number("FERROUI_BENCH_WARMUP").map_or(3, |value| value as usize),
            min_iteration_time: number("FERROUI_BENCH_MIN_ITERATION_MS").map(Duration::from_millis),
        }
    }

    fn matches(&self, descriptor: &Descriptor) -> bool {
        if self.filter.is_empty() {
            return true;
        }
        let name = descriptor.full_name();
        self.filter.iter().any(|part| name.contains(part.as_str()))
    }
}

/// What the runner measured for one benchmark.
#[derive(Clone, Debug)]
pub struct Measurement {
    /// The invocations of the body in one iteration.
    pub invocations: u64,
    /// The nanoseconds of an operation, one value per measured iteration.
    pub nanoseconds: Vec<f64>,
    /// The bytes an operation asked the allocator for, with the feature
    /// `count-allocations`.
    pub allocated_bytes: Option<f64>,
    /// The blocks an operation allocated, with the feature
    /// `count-allocations`.
    pub allocations: Option<f64>,
}

impl Measurement {
    pub fn mean(&self) -> f64 {
        self.nanoseconds.iter().sum::<f64>() / self.nanoseconds.len() as f64
    }

    pub fn median(&self) -> f64 {
        let mut sorted = self.nanoseconds.clone();
        sorted.sort_by(|a, b| a.total_cmp(b));
        let middle = sorted.len() / 2;
        if sorted.len() % 2 == 0 {
            (sorted[middle - 1] + sorted[middle]) / 2.0
        } else {
            sorted[middle]
        }
    }

    pub fn min(&self) -> f64 {
        self.nanoseconds.iter().copied().fold(f64::INFINITY, f64::min)
    }

    pub fn standard_deviation(&self) -> f64 {
        if self.nanoseconds.len() < 2 {
            return 0.0;
        }
        let mean = self.mean();
        let sum: f64 = self.nanoseconds.iter().map(|value| (value - mean) * (value - mean)).sum();
        (sum / (self.nanoseconds.len() - 1) as f64).sqrt()
    }
}

/// Runs the setup, the iteration setup and one invocation of the benchmark,
/// then drops its state.
fn run_once(entry: &Entry) {
    let mut instance = (entry.create)();
    instance.iteration_setup();
    instance.invoke();
}

/// Times `invocations` invocations of the body.
fn time_iteration(instance: &mut dyn Instance, invocations: u64, single: bool) -> (Duration, allocations::Counts) {
    if single {
        instance.iteration_setup();
    }
    let allocated = allocations::snapshot();
    let start = Instant::now();
    for _ in 0..invocations {
        instance.invoke();
    }
    let elapsed = start.elapsed();
    (elapsed, allocations::snapshot().since(&allocated))
}

fn measure(entry: &Entry, options: &Options) -> Measurement {
    let descriptor = &entry.descriptor;
    let single = descriptor.has_iteration_setup;
    let min_iteration_time =
        options.min_iteration_time.or(descriptor.min_iteration_time).unwrap_or(DEFAULT_MIN_ITERATION_TIME);
    let mut instance = (entry.create)();

    // The pilot: the invocations that fill the least time of an iteration.
    let mut invocations = 1_u64;
    if !single {
        loop {
            let (elapsed, _) = time_iteration(instance.as_mut(), invocations, false);
            if elapsed >= min_iteration_time || invocations >= 1 << 40 {
                break;
            }
            let needed = if elapsed.is_zero() {
                invocations * 16
            } else {
                let scale = min_iteration_time.as_secs_f64() / elapsed.as_secs_f64();
                ((invocations as f64 * scale * 1.1).ceil() as u64).min(invocations * 16)
            };
            invocations = needed.max(invocations + 1);
        }
    }

    for _ in 0..options.warmup {
        time_iteration(instance.as_mut(), invocations, single);
    }

    let operations = (invocations * u64::from(descriptor.operations_per_invoke)) as f64;
    let mut nanoseconds = Vec::with_capacity(options.iterations);
    let mut allocated = allocations::Counts::default();
    for _ in 0..options.iterations {
        let (elapsed, counts) = time_iteration(instance.as_mut(), invocations, single);
        nanoseconds.push(elapsed.as_secs_f64() * 1e9 / operations);
        allocated.add(&counts);
    }

    let all_operations = operations * options.iterations as f64;
    Measurement {
        invocations,
        nanoseconds,
        allocated_bytes: allocations::ENABLED.then(|| allocated.bytes as f64 / all_operations),
        allocations: allocations::ENABLED.then(|| allocated.allocations as f64 / all_operations),
    }
}

/// Runs `action` for the benchmark at `index` of what `register` registers,
/// on a thread of its own, and gives what it returned or the message of its
/// panic.
fn on_benchmark_thread<T: Send + 'static>(
    register: RegisterFn,
    index: usize,
    action: impl FnOnce(&Entry) -> T + Send + 'static,
) -> Result<T, String> {
    let thread = std::thread::Builder::new()
        .name(format!("benchmark-{index}"))
        .stack_size(STACK_SIZE)
        .spawn(move || {
            let mut registry = Registry::new();
            register(&mut registry);
            action(&registry.entries[index])
        })
        .expect("the thread of a benchmark starts");
    thread.join().map_err(|panic| {
        if let Some(message) = panic.downcast_ref::<&str>() {
            (*message).to_string()
        } else if let Some(message) = panic.downcast_ref::<String>() {
            message.clone()
        } else {
            "the benchmark panicked".to_string()
        }
    })
}

fn descriptors_of(register: RegisterFn) -> Vec<Descriptor> {
    let mut registry = Registry::new();
    register(&mut registry);
    registry.descriptors().cloned().collect()
}

/// Runs every benchmark of `class` among what `register` registers once:
/// the setup, the iteration setup and one invocation of the body, for every
/// parameter set.
///
/// # Panics
///
/// Panics when the class has no benchmark, and with the names of the
/// benchmarks that panicked.
pub fn smoke_class(register: RegisterFn, class: &str) {
    let descriptors = descriptors_of(register);
    let mut ran = 0;
    let mut failures = String::new();
    for (index, descriptor) in descriptors.iter().enumerate() {
        if descriptor.class != class {
            continue;
        }
        ran += 1;
        if let Err(message) = on_benchmark_thread(register, index, run_once) {
            let _ = writeln!(failures, "{}: {message}", descriptor.full_name());
        }
    }
    assert!(ran > 0, "no benchmark of the class {class} is registered");
    assert!(failures.is_empty(), "benchmarks failed:\n{failures}");
}

/// Runs every benchmark `register` registers once; see [`smoke_class`].
pub fn smoke(register: RegisterFn) {
    let descriptors = descriptors_of(register);
    let mut failures = String::new();
    for (index, descriptor) in descriptors.iter().enumerate() {
        if let Err(message) = on_benchmark_thread(register, index, run_once) {
            let _ = writeln!(failures, "{}: {message}", descriptor.full_name());
        }
    }
    assert!(!descriptors.is_empty(), "no benchmark is registered");
    assert!(failures.is_empty(), "benchmarks failed:\n{failures}");
}

/// Measures the benchmarks `register` registers that the environment
/// selects, and prints a table of them class by class.
pub fn run(register: RegisterFn) {
    run_with(register, &Options::from_environment());
}

/// [`run`] with the given options.
pub fn run_with(register: RegisterFn, options: &Options) {
    let descriptors = descriptors_of(register);
    let selected: Vec<usize> =
        (0..descriptors.len()).filter(|index| options.matches(&descriptors[*index])).collect();

    if options.list {
        for index in &selected {
            println!("{}", descriptors[*index].full_name());
        }
        println!("{} of {} benchmarks", selected.len(), descriptors.len());
        return;
    }

    println!(
        "{} of {} benchmarks, {} iterations after {} of warm-up{}",
        selected.len(),
        descriptors.len(),
        options.iterations,
        options.warmup,
        if cfg!(debug_assertions) { " (a build without optimisation: the times mean nothing)" } else { "" }
    );

    let mut results: Vec<(usize, Result<Measurement, String>)> = Vec::with_capacity(selected.len());
    for index in selected {
        let descriptor = &descriptors[index];
        let thread_options = options.clone();
        let result = on_benchmark_thread(register, index, move |entry| measure(entry, &thread_options));
        match &result {
            Ok(measurement) => println!(
                "  {}: {} ({} invocations an iteration)",
                descriptor.full_name(),
                format_time(measurement.mean()),
                measurement.invocations
            ),
            Err(message) => println!("  {}: FAILED: {message}", descriptor.full_name()),
        }
        results.push((index, result));
    }

    println!();
    print!("{}", format_table(&descriptors, &results));

    let failed = results.iter().filter(|(_, result)| result.is_err()).count();
    assert!(failed == 0, "{failed} benchmarks failed");
}

fn format_time(nanoseconds: f64) -> String {
    if nanoseconds < 1_000.0 {
        format!("{nanoseconds:.2} ns")
    } else if nanoseconds < 1_000_000.0 {
        format!("{:.3} us", nanoseconds / 1_000.0)
    } else if nanoseconds < 1_000_000_000.0 {
        format!("{:.3} ms", nanoseconds / 1_000_000.0)
    } else {
        format!("{:.3} s", nanoseconds / 1_000_000_000.0)
    }
}

fn format_bytes(bytes: f64) -> String {
    if bytes < 1024.0 {
        format!("{bytes:.0} B")
    } else if bytes < 1024.0 * 1024.0 {
        format!("{:.2} KB", bytes / 1024.0)
    } else {
        format!("{:.2} MB", bytes / (1024.0 * 1024.0))
    }
}

/// The table of the results: one block per class, with the ratio to the
/// baseline of the class (for the same parameters) when it has one.
fn format_table(descriptors: &[Descriptor], results: &[(usize, Result<Measurement, String>)]) -> String {
    let header = ["Benchmark", "Parameters", "Mean", "Median", "Min", "StdDev", "Ratio", "Allocated", "Blocks"];
    let mut out = String::new();
    let mut position = 0;
    while position < results.len() {
        let first = &descriptors[results[position].0];
        let mut end = position;
        while end < results.len()
            && descriptors[results[end].0].class == first.class
            && descriptors[results[end].0].category == first.category
        {
            end += 1;
        }
        let block = &results[position..end];

        let mut rows: Vec<[String; 9]> = Vec::with_capacity(block.len());
        for (index, result) in block {
            let descriptor = &descriptors[*index];
            let mut row: [String; 9] = Default::default();
            row[0] = descriptor.name.to_string();
            row[1] = descriptor.parameters.clone();
            match result {
                Ok(measurement) => {
                    row[2] = format_time(measurement.mean());
                    row[3] = format_time(measurement.median());
                    row[4] = format_time(measurement.min());
                    row[5] = format_time(measurement.standard_deviation());
                    let baseline = block.iter().find_map(|(other, other_result)| {
                        let other = &descriptors[*other];
                        match other_result {
                            Ok(other_measurement) if other.baseline && other.parameters == descriptor.parameters => {
                                Some(other_measurement.mean())
                            }
                            _ => None,
                        }
                    });
                    if let Some(baseline) = baseline.filter(|baseline| *baseline > 0.0) {
                        row[6] = format!("{:.2}", measurement.mean() / baseline);
                    }
                    if let Some(bytes) = measurement.allocated_bytes {
                        row[7] = format_bytes(bytes);
                    }
                    if let Some(blocks) = measurement.allocations {
                        row[8] = format!("{blocks:.1}");
                    }
                }
                Err(message) => row[2] = format!("FAILED: {message}"),
            }
            rows.push(row);
        }

        // Columns nothing was written to are left out.
        let columns: Vec<usize> =
            (0..header.len()).filter(|column| *column < 3 || rows.iter().any(|row| !row[*column].is_empty())).collect();
        let widths: Vec<usize> = columns
            .iter()
            .map(|column| rows.iter().map(|row| row[*column].len()).max().unwrap_or(0).max(header[*column].len()))
            .collect();

        let _ = writeln!(out, "{}::{}", first.category, first.class);
        let mut line = String::new();
        for (width, column) in widths.iter().copied().zip(&columns) {
            let _ = write!(line, "| {:<width$} ", header[*column]);
        }
        let _ = writeln!(out, "{line}|");
        line.clear();
        for width in &widths {
            let _ = write!(line, "|{}", "-".repeat(width + 2));
        }
        let _ = writeln!(out, "{line}|");
        for row in &rows {
            line.clear();
            for (width, column) in widths.iter().copied().zip(&columns) {
                if *column < 2 {
                    let _ = write!(line, "| {:<width$} ", row[*column]);
                } else {
                    let _ = write!(line, "| {:>width$} ", row[*column]);
                }
            }
            let _ = writeln!(out, "{line}|");
        }
        let _ = writeln!(out);

        position = end;
    }
    out
}

/// The allocations of a measured loop: with the feature `count-allocations`
/// the global allocator of the crate is the system allocator behind a count
/// of the blocks and bytes of each thread. Without the feature there is no
/// allocator here and [`snapshot`](allocations::snapshot) reads zeros.
pub mod allocations {
    /// Whether allocations are counted (the feature `count-allocations`).
    pub const ENABLED: bool = cfg!(feature = "count-allocations");

    /// What a thread has allocated so far, or between two moments.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct Counts {
        /// The blocks allocated (`alloc`, `alloc_zeroed` and `realloc`).
        pub allocations: u64,
        /// The bytes asked for: the size of each allocated block and what
        /// each resized block grew by.
        pub bytes: u64,
    }

    impl Counts {
        /// What was allocated between `earlier` and these counts.
        pub fn since(&self, earlier: &Counts) -> Counts {
            Counts { allocations: self.allocations - earlier.allocations, bytes: self.bytes - earlier.bytes }
        }

        /// Adds `other` to these counts.
        pub fn add(&mut self, other: &Counts) {
            self.allocations += other.allocations;
            self.bytes += other.bytes;
        }
    }

    /// What this thread has allocated so far; zeros without the feature
    /// `count-allocations`.
    pub fn snapshot() -> Counts {
        #[cfg(feature = "count-allocations")]
        {
            counting::snapshot()
        }
        #[cfg(not(feature = "count-allocations"))]
        {
            Counts::default()
        }
    }

    #[cfg(feature = "count-allocations")]
    mod counting {
        use super::Counts;
        use std::alloc::{GlobalAlloc, Layout, System};
        use std::cell::Cell;

        thread_local! {
            static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
            static BYTES: Cell<u64> = const { Cell::new(0) };
        }

        fn count(bytes: usize) {
            // A thread that is ending has no counters anymore.
            let _ = ALLOCATIONS.try_with(|allocations| allocations.set(allocations.get() + 1));
            let _ = BYTES.try_with(|total| total.set(total.get() + bytes as u64));
        }

        pub(super) fn snapshot() -> Counts {
            Counts {
                allocations: ALLOCATIONS.try_with(Cell::get).unwrap_or(0),
                bytes: BYTES.try_with(Cell::get).unwrap_or(0),
            }
        }

        struct CountingAllocator;

        // SAFETY: every member forwards to the system allocator with the
        // arguments it was given; the counters are plain thread-local cells
        // that do not allocate.
        unsafe impl GlobalAlloc for CountingAllocator {
            unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
                count(layout.size());
                // SAFETY: the caller upholds the contract of `alloc`.
                unsafe { System.alloc(layout) }
            }

            unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
                count(layout.size());
                // SAFETY: the caller upholds the contract of `alloc_zeroed`.
                unsafe { System.alloc_zeroed(layout) }
            }

            unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
                // SAFETY: the caller upholds the contract of `dealloc`.
                unsafe { System.dealloc(pointer, layout) }
            }

            unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
                count(new_size.saturating_sub(layout.size()));
                // SAFETY: the caller upholds the contract of `realloc`.
                unsafe { System.realloc(pointer, layout, new_size) }
            }
        }

        #[global_allocator]
        static ALLOCATOR: CountingAllocator = CountingAllocator;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn register(registry: &mut Registry) {
        let mut class = registry.class("harness", "Sample");
        class.benchmark("add", "", || 1_u64, |state| *state + 1).baseline();
        class.benchmark("add", "N=2", || 2_u64, |state| *state + 1).operations_per_invoke(2);
        class.benchmark_with_iteration_setup(
            "reset",
            "",
            || Cell::new(0_u32),
            |state| state.set(1),
            |state| {
                assert_eq!(state.replace(0), 1, "the iteration setup runs before every invocation");
            },
        );
    }

    #[test]
    fn names_hold_the_category_the_class_and_the_parameters() {
        let descriptors = descriptors_of(register);
        assert_eq!(descriptors[0].full_name(), "harness::Sample.add");
        assert_eq!(descriptors[1].full_name(), "harness::Sample.add[N=2]");
        assert!(descriptors[0].baseline);
        assert_eq!(descriptors[1].operations_per_invoke, 2);
        assert!(descriptors[2].has_iteration_setup);
    }

    #[test]
    fn every_benchmark_of_a_class_runs_once() {
        smoke_class(register, "Sample");
        smoke(register);
    }

    #[test]
    fn a_measurement_has_one_value_per_iteration() {
        let options = Options {
            filter: vec!["Sample.".to_string()],
            list: false,
            iterations: 3,
            warmup: 1,
            min_iteration_time: Some(Duration::from_millis(1)),
        };
        for index in 0..3 {
            let thread_options = options.clone();
            let measurement = on_benchmark_thread(register, index, move |entry| measure(entry, &thread_options))
                .expect("the benchmark runs");
            assert_eq!(measurement.nanoseconds.len(), 3);
            assert!(measurement.invocations >= 1);
            assert!(measurement.min() <= measurement.median());
        }
    }

    #[test]
    fn a_failing_benchmark_is_reported_with_its_name() {
        fn failing(registry: &mut Registry) {
            let mut class = registry.class("harness", "Failing");
            class.benchmark("fails", "", || (), |_: &mut ()| -> () { panic!("no scenario") });
        }
        let result = std::panic::catch_unwind(|| smoke_class(failing, "Failing"));
        let panic = result.expect_err("the smoke test fails");
        let message = panic.downcast_ref::<String>().cloned().unwrap_or_default();
        assert!(message.contains("harness::Failing.fails"), "{message}");
        assert!(message.contains("no scenario"), "{message}");
    }

    #[test]
    fn the_table_has_a_ratio_to_the_baseline() {
        let descriptors = descriptors_of(register);
        let measurement = |nanoseconds: f64| Measurement {
            invocations: 1,
            nanoseconds: vec![nanoseconds],
            allocated_bytes: None,
            allocations: None,
        };
        let results = vec![(0, Ok(measurement(100.0))), (2, Ok(measurement(250.0)))];
        let table = format_table(&descriptors, &results);
        assert!(table.contains("harness::Sample"), "{table}");
        assert!(table.contains("2.50"), "{table}");
        assert!(!table.contains("Allocated"), "{table}");
    }
}
