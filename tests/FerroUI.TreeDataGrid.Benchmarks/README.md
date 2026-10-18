# ferroui-treedatagrid-benchmarks

The benchmarks of the TreeDataGrid control: the counterpart of the benchmark project of the
reference's TreeDataGrid, file by file. They measure the layout and allocation costs most affected
by the virtualization of the grid:

- the geometry of the column list (the column at a position) and the commits of its widths;
- the recycle pool of the element factory and the recycling of template cells;
- the sources and their presentation without a control (creation, sorting, expansion);
- the first layout of a grid, with resolved column widths and with the natural width probe;
- small vertical scrolls, forced stationary layouts and one-row scrolls, which isolate the reuse of
  the layout and the recycling from the batching of scrolls;
- buffered small vertical scrolls and reversing scrolls that stay within the cache window;
- small horizontal scrolls across many columns, column-sized ones, and ones near column 900 of
  1 000;
- insert/remove and move bursts of the items inside the realized rows;
- repeated detaching and reattaching of the grid.

Each scenario runs for the classic columns of the control crate and for the framework-neutral
columns of the core where the reference has the parameter `NeutralSource`.

## Layout

One Rust file per reference file, in snake case. A benchmark class is a struct: its `new` is the
constructor and the global setup of the class, its methods are the benchmarks, named with the snake
case of the reference's methods, and its iteration setups and cleanups are methods too. Each file
ends with a `register` function, which gives its benchmarks to the harness with their class, name,
parameter values and operations per invocation, and with one test per benchmark class. The crate
uses the harness of the framework's benchmarks (`tests/FerroUI.Benchmarks`, `harness.rs`).

| Reference | Here | Benchmarks | Parameters |
|---|---|---|---|
| `ColumnGeometryBenchmarks.cs` | `column_geometry_benchmarks.rs` | `locate_viewport` (1 024 lookups an invocation) | `ColumnCount` 20, 200, 1000 |
| `ColumnLayoutBenchmarks.cs` | `column_layout_benchmarks.rs` | `commit_unchanged_widths` (100 000 commits, iteration setup) | `ColumnCount` 20, 200, 1000 |
| `ColumnWidthBatchBenchmarks.cs` | `column_width_batch_benchmarks.rs` | `incremental_width_commits` (baseline), `batched_width_commit` | `Layout` Auto, Mixed; `RowCount` 20, 40 |
| `ElementFactoryBenchmarks.cs` | `element_factory_benchmarks.rs` | `same_parent_reuse` (500 000 operations, iteration setup) | `PoolSize` 10, 100, 1000 |
| `TemplateCellRecyclingBenchmarks.cs` | `template_cell_recycling_benchmarks.rs` | `same_template_provider` (baseline), `alternating_template_providers` (1 000 operations each) | none |
| `NeutralSourceBenchmarks.cs` | `neutral_source_benchmarks.rs` | `create_source_and_presentation`, `sort_thousand_rows`, `expand_collapse_thousand_rows` | `NeutralSource` false, true |
| `FirstLayoutBenchmarks.cs` | `first_layout_benchmarks.rs` | `legacy_natural_width_probe` (baseline), `resolved_width_constraint` (16 first layouts each, iteration setups) | `Layout` Fixed, Auto, Star, Mixed; `CacheLength` 0, 0.5 |
| `VirtualizationBenchmarks.cs` | `virtualization_benchmarks.rs` | `create_and_layout_grid`; with iteration setups: `vertical_small_scrolls`, `vertical_unbatched_small_scrolls`, `vertical_row_scrolls`, `vertical_buffered_small_scrolls`, `vertical_jitter_scrolls`, `vertical_buffered_jitter_scrolls` (10 000 operations each), `forced_stationary_vertical_layouts` (25 000), `horizontal_small_scrolls`, `horizontal_unbatched_small_scrolls` (6 000 each), `horizontal_column_scrolls`, `horizontal_far_column_scrolls` (1 000 each), `collection_insert_remove_burst` (1 500), `collection_move_burst` (6 000), `detach_reattach` (200) | `NeutralSource` false, true |
| `PairedScrollComparison.cs` | `paired_scroll_comparison.rs` | the runner `paired_vertical_scroll` | none |
| `RevisionScrollComparison.cs` | `revision_scroll_comparison.rs` | the runner `compare_scroll_revisions` | none |
| `Program.cs` | `program.rs` | the ignored tests `run_benchmarks`, `paired_vertical_scroll`, `compare_scroll_revisions` | none |

