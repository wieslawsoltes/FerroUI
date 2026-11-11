# Windows renderer: main integration and resource reuse — 2026-11-09

**After merging current main, native has lower mean GPU-drained time than Skia
in all twelve recorded-path TDG workloads:** 10.7–26.9% lower in five paired runs.
The follow-up reduces repeated glyph-resource creation, ink queries and native
transform updates while preserving the rendered output.

## Integration and source identities

- Latest fetched `main`: `f91b679fe`, merged into the renderer work.
- Optimized source: `ddd74187892886f9f7df32a713e8685de657ab87`.
- Merged-baseline executable SHA-256:
  `40AB4372245CDA0B4FFD7A25E9CD04E8D19597EF9E6F82C94AD342A924118370`.
- Final executable SHA-256:
  `F72D08734CDE34672E207B9E3D9535AACF883996711AEBFBDC8B22456488D485`.

Main contains a reviewed copy of the original Quartz prototype. Conflict
resolution retains the later audited macOS/iOS implementation and its native
API, rather than mixing it with the older prototype's different fallback API.
Compatible upstream notice/noise-asset, shader line-ending, thread-local and
uniform-bounds review fixes are included. Shared main updates and Windows
backend/media feature selection are integrated. Apple runtime validation was
not performed on this Windows machine.

## Changes

- A per-face LRU reuses immutable positioned glyph arrays and their existing
  device-generation-aware ink-bounds cache when layout recreates identical
  runs. Keys include size, glyph IDs, every position and advance, and whether
  advances were supplied. Sharing always compares the full key.
- A fixed four-way fingerprint table admits repeated runs while leaving
  first-use text out of the resource LRU. Fingerprints only govern admission;
  collisions cannot make different glyph runs share resources. Long runs over
  256 glyphs bypass the cache. The 2 MiB per-face accounting budget includes
  probation storage and the full possible 32-entry ink cache per retained run.
  Cache entries own references; eviction and font disposal preserve live clients.
- Batched commands track logical and actual transforms separately. Deferred
  clips and combined glyphs use logical matrices without setting the device
  transform per command. Actual draws and batch boundaries publish the required
  transform; ink queries restore the actual state they temporarily replace.
- Glyph batching retains exact ink rectangles. A disjoint-row proof skips
  unnecessary overlap scans when safe. Ambiguous paint order, including text
  placed inside earlier gaps, still checks the original rectangles. This keeps
  native draw counts low without weakening overlap or blending correctness.

Separate profiles use two warm-up/five measured batches of twenty frames;
counts below include all 140 frames. Column-scrolling ink queries fall from
103.80 to **1.80/frame**, and far-column queries from 103.65 to **1.65/frame**
(over 98% fewer). The final profiles retain one core glyph draw per frame in
both classic and neutral workloads. These are operation counts, not GPU timing
estimates; the following uninstrumented totals are the performance comparison.

## Measurements

Same i5-12400F, GTX 1660 Ti, Windows 11 build 26200, driver 32.0.16.1088 and Rust
1.93.0 as the [previous report](windows-tdg-renderer-parity.md). Both renderers use
hardware on adapter LUID `00000000:000102c7`. Each build uses one executable for
both backends, with Skia first and native second in separate processes.

Merged baseline: three complete pairs, 45 measured samples per workload/backend.
Final: five complete pairs, 75 samples per workload/backend. Each sample is a
twenty-frame batch, after three warm-up batches. The table reports means and
sample standard deviations. No measured samples from these runs are discarded.
The builds run in separate phases; before/after variation is not independently
controlled by alternating source revisions.

