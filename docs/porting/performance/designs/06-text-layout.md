# 06. Text layout: shaped runs and line metrics

Status: the hypothesis is verified against the code, upstream and the counters of design 09 (see
"Hypotheses, verified"), and it is dropped: the cache the formatter has is upstream's, with
upstream's key, and it hits whenever upstream's does. Steps 2 to 4 are caches upstream does not
have, which is different logic. Nothing is implemented for this design: the reading of the text
path found no machinery of the port that does work upstream's does not do (see "What was looked
for"). A place where the port creates fewer layouts than upstream was found and is recorded, not
changed (see "A difference found"). Written without a build.

## Finding

- **[M]** Text layout accounts for 55 ms of the scrolling profile (10 %); shaping with HarfBuzz is
  16 ms of it, building the line (metrics, glyph runs) most of the rest.
- **[M]** Every recycled row measures five new text blocks, each of which creates a text layout for
  its string.

## Cause

A text block creates its text layout when it is measured; the layout formats one line, which
shapes the text and computes the metrics of the line. The table shows the same kind of content over
and over (a few hundred distinct strings in a handful of fonts and sizes), and scrolling back shows
strings that were laid out moments before.

The formatter already has a cache in its name (`format_line_with_cache`). **[H]** It caches less
than it could: check what its key is, how large it is and what its hit rate is while scrolling.

## What must not change

The glyphs, positions and metrics of every line. A cached result must be the result the formatter
would compute: the key has to contain everything that influences it (text, typeface, size, culture,
flow direction, features, letter spacing, the paragraph properties that affect one line).

## Design

1. **Measure** the hit rate of the existing cache during `scroll-profile.mjs`, and the split of the
   55 ms between shaping, line metrics and glyph run creation.
2. **A shaped-run cache** in the text shaper: a bounded map from (text, typeface, size, culture,
   direction, features) to the shaped buffer, shared by all text blocks, evicting the least
   recently used entry. Short runs only (the strings of cells, labels and buttons), so that the
   memory is bounded by a few hundred kilobytes.
3. **Line metrics from the cached run** when the paragraph properties are the defaults, so that a
   text block with a cached string creates its layout without formatting.
4. **Glyph run reuse**: the platform glyph run of a cached line is kept with it, so that the Skia
   text blob built for drawing (design 07) is reused as well.

## Hypotheses, verified

Against `TextBlock` (`measure_override`, `arrange_override`, `text_layout`, `create_text_layout`,
`invalidate_text_layout`, `on_property_changed`, `utf16_text`, `SimpleTextSource`), `TextLayout`
(`from_text_source`, `create_text_lines`), `TextFormatterImpl` (`format_line_with_cache`,
`format_line_from_cache`, `shape_text_runs`, `shape_together`), `TextRunCache`, `TextShaper`,
`TextShaperOptions`, `TextRunProperties`, `GenericTextRunProperties`, `ShapedTextRun`, the HarfBuzz
shaper (`harf_buzz_text_shaper.rs`) and `FontManager::try_get_glyph_typeface` of the port;
`TextBlock.cs`, `TextLayout.cs`, `TextFormatterImpl.cs`, `TextRunCache.cs` and `TextRunProperties.cs`
of upstream; and the counters per recycled row of design 09 (20 pixels per step).

**Step 1, the figures. [M]** Per recycled row: 10.00 text layouts created, 10.00 lines formatted,
5.00 text run cache hits, 5.00 misses, 5.00 runs shaped, 10.00 content presenter children replaced
(5 text blocks removed, 5 new ones added). So: two layouts per new text block, one line each; the
first misses the cache and shapes, the second hits it. The split of the time is for a profiler
(design 09, "what is not counted yet").

**Two layouts per text block: upstream creates two as well.** `TextBlock.MeasureOverride` reads
`TextLayout`, which creates the layout for the constraint of the measure. `ArrangeOverride` is
`DisposeTextLayout(); _constraint = availableSize; var textLayout = TextLayout;`: it always disposes
the layout of the measure and creates one for the arranged size, "preserving the TextRunCache so
shaped runs are reused". The port's `measure_override` and `arrange_override` do the same, so for a
cell whose arranged size is not its measure constraint there are two layouts in both. (Where the
two are the same the port creates one; see "A difference found".)

