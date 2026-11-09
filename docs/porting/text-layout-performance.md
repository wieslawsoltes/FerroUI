# Text layout performance

Where the time of text layout goes in FerroUI, how four other text stacks avoid it, and the plan
to change the algorithms (not to tune the current ones). Part of #644. The measurements are local
(`tests/FerroUI.Benchmarks`, `TextPipelineBenchmark`); nothing here runs in CI.

## 1. The case

The gauntlet (`samples/GauntletBenchmark`) lays out about 5,100 texts a frame and changes the
content width in every frame, so every wrapped text wraps again. Cranpose and egui draw it at 55
and 53 frames a second on the same machine; FerroUI at about 20. Text layout is about 20 % of the
UI thread there, and making and drawing glyph runs a large part of the render thread.

## 2. The stages, measured

`TextPipelineBenchmark` lays out the texts of the gauntlet with the real shaper (HarfBuzz), font
manager and render interface (Skia), stage by stage. Apple M3 Pro, `release`, 7 iterations; the
time of one operation over the whole corpus (Card: 500 card bodies of 22 to 38 words; Label:
2,000 short labels; Article: 12 paragraphs of 400 to 700 words).

| Stage | Card | per text | Label | Article |
|---|---|---|---|---|
| UAX #14 break opportunities alone | 2.34 ms | 4.7 us | 0.33 ms | 0.88 ms |
| Layout from nothing (shaping, breaking, lines) | 23.5 ms | 47 us | 6.17 ms | 8.62 ms |
| Layout at the same width, shaped runs cached | 7.22 ms | 14.4 us | 1.47 ms | 2.63 ms |
| Layout at the next width of a sweep, runs cached | 7.09 ms | 14.2 us | 1.45 ms | 2.65 ms |
| The same and the glyph runs of the lines | 7.14 ms | 14.3 us | 1.51 ms | 2.67 ms |
| A text block measured at the next width | 7.85 ms | 15.7 us | 1.97 ms | 2.70 ms |

What it shows:

- Shaping is cached (`TextRunCache`), but everything after it is done again for every layout,
  at a new width and at the same one alike: a layout at the width it had costs as much as one at
  a new width.
- A third of a cached layout of a card (4.7 of 14.4 us) is finding the UAX #14 break
  opportunities again, from the start of every line and of every run (`find_wrap_length`).
- The rest is building the lines: every wrapped line splits its shaped runs (two new buffers and
  two new runs), and builds a line, its bidi order, its metrics and the glyph runs whose ink bounds
  the metrics read (`create_line_metrics`), about 14 allocations a line.
- The glyph runs cost nothing extra in the fourth row because the layout has already made them.
- For comparison, Pretext's `layout` takes about 0.19 us for a text of this size (JavaScript),
  70 times less: it only adds up widths it measured once.

## 3. How the others do it

### Cranpose (`crates/cranpose-text`, `crates/cranpose-ui`)

Its own engine on HarfRust, `icu_segmenter` and `unicode-bidi`. It wins mostly by not doing the
work when the width changes.

1. **Width holds.** The greedy line breaker records, for the lines it made, the range of widths
   over which breaking again would give the same lines: from the widest line without its trailing
   white space, up to the narrowest width at which some line would take the next word
   (`line_hold.rs`, `Wrapping::greedy_limits` in `line_breaker.rs`). Inside the hold a new width
   only aligns the lines again (free for start-aligned text) and the layout keeps its identity.
   Tests compare a held layout with a new one at quarter-pixel steps (`tests/text_engine/width_hold.rs`).
2. **Measure holds.** A text node reports the range of constraints over which its size holds;
   box, flex and padding nodes intersect the ranges of their children
   (`ConstraintsHold`, `NodeCache::get_measurement`). A card whose texts do not wrap again is not
   measured at all.
3. **Relayout without shaping.** Outside the hold, the new layout reuses the glyphs the old one
   shaped and redoes only linear passes: prefix sums of cluster advances (a width of a range is one
   subtraction), break opportunities (a table for ASCII), greedy breaking (`TextLayouter::relayout`).
4. **Rarely shaping at all.** Plain left-to-right text is laid out from the font's tables
   (cmap, advances, single substitutions, pair kerning), proven equal to HarfRust per font; words
   are cached after their second use (`word_cache.rs`, admission on the second sighting).
5. **Caches with admission.** 1,024 texts with two widths each; a text that changes in place
   (a ticker) is admitted only when it comes back; entries age out after 120 passes.
6. **Rendering.** Glyph runs are cached by a key holding the layout's identity, which survives
   alignment, so they hit while the width animates; long runs keep their quads on the GPU.

