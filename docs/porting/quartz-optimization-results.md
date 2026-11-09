# macOS native rendering optimization results

Historical evidence recorded before the upstream rebase and privacy history rewrite. Superseded branch revision IDs and machine-specific metadata are omitted. These records do not measure the rebased code.

For the latest matched before/after measurements, see the [second optimization pass](quartz-optimization-round2-results.md). This report preserves the first pass and its original load session.

Measured 2026-11-08 on Apple M4, 24 GB RAM, macOS 27.2, at implementation commit `pre-rewrite revision`. The [committed results](quartz-optimization-results.json) retain trial medians/ranges, p95 values, source/font/binary identities, load snapshots and validation results. Full per-frame samples are in `target/rendering-perf/macos-optimized/comparison.json`.

Quartz has **19.8% lower frame time by geometric mean** across the eight rendering workloads (equivalent throughput factor 1.25×). This gives every workload equal weight and excludes shaping. It is a suite aggregate: Skia remains faster on rounded rectangles, geometry, bitmaps and opacity layers. Native text drawing and cached shadows are faster in the measured medians.

## Paired renderer comparison

Milliseconds per completed frame; `shape-*` rows are milliseconds per shaping call. The original-native column is the preceding [baseline session](quartz-performance.md), not another trial in this session.

| Workload | Original native | Optimized native | Current Skia | Native / Skia |
|---|---:|---:|---:|---:|
| rounded-rects | 5.2284 | 1.1624 | 0.8617 | 1.35× |
| geometry | 5.3366 | 1.3255 | 0.8667 | 1.53× |
| bitmaps | 1.9763 | 0.8592 | 0.6536 | 1.31× |
| text-latin | 46.0618 | 0.7480 | 0.8535 | 0.88× |
| text-arabic | 40.3504 | 0.6880 | 0.8909 | 0.77× |
| text-hebrew | 37.5692 | 0.6694 | 0.8657 | 0.77× |
| shadows | 6.4997 | 0.8703 | 10.7443 | 0.08× |
| opacity-layers | 40.3851 | 1.0760 | 0.7245 | 1.49× |
| shape-latin | 0.0189 | 0.0109 | 0.0122 | 0.89× |
| shape-arabic | 0.0145 | 0.0116 | 0.0050 | 2.32× |
| shape-hebrew | 0.0096 | 0.0066 | 0.0019 | 3.46× |

Text drawing is 12.4% faster for Latin, 22.8% faster for Arabic, 22.7% faster for Hebrew. Core Text remains the quality reference. Packed native masks match the unatlased reference within one channel value across scaling, rotation, mirroring and all interpolation modes. Color/subpixel raster data retain their full channels.

Core Text Latin shaping also improves; Arabic and Hebrew shaping remain slower than HarfBuzz. No complete text/shaping-result cache was added. The `quartz-harfbuzz` diagnostic adapter isolates shaper cost without changing the native renderer.

## Method and limits

One release binary with six adapters, four fresh processes per adapter, alternating forward/reverse order, 20 warmup frames and 100 timed frames per case. Rendering is 2560×1600 physical pixels at 2×. Workload definitions, fixed font bytes, glyph counts, Light hinting, antialiasing and GPU completion fences are unchanged. The new `--workload` filter is optional and was not used for this comparison.

All 24 runs passed validation: 19,200 timed frames and 230,400 timed shaping calls. Every native run reported zero renderer readbacks and zero GPU errors; explicit platform validation captures are outside timing. Backend sources and the executable were hashed before/after timing and did not change. The measured implementation checkout was clean.

The user’s pause state was not reconfirmed for this session. Background load was recorded, and this chat started no builds, tests, image processing or simulators while timing. OS services and desktop applications remained active. The four per-trial aggregate native/Skia ratios were 1.092, 0.762, 0.810, 0.784.

Variability matters: Latin native trial medians ranged 0.739–1.166 ms, while Skia ranged 0.771–0.879 ms. Median wins are not a claim that every run or tail sample is faster. Raw distributions are retained locally.

These are warmed full-redraw backend costs. Caches are production renderer caches and their population/invalidation costs are not included in warmed medians. The figures do not establish cold startup, displayed FPS, retained application performance, Intel/eGPU performance or physical iOS performance. The original session used the same machine/workloads but a different background-load session; before/after improvements should be read with that limitation.

## All adapters

