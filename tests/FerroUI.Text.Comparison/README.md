# ferroui-text-comparison

A comparison of the text layout of FerroUI with that of three Rust text layout libraries,
parley, cosmic-text and epaint (the text layout of egui), on identical inputs: the same font
file, the same size, the same widths and the same strings, measured by the same harness
(`tests/FerroUI.Benchmarks/harness.rs`) in the same process.

This is a local tool for comparing algorithms. It is not part of CI and is not a gate.

The crate is a Cargo workspace of its own: none of the compared libraries enters the workspace of
the framework or its lock file. The root manifest lists this directory in its `exclude`, as it
does the GPUI comparison (`tests/FerroUI.TreeDataGrid.GpuiComparison`). No lock file is
committed; the first build writes one. `rust-toolchain.toml` selects Rust 1.99.0, because
epaint 0.37 requires Rust 1.95.

## Libraries

| Column | What lays the text out | Version |
|---|---|---|
| FerroUI | `TextLayout` (`src/FerroUI.Base/media/text_formatting/text_layout.rs`), shaped by HarfBuzz (`ferroui-harfbuzz`), built as `TextBlock` builds it (`src/FerroUI.Controls/text_block.rs`), on a test application with the services of the text benchmarks (`tests/FerroUI.Benchmarks/text/`) | this revision |
| parley | `Layout` built by a `RangedBuilder` of a shared `LayoutContext` and `FontContext`, broken into lines by `Layout::break_all_lines` | `parley = "0.12"` |
| cosmic-text | One `Buffer` per string over a shared `FontSystem`, `Buffer::set_size` and `Buffer::shape_until_scroll` | `cosmic-text = "0.19"` |
| epaint | `LayoutJob::simple` laid out into a `Galley` by a shared `Fonts`, through the galley cache (`FontsView::layout_job`) or around it (`Fonts::layout_uncached`) | `epaint = "0.37"` |

The three libraries come from crates.io, without their system font backends
(`default-features = false`).

## Inputs

- Font: Roboto Regular of this repository (`src/FerroUI.Themes.Material/Assets/Roboto-Regular.ttf`),
  embedded with `include_bytes!` and given to every library from memory. No library discovers or
  loads a font of the system: FerroUI has a font backend that serves Roboto and nothing else
  (`media::fonts::testing::TestFontManagerImpl`, and the setup checks that the typeface resolves
  to Roboto); parley has a fontique `Collection` with `system_fonts: false` and Roboto
  registered; cosmic-text has a `fontdb::Database` with Roboto alone
  (`FontSystem::new_with_locale_and_db`, which scans nothing); epaint has `FontDefinitions` with
  Roboto as the only proportional font (built without `default_fonts`).
- Size: 14 px, at a scale of one pixel per unit or point for every library.
- Line height: the default of each library. FerroUI, parley (`LineHeight::MetricsRelative(1.0)`)
  and epaint derive it from the metrics of the font. cosmic-text has no default (its `Metrics`
  take a line height); it is given the line height of the font's `hhea` table, ascender minus
  descender plus line gap (2400 / 2048 em, 16.4 px).
- Wrapping: FerroUI `TextWrapping::Wrap`, parley its default, cosmic-text `Wrap::WordOrGlyph`
  (its default), epaint its default. No word of the corpus is wider than the narrowest width
  (180 px), so no library has to break inside a word or let a word overflow, and the modes lay
  out the same lines.
