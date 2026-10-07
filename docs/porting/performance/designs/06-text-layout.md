# 06. Text layout: shaped runs and line metrics

Status: design, not implemented.

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

## Expected gain

3 to 6 % **[E]** for the scrolling case; more for views that redraw the same strings continuously.
It also helps the compositor side: the same glyph run object means the same text blob in Skia.

## Risks

Low, if the key is complete. A missing component of the key shows wrong glyphs, which the text
tests catch only for the cases they cover; the cache is therefore limited to the default paragraph
properties at first.

## Verification

- The text formatting suites and the text block tests.
- A test that formats a set of strings twice, with and without the cache, and compares glyph
  indices, advances, offsets and line metrics exactly.
- `scroll-profile.mjs`, and the memory of the cache after scrolling the whole table.