The numbers in parentheses are the operations of one invocation: the harness divides the time of an
invocation by them (the reference's `OperationsPerInvoke`), so the tables report the time of one
operation.

## Running

Measure in an optimised build, alone, on one test thread:

```sh
cargo test -p ferroui-treedatagrid-benchmarks --release --lib run_benchmarks -- --ignored --nocapture --test-threads=1
```

Select with the variables of the harness:

```sh
# The names, without running anything.
FERROUI_BENCH_LIST=1 cargo test -p ferroui-treedatagrid-benchmarks --release --lib run_benchmarks -- --ignored --nocapture

# The benchmarks whose full name (treedatagrid::Class.name[parameters]) contains one of the parts.
FERROUI_BENCH_FILTER=VirtualizationBenchmarks.vertical_row_scrolls,VirtualizationBenchmarks.detach_reattach \
    cargo test -p ferroui-treedatagrid-benchmarks --release --lib run_benchmarks -- --ignored --nocapture --test-threads=1
```

| Variable | Default | Meaning |
|---|---|---|
| `FERROUI_BENCH_FILTER` | none | Parts of names separated by commas; a benchmark runs when its full name contains one |
| `FERROUI_BENCH_LIST` | not set | When set, the selected names are printed and nothing runs |
| `FERROUI_BENCH_ITERATIONS` | 15 | The measured iterations of a benchmark |
| `FERROUI_BENCH_WARMUP` | 3 | The iterations before them |
| `FERROUI_BENCH_MIN_ITERATION_MS` | 500 | The least time of an iteration |

The reference states its warm-up and measured iterations per class (5 and 10 for most classes, 10
and 20 for `ColumnWidthBatchBenchmarks`, 30 and 30 for `FirstLayoutBenchmarks`); set them with
`FERROUI_BENCH_WARMUP` and `FERROUI_BENCH_ITERATIONS` when a run filters to one class.

### Allocations

With the feature `count-allocations` the counting allocator of the harness is the global allocator,
and the tables have the bytes and the blocks an operation allocated (what the memory diagnoser of
the reference's benchmark library reports):

```sh
cargo test -p ferroui-treedatagrid-benchmarks --release --features count-allocations --lib run_benchmarks -- --ignored --nocapture --test-threads=1
```

Take the times from a run without the feature: counting costs time.

### The comparison runners

```sh
# The one-row vertical scrolls for the classic and the neutral columns in turn, in one process.
cargo test -p ferroui-treedatagrid-benchmarks --release --lib paired_vertical_scroll -- --ignored --nocapture --test-threads=1

# Four scrolling workloads of the neutral columns in rounds.
cargo test -p ferroui-treedatagrid-benchmarks --release --lib compare_scroll_revisions -- --ignored --nocapture --test-threads=1
```

Both print their samples as JSON (the microseconds and, with `count-allocations`, the bytes of an
operation, and the realized rows). Use the same machine and build profile when comparing runs.

## Smoke tests

`cargo test -p ferroui-treedatagrid-benchmarks` runs every benchmark once: each class has a test that
runs the setup, the iteration setup and one invocation of every benchmark of the class, for every
parameter set. The smoke tests measure nothing. The invocations of the virtualization benchmarks
are thousands of layout passes each, so their test takes a while in a build without optimisation.

## Differences from the reference

- **Harness.** The reference measures with its platform's benchmark library, one process a
  benchmark; the harness runs each benchmark on a thread of its own and measures with the clock of
  the standard library. A fixed number of invocations an iteration (`InvocationCount` of
  `TemplateCellRecyclingBenchmarks` and `ColumnWidthBatchBenchmarks`) is not kept: the pilot of the
  harness finds the invocations that fill the least time of an iteration. A benchmark with an
  iteration setup invokes its body once an iteration, as in the reference.
- **Iteration cleanups.** The harness has iteration setups only. The reference's
  `[IterationCleanup]` of a benchmark runs at the start of the next iteration setup and when the
  benchmark is dropped; either way it is outside the measurement.
- **Application.** The reference starts its headless platform with an application without a theme.
  The port starts the unit test application of the framework with the services of a styled window
  (windows, text, the test theme that gives windows their template). The grid, its rows and its
  scroll viewers have the templates of the benchmarks, as in the reference.
- **Classic and neutral sources.** The reference's classic sources are a separate API presented
  through a compatibility presentation. The port's sources are all the core's: `NeutralSource =
  false` is a core source holding the view columns of the control crate (shown through the grid's
  `Source`), `NeutralSource = true` a core source holding the neutral columns of the core (shown
  through the grid's `Model`), and the same presentation presents both.
- **Scroll viewer template.** The reference binds the content and the offset of the scroll content
  presenter in its template; the port's scroll content presenter attaches itself to the scroll
  viewer it is in, so the template only names it `PART_ContentPresenter`.
- **Always-measuring presenters.** `vertical_unbatched_small_scrolls` and
  `horizontal_unbatched_small_scrolls` use presenters that measure again for every change of their
  viewport. The reference overrides `NeedsMeasureForViewportChange` in derived presenters; the
  port's presenters take that decision in hooks a derived class cannot override, so the derived
  presenters of the benchmark observe their effective viewport and invalidate their measure whenever
  it moves on their axis, which is what the overridden decision causes.
- **Natural width probe.** The baseline of `FirstLayoutBenchmarks` wraps its columns in a column that
  does not forward the measurement options of the column. In the port those options are
  `IColumn::requires_unconstrained_width_measurement`, whose default is `false`; the wrapper answers
  `true`, which is how the presenters of the reference treat a column without the options.
- **Revisions.** `RevisionScrollComparison.cs` loads several builds of the benchmark assembly into
  one process and rotates their order from round to round. A Rust process cannot load another build
  of the crate it is linked with, so the runner of the port runs the same workloads and rounds for
  the build it is; to compare two revisions, run it in each build on the same machine and compare
  the samples. The reference also records the processor time of the process, for which the standard
  library has no clock; the port records the elapsed time only. The reference collects the garbage
  before each sample; the port has no collector.
- **Allocations of the runners.** The runners read the bytes from the counting allocator of the
  harness (feature `count-allocations`); without the feature they print `null` for them.

## Reading the numbers

The times are not comparable with the reference's: the harness, the clock, the allocator and the
services of the test application differ. They are for comparing a change of the control with the
state before it, measured on a quiet machine in the same build. No numbers are recorded here; a
document that draws a conclusion from a measurement states the machine, the build and the command
with it.
