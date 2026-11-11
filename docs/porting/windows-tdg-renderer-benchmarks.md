# Windows TreeDataGrid renderer comparison — 2026-11-08

This is the initial baseline. See the [optimization follow-up](windows-tdg-renderer-optimization.md)
for fixes and subsequent measurements (native becomes 3.0–4.4× faster).
The later [parity follow-up](windows-tdg-renderer-parity.md) beats Skia across all
twelve measured workloads.

**Native is slower in every measured workload.** Through production render-data
recording/replay, Skia takes 0.399–1.790 ms per full redraw and native takes
3.650–8.693 ms: native/Skia ratios range from 4.2× to 9.4×. This is a measurable
deficit on this machine, not evidence of a native speedup. The separate immediate
drawing diagnostic also exposes a much larger opacity-layer overhead.

The existing TreeDataGrid suite uses mock drawing services: its layout timings
alone cannot compare GPU renderers. The optional `renderer-benchmarks` mode adds
real text and offscreen GPU drawing to the existing virtualization fixtures.
See [the runner instructions](../../tests/FerroUI.TreeDataGrid.Benchmarks/README.md#windows-gpu-renderer-comparison).

## Machine and workload

- Windows 11 Pro, build 26200; Intel Core i5-12400F; approximately 48 GiB RAM.
- NVIDIA GeForce GTX 1660 Ti, driver 32.0.16.1088.
- Both backends select adapter LUID `00000000:000102c7` (bytes
  `[199, 2, 1, 0, 0, 0, 0, 0]`). Skia uses ANGLE/D3D11 and HarfBuzz;
  native uses D3D11/Direct2D and DirectWrite.
- Rust 1.93.0, x86_64 MSVC, Cargo `--release`. Each comparison uses one executable
  containing both backends, in separate processes: **Skia first, native second**.
  No concurrent compilation or other benchmark during timed runs.
- 800×500 physical-pixel GPU target, scale 1, Segoe UI 12, 24-unit rows.
  Cell templates bind actual text values, and nonempty realized text is asserted.
- Both classic and neutral sources; six scenarios per source. Vertical grids
  have 10,000 rows × 12 columns; horizontal grids 1,000 × 200; far-column grids
  1,000 × 1,000. The scrolling script, data and viewport match across backends.
- Three warm-up batches and fifteen measured batches of twenty frames per case
  (360 frames/case, 4,320 frames/backend). Each batch ends with GPU completion,
  using `glFinish` for Skia or a D3D11 event query for native. Skia also flushes
  each frame so deferred commands cannot discard earlier full redraws.
- PNG validation exports occur before timing. Native timed intervals assert no
  increase in readback or renderer-error counters.

## Recorded drawing results

Source: `329a6a9f343ff1422e2ad033bb7091109b2ee114`.
Executable SHA-256:
`11163D99077B3CF8992BB38C45B514E1742AD75019D51F0B6B6E07D35C1E63B8`.
All twelve cases completed on each backend; native assertions confirm no timed
readbacks or renderer errors. Both backends realize the same row counts and
189 nonempty text blocks per initial viewport.

| Scenario | Source | Skia ms/frame | Native ms/frame | Native / Skia |
|---|---|---:|---:|---:|
| Stationary redraw | Classic | 0.401 ± 0.027 | 3.666 ± 0.094 | 9.1× |
| Vertical, 1 px | Classic | 0.452 ± 0.025 | 3.802 ± 0.149 | 8.4× |
| Vertical, 1 row | Classic | 0.590 ± 0.039 | 4.201 ± 0.300 | 7.1× |
| Horizontal, 1 px | Classic | 0.429 ± 0.024 | 3.650 ± 0.097 | 8.5× |
| Horizontal, column | Classic | 1.656 ± 0.059 | 8.333 ± 1.661 | 5.0× |
| Horizontal, far column | Classic | 1.682 ± 0.036 | 8.693 ± 0.564 | 5.2× |
| Stationary redraw | Neutral | 0.399 ± 0.022 | 3.749 ± 0.208 | 9.4× |
| Vertical, 1 px | Neutral | 0.479 ± 0.061 | 4.089 ± 0.468 | 8.5× |
| Vertical, 1 row | Neutral | 0.609 ± 0.032 | 3.871 ± 0.182 | 6.4× |
| Horizontal, 1 px | Neutral | 0.464 ± 0.026 | 3.683 ± 0.204 | 7.9× |
| Horizontal, column | Neutral | 1.754 ± 0.046 | 7.403 ± 0.444 | 4.2× |
| Horizontal, far column | Neutral | 1.790 ± 0.057 | 8.615 ± 0.498 | 4.8× |

Values are mean ± sample standard deviation. The mean differences exceed batch
variation in every case. These are fifteen batches within one ordered run per
backend, not independent repeated experiments or a randomized paired study.

The deficit includes more than asynchronous GPU completion. For classic
stationary redraw, mean layout/submission/drained totals are respectively
0.003/0.384/0.401 ms for Skia and 0.007/3.350/3.666 ms for native. For classic
column scrolling, layout alone is 1.122 ms versus 4.565 ms. Real platform font
services participate in layout here. Submission can also block inside driver
calls: these counters do not isolate pure CPU or pure GPU cost, and a GPU/CPU
profile is needed before assigning the remaining deficit to particular calls.

Raw samples and successful completion logs:
[Skia JSON](benchmarks/windows-tdg-2026-11-08/recorded-skia.json),
[native JSON](benchmarks/windows-tdg-2026-11-08/recorded-direct2d.json),
[Skia log](benchmarks/windows-tdg-2026-11-08/recorded-skia.log),
[native log](benchmarks/windows-tdg-2026-11-08/recorded-direct2d.log).

## Interpretation

Every frame includes layout and a complete redraw into an offscreen target.
These measurements do not include swapchain presentation, vsync, input dispatch,
retained-scene caching or dirty-region savings. A stationary case is deliberately
a redraw, not an idle compositor frame. Do not interpret reciprocal times as
observed catalog FPS. Standard deviations and p95 values in the JSON describe
twenty-frame **batch means**, not individual frame latency.

The `recorded` path rebuilds a production `RenderDataDrawingContext` stream and
replays it every frame. It includes both recording and drawing costs and the
recorder's normal elimination of identity transforms and opacity-1 scopes. It
uses production recording/replay code, but is not a full window compositor test.

The diagnostic `immediate` path draws straight through `PlatformDrawingContext`.
`ImmediateRenderer` pushes opacity even when it is 1 for every visible visual;
Direct2D currently allocates a layer for these pushes. The production recorder
and retained visual renderer skip opacity-1 scopes. This distinction makes the
immediate results unsuitable as a proxy for normal catalog frame time.

## Diagnostic immediate drawing

Source: `4c6f9e97f6c010b788a88d5d72ab561e70611c2d`.
Executable SHA-256:
`8F9B01698BE159E1177775364D7A554E81589EF77273D5ABCD83D5FED1BFDE62`.
All twelve cases completed on both backends, without native timed readbacks or
renderer errors. This run preceded the addition of the draw-path selector; that
executable only implements immediate drawing.

| Scenario | Source | Skia ms/frame | Native ms/frame | Native / Skia |
|---|---|---:|---:|---:|
| Stationary redraw | Classic | 0.367 ± 0.034 | 125.338 ± 4.214 | 341.7× |
| Vertical, 1 px | Classic | 0.450 ± 0.051 | 128.025 ± 2.886 | 284.5× |
| Vertical, 1 row | Classic | 0.587 ± 0.036 | 118.322 ± 4.276 | 201.5× |
| Horizontal, 1 px | Classic | 0.404 ± 0.019 | 116.512 ± 4.276 | 288.3× |
| Horizontal, column | Classic | 1.766 ± 0.172 | 121.284 ± 4.023 | 68.7× |
| Horizontal, far column | Classic | 1.760 ± 0.140 | 124.825 ± 9.108 | 70.9× |
| Stationary redraw | Neutral | 0.384 ± 0.047 | 121.448 ± 20.149 | 316.6× |
| Vertical, 1 px | Neutral | 0.422 ± 0.024 | 125.572 ± 5.162 | 297.5× |
| Vertical, 1 row | Neutral | 0.565 ± 0.036 | 116.798 ± 2.996 | 206.7× |
| Horizontal, 1 px | Neutral | 0.450 ± 0.030 | 114.957 ± 5.427 | 255.2× |
| Horizontal, column | Neutral | 1.808 ± 0.205 | 119.022 ± 2.666 | 65.8× |
| Horizontal, far column | Neutral | 1.798 ± 0.092 | 120.201 ± 3.639 | 66.9× |

Values are mean ± sample standard deviation. The native immediate path has a
large, measurable performance deficit. Full-target opacity layers are a likely
contributor based on code inspection; this is not a GPU-profiled attribution of
all elapsed time. No production renderer optimization was made between runs.

Raw immediate samples and completion logs:
[Skia JSON](benchmarks/windows-tdg-2026-11-08/immediate-skia.json),
[native JSON](benchmarks/windows-tdg-2026-11-08/immediate-direct2d.json),
[Skia log](benchmarks/windows-tdg-2026-11-08/immediate-skia.log),
[native log](benchmarks/windows-tdg-2026-11-08/immediate-direct2d.log).

Preflight captures of the recorded path:
[Skia](benchmarks/windows-tdg-2026-11-08/recorded-skia-validation.png) and
[native](benchmarks/windows-tdg-2026-11-08/recorded-direct2d-validation.png).
All twelve preflight cases passed on each backend. These captures are visual
workload checks from the unoptimized build; their timings are not used here.

## Reproduction

Build using the command in the runner README, then invoke the resulting
`ferroui_treedatagrid_benchmarks-*.exe` directly. The measured runs used:

```powershell
$env:FERROUI_BENCH_ITERATIONS = '15'
$env:FERROUI_BENCH_WARMUP = '3'
$env:FERROUI_TDG_FRAMES_PER_BATCH = '20'
$env:FERROUI_TDG_DRAW_PATH = 'recorded'
$env:FERROUI_TDG_BACKEND = 'skia'
$env:FERROUI_TDG_RESULTS_JSON = 'target/native-evidence/tdg-recorded-release-skia.json'
& target/tdg-renderer-release/release/deps/ferroui_treedatagrid_benchmarks-b0697de91f0ec0ec.exe run_renderer_benchmarks --ignored --nocapture --test-threads=1
$env:FERROUI_TDG_BACKEND = 'direct2d'
$env:FERROUI_TDG_RESULTS_JSON = 'target/native-evidence/tdg-recorded-release-direct2d.json'
& target/tdg-renderer-release/release/deps/ferroui_treedatagrid_benchmarks-b0697de91f0ec0ec.exe run_renderer_benchmarks --ignored --nocapture --test-threads=1
```

No scenario filter or allocation counting was enabled, and capture exports were
disabled for the measured recorded-path runs. The diagnostic immediate build
used the same counts and order, before the path selector existed. It was built
and measured before the recorded-path build. Neither comparison modified
production renderer behavior. Later branch integration can change the workspace
revision; the source commits and executable hashes above identify these runs.