| Scenario | Source | Merged native ms | Final native ms ± SD | Final Skia ms ± SD | Native / Skia |
|---|---|---:|---:|---:|---:|
| Stationary redraw | Classic | 0.355 | 0.304 ± 0.022 | 0.416 ± 0.071 | 0.731 |
| Vertical, 1 px | Classic | 0.426 | 0.405 ± 0.063 | 0.484 ± 0.095 | 0.838 |
| Vertical, 1 row | Classic | 0.615 | 0.542 ± 0.098 | 0.634 ± 0.115 | 0.854 |
| Horizontal, 1 px | Classic | 0.426 | 0.362 ± 0.042 | 0.455 ± 0.079 | 0.796 |
| Horizontal, column | Classic | 1.612 | 1.484 ± 0.207 | 1.663 ± 0.174 | 0.893 |
| Horizontal, far column | Classic | 1.719 | 1.550 ± 0.230 | 1.753 ± 0.245 | 0.884 |
| Stationary redraw | Neutral | 0.366 | 0.327 ± 0.041 | 0.415 ± 0.060 | 0.787 |
| Vertical, 1 px | Neutral | 0.409 | 0.400 ± 0.048 | 0.491 ± 0.078 | 0.815 |
| Vertical, 1 row | Neutral | 0.532 | 0.536 ± 0.073 | 0.689 ± 0.138 | 0.779 |
| Horizontal, 1 px | Neutral | 0.438 | 0.395 ± 0.050 | 0.521 ± 0.096 | 0.758 |
| Horizontal, column | Neutral | 1.677 | 1.593 ± 0.222 | 1.902 ± 0.315 | 0.838 |
| Horizontal, far column | Neutral | 1.551 | 1.605 ± 0.205 | 1.909 ± 0.305 | 0.841 |

Ten final native means improve over the merged native baseline by 2.4–14.9%.
Neutral row scrolling and far-column scrolling have 0.9% and 3.5% slower point
estimates, respectively; this is not a uniform before/after speedup. All twelve
final aggregate native/Skia ratios favor native. Individual pairs vary: 56 of
60 workload comparisons favor native, and the largest individual ratio is
1.104. This establishes measured mean performance on this machine, not a win
for every individual sample or scene.

The workload remains an 800×500 physical viewport at scale 1, Segoe UI 12,
24-pixel rows, real shaping, complete per-frame recording/replay and the same
classic/neutral data and scroll scripts. Every batch drains GPU work; Skia
flushes each frame. There is no presentation/vsync or retained-frame shortcut.
No compilation, capture or profiling runs during primary measurements.
Timed native runs assert zero additional readbacks and renderer errors.

## Validation and reproduction

- **55 native tests pass**, with one optional lighting benchmark ignored.
  Cache tests cover complete keys, first-use admission, eviction and surviving
  font disposal. Pixel comparisons cover clips, transforms, aliasing, opacity,
  overlap and nonsequential gap-filling paint order.
- All twelve native TDG captures match the merged baseline byte for byte;
  hashes are recorded. The representative PNG remains
  `E74F66241E77FCF794361D6D091ED721F24D9E5744C68B3A8B2BD4EB3F448087`.
- The catalog builds and passes native text quality, RTL selection, hit testing,
  replacement and undo at 175% desktop scaling. Renderer parity, Reveal and
  Acrylic brush captures were inspected. The screenshot run was instrumented
  and is functional evidence, not a primary timing result.
- Native video delivers 122 frames in 5,103 ms (23.90 fps) for the bundled 24 fps
  clip. Shared textures, zero CPU pixel transfers, navigation retirement and
  QR decoding pass. Physical discovery finds no camera; hardware capture remains
  untested. The explicit 640×480/30 fps synthetic GPU camera passes capture and
  navigation retirement with zero CPU pixel transfers. The normal catalog was
  rebuilt without test camera fixtures afterward.

[Raw evidence](benchmarks/windows-tdg-2026-11-09/) contains both baseline/final
timing matrices, source/binary metadata, profiling, tests, builds, captures and
media logs. Intermediate diagnostic iterations remain under
`target/native-evidence/main-merge-2026-11-09/` and are not pooled into the table.

Use the build commands and [comparison runner](../../scripts/compare-windows-tdg-renderers.ps1)
from the [previous report](windows-tdg-renderer-parity.md#reproduce) on each source
revision. The native catalog's normal and synthetic-camera smoke options are
in [the renderer guide](windows-native-renderer.md).
