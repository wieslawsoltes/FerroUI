# ferroui-treedatagrid-gpui-comparison

A comparison of FerroUI's TreeDataGrid with the tables of GPUI, in the scenarios of the
TreeDataGrid benchmarks (`tests/FerroUI.TreeDataGrid.Benchmarks`). Every target shows the same
rows and columns in a viewport of the same size, performs the same sequence of offsets and edits
with the same operation counts, and is measured by the same harness
(`tests/FerroUI.Benchmarks/harness.rs`) in the same process, so that the FerroUI numbers are
directly comparable with those of the TreeDataGrid benchmarks and the GPUI numbers with both.

The crate is a Cargo workspace of its own: no GPUI crate enters the workspace of the framework
or its lock file. The root manifest lists this directory in its `exclude`, as it does the
FerroBoard component. No lock file is committed; the first build writes one.

## Targets

| Column | What it is | Version |
|---|---|---|
| FerroUI TDG layout | The TreeDataGrid of this repository with the classic columns of the control crate, shown through its `Source` (`NeutralSource = false` of the benchmarks). A frame is a layout pass of the window: what the TreeDataGrid benchmarks measure. | this revision |
| FerroUI TDG layout+render | The same grid. A frame is the layout pass and the drawing of the whole window into a recording drawing context (see "Rendering"). | this revision |
| gpui-component | The `DataTable` of gpui-component (`crates/component/src/table/`): virtualized rows and columns, sortable, resizable and movable columns, row selection. For the tree scenario its `Tree` (`crates/component/src/tree.rs`). | `gpui-component = "=0.9.0"`, `gpui-base = "=0.9.0"` |
| gpui uniform_list | A table built directly on GPUI for this comparison: a header row and a `uniform_list` of rows, each row a line of fixed-width cells of text. The least a GPUI table is. For the tree scenario a `uniform_list` of the root and its children. | `gpui = { package = "gpui-pre", version = "=0.3.11" }` |

All the GPUI crates are Apache-2.0. GPUI is the one gpui-component depends on (`gpui-pre`
0.3.11, named `gpui` in its workspace), with its `test-support` feature for the test platform.
gpui-component and gpui-base are built without their own `test-support` features, which wrap
elements in test observers. On macOS the platform text variants use `gpui-pre-apple` 0.3.11
(its `font-kit` text system; `runtime_shaders`, so that the build needs no Metal compiler).
Nothing of Zed's `ui` crate is used.

## Running

Measure in an optimised build, alone, on one test thread, from this directory:

```sh
cargo test --release --lib run_comparison -- --ignored --nocapture --test-threads=1
```

The variables of the harness select and shape the run, as in the TreeDataGrid benchmarks:

```sh
# The names, without running anything.
FERROUI_BENCH_LIST=1 cargo test --release --lib run_comparison -- --ignored --nocapture

# One scenario on every target, or one target in every scenario.
FERROUI_BENCH_FILTER=vertical_small_scrolls cargo test --release --lib run_comparison -- --ignored --nocapture --test-threads=1
FERROUI_BENCH_FILTER=GpuiComponent. cargo test --release --lib run_comparison -- --ignored --nocapture --test-threads=1

# The synthetic text only, with fewer iterations.
FERROUI_BENCH_FILTER=Text=Synthetic FERROUI_BENCH_ITERATIONS=5 FERROUI_BENCH_WARMUP=1 \
    cargo test --release --lib run_comparison -- --ignored --nocapture --test-threads=1
```

| Variable | Default | Meaning |
|---|---|---|
| `FERROUI_BENCH_FILTER` | none | Parts of names separated by commas; a benchmark runs when its full name contains one |
| `FERROUI_BENCH_LIST` | not set | When set, the selected names are printed and nothing runs |
| `FERROUI_BENCH_ITERATIONS` | 15 | The measured iterations of a benchmark |
| `FERROUI_BENCH_WARMUP` | 3 | The iterations before them |
| `FERROUI_BENCH_MIN_ITERATION_MS` | 500 | The least time of an iteration |

A benchmark is named `comparison::<Target>.<scenario>[<parameters>]`, the target being
`FerroUiLayout`, `FerroUiRender`, `GpuiComponent` or `GpuiUniformList`. The run prints a line per
benchmark as it ends, then one table: a row per scenario and parameters, the mean time of an
operation on each target, and the time of each GPUI target over FerroUI's (the layout and render
target where it has run, the layout target otherwise). A ratio above 1 means that FerroUI took
less time. A full run takes a long time: a GPUI frame costs far more than a FerroUI layout pass,
and the operation counts are those of the TreeDataGrid benchmarks.

