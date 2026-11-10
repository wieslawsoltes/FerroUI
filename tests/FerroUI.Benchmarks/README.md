# ferroui-benchmarks

The benchmarks of the framework: the counterpart of the benchmark project of the upstream project,
file by file. They cover the property system, bindings, styling, themes, layout, text, the visual
tree, navigation pages, rendering and the collections of the property system.

## Layout

One Rust file per upstream file, under the same folders in snake case (`base/`, `data/`, `layout/`,
`styling/`, `text/`, ...). A benchmark class is a struct: its `new` is the constructor and the global
setup of the class, its methods are the benchmarks, named with the snake case of the upstream
methods. Each file ends with a `register` function, which gives its benchmarks to the harness with
their class, name and parameter values, and with one test per benchmark class.

The crate root has what the benchmarks share: the harness (`harness.rs`), the entry points
(`program.rs`), the root of the trees (`test_root.rs`, the test root of the upstream unit test
library) and the helpers of the upstream project (`control_hierarchy_creator.rs`, `test_styles.rs`,
`test_binding_observable.rs`, `test_types.rs`).

Four folders have no upstream counterpart. `trees/` measures whole trees of controls.
`layout_subsystem/` and `styling_subsystem/` measure the layout and the styling of the framework
one mechanism at a time (a class for each: the measure and arrange of each panel, invalidation,
the passes of the layout manager, selector matching, activation, setters, resources, ...), on
trees built from the base controls alone, so that a change to one mechanism moves the numbers of
its class. `property_subsystem/` measures one read and one write of a property of a control, by
where the value comes from and by the kind of the write (`docs/porting/desktop-performance.md`,
section 9.35). The local gate measures the four folders (`scripts/perf/gate.py --only framework`).
`layout/compute_apply_spike.rs` is the record of an experiment, not of a mechanism of the
framework: `docs/porting/desktop-performance.md`, section 9.34.

## The harness

The upstream project measures with the benchmark library of its platform. This crate has a small
harness of its own instead of a benchmark library: the workspace measures with the clock of the
standard library in ignored tests (the frame benchmarks of the control catalog), and the benchmarks
follow that. `harness.rs` documents how each attribute of the upstream library maps to it. In short:

- a benchmark runs on a thread of its own, so it starts from the state of a new process (the
  property system, the dispatcher and the services belong to a thread);
- the setup runs once and is not measured; a pilot run finds how many invocations fill the least
  time of an iteration (500 ms, or what the class states); the warm-up iterations and the measured
  iterations follow;
- a benchmark with an iteration setup invokes its body once an iteration;
- the table has, per benchmark and parameter set, the mean, the median, the least and the standard
  deviation of the time of an operation, and the ratio to the baseline of the class when it has one.

## Running

Measure in an optimised build, alone, on one test thread:

```sh
cargo test -p ferroui-benchmarks --release --lib run_benchmarks -- --ignored --nocapture --test-threads=1
```

Every benchmark with its defaults takes a long time (several hundred benchmarks and parameter sets
of some seconds each), so select:

```sh
# The names, without running anything.
FERROUI_BENCH_LIST=1 cargo test -p ferroui-benchmarks --release --lib run_benchmarks -- --ignored --nocapture

# The benchmarks whose full name (category::Class.name[parameters]) contains one of the parts.
FERROUI_BENCH_FILTER=styling::,ControlsBenchmark.create_button \
    cargo test -p ferroui-benchmarks --release --lib run_benchmarks -- --ignored --nocapture --test-threads=1
```

| Variable | Default | Meaning |
|---|---|---|
| `FERROUI_BENCH_FILTER` | none | Parts of names separated by commas; a benchmark runs when its full name contains one |
| `FERROUI_BENCH_LIST` | not set | When set, the selected names are printed and nothing runs |
| `FERROUI_BENCH_ITERATIONS` | 15 | The measured iterations of a benchmark |
| `FERROUI_BENCH_WARMUP` | 3 | The iterations before them |
| `FERROUI_BENCH_MIN_ITERATION_MS` | per class | The least time of an iteration, in place of what the class states |

