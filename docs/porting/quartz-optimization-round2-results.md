# macOS native rendering: second optimization pass

Historical evidence recorded before the upstream rebase and privacy history rewrite. Superseded branch revision IDs and machine-specific metadata are omitted. These records do not measure the rebased code.

See the [subsequent geometry/opacity pass](quartz-optimization-round3-results.md) for the latest matched measurements and shipping-size checkpoint. This report preserves its original session.

Measured 2026-11-09 on Apple M4, 24 GB RAM, macOS 27.2. Implementation: `pre-rewrite revision`; previous implementation: `pre-rewrite revision`. The [committed measurements](quartz-optimization-round2-results.json) retain executable/source/font identities, trial medians/ranges, percentiles, GPU counters and background load. Raw distributions remain in `target/rendering-perf/macos-opt2-final`.

Native has **12.5% lower frame time by geometric mean** than the previous native implementation across eight rendering workloads. All eight median frame times decrease. The targeted rounded-rectangle, geometry, bitmap and opacity workloads improve; CPU recording costs also decrease. Core Text remains the text-quality reference.

**Skia still leads the remaining workloads.** Under this session’s recorded background load, native is 7.8% slower than Skia by the same aggregate, winning only cached shadows. The previous session’s native text-drawing wins are not reproduced for either the preserved previous implementation or the new one. This is why the new runner interleaves both revisions in the same session; the before/after improvement should not be confused with a universal Skia win.

## Completed frame time

Milliseconds per completed frame, including successful GPU completion. `shape-*` rows are milliseconds per shaping call. Positive reduction means native improves.

| Workload | Previous native | New native | Native reduction | Current Skia |
|---|---:|---:|---:|---:|
| rounded-rects | 1.0240 | 0.8813 | 13.9% | 0.6772 |
| geometry | 1.0381 | 0.9617 | 7.4% | 0.6400 |
| bitmaps | 0.7620 | 0.5509 | 27.7% | 0.4873 |
| text-latin | 1.0704 | 0.9494 | 11.3% | 0.6198 |
| text-arabic | 1.0328 | 0.9181 | 11.1% | 0.6503 |
| text-hebrew | 0.9704 | 0.9119 | 6.0% | 0.6144 |
| shadows | 1.0799 | 1.0529 | 2.5% | 7.3274 |
| opacity-layers | 1.2289 | 1.0101 | 17.8% | 0.5664 |
| shape-latin | 0.0119 | 0.0113 | 4.7% | 0.0088 |
| shape-arabic | 0.0131 | 0.0128 | 2.3% | 0.0044 |
| shape-hebrew | 0.0078 | 0.0078 | -0.9% | 0.0018 |

## CPU recording evidence

`CPU before wait` is completed wall time minus the explicit fence wait; it includes preemption and driver work and is not a hardware CPU timer. The four targeted rows improve in both metrics.

| Workload | Previous CPU ms | New CPU ms | Reduction |
|---|---:|---:|---:|
| rounded-rects | 0.3629 | 0.2885 | 20.5% |
| geometry | 0.3152 | 0.2406 | 23.7% |
| bitmaps | 0.1160 | 0.0770 | 33.6% |
| opacity-layers | 0.4176 | 0.2724 | 34.8% |

Core Text shaping changes are small relative to noise: Latin and Arabic medians decrease, while Hebrew is effectively unchanged (about 1% higher). HarfBuzz remains faster. No text/shaping-result cache or font/hinting/AA simplification was introduced.

## Method and limits

Two prebuilt release binaries, each with the same backend adapters, interleaved before/after Skia and Quartz. Six fresh processes per adapter/revision, alternating forward/reverse order; 30 warmup and 150 timed samples per case, 2560×1600 physical pixels at 2×. Workload definitions, platform surface, font bytes, per-adapter glyph counts, hinting and antialiasing are unchanged. The original baseline executable hash matches its previously committed build report. Both executable hashes and current backend sources remain unchanged throughout timing; the measured checkout was clean.

The user explicitly requested running with background load recorded. Simulator/build activity from other work and OS/desktop services remained active. This chat started no builds, tests, simulators or image processing during timing. There are 24 validated runs, 28,800 timed rendering frames and 345,600 timed shaping calls. Every native run reports zero renderer readbacks and zero GPU errors; validation readbacks are untimed.

Per-trial equal-weight native after/before ratios: 0.953, 0.819, 0.873, 0.773, 0.797, 0.911. Individual trial ranges and percentiles are retained. Background-load variation prevents interpreting small differences as statistically established improvements. No samples or outlier trials were discarded from the final run.

These are warmed backend redraw costs, not displayed FPS, cold startup, catalog scrolling, process-memory limits, Intel/eGPU results or physical iOS performance. Existing cache budgets are unchanged. Atlas/cache invalidation and quality regressions are covered separately from warmed timing.

## Implementation