With the feature `count-allocations` the counting allocator of the harness is the global
allocator, and every cell also has the bytes and the blocks an operation allocated:

```sh
cargo test --release --features count-allocations --lib run_comparison -- --ignored --nocapture --test-threads=1
```

Take the times from a run without the feature: counting costs time.

`cargo test` (without `--ignored`) runs the quick tests: every benchmark has a name of its own,
every scenario has every target that can show it, every benchmark runs one invocation of at
most two operations on a thread of its own, and the two GPUI tables scroll and virtualize their
rows and columns. They measure nothing.

## Scenarios

The FerroUI side replicates the setup of `VirtualizationBenchmarks.CreateGrid` (window of
800 x 500, rows 24 pixels high, an id column 80 pixels wide and text columns 100 pixels wide, the
templates of the benchmark without column headers, scroll bars or cache) in `ferroui_targets.rs`,
so that the row and column counts can vary. The source-only scenarios call
`NeutralSourceBenchmarks` of the TreeDataGrid benchmarks itself. Every scenario body is written
once against a target interface (`scenarios.rs`), so every target performs the same sequence.

| Scenario | Mirrors | Operations an invocation | Parameters | Targets |
|---|---|---|---|---|
| `create_and_layout_grid` | `VirtualizationBenchmarks.create_and_layout_grid` | 1 (create, first frame, close) | Rows 1 000, Columns 12 | all |
| `create_and_first_layout` | the columns and rows of `FirstLayoutBenchmarks.resolved_width_constraint` | 1 (create, first frame, close) | Rows 1 000, Columns 6, Layout Fixed, Auto, Star | FerroUI: all layouts; gpui-component: Fixed; uniform_list: Fixed, Star |
| `vertical_small_scrolls` | `vertical_small_scrolls` | 10 000 | Rows 10 000 and 100 000, Columns 12 | all |
| `vertical_row_scrolls` | `vertical_row_scrolls` | 10 000 | Rows 10 000 and 100 000, Columns 12 | all |
| `vertical_jitter_scrolls` | `vertical_jitter_scrolls` | 10 000 | Rows 10 000 and 100 000, Columns 12 | all |
| `forced_stationary_vertical_layouts` | `forced_stationary_vertical_layouts` | 25 000 | Rows 10 000, Columns 12 | all |
| `horizontal_small_scrolls` | `horizontal_small_scrolls` | 6 000 | Rows 1 000, Columns 200 | all |
| `horizontal_column_scrolls` | `horizontal_column_scrolls` | 1 000 | Rows 1 000, Columns 200 | all |
| `horizontal_far_column_scrolls` | `horizontal_far_column_scrolls` (the jump to 90 000 pixels outside the measurement) | 1 000 | Rows 1 000, Columns 1 000 | all |
| `sort_thousand_rows` | `NeutralSourceBenchmarks.sort_thousand_rows` | 1 (alternating directions) | Rows 1 000, Columns 2 | FerroUI layout column: the benchmark itself; gpui-component: the delegate's sort, no frame |
| `sort_thousand_rows_with_frame` | the same sort with the grid on screen | 1 (sort and the next frame) | Rows 1 000, Columns 2 | all |
| `collection_insert_remove_burst` | `collection_insert_remove_burst` | 1 500 (insert, frame, remove, frame) | Rows 10 000 and 100 000, Columns 12 | all |
| `collection_move_burst` | `collection_move_burst` | 6 000 (move, frame, move back, frame) | Rows 10 000 and 100 000, Columns 12 | all |
| `detach_reattach` | `detach_reattach` | 200 (detach, frame, attach, frame) | Rows 10 000, Columns 50 | all |
| `expand_collapse_thousand_rows` | `NeutralSourceBenchmarks.expand_collapse_thousand_rows` | 1 (expand and collapse) | Rows 1 000 | FerroUI layout column: the benchmark itself; gpui-component: its `Tree`, no frames |
| `expand_collapse_thousand_rows_with_frames` | the same with the tree on screen | 1 (expand, frame, collapse, frame) | Rows 1 000, Columns 2 | all |

Every grid scenario runs with `Text=Synthetic` and `Text=Platform` (see "Text"). The row counts
the TreeDataGrid benchmarks fix are kept; the vertical scrolls and the collection edits also run
with 100 000 rows, which the benchmarks do not have. The benchmarks' buffered scrolls (a cache
around the viewport), unbatched presenters and `NeutralSource = true` path are TreeDataGrid
mechanisms without a GPUI counterpart and are not repeated here; their own benchmarks measure
them.

