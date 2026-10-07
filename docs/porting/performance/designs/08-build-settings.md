# 08. Build settings of the browser module

Status: the first step is implemented in pull request 47; the rest is design.

## Finding

- **[M]** With `ferroui-base` and `ferroui-controls` at `opt-level = "s"` instead of "z", scrolling
  takes 20 % less processor time (612 to 492 ms) for 0.76 MB more of module with gzip (10.10 to
  10.86 MB); the time to the first frame is unchanged.
- **[M]** With every crate at level 3 the scrolling takes 26 % less (557 to 412 ms on modules linked
  with names), for 2.5 MB more with gzip and a module of 60 MB before compression, which the
  browser has to compile.
- **[M]** The allocator has 11 ms of self time (2 %), `memcpy` and its relatives almost none.

## What is left

Six percentage points between what is taken and the upper bound, and a few settings that were not
measured.

## Design

Each item is one build and one run of `scripts/browser/scroll-profile.mjs` and
`scripts/browser/first-frame.mjs`, with the module sizes from `scripts/browser/module-sizes.mjs`;
an item is adopted when it pays for its bytes.

1. **Which crates carry the remaining six points.** Raise one crate at a time to "s" or 3:
   `ferroui-skia` (the drawing context, glyph runs), `ferroui-harfbuzz`, `ferroui-browser`, the
   markup crates at run time, the themes (large, run once: expected to stay at "z").
2. **Level 3 for the hot modules only.** If one or two modules of `ferroui-base` carry most of the
   gain of level 3 (the property store, the layout manager), measure whether moving them is worth
   the bytes; a per-crate setting cannot do this, so the result would be a note for a later split
   of the crate, not a change now.
3. **The optimiser of the link.** rustc passes the level of the final crate to `em++`, which runs
   `wasm-opt -Oz` on the whole module. Measure `-O2` and `-Os` for that pass
   (`-Clink-arg=-O2`), which inline across the crate boundaries that thin LTO leaves.
4. **WebAssembly features.** `bulk-memory` and `nontrapping-fptoint` are supported by every browser
   the module already requires (it uses exception handling); measure them
   (`-Ctarget-feature=+bulk-memory,+nontrapping-fptoint`) for size and speed. `simd128` only if a
   measurement shows Skia's raster path or the text code using it.
5. **The allocator.** Emscripten's default against `mimalloc` (`-sMALLOC=mimalloc`): 2 % of the
   time is the ceiling, so this is taken only if it is free in bytes.

## Expected gain

Up to the six points measured for item 1 **[M]**; items 3 to 5 a few percent together **[E]**.

## Risks

Low. Every item is a setting; the budget of the Pages workflow (14 MB with gzip) and the time to
the first frame bound what can be adopted. The fat-LTO memory limit of
`browser-platform.md`, section 18, "Build memory", still applies.

## Verification

- `scripts/build-browser.sh` for the three sites and their tests in `scripts/browser/tests/`.
- The size, first-frame and scrolling figures in a table in `browser-platform.md`, section 18.