See the [second-pass design](quartz-metal-optimization.md#second-optimization-pass): full-content float-key hashing, removal of redundant property locks under the renderer lock, compact solid rectangles, exact fractional image coverage with canvas clipping, and immutable uniform Core Text attributes. Ordinary image/native text pipelines retain their previous vertex payload and sampling. Fractional nearest-neighbor, high-quality reconstruction, arbitrary transforms and clipped images retain the general path.

## Reproduce

For a new comparison, set `BASELINE_COMMIT` to the public commit used to build your preserved baseline executable. Historical source revisions are omitted from this branch.

Preserve the earlier release benchmark executable and its build/source manifest, then build the new executable before timing. The baseline source revision is supplied by the caller and its executable identity is verified separately.

```sh
cargo build --locked --release -p ferroui-rendering-benchmarks --features vello
python3 scripts/rendering-perf/compare.py \
  --backends skia,quartz \
  --baseline-binary target/quartz-opt2-profile/rendering-benchmarks-before \
  --baseline-source-commit "$BASELINE_COMMIT" \
  --trials 6 --samples 150 --warmup 30 --scale 2 \
  --out target/rendering-perf/macos-opt2-final
```

All validation builds and shipping-size measurements below take place after the formal timing run.


## Quality and application validation

A separate capture-isolation regression run compares the previous and new executables:
**all eight untimed benchmark validation frames are byte-for-byte identical**, including
Latin, Arabic and Hebrew text. This checks the suite's fixed validation frame, not every
animated/fractional frame. The separate fractional-image/native geometry oracles cover
those additional paths. Both revisions' BGRA images and their hashes are retained in
`target/rendering-perf/quartz-opt2-capture-isolation-test`. Its short-run timing data is
excluded from the performance report. The runner now isolates capture directories by
revision as well as adapter; the formal run predates this output-path-only fix.

All 68 Quartz tests pass, as do the same 68 tests with separate destination textures
forced, and 26 CPU Quartz tests. New checks compare plain rectangle pixels exactly with
native geometry, and compare 80 fractional-image cases against the general path within
one channel value. These include scaling, mirroring, rotation, clipping, all interpolation
modes, aliased/antialiased edges, two opacity groups and painting beyond a shared atlas
view's logical canvas. Nearest-neighbor draws retain the general implementation after
that oracle identified a sampling-boundary difference. The existing packed-versus-unatlased
Core Text quality test passes unchanged; so do exact complex-script shaping checks.

The macOS Control Catalog captured Home, TextBlock, TextBox, Image, Visuals, Composition
brushes, Shadows and lights, and Shader effects through both render paths: 16 PNGs at
2200×1600. Image, Visuals and TextBox composed captures were visually reviewed. The
manifest/logs are under `target/quartz-opt2-catalog` and `target/quartz-opt2-profile`.

The iOS native runtime-shader simulator smoke test passed 41 checks. Its catalog visited
TextBlock, TextBox, Image and Visuals, captured all four pages, and reported zero renderer
readbacks and zero GPU errors. The Visuals screenshot was visually reviewed. The application
was uninstalled and this test's simulator shut down after both runs. Copied screenshots
are in `target/quartz-opt2-ios`; logs are `target/quartz-opt2-profile/ios-{smoke,catalog}.log`.
These checks establish selected-page/simulator coverage, not exhaustive visual parity or
physical-device performance. No physical webcam is available; GPU media fixture tests pass.


## Shipping binary size

HelloWorld uses identical UI/theme and locked dependencies; `dist` profile, fat LTO,
one codegen unit, stripped symbols and `panic = "abort"`, with the same compiler.
The optional native runtime-shader compiler is excluded from this minimal app.

| Backend / revision | Executable bytes | Decimal MB |
|---|---:|---:|
| Skia, current | 17,839,632 | 17.84 |
| Native, previous | 13,310,640 | 13.31 |
| Native, current | 13,310,592 | 13.31 |

Native size is effectively unchanged: **48 bytes smaller** than before. It remains
**4,529,040 bytes (25.39%) smaller than Skia**. The rebuilt Skia executable is
byte-identical to the preceding shipping-size checkpoint. These are executable sizes,
not bundles, compressed downloads or resident memory. The native dependency graph
continues to exclude Skia, HarfBuzz, Vello, wgpu, ANGLE and the optional Naga compiler.

The committed JSON retains both build commands, source/compiler/executable identities,
linked libraries, native dependency exclusion and the previous native artifact identity.
Local reports, dependency graph and executables are in `target/quartz-opt2-size`.
The earlier first-pass prose incorrectly called this profile unwinding; that description
is corrected without changing the original measurements. Both checkpoints use abort.

Further work should target the remaining geometry submission and GPU-completion costs,
opacity-group overhead and representative catalog scrolling/startup, using this same
matched runner. Skia still leads general rendering and shaping in this load session;
Core Text remains the appearance reference throughout further optimization.