## Methodology

- **Harness.** The harness of the framework's benchmarks: each benchmark on a thread of its own,
  a pilot that finds the invocations that fill the least time of an iteration, warm-up and
  measured iterations, the mean, median, minimum and deviation of an operation. A benchmark with
  an iteration setup creates its grid there and invokes its body once an iteration, as in the
  TreeDataGrid benchmarks. The comparison uses `harness::measure_selected`, which measures as
  `harness::run` does and returns the measurements for the table of the comparison.
- **One frame.** FerroUI: `Window::update_layout` (the layout pass `Layoutable::update_layout`
  of the grid runs, the TreeDataGrid benchmarks' `_grid.UpdateLayout()`), and for the render
  target the drawing of the window. GPUI: `Window::draw` of the test platform, which renders the
  root view and every view in it again, lays the elements out, prepaints them (where
  `uniform_list` and the virtual lists render their visible items) and paints them into the scene
  of the frame, as a GPUI application does for every frame. GPUI's scene is not presented.
- **The same machine and process.** All targets run in one test binary; the GPUI applications
  use GPUI's test dispatcher, which runs every task on the thread of the benchmark, so no work
  happens on other threads.
- **Allocations.** With `count-allocations`, the bytes and blocks an operation allocated on the
  thread of the benchmark, for every target.

## Fairness

### Text

GPUI's test platform has a no-op text system (`NoopTextSystem` in `gpui-pre`'s
`src/platform.rs`, the default of `TestPlatform::new` in `src/platform/test/platform.rs`): one
glyph per character with a fixed advance and no shaping. FerroUI's test services have the same
kind of text: the test shaper of `TestServices::styled_window` (one glyph per codepoint with
the advance of the test fonts), which is what the TreeDataGrid benchmarks run with. The
`Text=Synthetic` rows compare these two.

The `Text=Platform` rows give both sides the text of the system fonts, in Helvetica at 12 pixels:
FerroUI shapes with HarfBuzz over the fonts of the Skia font manager (`ferroui-harfbuzz`,
`ferroui-skia`), GPUI uses the text system of its macOS platform, `gpui_apple::AppleTextSystem`
(CoreText through font-kit), given to `TestAppContext::build_with_text_system`. GPUI's macOS
platform itself must be created on the main thread (`MacPlatform::new` in `gpui-pre-macos`), but
its text system does not, so it is created directly. On other systems the GPUI targets have no
`Text=Platform` rows (`n/a`).

Both sides keep text layouts between frames: GPUI caches the shaped lines of the previous frame
(`LineLayoutCache`), FerroUI's text cells keep their layout until their text changes.

### Rendering

The TreeDataGrid benchmarks measure layout only. GPUI cannot lay out without rendering and
painting: its virtualized lists create their items in the prepaint phase, and a frame paints
every element again. So FerroUI has two columns:

- **layout**: the number of the TreeDataGrid benchmarks;
- **layout+render**: the layout pass and the drawing of the whole visual tree of the window,
  every visual of it, through `ImmediateRenderer` into a `RenderDataDrawingContext` without a
  compositor, the recorder of the compositor's render data, which records the drawing operations
  (glyph runs, rectangles, clips, transforms) without drawing them. The recording is discarded
  after each frame.

