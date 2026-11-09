# Geometry and opacity optimization results

Historical evidence recorded before the upstream rebase and privacy history rewrite. Superseded branch revision IDs and machine-specific metadata are omitted. These records do not measure the rebased code.

The subsequent [opacity recording pass](quartz-optimization-round4-results.md) preserves this checkpoint and records the latest measurements and size.

Measured 2026-11-09 on Apple M4, 24 GB RAM, macOS 27.2. Implementation `pre-rewrite revision` is compared with the preserved [second-pass executable](quartz-optimization-round2-results.md), source `pre-rewrite revision`. The [committed JSON](quartz-optimization-round3-results.json) retains trial medians/ranges, percentiles, binary/source/font identities and load snapshots; raw distributions and BGRA frames are in `target/rendering-perf/macos-opt3-final`.

**Geometry frame time falls 21.9%; opacity-layer frame time falls 35.0%** versus the previous native implementation. Their CPU-before-wait wall time falls 48.6% and 57.7%, respectively. All eight rendering workload medians decrease in this session. Native Core Text remains the appearance reference, and all eight untimed validation frames are byte-identical before/after.

## Completed frame time

Milliseconds per completed frame; `shape-*` rows are milliseconds per shaping call. Both Skia columns are included because its implementation is unchanged, while its measured results vary with the recorded background load.

| Workload | Previous native | New native | Reduction | Previous-binary Skia | New-binary Skia |
|---|---:|---:|---:|---:|---:|
| rounded-rects | 0.9928 | 0.9126 | 8.1% | 0.7034 | 0.9271 |
| geometry | 1.2415 | 0.9692 | 21.9% | 0.7307 | 0.9565 |
| bitmaps | 0.6299 | 0.5794 | 8.0% | 0.5089 | 0.6261 |
| text-latin | 1.0181 | 0.9801 | 3.7% | 0.6539 | 0.6729 |
| text-arabic | 1.2164 | 0.9546 | 21.5% | 0.6560 | 0.6816 |
| text-hebrew | 1.1823 | 0.9395 | 20.5% | 0.6059 | 0.7369 |
| shadows | 1.4102 | 1.0654 | 24.5% | 7.2200 | 7.1053 |
| opacity-layers | 1.2027 | 0.7819 | 35.0% | 0.6304 | 0.6521 |
| shape-latin | 0.0105 | 0.0109 | -3.4% | 0.0083 | 0.0083 |
| shape-arabic | 0.0121 | 0.0124 | -2.4% | 0.0044 | 0.0044 |
| shape-hebrew | 0.0072 | 0.0072 | -0.1% | 0.0019 | 0.0018 |

Geometry is 0.9692 ms versus 0.9565 ms for Skia in the new executable, a 1.3% gap. Opacity is 0.7819 ms versus 0.6521 ms, a 19.9% gap. These are measured medians, not established parity: Skia geometry also varies from 0.7307 ms in the previous executable to 0.9565 ms in the new one, despite unchanged Skia/workload sources. Background variation is material.

## CPU recording evidence

CPU-before-wait is completed wall time minus the explicit completion-fence wait, including preemption/driver work. It is not a hardware CPU timer or an asynchronous submission measurement.

| Workload | Previous CPU ms | New CPU ms | Reduction |
|---|---:|---:|---:|
| geometry | 0.3594 | 0.1849 | 48.5% |
| opacity-layers | 0.3158 | 0.1335 | 57.7% |

Across all eight rendering cases, the equal-weight geometric mean of native after/before ratios is 0.815 (18.5% lower frame time). Against new-binary Skia the ratio is 0.900. The aggregate is helped by cached shadows and does not mean every workload beats Skia. Shaping code is unchanged; small shaping differences are within the session’s variability.

## Method and limitations

Two prebuilt release executables, Skia and Quartz in each, six fresh processes per adapter/revision. Revisions are interleaved and the complete order reversed on alternate trials. Each case uses 30 warmup and 150 timed samples, a 2560×1600 physical surface at 2×, the same workload definitions, fixed font bytes, glyph counts, Light hinting and grayscale AA, and a successful GPU-completion fence per frame. The baseline executable hash matches its previous committed report. The measured checkout was clean; both binaries and backend sources were unchanged during timing.

The user requested background load to be recorded. Other simulator/build/application and OS activity continued. This chat started no builds, tests, simulators or image processing during the timed run. All 24 runs validated: 28,800 timed rendering frames and 345,600 timed shaping calls. Native counters report zero renderer readbacks and zero GPU errors. Platform validation readbacks are outside timing. Raw samples and all outlier trials are retained locally.