**The hypothesis: the cache is upstream's, and it does not cache less.** `TextRunCache` is a port of
upstream's class of the same name:

- **Whose it is**: one per text block (`TextBlock._textRunCache`, created with the first layout and
  passed to every `TextLayout` of that block), not one for the formatter or the application.
- **What it holds**: per paragraph, the shaped runs, the resolved flow direction, the end of line
  and the length (`CachedShapingResult`): the result of fetching, bidi and shaping, which is what
  does not depend on the width.
- **Its key**: the index in the text source of the first character of the paragraph, an integer.
  One entry inline, a map from the second one on. Nothing is built or hashed for a lookup of a
  one-paragraph text block. The run properties, the typeface and the culture are not in the key
  because the cache belongs to one text block, which empties it when a property that affects
  shaping changes (`invalidate_text_layout`: the same eleven properties as upstream's
  `OnPropertyChanged`, and the same seven that keep the cache).
- **When it hits**: on every layout of a text block after its first, until it is invalidated.
  `FormatLine` looks it up after the test for a wrapped previous line and before fetching the runs;
  the port's `format_line_with_cache` has the same three steps in the same order.

So "half the lookups miss" is not a key that differs from upstream's: each of the 5 misses is the
first layout of a text block that did not exist before the row was prepared (its cache is new and
empty), and each of the 5 hits is its second layout. Upstream has the same 5 and 5. A text block
that lives on (a row that is measured again without being recycled) hits every time.