This is the counterpart of GPUI's paint into its scene. It is an upper bound for FerroUI: an
application's compositor records only the visuals that changed since the last frame, where this
records them all, as an immediate-mode frame does. The replay of the recording (FerroUI's
compositor drawing on a device) and the presentation of the scene (GPUI's renderer) are left
out on both sides. The ratios are to the layout+render column.

### Column virtualization

gpui-component virtualizes columns: each row of the `DataTable` renders its cells through a
horizontal virtual list over the column widths (`render_table_row` in
`crates/component/src/table/state.rs`, `virtual_list(.., Axis::Horizontal, col_sizes, ..)`), and
the header renders only the visible range of columns (`render_table_header`). Left-fixed columns
are always rendered; none are fixed here. The table of one's own virtualizes its columns itself:
with fixed widths the first and last visible column are a binary search over the column edges,
and only those cells are rendered, placed at their offset. The TreeDataGrid realizes the
columns in its viewport. So the horizontal scenarios, near column 900 of 1 000 too, compare
virtualizing tables on all targets.

### Configuration

- The GPUI tables have rows 24 pixels high (`Size::Size(px(24.))`), a header row of 24 pixels,
  no borders, no stripes, no scroll bars (the templates of the TreeDataGrid benchmarks have none
  either) and no scroll bounce. The `DataTable` keeps its default features (sortable, resizable,
  movable columns, row selection), which add event handlers to its rows and header cells every
  frame.
- Accessibility is turned off in the GPUI windows (`TestAppContext::disable_accessibility`), as
  on a desktop without an assistive technology connected; FerroUI creates no automation peers
  unless asked.
- The text is 12 pixels on both sides (FerroUI's default, given to GPUI explicitly).
- The GPUI sort is delegate code: `TableDelegate::perform_sort` is what a click on a header
  calls, and the table only re-renders. The `sort_thousand_rows_with_frame` row measures the sort
  and the next frame on every target; the `sort_thousand_rows` row measures FerroUI's sort of a
  source with its presentation against the delegate's sort.
- The `Tree` of gpui-component is a list of entries (one column), not a grid. Its expansion is
  private to its state; it is driven through its keyboard actions (`SelectRight` and
  `SelectLeft` of a focused tree with the root selected), as the arrow keys drive it.

## Caveats

- **Header row.** The GPUI tables always have a header row; the templates of the
  `VirtualizationBenchmarks` grid have no column headers. The GPUI tables therefore show one row
  less of the body (20 rows in 476 pixels against the TreeDataGrid's 21 in 500) plus a header
  row of the same number of cells.
- **Widths.** The `DataTable` has widths in pixels only: the `Auto` and `Star` layouts of
  `create_and_first_layout` are `n/a` for it. The table of one's own has fixed and star widths
  (flex items), no automatic width. `FirstLayoutBenchmarks` itself uses synthetic cells, which
  are not text; the `create_and_first_layout` rows use its rows, columns and widths with the
  text cells of the grid, and include the creation of the grid and the window and their closing
  (GPUI draws the first frame of a window while it opens it, so creation and first frame cannot
  be measured apart).
- **The sort columns.** `NeutralSourceBenchmarks.CreateFlat` gives its two columns no width
  (automatic); `sort_thousand_rows_with_frame` gives them 80 and 100 pixels, since the GPUI
  tables have no automatic width.
- **The FerroUI tree.** The expander cells of the hierarchical grid have no template in the test
  application (its theme gives windows their template and nothing else), so they lay out but
  present no content; the title column's text cells draw their text. The GPUI trees draw the id
  and the title of every entry.
- **The test shaper.** The test shaper of FerroUI's test services records the text of every
  shape call (`TestTextShaperImpl::shape_calls`), which allocates when a recycled cell shapes a
  new text. It is kept because the TreeDataGrid benchmarks run with it; the `Text=Platform` rows
  have no such cost.
- **GPUI tasks.** Every update of a GPUI entity flushes the effects of the application, as in
  an application; the test dispatcher runs no background work in these scenarios.
- **Only one path of FerroUI.** The TreeDataGrid benchmarks measure the classic and the neutral
  columns; this comparison uses the classic columns (`NeutralSource = false`) throughout.
- **Same machine.** The numbers are this machine's and this build's. A document that draws a
  conclusion from them states the machine, the build and the command with them.

## Results

Measured on 2026-11-07 on an Apple M3 Pro (macOS), Rust 1.99.0, the release profile, one test thread, with
`FERROUI_BENCH_ITERATIONS=5 FERROUI_BENCH_WARMUP=1 FERROUI_BENCH_MIN_ITERATION_MS=200` (the full
166 benchmarks take 85 minutes at these settings; the default 15 iterations take several hours,
because a GPUI operation is a whole frame and the operation counts are the TreeDataGrid
benchmarks'). Times are per operation. The ratios divide the GPUI time by the FerroUI layout and
render time, so a ratio above 1 means FerroUI is faster.

- **gpui-component's `DataTable`:** the FerroUI TreeDataGrid is faster in every scenario both can run. First layouts are 1.3 to 3.4 times faster, sorting with a frame 3.8 to 5.2, detach and reattach 5.2 to 8.5, expand and collapse with frames 4.0 to 4.7, and scrolls and collection edits 14 to 90.
- **The hand-built `uniform_list` table:** this table has no column model, no resizing, no selection and no templates. With synthetic text it builds and lays out its first frame faster than the TreeDataGrid (0.62 to 0.71 of its time). With platform text it is 2.0 to 2.3 times slower. In every other scenario it is 1.3 to 44 times slower.
- **Why scrolls differ so much:** the TreeDataGrid is retained. A scroll re-arranges and re-records only what moved, and recycles the realized rows. A GPUI scroll lays out and paints the whole visible window again.

| Scenario | Parameters | FerroUI TDG layout | FerroUI TDG layout+render | gpui-component | gpui uniform_list | gpui-component / FerroUI | uniform_list / FerroUI |
| ----------------------------------------- | -------------------------------------------------- | -----------------: | ------------------------: | -------------: | ----------------: | -----------------------: | ---------------------: |
| create_and_layout_grid | Rows=1000, Columns=12, Text=Synthetic | 888.106 us | 874.537 us | 1.431 ms | 620.628 us | 1.64 | 0.71 |
| create_and_layout_grid | Rows=1000, Columns=12, Text=Platform | 897.101 us | 935.737 us | 3.183 ms | 2.113 ms | 3.40 | 2.26 |
| create_and_first_layout | Rows=1000, Columns=6, Layout=Fixed, Text=Synthetic | 774.594 us | 790.079 us | 1.044 ms | 556.722 us | 1.32 | 0.70 |
| create_and_first_layout | Rows=1000, Columns=6, Layout=Fixed, Text=Platform | 812.834 us | 866.698 us | 2.338 ms | 1.743 ms | 2.70 | 2.01 |
| create_and_first_layout | Rows=1000, Columns=6, Layout=Auto, Text=Synthetic | 832.099 us | 842.009 us | n/a | n/a | n/a | n/a |
| create_and_first_layout | Rows=1000, Columns=6, Layout=Auto, Text=Platform | 846.819 us | 900.591 us | n/a | n/a | n/a | n/a |
| create_and_first_layout | Rows=1000, Columns=6, Layout=Star, Text=Synthetic | 848.109 us | 863.705 us | n/a | 532.326 us | n/a | 0.62 |
| create_and_first_layout | Rows=1000, Columns=6, Layout=Star, Text=Platform | 861.107 us | 880.353 us | n/a | 1.722 ms | n/a | 1.96 |
| vertical_small_scrolls | Rows=10000, Columns=12, Text=Synthetic | 14.885 us | 34.086 us | 2.976 ms | 1.078 ms | 87.30 | 31.62 |
| vertical_small_scrolls | Rows=10000, Columns=12, Text=Platform | 15.029 us | 34.284 us | 2.681 ms | 1.321 ms | 78.20 | 38.52 |
| vertical_small_scrolls | Rows=100000, Columns=12, Text=Synthetic | 14.672 us | 34.058 us | 2.348 ms | 1.042 ms | 68.94 | 30.59 |
| vertical_small_scrolls | Rows=100000, Columns=12, Text=Platform | 14.481 us | 33.692 us | 2.605 ms | 1.290 ms | 77.32 | 38.28 |
| vertical_row_scrolls | Rows=10000, Columns=12, Text=Synthetic | 24.685 us | 42.432 us | 2.295 ms | 1.016 ms | 54.09 | 23.93 |
| vertical_row_scrolls | Rows=10000, Columns=12, Text=Platform | 23.917 us | 42.436 us | 2.644 ms | 1.358 ms | 62.29 | 32.00 |
| vertical_row_scrolls | Rows=100000, Columns=12, Text=Synthetic | 24.071 us | 42.543 us | 2.324 ms | 1.013 ms | 54.63 | 23.81 |
| vertical_row_scrolls | Rows=100000, Columns=12, Text=Platform | 23.877 us | 42.502 us | 2.656 ms | 1.353 ms | 62.49 | 31.83 |
| vertical_jitter_scrolls | Rows=10000, Columns=12, Text=Synthetic | 83.359 us | 102.777 us | 2.336 ms | 1.009 ms | 22.73 | 9.81 |
| vertical_jitter_scrolls | Rows=10000, Columns=12, Text=Platform | 84.446 us | 103.240 us | 2.807 ms | 1.468 ms | 27.19 | 14.22 |
| vertical_jitter_scrolls | Rows=100000, Columns=12, Text=Synthetic | 84.149 us | 103.045 us | 2.332 ms | 1.010 ms | 22.63 | 9.80 |
| vertical_jitter_scrolls | Rows=100000, Columns=12, Text=Platform | 84.461 us | 103.135 us | 2.815 ms | 1.432 ms | 27.30 | 13.88 |
| forced_stationary_vertical_layouts | Rows=10000, Columns=12, Text=Synthetic | 1.625 us | 19.396 us | 1.144 ms | 495.331 us | 59.01 | 25.54 |
| forced_stationary_vertical_layouts | Rows=10000, Columns=12, Text=Platform | 1.636 us | 19.435 us | 1.250 ms | 605.206 us | 64.29 | 31.14 |
| horizontal_small_scrolls | Rows=1000, Columns=200, Text=Synthetic | 14.532 us | 33.546 us | 2.282 ms | 1.012 ms | 68.02 | 30.18 |
| horizontal_small_scrolls | Rows=1000, Columns=200, Text=Platform | 14.837 us | 34.860 us | 2.563 ms | 1.263 ms | 73.51 | 36.23 |
| horizontal_column_scrolls | Rows=1000, Columns=200, Text=Synthetic | 142.653 us | 163.154 us | 2.307 ms | 1.007 ms | 14.14 | 6.17 |
| horizontal_column_scrolls | Rows=1000, Columns=200, Text=Platform | 144.230 us | 163.963 us | 2.709 ms | 1.404 ms | 16.52 | 8.56 |
| horizontal_far_column_scrolls | Rows=1000, Columns=1000, Text=Synthetic | 130.829 us | 149.594 us | 2.392 ms | 1.026 ms | 15.99 | 6.86 |
| horizontal_far_column_scrolls | Rows=1000, Columns=1000, Text=Platform | 131.243 us | 149.262 us | 2.861 ms | 1.468 ms | 19.17 | 9.84 |
| sort_thousand_rows_with_frame | Rows=1000, Columns=2, Text=Synthetic | 207.866 us | 216.583 us | 825.911 us | 299.902 us | 3.81 | 1.38 |
| sort_thousand_rows_with_frame | Rows=1000, Columns=2, Text=Platform | 214.314 us | 222.874 us | 1.157 ms | 596.525 us | 5.19 | 2.68 |
| sort_thousand_rows | Rows=1000, Columns=2, Text=Synthetic | 38.955 us | n/a | 415.590 us | n/a | 10.67 | n/a |
| collection_insert_remove_burst | Rows=10000, Columns=12, Text=Synthetic | 43.017 us | 80.435 us | 4.576 ms | 2.017 ms | 56.89 | 25.08 |
| collection_insert_remove_burst | Rows=10000, Columns=12, Text=Platform | 43.432 us | 80.367 us | 5.197 ms | 2.554 ms | 64.66 | 31.78 |
| collection_insert_remove_burst | Rows=100000, Columns=12, Text=Synthetic | 67.300 us | 104.901 us | 4.670 ms | 2.089 ms | 44.52 | 19.92 |
| collection_insert_remove_burst | Rows=100000, Columns=12, Text=Platform | 69.248 us | 105.779 us | 5.273 ms | 2.681 ms | 49.85 | 25.34 |
| collection_move_burst | Rows=10000, Columns=12, Text=Synthetic | 18.612 us | 55.921 us | 4.601 ms | 2.006 ms | 82.28 | 35.87 |
| collection_move_burst | Rows=10000, Columns=12, Text=Platform | 18.599 us | 56.053 us | 5.057 ms | 2.464 ms | 90.21 | 43.96 |
| collection_move_burst | Rows=100000, Columns=12, Text=Synthetic | 73.280 us | 106.154 us | 4.992 ms | 2.206 ms | 47.02 | 20.79 |
| collection_move_burst | Rows=100000, Columns=12, Text=Platform | 68.540 us | 105.306 us | 5.252 ms | 2.630 ms | 49.87 | 24.97 |
| detach_reattach | Rows=10000, Columns=50, Text=Synthetic | 441.490 us | 455.314 us | 2.355 ms | 1.104 ms | 5.17 | 2.43 |
| detach_reattach | Rows=10000, Columns=50, Text=Platform | 456.703 us | 479.139 us | 4.073 ms | 2.486 ms | 8.50 | 5.19 |
| expand_collapse_thousand_rows | Rows=1000, Columns=2, Text=Synthetic | 6.362 us | n/a | 389.920 us | n/a | 61.29 | n/a |
| expand_collapse_thousand_rows_with_frames | Rows=1000, Columns=2, Text=Synthetic | 175.392 us | 184.906 us | 743.271 us | 241.016 us | 4.02 | 1.30 |
| expand_collapse_thousand_rows_with_frames | Rows=1000, Columns=2, Text=Platform | 181.818 us | 192.928 us | 902.068 us | 539.580 us | 4.68 | 2.80 |