The build that is shipped is `--profile dist` in place of `--release` (whole-program optimisation;
it takes longer to build).

### Allocations

With the feature `count-allocations` the global allocator of the crate counts what each thread
allocates, and the table has the bytes and the blocks an operation allocated (what the memory
diagnoser of the upstream library reports):

```sh
cargo test -p ferroui-benchmarks --release --features count-allocations --lib run_benchmarks -- --ignored --nocapture --test-threads=1
```

Take the times from a run without the feature: counting costs time.

### Profiling text layout

The profiling entry of the upstream project builds text layouts for a fixed time without the
harness (five hundred to warm up, then eighteen seconds), with the Skia render backend and the
HarfBuzz shaper, for a sampling profiler to attach to:

```sh
cargo test -p ferroui-benchmarks --release --lib profile_text_layout -- --ignored --nocapture --test-threads=1
```

## Smoke tests

`cargo test -p ferroui-benchmarks` runs every benchmark once: each benchmark class has a test that
runs the setup, the iteration setup and one invocation of every benchmark of the class, for every
parameter set. A benchmark whose scenario no longer builds fails the test run, with the name of the
benchmark in the message. The smoke tests measure nothing.

## Reading the numbers

- Numbers measured on a loaded machine are not conclusions. Measure on a quiet machine, in the same
  build, and compare runs with each other rather than reading a single run; a run prints the
  standard deviation so that a noisy one shows.
- A build without optimisation says nothing about speed; the runner says so when it is one.
- The times are not comparable with the ones of the upstream project: the harness, the clock, the
  allocator and the test doubles of the services differ. They are for comparing a change of this
  framework with the state before it.
- No numbers are recorded here. A document that draws a conclusion from a measurement states the
  machine, the build and the command with it.

## Differences from the upstream project

- Where an upstream benchmark shuffles or generates its input with the random number generator of
  its platform and a fixed seed, the port uses a small deterministic generator of its own: the
  input is the same on every run, but it is not the upstream sequence.
- Every upstream benchmark of the tracked commit is ported. A benchmark that needs a member the
  framework does not have is left out rather than approximated, and its file says what it needs.

## Internal members

Some benchmarks reach members that are internal or private upstream, which upstream's benchmark
project reaches through its access to the internals of the framework. The crate that has such a
member exposes it through its `testing` feature, which this crate turns on:

| Upstream file | Member | Where the port exposes it |
|---|---|---|
| `Layout/Measure.cs` | the private setters of `IsMeasureValid` and `IsArrangeValid` (reached with `UnsafeAccessor`) | `ferroui_base::layout::testing` |
| `Styling/Style_Apply.cs`, `Styling/Style_ClassSelector.cs` | the styling pass of the value store (`GetValueStore().BeginStyling()`, `EndStyling()`) | `ferroui_base::styling::testing` |
| `Styling/SelectorBenchmark.cs` | a selector class of the benchmark (`AlwaysMatchSelector`): the abstract members of `Selector` | `ferroui_base::styling::testing::SelectorNode` |
| `Text/GlyphBoundsBenchmark.cs` | `GlyphTypeface.TryGetGlyphBounds` and `GlyphBounds` | `ferroui_base::media::testing` |
| `Text/UnicodeTrieBenchmark.cs` | the generated Unicode tries | `ferroui_base::media::text_formatting::unicode::testing` |
| `Compositor/CompositionTargetUpdate.cs` | a draw list visual without a visual of the tree | `ferroui_base::rendering::composition::testing` |
| `Rendering/ShapeRendering.cs` | the render interface of the headless platform and its drawing context stub | `ferroui_headless::testing` |

One ported benchmark fails in its setup, as upstream's does at the tracked commit:
`ApplyStyling` checks that the border of a text box is two units thick under the Simple theme,
where the theme gives it one (`styling/apply_styling.rs`; its smoke test expects the failure).