### Pretext (`src/analysis.ts`, `src/layout.ts`, `src/line-break.ts`)

1. **Prepare once, lay out by arithmetic.** `prepare` segments the text (a word pass, then merge
   passes that apply the line breaking rules), measures every segment once and stores parallel
   arrays: widths, two end-of-line widths (one to fit, one to paint), break kinds, grapheme
   advances for long words. `layout(width)` is one greedy pass over the segments, about 5 ns a
   segment, with no measuring and no strings.
2. **Break rules at prepare time.** After the merge passes every boundary is a legal break; the
   loop needs no table.
3. **Hanging white space** has a fit width of zero.
4. **Accuracy.** Measuring words apart loses kerning across their boundaries; their browser sweep
   matches exactly, long corpora are off by a line at some widths. FerroUI shapes paragraphs whole
   and cuts lines out of them, so it does not have to take that loss.
5. **Line APIs without lines.** `walkLineRanges`, `layoutNextLineRange` (a width per line),
   `measureLineStats`, `measureNaturalWidth`.

### parley (`parley/src/layout`)

1. Shapes and analyses a paragraph once (ICU4X segmentation, its own bidi), whole runs through
   HarfRust, so kerning across spaces is kept.
2. `Layout::break_all_lines(width)` walks precomputed advances of unbreakable units, without
   shaping and, in steady state, without allocating; alignment is an offset a line.
3. Lines are ranges into flat arrays of characters, clusters and glyphs; glyph positions are made
   only to draw.

### egui (`crates/epaint/src/text`) and cosmic-text

1. egui lays out a text once a frame and caches the result by a hash of the whole job, width
   included (rounded to whole points), keeping what was used this frame or the last. A new width
   shapes again; it wins by doing one layout a label a frame, a fast path for text that fits on
   one line, and keeping the tessellated mesh with the layout.
2. cosmic-text shapes a line word by word after UAX #14 segmentation and wraps again without
   shaping (`Buffer::set_size`), writing every glyph out again.

## 4. The plan

In order of what it saves in the gauntlet; each step keeps the result identical to a layout made
from nothing (the existing wrap characterization, split and line tests, and new sweep tests), is
measured with `TextPipelineBenchmark` and the frame gate, and is a documented deviation where it
changes behavior upstream defines.

1. **Width holds for text layouts.** The wrapping loop records the range of widths over which its
   lines stay the same (the widest line without trailing white space; the narrowest width at
   which a line would take the next unit). `TextBlock` keeps its layout for any width inside the
   range (extends section 9.7 of desktop-performance.md, which keeps a layout only for narrower
   widths), and a sweep test checks it against a new layout at quarter-pixel steps.
2. **Measure holds in layout.** A layoutable may report the range of available widths over which
   its desired size holds; `TextBlock` reports its text's hold; `Border`, `Decorator`, a vertical
   `StackPanel` and pixel or auto `Grid` columns intersect their children's ranges. An element
   whose measure is valid and whose new available width is inside its hold returns its desired
   size without `MeasureOverride`. Opt-in per type, so a control that relies on being measured at
   every constraint is unchanged.
3. **Prepared paragraphs.** The break opportunities of a shaped run are found once and kept
   with its cluster advances (beside the prefix sums of `ShapedBuffer`); wrapping becomes a search
   over cumulative advances at break positions; lines are ranges of the shaped paragraph, made
   into line objects on demand; ink bounds (and with them the glyph runs) are computed when drawn
   or asked for, not to measure. A public API in `ferroui-base` (`PreparedParagraph`: prepare,
   layout at a width, line ranges, natural width, line count) serves `TextBlock`, `RichTextBlock`,
   `SelectableTextBlock`, `TextPresenter`, `FormattedText` and applications.
4. **Cache fixes.** `SelectableTextBlock` passes no run cache (it shapes on every layout);
   `TextBlock` drops its shaped runs on a foreground or decoration change, which do not change
   shaping.
5. **Stable glyph runs.** Lines cut at the same range of the same shaped run reuse their glyph
   runs, so the renderer's caches keyed by a run's identity keep hitting while the width moves.
6. **A shared text service.** The prepared paragraphs and their caches (key: text, properties
   and paragraph properties; admission of a changed text on its second use; ageing by layout
   pass) as a service any control or application uses, as Cranpose's layout cache is.

The cross-library comparison (`tests/FerroUI.Text.Comparison`: FerroUI, parley, cosmic-text and
epaint on the same font, corpus and widths) measures each step against the others.
