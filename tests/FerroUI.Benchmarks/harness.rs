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
//! | the hardware counters diagnoser | the instructions and cycles of an operation ([`processor`]) |
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
//! - `FERROUI_BENCH_CLASS`: a class (`category::Class`); of the benchmarks
//!   the filter selects, those of that class run. A runner that wants a
//!   process per class (the local performance gate) lists the names and
//!   starts one run per class with it.
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
//! Beside the time, an iteration reads what the processor did
//! ([`processor`]): the instructions of an operation do not depend on where
//! the build put the code and the data or on what else the machine does, as
//! its time does, so a time that changed with the same instructions is not
//! a change of the work. The table has both, and the cycles.
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
    /// The one class (`category::Class`) whose benchmarks run, of those the
    /// filter selects.
    pub class: Option<String>,
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
            class: std::env::var("FERROUI_BENCH_CLASS")
                .ok()
                .map(|class| class.trim().to_string())
                .filter(|class| !class.is_empty()),
            list: std::env::var_os("FERROUI_BENCH_LIST").is_some(),
            iterations: number("FERROUI_BENCH_ITERATIONS").map_or(15, |value| value.max(1) as usize),
            warmup: number("FERROUI_BENCH_WARMUP").map_or(3, |value| value as usize),
            min_iteration_time: number("FERROUI_BENCH_MIN_ITERATION_MS").map(Duration::from_millis),
        }
    }

    /// Whether the filter and the class select `descriptor` (every benchmark
    /// without either).
    pub fn matches(&self, descriptor: &Descriptor) -> bool {
        if let Some(class) = &self.class {
            let of_class = class
                .split_once("::")
                .is_some_and(|(category, class)| category == descriptor.category && class == descriptor.class);
            if !of_class {
                return false;
            }
        }
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
    /// The instructions the processor retired for an operation (the median
    /// of the iterations), where the system counts them ([`processor`]).
    pub instructions: Option<f64>,
    /// The cycles the processor took for an operation (the median of the
    /// iterations), where the system counts them.
    pub cycles: Option<f64>,
}

impl Measurement {
    pub fn mean(&self) -> f64 {
        self.nanoseconds.iter().sum::<f64>() / self.nanoseconds.len() as f64
    }

    pub fn median(&self) -> f64 {
        median_of(&self.nanoseconds)
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

/// The median of `values`, which are not empty.
fn median_of(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let middle = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[middle - 1] + sorted[middle]) / 2.0
    } else {
        sorted[middle]
    }
}

/// Runs the setup, the iteration setup and one invocation of the benchmark,
/// then drops its state.
fn run_once(entry: &Entry) {
    let mut instance = (entry.create)();
    instance.iteration_setup();
    instance.invoke();
}

/// What one iteration took.
struct Iteration {
    elapsed: Duration,
    allocated: allocations::Counts,
    processor: Option<processor::Counts>,
}

