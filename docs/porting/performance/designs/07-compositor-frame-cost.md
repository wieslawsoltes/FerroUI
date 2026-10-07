# 07. Cost of a composed frame

Status: design, not implemented.

## Finding

- **[M]** Rendering in the compositor accounts for 145 ms of the scrolling profile (25 %):
  walking the visual tree and replaying the draw lists 45 ms, flushing Skia 29 ms, deserialising
  the changes of the frame 17 ms, the diagnostic overlay 13 ms, beginning the frame 8 ms.
- **[M]** The commit on the UI side accounts for another 32 ms (6 %).
- **[M]** In the trace of the published site the page fired 830 animation frames in 13 s, 0.7 ms
  each on average, whether or not anything changed.
- **[M]** In Skia, comparing text blob keys has 8 ms of self time, and font metrics are fetched
  during the flush (8 ms).

## Cause

Several separate things, each small:

1. The browser host of the sample turns on the overlay that shows frames per second, as the
   upstream host does. The overlay draws every frame and so keeps the render loop producing frames
   when nothing else changes.
2. The server side of the compositor walks the whole visual tree on every frame
   (`pre_subgraph` and `post_subgraph` have 11 ms of self time), also the parts that did not change.
3. Glyph runs are drawn through text blobs that Skia looks up in its cache by key; **[H]** the port
   creates a new blob for a glyph run that upstream keeps with the glyph run, so that the cache is
   searched and filled again for text that was drawn in the frame before.
4. Every frame begins and ends a GPU session, which makes the context current, binds the
   framebuffer and flushes.

## What must not change

What is drawn, and that the overlay is on in the sample (it is upstream's choice for this host; a
query option to turn it off would be an addition of this project and belongs in the host page, not
in the framework).

## Design

1. **Measure** a frame in which nothing changed, a frame in which one row changed and a frame in
   which the viewport changed: visuals walked, draw list items replayed, text blobs created and
   found, GL calls issued.
2. **Text blobs kept with the glyph run**, per edging mode, as upstream does; check the port
   against `GlyphRunImpl` of the Skia backend and close any gap. With design 06 the same glyph run
   object survives across recycling.
3. **Dirty-subtree walk**: check that the walker skips subtrees without changes and outside the
   dirty rectangle in the same way as upstream's; the self time of the visitor suggests it visits
   more than it draws.
4. **The overlay**: **[H]** upstream's diagnostic text renderer keeps one glyph run per character
   it can draw; check that the port keeps them too and that drawing them reuses their text blobs. Document on the host page how to turn it off for a measurement.
5. **The idle loop**: confirm that without the overlay the render timer stops requesting frames
   when the compositor has nothing to do.

## Expected gain

3 to 8 % of the scrolling time **[E]**, and a lower idle cost of the page, which matters for
battery use more than for scrolling.

## Risks

Medium for step 3 (a visual that is skipped wrongly is not drawn); low for the others.

## Verification

- The composition suites, the render tests of the Skia backend, the screenshot comparison of
  `scripts/browser/first-frame.mjs --compare` and of the catalog test.
- Counters from step 1 before and after.
- `scroll-profile.mjs`, and the processor time of an idle page over ten seconds.