Per-trial geometric native after/before ratios: 0.863, 0.929, 0.734, 0.719, 0.931, 0.867. Ranges and percentiles are retained in JSON. Load variation and the unchanged-Skia control results limit conclusions about small gaps or regression differences. These warmed backend measurements do not establish displayed FPS, cold startup, scrolling, process memory, Intel/eGPU behavior or physical iOS performance.

## Implementation and quality

The [geometry/opacity submission design](quartz-metal-optimization.md#geometry-and-opacity-submission-pass) describes allocation-free fixed-key lookups with owned insertion copies; lazy group storage; compatible ordered drawable-batch merging; render-pass descriptor reuse; and fewer repeated Objective-C state lookups. Dependency drains, clears, target/texture/clip/pipeline boundaries, source-owner leases and existing memory/command budgets remain intact. Core Text raster inputs, sampling and coverage retain their existing implementations.

All 71 Quartz tests pass normally and with separate destination textures forced. The new regression fixtures require full-key equality even when every hash collides; metadata-only empty groups and full-canvas empty Source behavior; and exact pixels with fewer encoded batches for overlapping groups crossing atlas pages and dependencies. Existing native mask, glyph eviction, effects, media, clipping, recovery and format regressions also pass.

All eight untimed benchmark validation frames match the preserved native executable byte-for-byte, including geometry, opacity and Latin/Arabic/Hebrew text. This covers the suite’s fixed validation frame; fractional, transformed, clipped, eviction and dependency cases are checked by the separate native pixel oracles. Rendering remains on the GPU, with the same native application dependency graph and Core Text appearance reference.

## Reproduce

For a new comparison, set `BASELINE_COMMIT` to the public commit used to build your preserved baseline executable. Historical source revisions are omitted from this branch.

Preserve the earlier release benchmark executable and its original source/build manifest, build the new executable before timing, then run:

```sh
cargo build --locked --release -p ferroui-rendering-benchmarks --features vello
python3 scripts/rendering-perf/compare.py \
  --backends skia,quartz \
  --baseline-binary target/quartz-opt3-profile/rendering-benchmarks-before \
  --baseline-source-commit "$BASELINE_COMMIT" \
  --trials 6 --samples 150 --warmup 30 --scale 2 \
  --out target/rendering-perf/macos-opt3-final
```

Catalog/simulator validation and shipping-size builds below run after formal timing.


## Catalog and simulator validation

The macOS Quartz Metal Control Catalog captured Home, TextBlock, TextBox, Image, Visuals,
Composition brushes, Shadows and lights, and Shader effects through both render paths:
16 PNGs, each 2200×1600. The composed Visuals frame was visually reviewed, including
transformed geometry, blur and redirected layers. The capture manifest and logs are in
`target/quartz-opt3-catalog` and `target/quartz-opt3-profile`.

The iOS runtime-shader simulator smoke test passed 41 checks. The catalog visited
TextBlock, TextBox, Image and Visuals, captured every selected page, and reported zero
renderer readbacks and zero GPU errors. The Visuals screenshot was visually reviewed.
Logs are `target/quartz-opt3-profile/ios-{smoke,catalog}.log`, with copied screenshots
in `target/quartz-opt3-ios`. The test app was uninstalled and its simulator shut down.
These checks establish selected-page coverage, not exhaustive catalog parity or physical
iOS performance. No physical webcam is available; GPU media fixture regressions pass.

## Shipping binary size

HelloWorld, identical UI/theme and dependency lock, same compiler, `dist` profile with
fat LTO, one codegen unit, stripped symbols and `panic = "abort"`:

| Backend / revision | Executable bytes | Decimal MB |
|---|---:|---:|
| Skia | 17,839,632 | 17.84 |
| Native, previous | 13,310,592 | 13.31 |
| Native, current | 13,310,848 | 13.31 |

The optimization costs **256 bytes**, about 0.002% of the native executable. Native remains
**4,528,784 bytes (25.39%) smaller than Skia**. The rebuilt Skia executable is byte-identical
to the preceding checkpoint. This measures executable size, not app bundles or resident
memory. The size-first native dependency graph continues to exclude Skia, HarfBuzz, Vello,
wgpu, ANGLE and the optional Naga compiler; catalog validation enables runtime shaders.

The JSON records commands, compiler/source/binary identities, linked libraries, native
dependency verification and the previous native artifact identity. Local binaries and the
dependency graph are in `target/quartz-opt3-size`.

The remaining work is to reduce geometry's recording/completion latency and opacity's
residual group overhead, then measure catalog scrolling and cold paths. Quiet-machine
repeat measurements would be needed to establish a small native-versus-Skia geometry gap.