- Alignment: the start of the line (every library's default).

## Corpus

Generated in code (`corpus.rs`), deterministic and with no text of anyone's: the 32-bit xorshift
of the gauntlet benchmark (`samples/GauntletBenchmark/perf_data.rs`), seeded per string as the
gauntlet seeds a post.

| Corpus | Strings | What a string is |
|---|---|---|
| `label` | 5,000 | 3 to 20 characters: a cell coordinate (`C12.L3.4`), a price (`$1,234.56`), a signed percentage (`+2.31%`) or one or two capitalized pseudo-words |
| `card` | 500 | 120 to 260 characters of pseudo-words (2 to 11 letters, consonants and vowels alternating) with spaces, commas and periods: the length of the post bodies of the gauntlet |
| `article` | 20 | A paragraph of 2,000 to 4,000 characters of the same prose |

No string has a line break, so every string is one paragraph for every library.

## Scenarios

An operation lays out every string of the corpus once; the time in the table is the time of an
operation (the whole corpus), not of one string. Every benchmark returns the lines and the total
height it laid out, through `std::hint::black_box`.

| Scenario | Setup | Operation |
|---|---|---|
| `cold` | The library's shared state with the font loaded; nothing of a string laid out | Every string laid out at 240 px from its text, with no cache of that string |
| `rewrap_sweep` | Every string shaped once and laid out at 240 px | Every string laid out at the next width of a sweep from 180 to 320 px in steps of 7 (21 widths, cycled): the width changes every operation, as when a window is resized every frame |
| `same_width` | As `rewrap_sweep` | Every string laid out at 240 px again, every cache warm: the steady state when nothing changed |
| `rewrap_cached_same_width` | As `rewrap_sweep` (epaint alone) | Every string through epaint's galley cache at 240 px without `Fonts::begin_pass` in between: every string a cache hit, the cheapest way to get a laid-out galley from epaint |

Per library:

| Library | `cold` | `rewrap_sweep` and `same_width` |
|---|---|---|
| FerroUI | `TextLayout::new` from the `&str` (transcoded to UTF-16) with no `TextRunCache` | A new `TextLayout` per string from its UTF-16 text, with the `TextRunCache` of that string (one per string, as a text block has one) warm; each layout is disposed after its lines are read, as a text block disposes the layout it replaces |
| parley | A new `Layout` built from the text (shaping) and `break_all_lines(Some(240))` | `break_all_lines(Some(width))` on the layout built once in the setup |
| cosmic-text | A new `Buffer` (`new_empty`, `set_size`, `set_text`, `shape_until_scroll`) | `set_size(Some(width), None)` and `shape_until_scroll` on the buffer of the string |
| epaint | `Fonts::layout_uncached` of a new `LayoutJob::simple` | `Fonts::begin_pass` (as egui calls before each frame), then `FontsView::layout_job` of a new `LayoutJob::simple` per string |

## What each library caches

What persists between operations, and therefore what each scenario has warm:

- FerroUI. Shared: the glyph typeface of Roboto and its HarfBuzz font (the font manager of the
  application), the glyph metrics the typeface caches, and the pools of the text formatter. Per
  string: the `TextRunCache` holds the shaped runs of each paragraph by its index in the text,
  so a layout at a new width breaks lines again without shaping again. `TextLayout` itself keeps
  no result for a width; a `TextBlock` keeps its layout while its constraint does not change, in
  which case it does no work at all, which `same_width` does not measure (it builds the layout
  again over the warm cache, what a text block does when its layout was invalidated by anything
  but its text).
- parley. Shared: the `FontContext` (the fontique collection with its query caches and the
  source cache of loaded font data) and the `LayoutContext` (scratch buffers and the shaper,
  reused between layouts). Per string, in `rewrap_sweep` and `same_width`: the `Layout`, which
  holds the shaped runs; `break_all_lines` always breaks and aligns the lines again, at the same
  width too.
- cosmic-text. Shared: the `FontSystem` (the loaded font, the font match cache, the codepoint
  support cache and the shaping scratch buffer; the shaped-run cache of the feature
  `shape-run-cache` is off). Per string, in `rewrap_sweep` and `same_width`: the `Buffer`,
  whose lines keep their shaping and their layout. A new width marks the layout dirty and
  `shape_until_scroll` lays the lines out again from the kept shaping; at the same width
  nothing is dirty and `shape_until_scroll` returns at once, so `same_width` measures reading
  the layout runs.
- epaint. Shared: the `Fonts` (the parsed font, the glyph atlas, into which each glyph is
  rasterized once, the first time it is laid out, and the galley cache). The galley cache keys on
  the whole job, which holds the text and the wrap width: a new width is a miss that lays the
  text out from the start, and `Fonts::begin_pass` evicts the galleys the previous pass did not
  use. epaint keeps no shaping apart from the galley, so `rewrap_sweep` lays every string out
  from its text; `same_width` is a cache hit per string (it still hashes the job, whose text is
  copied into it as an egui label copies it), as is `rewrap_cached_same_width`.

## Side check

Before the measurements, a run prints to stderr the lines and the total height each library
lays the 500 cards out in at 240 px, the ratio of its lines to FerroUI's, and the cards it lays
out in as many lines as FerroUI. The libraries should agree closely: the same font, size and
width give the same breaks, apart from rounding of advances and line metrics. The test
`the_libraries_lay_the_cards_out_in_roughly_the_same_lines` checks that every library is within
15 percent of FerroUI's lines.

## Running

Measure in an optimised build, alone, from this directory:

```sh
cargo run --release
```

The variables of the harness select and shape the run, as in the framework's benchmarks:

```sh
# The names, without running anything.
FERROUI_BENCH_LIST=1 cargo run --release

# One scenario on every library, one library in every scenario, or one corpus.
FERROUI_BENCH_FILTER=rewrap_sweep cargo run --release
FERROUI_BENCH_FILTER=Parley. cargo run --release
FERROUI_BENCH_FILTER=Corpus=card cargo run --release

# Fewer iterations.
FERROUI_BENCH_ITERATIONS=5 FERROUI_BENCH_WARMUP=1 cargo run --release
```

| Variable | Default | Meaning |
|---|---|---|
| `FERROUI_BENCH_FILTER` | none | Parts of names separated by commas; a benchmark runs when its full name contains one |
| `FERROUI_BENCH_LIST` | not set | When set, the selected names are printed and nothing runs |
| `FERROUI_BENCH_ITERATIONS` | 15 | The measured iterations of a benchmark |
| `FERROUI_BENCH_WARMUP` | 3 | The iterations before them |
| `FERROUI_BENCH_MIN_ITERATION_MS` | 500 | The least time of an iteration |

A benchmark is named `text_comparison::<Library>.<scenario>[Corpus=<corpus>]`, the library being
`FerroUi`, `Parley`, `CosmicText` or `Epaint`. The run prints a line per benchmark as it ends,
then one table: a row per scenario and corpus, the mean time of an operation of each library,
and the time of each other library over FerroUI's. A ratio above 1 means that FerroUI took less
time. Each benchmark runs on a thread of its own, from the state a new thread has.

The same run is the ignored test `run_comparison`:

```sh
cargo test --release --lib run_comparison -- --ignored --nocapture --test-threads=1
```

With the feature `count-allocations` the counting allocator of the harness is the global
allocator, and every cell also has the bytes and the blocks an operation allocated:

```sh
cargo run --release --features count-allocations
```

Take the times from a run without the feature: counting costs time.

`cargo test` (without `--ignored`) runs the quick tests: the corpora have the stated counts and
lengths, every benchmark has a name of its own, every scenario of every library runs one
invocation with a few strings of each corpus (one test per library), the prepared layout at
240 px has the lines of the cold layout, and the libraries agree on the lines of the cards.

## Caveats

- The libraries do not do the same work beyond line breaking. FerroUI builds text lines with
  their runs and metrics for hit testing and drawing (the glyph runs of the render interface are
  created when drawn, not here); parley positions glyph runs and aligns lines; cosmic-text
  produces laid-out glyphs per line; epaint builds the rows of a galley with their glyph
  positions and rasterizes glyphs it has not seen into its atlas.
- FerroUI's input is UTF-16; the cold path includes transcoding the `&str`, the prepared paths
  start from the UTF-16 text, as a text block keeps it.
- `same_width` is each library's own path when the width did not change, which ranges from no
  layout at all (cosmic-text, and epaint's cache hit) to breaking the lines again (parley,
  FerroUI over its warm shaping cache). Read it with "What each library caches".
- The numbers are of one machine and one build; compare them within a run.
