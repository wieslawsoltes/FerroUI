# Windows Gauntlet renderer optimization

The Windows renderer branch merged upstream main at `e1b36bb74` in
`eb0b1f450`. That revision accepts composition batches before rendering them,
which let the existing desktop benchmark advance two scene indices for some
presented frames. The fixed-frame mode now advances after the previous batch
reports it has rendered. Native present counts are checked inside the measured
interval. The timings cover the desktop scene cycle through render submission.

The optimized Direct2D renderer beat Skia in all three matched tier-16 pairs
at 125% scaling. The pooled mean was 87.53 ms per scene on Direct2D and
90.00 ms on Skia. See [the repeated runs](#solid-path-and-stroke-batching).

## Before glyph texture reuse

At `47d4c54af`, one matched 240-scene pair completed after 60 warm-up
scenes at 125% scaling and a 1028×743 physical client area. The pointer was
over the window in all measured scenes for both backends. The viewport
remained stable throughout.

| Backend | Mean ms | p95 ms | Scenes per second |
| --- | ---: | ---: | ---: |
| Skia/ANGLE | 81.55 | 110.90 | 12.26 |
| Direct2D | 97.25 | 126.26 | 10.28 |

Direct2D took 19.3% longer per scene and submitted 240 presents with zero
renderer errors and zero pixel readbacks during the timed run.

## Glyph textures and presentation

`9532b8a7f` reuses small opaque monochrome glyph runs on the GPU when the
same run appears in multiple frames with the same raster phase, scale and
paint. Textures stay immutable after publication. The atlas uses at most
16 MiB of ready pages and one in-progress page. DirectWrite still shapes the
text and Direct2D draws the grayscale glyph coverage. Color fonts and
unsupported transforms use their existing paths.

Finished window frames now copy their premultiplied BGRA texture directly
to the DXGI back buffer, keeping the root texture for dirty redraws. The
Windows SDK regression suite exercised the new copy under the D3D11 debug
layer with zero errors and warnings. The glyph test matched direct text
pixels at 125% DPI, including an integer translation, a clip and a
translucent destination. The 56 Rust native renderer tests also passed.

The first 240-scene pilot of these changes averaged 91.50 ms on Direct2D
and 91.98 ms on Skia, with p95 values of 114.03 ms and 124.91 ms respectively.
Direct2D submitted every measured scene. Its frame-300 compositor capture
was byte-identical to the native capture from `47d4c54af`.

Three subsequent pairs at `9532b8a7f` used 60 warm-up scenes and 240 measured
scenes per backend and pair. All runs used the 1028×743 physical client area
at 125% scaling, with the pointer over the window throughout. Direct2D
submitted all 720 measured presents with zero renderer errors and readbacks.

| Backend | Pair means (ms) | Pooled mean (ms) | Pooled p95 (ms) |
| --- | --- | ---: | ---: |
| Skia/ANGLE | 89.66, 86.88, 85.31 | 87.28 | 121.48 |
| Direct2D | 92.59, 91.96, 91.58 | 92.04 | 117.39 |

Direct2D took 5.5% longer per scene on the pooled mean. Its final frame-300
capture matched the pre-atlas native capture byte for byte.

In a separate 24-scene profile, glyph-core time averaged 4.80 ms and
submission work averaged 50.70 ms per scene. Tightening cache admission
reduced texture builds from 15,432 to 8,737 across startup, warm-up and
measurement in the two diagnostic processes. The [atlas timing samples and
profile logs](benchmarks/windows-gauntlet-glyph-atlas/) record these runs.

## Deferred clip and opacity draws

`89999ca34` reuses the pending rectangle intersection within a draw batch
and leaves a clip deferred when a solid fill fits within it with a full
device-pixel margin. It applies source-over opacity during the final bitmap
draw for ordinary isolated layers. Advanced blends and filters keep the
original compositing path.

Three further pairs matched the viewport, DPI, scene count and pointer
conditions of the atlas comparison. All 720 measured Direct2D scenes
presented with zero renderer errors and pixel readbacks.

| Backend | Pair means (ms) | Pooled mean (ms) | Pooled p95 (ms) |
| --- | --- | ---: | ---: |
| Skia/ANGLE | 85.26, 85.28, 87.32 | 85.95 | 118.91 |
| Direct2D | 90.18, 90.74, 90.80 | 90.57 | 111.27 |

Direct2D reduced its pooled mean by 1.47 ms from the previous three pairs.
Skia still led by 4.62 ms per scene. The native frame-300 capture remained
byte-identical, and all 56 Rust tests and the Windows SDK debug tests passed.
The 24-scene profile averaged 48.54 ms in submission work, with 12.35 ms in
glyph handling (4.82 ms in core glyph drawing), 6.12 ms in clip setup and
4.69 ms in ending layers.
[Clip and opacity samples and profile](benchmarks/windows-gauntlet-glyph-atlas/clip-opacity-pairs/)

## Solid path and stroke batching

`518d4f435` queues solid fills and keeps widened strokes in Direct2D from
widening through drawing. The previous path exported the widened outline to
Rust and reconstructed it before filling. The bridge also reuses eight
solid stroke styles. Dashed styles retain their existing creation path.

A 240-scene pilot averaged 88.07 ms on Direct2D and 89.17 ms on Skia.
The native frame-300 capture matched the earlier native build byte for byte.
The Rust suite passed 57 tests, including a pixel comparison between retained
and reconstructed strokes with round caps, curved joins, dash offsets and
fractional clipping. The Windows SDK debug suite passed with zero warnings
and errors.

Three confirmation pairs at `518d4f435` used 60 warm-up and 240 measured
scenes per backend and pair. Both backends ran on the same GTX 1660 Ti with
a 1028×743 physical client area at 125% scaling. The viewport stayed stable
and the pointer was over the window throughout every measured interval.

| Backend | Pair means (ms) | Pooled mean (ms) | Pooled p95 (ms) |
| --- | --- | ---: | ---: |
| Skia/ANGLE | 89.97, 89.42, 90.61 | 90.00 | 123.32 |
| Direct2D | 87.30, 86.72, 88.56 | 87.53 | 107.03 |

Direct2D used 2.75% less time per scene on the pooled mean and recorded all
720 measured presents with zero renderer errors or pixel readbacks.
Its frame-300 PNG matched the pre-atlas native capture at SHA-256
`9cb3e5918aec09999dd37adb3fe8437449810d37670c5cc14ef105ac7d84ad15`.
A separate 24-scene profile recorded zero time in native path export and
1.08 ms per scene in path creation. The
[timing samples, profiles and test logs](benchmarks/windows-gauntlet-stroke-batching/)
include executable hashes and adapter identifiers for the confirmation pairs.

## Matched 125% runs

Each pilot used Gauntlet tier 16, 60 warm-up and 48 measured scene frames, the
same Roboto files, a 1028×743 physical client area at 125% scaling, and the
pointer outside the client area. Both backends used the GTX 1660 Ti hardware
adapter on Windows 11 build 26200. Skia ran first in each pair. The baseline
used the rendered-batch harness from `0288704ef` and the renderer from the
merge. The optimized renderer used the changes in `0e3d7a25c`.

| Native build | Skia mean ms | Native mean ms | Native p95 ms | Native presents |
| --- | ---: | ---: | ---: | ---: |
| After upstream merge, before renderer optimizations | 87.06 | 194.22 | 254.50 | 48 |
| Shadow cache and retained paths | 88.94 | 113.92 | 155.98 | 48 |

Native mean scene time fell 41.3%. The optimized pilot took 28.1% longer per
scene than its paired Skia run. The two native frame-108 compositor PNGs were
byte-identical (SHA-256
`2f3d1b16c50d2d643351780892ac6d1cee901f7f079471daf044b4e65e7673e0`).
Native recorded zero errors and explicit readbacks in both timed runs.
The [timing samples and profiles](benchmarks/windows-gauntlet-2026-11-11/)
include the viewport, binary hashes, and per-run scene counts.

The guarded 240-frame comparison used the same 125% viewport with the
pointer over the window for all measured scenes on both backends. The viewport
stayed constant. Skia averaged 80.87 ms with a 93.72 ms p95. Direct2D averaged
96.58 ms with a 117.70 ms p95, 19.4% longer on the mean. Direct2D submitted 240
presents with zero renderer errors and readbacks. This pair used the candidate
with a small solid-brush cache. A follow-up A/B check measured 92.56 ms
without that cache and 96.32 ms with it across the same 48 scene frames. Skia
measured 83.78 ms and 83.98 ms in the respective pairs. The brush cache was
discarded. A prior long comparison was excluded after the display changed DPI.

## Profile and changes

The baseline repeatedly rasterized two masks, blurred the caster, dispatched
the box-shadow kernel, and copied the result through an intermediate target.
The device now retains up to 512 completed shadow textures within 32 MiB.
Reuse includes the exact local raster transform, shadow settings and target
crop. The final shader output can draw directly into the parent on a cache miss.
Across the two matched runs including warm-up, GPU copies fell from 64,045
to 930 and compute dispatches fell from 15,982 to 271.

Solid rounded fills and rounded clips reuse the original path geometry and
join the draw command batch. Solid paints also skip path bounds calculation
when the bounds are unused. The native suite passed 56 tests with one ignored.

In the separate 24-frame optimized CPU profile at 125% scaling, renderer
submission work averaged 65.27 ms per frame. Core glyph drawing averaged
16.39 ms, clip application 8.01 ms, and `EndDraw` 6.40 ms. These timers can
nest. A matched 175% pilot at the full 1280×820 logical size measured
207.58 ms for the intermediate native build and 86.54 ms for Skia. Keep those
results separate from the smaller 125% runs.

## Reproduction

Build the two release binaries from the same source. The native build uses
`--no-default-features --features direct2d-runtime-shaders`. Pass the binaries
to `scripts/compare-windows-gauntlet-renderers.ps1 -Scenarios gauntlet` using
`-SkiaExecutable` and `-NativeExecutable`. The defaults run three pairs of
60 warm-up and 240 measured scenes. Keep the desktop resolution, scaling and
pointer position stable. Enable `FERROUI_D3D_PROFILE=1` only in a separate
native diagnostic run.