| Workload | skia | quartz | quartz-harfbuzz | vello-hybrid | vello-gpu | vello-cpu |
|---|---:|---:|---:|---:|---:|---:|
| rounded-rects | 0.8617 | 1.1624 | 1.1689 | 2.6617 | 2.6930 | 3.8889 |
| geometry | 0.8667 | 1.3255 | 1.3136 | 2.5888 | 2.4616 | 3.6678 |
| bitmaps | 0.6536 | 0.8592 | 0.8604 | 1.8453 | 2.2515 | 3.8763 |
| text-latin | 0.8535 | 0.7480 | 0.7576 | 2.2767 | 2.9747 | 4.7717 |
| text-arabic | 0.8909 | 0.6880 | 0.6827 | 2.1965 | 2.9957 | 4.7990 |
| text-hebrew | 0.8657 | 0.6694 | 0.6659 | 2.1598 | 2.7702 | 4.4099 |
| shadows | 10.7443 | 0.8703 | 0.8677 | 2.4850 | 16.5330 | 6.0257 |
| opacity-layers | 0.7245 | 1.0760 | 0.8275 | 1.6747 | 2.2028 | 3.2273 |
| shape-latin | 0.0122 | 0.0109 | 0.0075 | 0.0076 | 0.0078 | 0.0084 |
| shape-arabic | 0.0050 | 0.0116 | 0.0043 | 0.0042 | 0.0042 | 0.0044 |
| shape-hebrew | 0.0019 | 0.0066 | 0.0018 | 0.0018 | 0.0018 | 0.0018 |

## Design and remaining work

See the [implementation design](quartz-metal-optimization.md) for batching, direct drawable access, glyph/coverage/layer atlases, bounded caches, source ownership and generation recovery. The graphics API and GPU media contracts remain unchanged. The native-only application graph continues to exclude bundled rendering/text stacks.

Further performance work should target general geometry and fractional bitmap coverage, opacity-group recording/allocation overhead, and representative catalog scrolling/startup. The remaining gaps above are measured rather than hidden by the aggregate.

## Reproduce

```sh
cargo build --locked --release -p ferroui-rendering-benchmarks --features vello
python3 scripts/rendering-perf/compare.py \
  --backends skia,quartz,quartz-harfbuzz,vello-hybrid,vello-gpu,vello-cpu \
  --trials 4 --samples 100 --warmup 20 --scale 2 \
  --out target/rendering-perf/macos-optimized
```

The benchmark executable links multiple backends and is not a shipping binary-size measurement. Application-size and catalog/simulator validation artifacts are recorded separately.


## Shipping binary size

Current HelloWorld, identical UI/theme, locked dependencies and `dist` profile (with `panic = "abort"`):

| Backend | Executable bytes | Decimal MB |
|---|---:|---:|
| Skia | 17,839,632 | 17.84 |
| Quartz Metal | 13,310,640 | 13.31 |

Native saves **4,528,992 bytes (25.39%)**. Its dependency graph excludes Skia, HarfBuzz,
Vello, wgpu and ANGLE; the optional runtime-shader compiler is excluded from this minimal
application. Catalog validation below enables runtime shaders. These are executable sizes,
not app bundle or resident-memory sizes. Full identities, commands, dependencies and library
lists are retained in the committed JSON; local artifacts are in
`target/quartz-optimization-size`.

The immediately preceding native source (`pre-rewrite revision`), rebuilt with the same compiler, lock
and profile, is 13,226,240 bytes. These optimizations add **84,400 bytes (0.64%)**. That
comparison uses a detached checkout in the same target directory; absolute checkout paths
differ. Its executable identity is recorded alongside the current pair.

## Application and regression validation

The implementation passed 65 Quartz tests, the same 65 tests with a forced separate render
destination, 25 CPU Quartz tests, and two shared-harness tests. New checks exercise atlas
eviction, nested/cropped opacity, undersized layer hints, foreign drawables without shader-write
access, draw ordering, and native glyph quality. Renderer error recovery and GPU media fixture
tests continue to pass. A physical webcam is unavailable.

On macOS, Control Catalog captured **15 selected pages through both render paths** (30 PNGs,
1100×800 pixels): Home, TextBlock, TextBox, Image, Visuals, Composition brushes, Shadows and
lights, Effect brushes, Acrylic brush, Blend and colour effects, Colour effects, Image effects,
Shader effects, Interactive effects and Page transitions. Representative TextBox, Visuals,
Shader effects and Acrylic captures were visually reviewed. The first capture attempt correctly
rejected the obsolete 2× display expectation; the completed run used the actual 1× display.
Artifacts and the manifest are in `target/quartz-optimization-catalog-1x`.

The native iOS simulator smoke test passed **41 checks**. The iOS runtime-shader catalog visited
Home, TextBlock, TextBox, Image, Visuals and Shader effects, produced all six screenshots and
exited successfully. Its counters reported zero renderer readbacks and zero GPU errors.
The shader-effects screenshot was visually reviewed. Logs are
`target/quartz-opt-ios-smoke.log` and `target/quartz-opt-ios-catalog.log`; captures are in
`~/Library/Caches/ferroui-ios-smoke/quartz-runtime-shaders`.

These captures validate selected pages and paths; they do not constitute an exhaustive visual
parity audit or physical iOS performance measurement. All builds, tests and captures took place
after the formal timed comparison.