/// Times `invocations` invocations of the body. The clock is read inside the
/// two readings of the allocator and of the processor, so neither reading is
/// in the time.
fn time_iteration(instance: &mut dyn Instance, invocations: u64, single: bool) -> Iteration {
    if single {
        instance.iteration_setup();
    }
    let processor = processor::snapshot();
    let allocated = allocations::snapshot();
    let start = Instant::now();
    for _ in 0..invocations {
        instance.invoke();
    }
    let elapsed = start.elapsed();
    let allocated = allocations::snapshot().since(&allocated);
    let processor = processor.and_then(|earlier| Some(processor::snapshot()?.since(&earlier).without_the_reading()));
    Iteration { elapsed, allocated, processor }
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
            let elapsed = time_iteration(instance.as_mut(), invocations, false).elapsed;
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
    let mut instructions = Vec::with_capacity(options.iterations);
    let mut cycles = Vec::with_capacity(options.iterations);
    for _ in 0..options.iterations {
        let iteration = time_iteration(instance.as_mut(), invocations, single);
        nanoseconds.push(iteration.elapsed.as_secs_f64() * 1e9 / operations);
        allocated.add(&iteration.allocated);
        if let Some(counts) = iteration.processor {
            instructions.push(counts.instructions as f64 / operations);
            cycles.push(counts.cycles as f64 / operations);
        }
    }

    // A count of some iterations only is none: the system stopped answering in between.
    let counted = instructions.len() == nanoseconds.len();
    let all_operations = operations * options.iterations as f64;
    Measurement {
        invocations,
        nanoseconds,
        allocated_bytes: allocations::ENABLED.then(|| allocated.bytes as f64 / all_operations),
        allocations: allocations::ENABLED.then(|| allocated.allocations as f64 / all_operations),
        instructions: counted.then(|| median_of(&instructions)),
        cycles: counted.then(|| median_of(&cycles)),
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

/// Measures the benchmarks `register` registers that `options` selects, each
/// on a thread of its own as [`run_with`] does, prints a line for each as it
/// ends, and returns their descriptors with what was measured, in the order
/// of registration. For runners that print a table of their own, such as a
/// comparison of the same scenario on several targets; `options.list` is
/// not read here.
pub fn measure_selected(register: RegisterFn, options: &Options) -> Vec<(Descriptor, Result<Measurement, String>)> {
    let descriptors = descriptors_of(register);
    let mut results = Vec::new();
    for (index, descriptor) in descriptors.iter().enumerate() {
        if !options.matches(descriptor) {
            continue;
        }
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
        results.push((descriptor.clone(), result));
    }
    results
}

/// A time in nanoseconds with the unit that suits it (`ns`, `us`, `ms`, `s`),
/// as the tables print it.
pub fn format_time(nanoseconds: f64) -> String {
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

/// A number of bytes with the unit that suits it (`B`, `KB`, `MB`), as the
/// tables print it.
pub fn format_bytes(bytes: f64) -> String {
    if bytes < 1024.0 {
        format!("{bytes:.0} B")
    } else if bytes < 1024.0 * 1024.0 {
        format!("{:.2} KB", bytes / 1024.0)
    } else {
        format!("{:.2} MB", bytes / (1024.0 * 1024.0))
    }
}

/// A count an operation (instructions, cycles) as the tables print it: the
/// digits alone, with a decimal while the count is small enough for one to
/// say something.
pub fn format_count(count: f64) -> String {
    if count < 100_000.0 {
        format!("{count:.1}")
    } else {
        format!("{count:.0}")
    }
}

/// The table of the results: one block per class, with the ratio to the
/// baseline of the class (for the same parameters) when it has one.
fn format_table(descriptors: &[Descriptor], results: &[(usize, Result<Measurement, String>)]) -> String {
    let header = [
        "Benchmark",
        "Parameters",
        "Mean",
        "Median",
        "Min",
        "StdDev",
        "Ratio",
        "Allocated",
        "Blocks",
        "Instructions",
        "Cycles",
    ];
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

        let mut rows: Vec<[String; 11]> = Vec::with_capacity(block.len());
        for (index, result) in block {
            let descriptor = &descriptors[*index];
            let mut row: [String; 11] = Default::default();
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
                    if let Some(instructions) = measurement.instructions {
                        row[9] = format_count(instructions);
                    }
                    if let Some(cycles) = measurement.cycles {
                        row[10] = format_count(cycles);
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

/// What the processor did in a measured loop: the instructions it retired
/// and the cycles it took, as the system counts them for the process.
///
/// The time of an operation moves with things that are not its work: where
/// the linker and the allocator put the code and the data (a loop that
/// crosses a boundary of the cache or of the branch predictor in one build
/// and not in the next), the core the thread runs on and its frequency, and
/// what else the machine does. The instructions do not: two runs of the same
/// program count the same to a hundredth of a percent, and two builds of the
/// same source to a few tenths. A change of the work shows in them; a change
/// of the time alone is one of the placement or of the machine. The cycles
/// are the time without the waits and the changes of frequency, and move
/// with the caches as the time does.
///
/// The counts are those of the whole process. A benchmark runs on a thread
/// of its own while the thread that started it waits, so they are those of
/// the benchmark unless it starts threads, whose work then counts too.
///
/// macOS counts them (`proc_pid_rusage`, the fields `ri_instructions` and
/// `ri_cycles`, which Apple silicon fills); elsewhere [`snapshot`] answers
/// with nothing and the table has no such columns.
///
/// [`snapshot`]: processor::snapshot
pub mod processor {
    /// The instructions and the cycles of the process up to a moment.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct Counts {
        pub instructions: u64,
        pub cycles: u64,
    }

    impl Counts {
        /// The counts since `earlier`.
        pub fn since(&self, earlier: &Counts) -> Counts {
            Counts {
                instructions: self.instructions.saturating_sub(earlier.instructions),
                cycles: self.cycles.saturating_sub(earlier.cycles),
            }
        }

        /// These counts, of a stretch between two readings, less what the
        /// readings themselves count between them (the return of the first
        /// and the call of the second: [`reading`]).
        pub fn without_the_reading(&self) -> Counts {
            self.since(&reading())
        }
    }

    /// The counts of the process now, or nothing where the system does not
    /// count them.
    pub fn snapshot() -> Option<Counts> {
        system::snapshot().filter(|counts| counts.instructions != 0)
    }

    /// What two readings that follow each other count between them: the
    /// least of sixteen pairs, read once a process.
    pub fn reading() -> Counts {
        static READING: std::sync::OnceLock<Counts> = std::sync::OnceLock::new();
        *READING.get_or_init(|| {
            let mut least: Option<Counts> = None;
            for _ in 0..16 {
                let (Some(first), Some(second)) = (snapshot(), snapshot()) else { return Counts::default() };
                let between = second.since(&first);
                least = Some(least.map_or(between, |least| Counts {
                    instructions: least.instructions.min(between.instructions),
                    cycles: least.cycles.min(between.cycles),
                }));
            }
            least.unwrap_or_default()
        })
    }

    #[cfg(target_os = "macos")]
    mod system {
        use super::Counts;

        /// `RUSAGE_INFO_V4` of `<sys/resource.h>`: the first version of the
        /// structure with the two counts.
        const FLAVOR: i32 = 4;
        /// `rusage_info_v4` is a 16-byte identifier and then 64-bit fields:
        /// `ri_instructions` and `ri_cycles` are the thirtieth and the
        /// thirty-first of them.
        const INSTRUCTIONS: usize = 2 + 29;
        const CYCLES: usize = 2 + 30;
        /// The structure is 37 words long; later versions are longer, and
        /// the system writes the one that was asked for.
        const WORDS: usize = 64;

        extern "C" {
            fn proc_pid_rusage(pid: i32, flavor: i32, buffer: *mut u64) -> i32;
            fn getpid() -> i32;
        }

        pub(super) fn snapshot() -> Option<Counts> {
            let mut buffer = [0_u64; WORDS];
            // SAFETY: the buffer is longer than the structure of the flavor asked for, and aligned
            // as its 64-bit fields are; the call writes that structure and nothing else.
            let status = unsafe { proc_pid_rusage(getpid(), FLAVOR, buffer.as_mut_ptr()) };
            (status == 0).then(|| Counts { instructions: buffer[INSTRUCTIONS], cycles: buffer[CYCLES] })
        }
    }

    #[cfg(not(target_os = "macos"))]
    mod system {
        use super::Counts;

        pub(super) fn snapshot() -> Option<Counts> {
            None
        }
    }
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

        ferroui_base::ferro_thread_local! {
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
            class: None,
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
            // Where the system counts them, an operation has instructions and cycles; elsewhere
            // neither. (How many is not asserted here: the counts are those of the process, and
            // the tests beside this one run on its other threads.)
            assert_eq!(measurement.instructions.is_some(), processor::snapshot().is_some());
            assert_eq!(measurement.cycles.is_some(), measurement.instructions.is_some());
        }
    }

    #[test]
    fn a_class_selects_its_benchmarks_of_those_the_filter_selects() {
        fn two_classes(registry: &mut Registry) {
            register(registry);
            let mut class = registry.class("harness", "Sample2");
            class.benchmark("add", "", || 1_u64, |state| *state + 1);
        }
        let options = |filter: &[&str], class: Option<&str>| Options {
            filter: filter.iter().map(|part| part.to_string()).collect(),
            class: class.map(str::to_string),
            list: false,
            iterations: 1,
            warmup: 0,
            min_iteration_time: None,
        };
        let selected = |options: Options| -> Vec<String> {
            descriptors_of(two_classes)
                .iter()
                .filter(|descriptor| options.matches(descriptor))
                .map(Descriptor::full_name)
                .collect()
        };
        assert_eq!(selected(options(&[], None)).len(), 4);
        // The whole name of the class, not a part of it: `Sample` is not `Sample2`.
        assert_eq!(
            selected(options(&[], Some("harness::Sample"))),
            ["harness::Sample.add", "harness::Sample.add[N=2]", "harness::Sample.reset"]
        );
        assert_eq!(selected(options(&[], Some("harness::Sample2"))), ["harness::Sample2.add"]);
        assert_eq!(
            selected(options(&["add"], Some("harness::Sample"))),
            ["harness::Sample.add", "harness::Sample.add[N=2]"]
        );
        assert!(selected(options(&["reset"], Some("harness::Sample2"))).is_empty());
        assert!(selected(options(&[], Some("Sample"))).is_empty());
    }

    #[test]
    fn the_counts_of_the_processor_grow_with_the_work() {
        let Some(start) = processor::snapshot() else { return };
        let mut sum = 0_u64;
        for value in 0..200_000_u64 {
            sum = black_box(sum.wrapping_add(value));
        }
        let counts = processor::snapshot().expect("the system answered once").since(&start).without_the_reading();
        // A loop of 200,000 additions is at least as many instructions, and what the reading
        // itself counts is far less than that.
        assert!(counts.instructions >= 200_000, "{counts:?}");
        assert!(counts.cycles > 0, "{counts:?}");
        assert!(processor::reading().instructions < 200_000, "{:?}", processor::reading());
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
            instructions: None,
            cycles: None,
        };
        let results = vec![(0, Ok(measurement(100.0))), (2, Ok(measurement(250.0)))];
        let table = format_table(&descriptors, &results);
        assert!(table.contains("harness::Sample"), "{table}");
        assert!(table.contains("2.50"), "{table}");
        assert!(!table.contains("Allocated"), "{table}");
        assert!(!table.contains("Instructions"), "{table}");
    }

    #[test]
    fn the_table_has_the_counts_of_the_processor_when_they_were_read() {
        let descriptors = descriptors_of(register);
        let measurement = Measurement {
            invocations: 1,
            nanoseconds: vec![100.0],
            allocated_bytes: None,
            allocations: None,
            instructions: Some(152_210.4),
            cycles: Some(48.25),
        };
        let table = format_table(&descriptors, &[(0, Ok(measurement))]);
        let header = table.lines().nth(1).expect("the header");
        assert!(header.ends_with("| Instructions | Cycles |"), "{table}");
        let row = table.lines().nth(3).expect("the row");
        assert!(row.ends_with("|       152210 |   48.2 |") || row.ends_with("|       152210 |   48.3 |"), "{table}");
    }
}