**Steps 2 to 4 are dropped.** A map of shaped buffers shared by all text blocks, line metrics taken
from it, and platform glyph runs kept with it are caches upstream does not have. They add state that
outlives a text block, with its own questions (what is in the key, when a font or a culture
invalidates an entry, how much it may hold), and they change what the framework does for a new text
block: no shaping where upstream shapes. That is different logic; like the caches of design 04 it
needs a decision as a deviation and is not implemented. Within one text block upstream's cache
already gives what step 4 asks for: the two layouts share the shaped runs, and a shaped run creates
its glyph run once (`ShapedTextRun::glyph_run`, upstream's `GlyphRun` property).

## What was looked for

The kinds of machinery the rule allows, each against upstream's statement sequence. None was found.

- **A cache key rebuilt or hashed per lookup**: the key is an integer; see above.
- **A miss upstream's cache does not have**: none; see above.
- **A layout the port creates where upstream reuses one**: none; the port creates as many or fewer.
- **A shaped buffer copied where it could be moved**: the HarfBuzz shaper fills the `ShapedBuffer` it
  returns in place from the buffer of the thread (which it keeps between calls);
  `shape_together` moves the buffer or its split parts into the shaped runs; the cache takes a
  counted slice of the run handles (upstream's `shapedTextRuns.ToArray()`), and the line of an
  unwrapped paragraph takes the handles out of the rented list (upstream's second `ToArray()`).
- **Scratch state allocated per line**: the bidi data and algorithm are kept per thread, as
  upstream's `t_bidiData` and `t_bidiAlgorithm`; the run lists come from `FormattingObjectPool`, as
  upstream's.
- **A value upstream caches and the port recomputes**: the glyph typeface of the run properties is
  kept after its first use (`GenericTextRunProperties::cached_glyph_typeface`, upstream's
  `CachedGlyphTypeface`); the glyph run of a shaped run likewise.
- **A conversion performed twice**: the text of a text block is UTF-8 and the formatter works on
  UTF-16; the text block keeps the transcoded text with the string it was made from (`utf16_text`),
  so the second layout does not transcode again. (Upstream's strings are UTF-16; this cost exists
  only in the port and is already paid once per string.)
- **A string compared where an identity would do**: `on_property_changed` selects by the name of the
  property, as upstream's `switch (change.Property.Name)` does.

### Left for a profile

Not changed; nothing measured points at them, and the first is not on the path of the table.

- `shape_text_runs` copies the font features of the run properties into a new list for the options
  of the shaper (`Rc::new(features.to_vec())`) for every shaped run that has features; upstream
  passes the list. The text blocks of the table have none.
- `FontManager::current()` and `TextShaper::current()` are looked up in the locator once per
  formatted line, where upstream reads `FontManager.Current` and `TextShaper.Current` at the same
  places.
- `FontManager::try_get_glyph_typeface` makes the source of the font family key absolute for every
  lookup (at most once per layout: the run properties of a layout are new, so their cached glyph
  typeface is empty until something asks for it), as upstream's `key.Source.EnsureAbsolute(key.BaseUri)`
  does.

## A difference found

Upstream's `ArrangeOverride` disposes the text layout and creates it again unconditionally. The
port's `arrange_override` keeps the layout when the arranged constraint is the constraint it was
created for and the resolved text alignment is the one it was created with
(`TextBlock::layout_text_alignment`), and says so in a comment. The layout kept is equal to the one
upstream would create, so the glyphs and metrics are the same; what differs is that a second layout
is not created: `create_text_layout`, which a subclass can override, is called once where upstream
calls it twice, and the layout object read after the arrange is the one of the measure.

This was there before this work and is not changed by it. It is an optimisation of the kind this
folder's rule sends to `DEVIATIONS.md` for a decision, and it was not listed there; it is now (Text
block). It does not affect the figures above: in the table every text block is arranged at a size
other than its measure constraint, so both layouts are created, as upstream's are.

## What is not implemented, and why

Nothing is implemented.

- **Steps 2 to 4**: caches upstream does not have; see above.
- **One layout instead of two** (measuring at the size the cell will be arranged at, or keeping the
  line and only aligning it) would halve the layouts, and is a different sequence from upstream's
  measure and arrange.
- **Not replacing the text block of a cell** would remove all ten layouts of a recycled row, since a
  text block that keeps its text keeps its cache; the child is replaced by upstream's
  `ContentPresenter.UpdateChild` and the default data template (designs 04 and 05).

What remains of the cost is upstream's own logic: per recycled row five strings shaped once and ten
lines built.

## Expected gain

None from this design as it stands. The 3 to 6 % **[E]** of the design came from steps 2 to 4.

## Risks

None: no code changes.

## Verification

Nothing to verify for a change. The suites that hold the behaviour described above are the text
formatting suites (`text_formatter_tests.rs`, `text_layout_tests.rs`, `text_line_tests.rs`,
`text_run_cache_tests.rs`, `shaped_buffer_tests.rs`) and the text block tests
(`src/FerroUI.Controls/text_block_tests.rs`); the test of the counters of design 09 lays out two
layouts over one text run cache and expects a miss and then a hit.

## Counters: before, expected after

Per recycled row, 20 pixels per step. "Before" is the table of design 09. Nothing is expected to
move; the column is here so that a later change to this design has its baseline.

| Counter | Before | Expected after | Measured after |
|---|---:|---|---|
| text layouts created | 10.00 | 10.00 | to be measured |
| text lines formatted | 10.00 | 10.00 | to be measured |
| text run cache hits | 5.00 | 5.00 | to be measured |
| text run cache misses | 5.00 | 5.00 | to be measured |
| text runs shaped | 5.00 | 5.00 | to be measured |
| content presenter children replaced | 10.00 | 10.00 | to be measured |
| allocations | 1175.0 | unchanged by this design | to be measured |

### Commands

The suites:

```sh
cargo test -p ferroui-base --lib text_formatting
cargo test -p ferroui-base --features perf-counters --lib perf_counters
cargo test -p ferroui-controls --lib text_block
```

The figures, with the commands of design 09:

```sh
cargo test -p control-catalog --release --features perf-counters --lib recycling_benchmark -- --ignored --nocapture --test-threads=1
cargo test -p control-catalog --release --features count-allocations --lib recycling_benchmark -- --ignored --nocapture --test-threads=1
```

To split the 55 ms between shaping, line metrics and glyph run creation (step 1), profile the
benchmark as design 09 describes ("To profile the benchmark with native tools").

## Doubts

Most likely first.

1. The reading was of the path a one-line, unwrapped text block takes (the cells of the table).
   Wrapping, trimming, inlines and bidi text go through `perform_text_wrapping`, the collapsing
   properties and the reordering, which were not compared with upstream line by line for this
   design.
2. That the two layouts of a cell differ in their constraint was concluded from the counter (10
   layouts for 5 text blocks) and the code, not observed.
3. Whether the difference found (the layout kept across an arrange) should stay: it needs a
   decision. Removing it would raise the layouts created wherever a text block is arranged at its
   measure constraint.
4. A cache across text blocks is where the gain of this design was expected. If it is wanted, it is
   a deviation to decide on with its key, its bound and its invalidation written down first.
