# macOS rendering and text performance

This report preserves the original prototype baseline. The subsequent
[batching and native-text implementation](quartz-metal-optimization.md) addresses these costs.
See the [first-pass measurements](quartz-optimization-results.md) and the latest
[matched second-pass measurements](quartz-optimization-round2-results.md), followed by the
[geometry/opacity measurements](quartz-optimization-round3-results.md) and the
[current opacity recording measurements](quartz-optimization-round4-results.md).

Measured 2026-11-08 on Apple M4, 24 GB RAM, macOS 27.2. The common harness lives in
[Rendering.Benchmarks](../../tests/FerroUI.Rendering.Benchmarks/README.md). The committed
[results](quartz-performance-results.json) include source/binary/font identities, individual
trial medians/ranges, p95 values, validation counts, native GPU counters and background load.
Full raw samples and logs remain in `target/rendering-perf/macos`.

## Method

One optimized `release` binary, six compatible renderer/font/shaper configurations, four
fresh processes per configuration, alternating forward/reverse order. Each rendering case
warms 20 frames and times 100 full-redraw frames at 2560×1600 physical pixels (1280×800 at 2×).
Every measured frame waits for successful GPU completion; no display or vsync is involved.
Results are medians of the four per-process medians. Percentiles and ranges are retained in
JSON. There were 19,200 timed rendering frames and 230,400 timed shaping calls in total.

Other user work was paused. OS services still consumed CPU; the initial snapshot included
CGPDFService at about one core and WindowServer activity. This is a desktop measurement,
not a claim of isolated peak hardware performance. Runs record load snapshots; for example,
Skia's dense-Latin medians ranged 0.649–0.823 ms, Quartz's 43.738–46.422 ms.

The shapes, curves, raw bitmap, nested opacity layers, fixed font files, UTF-16 strings,
direction, em sizes and glyph positions are common workload definitions. Rendering includes
warmed glyph runs; shaping is timed separately through `ITextShaperImpl` with preloaded fonts
and 32 calls per sample. The three fonts are Inter Regular, Noto Sans Arabic and Noto Sans
Hebrew. No system fallback font is substituted. Both native and HarfBuzz cases produced
56/50/46 glyphs respectively, with nonzero glyph IDs and valid UTF-16 clusters.

Light hinting and grayscale antialiasing are requested for every renderer. Core Text remains
the text-quality reference. Each scene has an untimed validation readback, checked for its
marker and visible content outside that marker. All 24 runs passed; timed frames use private
GPU textures without readback. Both Quartz configurations reported zero renderer readbacks
and zero GPU errors in every run. This does not count the separate platform validation captures.
Representative Skia/Quartz text and shadow captures were visually inspected. This suite is not
a pixel-parity certification or a test of every text feature.

## Skia versus Quartz Metal

Milliseconds per completed frame, or per shaping call for `shape-*`. Lower is better.
A ratio above one means Quartz took longer. These are backend full-redraw costs, not
Control Catalog displayed FPS, retained dirty-region costs, layout or startup measurements.

| Workload | Skia + HarfBuzz (ms) | Quartz + Core Text (ms) | Quartz / Skia |
|---|---:|---:|---:|
| rounded-rects | 0.7441 | 5.2284 | 7.03× |
| geometry | 0.7625 | 5.3366 | 7.00× |
| bitmaps | 0.5645 | 1.9763 | 3.50× |
| text-latin | 0.7518 | 46.0618 | 61.27× |
| text-arabic | 0.7741 | 40.3504 | 52.13× |
| text-hebrew | 0.7611 | 37.5692 | 49.36× |
| shadows | 10.3694 | 6.4997 | 0.63× |
| opacity-layers | 0.7191 | 40.3851 | 56.16× |
| shape-latin | 0.0121 | 0.0189 | 1.56× |
| shape-arabic | 0.0049 | 0.0145 | 2.95× |
| shape-hebrew | 0.0019 | 0.0096 | 5.09× |

Dense text means 96 pre-shaped runs, 48 rows in each of two columns. Rounded rectangles and
curved geometries draw 256 instances; bitmaps draw 96; shadows draw 24 rounded boxes; opacity
layers draw 24 groups of four overlapping rectangles. Each frame clears the complete canvas.

## All adapter configurations

Milliseconds per completed frame or shaping call. Vello CPU draws its canvas on the CPU
and uploads/presents through Metal; it is deliberately labeled separately from GPU rendering.

| Workload | skia | quartz | quartz-harfbuzz | vello-hybrid | vello-gpu | vello-cpu |
|---|---:|---:|---:|---:|---:|---:|
| rounded-rects | 0.7441 ms | 5.2284 ms | 5.2457 ms | 2.6254 ms | 2.6564 ms | 3.9085 ms |
| geometry | 0.7625 ms | 5.3366 ms | 5.3324 ms | 2.5670 ms | 2.4211 ms | 3.7307 ms |
| bitmaps | 0.5645 ms | 1.9763 ms | 1.9342 ms | 1.7988 ms | 2.2170 ms | 3.8892 ms |
| text-latin | 0.7518 ms | 46.0618 ms | 45.4502 ms | 2.2406 ms | 2.9611 ms | 4.7999 ms |
| text-arabic | 0.7741 ms | 40.3504 ms | 40.0884 ms | 2.1670 ms | 2.9876 ms | 4.7984 ms |
| text-hebrew | 0.7611 ms | 37.5692 ms | 37.1668 ms | 2.1191 ms | 2.7615 ms | 4.4423 ms |
| shadows | 10.3694 ms | 6.4997 ms | 6.5361 ms | 2.4652 ms | 16.5787 ms | 6.2964 ms |
| opacity-layers | 0.7191 ms | 40.3851 ms | 40.0319 ms | 1.6301 ms | 2.1670 ms | 3.2493 ms |
| shape-latin | 0.0121 ms | 0.0189 ms | 0.0084 ms | 0.0080 ms | 0.0079 ms | 0.0086 ms |
| shape-arabic | 0.0049 ms | 0.0145 ms | 0.0044 ms | 0.0043 ms | 0.0043 ms | 0.0044 ms |
| shape-hebrew | 0.0019 ms | 0.0096 ms | 0.0018 ms | 0.0018 ms | 0.0018 ms | 0.0018 ms |

## What to optimize next

The native renderer is faster in this uniform shadow case (6.50 versus 10.37 ms). It has
large gaps for dense text (about 49–61×), opacity layers (56×), geometry (7×) and bitmaps (3.5×).
Do not generalize the shadow result to every blur, shadow radius or clipping configuration.

For dense Latin text, Quartz spends about 43.28 ms of the 46.06 ms frame before the explicit
completion wait. Switching only the native shaper to HarfBuzz gives 45.45 ms and identical
native dispatch counters: it does not close the drawing gap. The measured isolated Latin
shaping calls are 18.93 µs for Core Text and 8.40 µs for HarfBuzz with the native font provider.
The difference in warmed drawing points to CPU recording/submission rather than primarily
text shaping. Source inspection shows one image dispatch per cached glyph (5,376 placements
in the Latin frame). Prioritize native glyph atlas/batched quad submission and font/cache
lookup overhead, preserving Core Text's masks, hinting and color glyphs. This is an inference
from timings/counters and source, not a sampled CPU or hardware-GPU profile.

Opacity layers have a different signature: about 2.71 ms CPU-before-wait out of 40.39 ms total.
Inspect bounded layer allocation/clearing/composition and bandwidth. The prototype currently
uses whole-surface layer storage; batching glyphs alone will not fix this workload. Completion
wait time is not hardware GPU duration: execution can overlap CPU submission.

The shared adapter registry also feeds the existing catalog frame benchmarks, which now
accept Quartz on Metal for the separate application-level layer. Before claiming application
performance parity, benchmark representative catalog interactions, retained dirty regions,
scrolling and startup as well. Other platforms need shared surface/completion adapters; the
initial GPU measurement surface is macOS Metal.

## Reproduce

```sh
cargo build --locked --release -p ferroui-rendering-benchmarks --features vello
python3 scripts/rendering-perf/compare.py \
  --backends skia,quartz,quartz-harfbuzz,vello-hybrid,vello-gpu,vello-cpu \
  --trials 4 --samples 100 --warmup 20 --scale 2 --other-work-paused \
  --out target/rendering-perf/macos
```

Run the binary directly after building. Do not compile, profile or process screenshots during
the timed session. Use the raw per-run JSON for distributions rather than comparing one lucky
minimum. Benchmark executables link multiple backends intentionally and are not binary-size
benchmarks of shipping applications.
