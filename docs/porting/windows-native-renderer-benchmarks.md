# Windows native renderer: benchmarks

Measurements of the native Windows renderer (`ferroui-direct2d`: Direct2D and DirectWrite on
Direct3D 11) against the default renderer (Skia on ANGLE with HarfBuzz) on the same machine. The
backend itself is described in [windows-native-renderer.md](windows-native-renderer.md).

The numbers are from one machine and one set of workloads. They say how the two renderers compare
there, not how fast an application will be on other hardware.

## Method

Machine: Intel Core i5-12400F, NVIDIA GeForce GTX 1660 Ti, Windows 11 Pro build 26200, Rust 1.93.0
for `x86_64-pc-windows-msvc`, release builds. Both renderers ran on the hardware adapter; a
software adapter is refused by both benchmark programs.

Rules common to all runs:

- The executables are built before the measurement and their hashes are checked after every run.
  Nothing is compiled, profiled or captured while a measurement runs.
- The backends run in pairs, Skia first and the native renderer second, and the pairs are
  repeated. No pair and no sample is discarded.
- A native run counts only when the renderer reports no error and no pixel readback during the
  measured interval.
- `FERROUI_D3D_PROFILE` is not set: collecting the timings changes them.

### TreeDataGrid

The renderer benchmarks of `tests/FerroUI.TreeDataGrid.Benchmarks` (feature `renderer-benchmarks`)
lay out a `TreeDataGrid` and draw the whole window into an offscreen GPU target. One release
executable contains both backends, and each process runs one of them.

- Viewport 800 x 500 physical pixels at a scaling of 1, Segoe UI at size 12, rows of 24 pixels,
  text shaped by the shaper of the backend.
- Two data sources (classic and neutral) and six workloads each: a stationary redraw, vertical
  scrolling by one pixel and by one row, horizontal scrolling by one pixel, by one column and to a
  far column. The grids have 10,000 rows of 12 columns for the vertical workloads, 1,000 rows of
  200 columns for the horizontal ones and 1,000 rows of 1,000 columns for the far column.
- Every frame scrolls, lays out, records the render data of the window and replays it onto the
  GPU. Skia flushes and submits each frame. A batch of twenty frames ends when the GPU has
  completed its work (`glFinish` for Skia, an event query for Direct3D 11), and the time of the
  batch is divided by its frames.
- Per workload: three warm-up batches, then fifteen measured batches. Five pairs give 75 batch
  samples per backend and workload.

The time of a frame therefore contains scrolling, layout, recording, replay, submission and GPU
completion. It contains no presentation and no vertical synchronization, and no reuse of a
retained frame: every frame is a full redraw.

### Gauntlet and still wall

`samples/GauntletBenchmark` (feature `renderer-measurement`) shows its two screens in a real
window: the gauntlet, an animated screen with heavy effects, at tier 16, and the still wall, a
static screen, at tier 20. A scene frame is advanced on the UI thread when the compositor reports
the previous one as rendered, so both backends draw the same scene frames. The measured value is
the interval between scene frames: the UI and compositor cycle through render submission. The
native swap chain presents with a sync interval of zero.

- One executable per backend, built from the same source: the default features for Skia,
  `direct2d-runtime-shaders` for the native renderer.
- 60 warm-up scene frames, then 240 measured ones per process; three pairs give 720 intervals per
  backend and scenario.
- Desktop of 1024 x 768 at 60 Hz with a scaling of 125 %. The requested window of 1280 x 820
  points is clamped by the desktop to a client area of 1028 x 743 physical pixels, the same in
  every run. A run in which the viewport or the scaling changes is rejected.
- The same Roboto font files are loaded into both backends.
- Each native run has to present once per measured scene frame.

## Results

### TreeDataGrid

Measured on 2026-11-09. Mean time per frame in milliseconds with the sample standard deviation of
the 75 batch samples; lower is better.

| Workload | Source | Skia ms | Native ms | Native / Skia |
| --- | --- | ---: | ---: | ---: |
| Stationary redraw | Classic | 0.416 ± 0.071 | 0.304 ± 0.022 | 0.731 |
| Vertical, 1 px | Classic | 0.484 ± 0.095 | 0.405 ± 0.063 | 0.838 |
| Vertical, 1 row | Classic | 0.634 ± 0.115 | 0.542 ± 0.098 | 0.854 |
| Horizontal, 1 px | Classic | 0.455 ± 0.079 | 0.362 ± 0.042 | 0.796 |
| Horizontal, column | Classic | 1.663 ± 0.174 | 1.484 ± 0.207 | 0.893 |
| Horizontal, far column | Classic | 1.753 ± 0.245 | 1.550 ± 0.230 | 0.884 |
| Stationary redraw | Neutral | 0.415 ± 0.060 | 0.327 ± 0.041 | 0.787 |
| Vertical, 1 px | Neutral | 0.491 ± 0.078 | 0.400 ± 0.048 | 0.815 |
| Vertical, 1 row | Neutral | 0.689 ± 0.138 | 0.536 ± 0.073 | 0.779 |
| Horizontal, 1 px | Neutral | 0.521 ± 0.096 | 0.395 ± 0.050 | 0.758 |
| Horizontal, column | Neutral | 1.902 ± 0.315 | 1.593 ± 0.222 | 0.838 |
| Horizontal, far column | Neutral | 1.909 ± 0.305 | 1.605 ± 0.205 | 0.841 |

The native renderer takes 10.7 % to 26.9 % less time per frame in all twelve workloads.

### Gauntlet, tier 16

Measured on 2026-11-11. Time per scene frame in milliseconds over three pairs of 240 scene frames;
lower is better. The pointer was over the window during every measured frame of both backends.

| Backend | Pair means (ms) | Pooled mean (ms) | Pooled 95th percentile (ms) |
| --- | --- | ---: | ---: |
| Skia on ANGLE | 89.97, 89.42, 90.61 | 90.00 | 123.32 |
| Native | 87.30, 86.72, 88.56 | 87.53 | 107.03 |

The native renderer takes 2.75 % less time per scene frame on the pooled mean and is faster in
each of the three pairs. It presented all 720 measured frames.

### Still wall, tier 20

Measured on 2026-11-10. Mean time per scene frame over 720 intervals, with the sample standard
deviation of the three run means, and the rate at which scene frames were delivered. The pointer
was outside the window.

| Backend | Mean (ms) | Scene frames per second |
| --- | ---: | ---: |
| Skia on ANGLE | 79.13 ± 0.06 | 12.64 |
| Native | 26.95 ± 0.58 | 37.10 |

The native renderer takes 0.341 of the time of Skia. This measurement is older than the gauntlet
one above: it was taken before the optimisation steps of the gauntlet in the next section and
before scene frames were advanced on the rendered report of the compositor, and it has not been
repeated since.

### Binary size

HelloWorld in the `dist` profile for `x86_64-pc-windows-msvc`, measured by
`scripts/measure-windows-native.ps1` on the source of 2026-11-07. Each deployment is one
executable; the libraries it imports belong to Windows.

| Build | Executable (bytes) | gzip (bytes) | Against Skia |
| --- | ---: | ---: | ---: |
| Skia and HarfBuzz | 42,464,768 | 15,558,431 | |
| Native, built-in shaders | 28,883,968 | 10,124,067 | 31.98 % smaller |
| Native with `runtime-shaders` | 30,820,864 | 10,887,986 | 27.42 % smaller |

## Optimisation steps

Each row is one step and what was measured after it. The TreeDataGrid rows give the range of the
native to Skia time ratio over the twelve workloads; the gauntlet rows give the mean time per scene
frame at tier 16 of the native renderer and of Skia in the same pairs.

| Benchmark | Step | Native | Skia in the same run |
| --- | --- | ---: | ---: |
| TreeDataGrid | First complete implementation | 4.2 to 9.4 times the Skia time (3.650 to 8.693 ms) | 0.399 to 1.790 ms |
| TreeDataGrid | Axis-aligned clips without layers, device checks at session boundaries only, glyph outlines on demand, no scope for an opacity of 1, glyph metric cache, reused brush and text analyzer | 1.38 to 2.18 times (0.844 to 2.546 ms) | 0.393 to 1.768 ms |
| TreeDataGrid | Draw commands in batches across the C ABI, retained positioned glyph runs, deferred text clips, combined glyph draws, shaping cache, fast path for simple shaping | 0.80 to 0.92 times (0.324 to 1.558 ms) | 0.396 to 1.797 ms |
| TreeDataGrid | Least-recently-used cache of positioned runs admitted on repeated use, deferred transform updates, shortcut for the overlap scan of glyph runs | 0.73 to 0.89 times (0.304 to 1.605 ms) | 0.415 to 1.909 ms |
| Gauntlet | Before the steps below (48 scene frames) | 194.22 ms | 87.06 ms |
| Gauntlet | Cache of finished box shadows, retained path geometry for rounded fills and clips (48 scene frames) | 113.92 ms | 88.94 ms |
| Gauntlet | Glyph atlas for repeated runs, root frame copied directly to the back buffer | 92.04 ms | 87.28 ms |
| Gauntlet | Deferred rectangular clips, opacity applied in the final draw of an isolated layer | 90.57 ms | 85.95 ms |
| Gauntlet | Solid fills in the command batch, strokes widened by Direct2D instead of being exported and rebuilt | 87.53 ms | 90.00 ms |

In the second gauntlet step the GPU copies of a run, warm-up included, fell from 64,045 to 930 and
the compute dispatches from 15,982 to 271.

## Reproduce

Both comparison scripts need PowerShell 7.2 or later and a Windows machine with a hardware GPU.
Paths are relative to the repository root. Close other programs, do not touch the machine during a
run, and keep the desktop resolution and scaling fixed.

### TreeDataGrid

Build the test executable once. With `--message-format=json` cargo reports its path, which
contains a hash that changes with the build:

```powershell
$executable = cargo test --locked --release -p ferroui-treedatagrid-benchmarks `
        --no-default-features --features renderer-benchmarks --lib --no-run `
        --target-dir target/tdg-renderer-release --message-format=json |
    ForEach-Object { $_ | ConvertFrom-Json } |
    Where-Object { $_.reason -eq 'compiler-artifact' -and $_.profile.test -and $_.executable } |
    Select-Object -Last 1 -ExpandProperty executable

./scripts/compare-windows-tdg-renderers.ps1 -Executable $executable
```

Without `--message-format=json` the same command prints the path in its last line, after
`Executable`.

The script runs five pairs (`-Repetitions`) and writes `run.json` and, per pair and backend, a
result file and a log to `target/tdg-renderer-comparison` (`-OutputDirectory`). Each result file
has, per workload, the batch samples, their mean and deviation, and the layout, recording, replay
and submission parts. The environment variables the script sets, and those for a single run or a
diagnostic one, are described in the README of the benchmarks, section
[Windows GPU renderer comparison](../../tests/FerroUI.TreeDataGrid.Benchmarks/README.md#windows-gpu-renderer-comparison).

### Gauntlet and still wall

Build one executable per backend into its own target directory, both with the feature
`renderer-measurement`:

```powershell
cargo build --locked --release -p gauntlet-benchmark --features renderer-measurement `
    --target-dir target/gauntlet-skia
cargo build --locked --release -p gauntlet-benchmark --no-default-features `
    --features direct2d-runtime-shaders,renderer-measurement --target-dir target/gauntlet-native

./scripts/compare-windows-gauntlet-renderers.ps1 `
    -SkiaExecutable target/gauntlet-skia/release/perf-compare-ferroui.exe `
    -NativeExecutable target/gauntlet-native/release/perf-compare-ferroui.exe
```

The script runs three pairs of both scenarios with 60 warm-up and 240 measured scene frames and
writes its results to `target/gauntlet-renderer-comparison`. `-Scenarios gauntlet` or
`-Scenarios wall` runs one scenario; `-Repetitions`, `-WarmupFrames` and `-Frames` change the
counts, and `-GauntletTier` and `-WallTier` the tiers. It refuses an executable built without
`renderer-measurement`. It fails when a run is incomplete, when the viewport or the scaling
changed, when the native renderer did not present every measured frame or reported an error or a
readback, or when the two backends did not have the same client size, scaling and pointer
condition. Keep the pointer either outside the window or over it for the whole comparison.

The measurement mode and its environment variables are described in the README of the sample,
section
[Windows GPU backend comparison](../../samples/GauntletBenchmark/README.md#windows-gpu-backend-comparison).

### Binary size

```powershell
./scripts/measure-windows-native.ps1 -IncludeRuntimeShaders
```

The script needs a clean, committed working tree and writes
`target/windows-native-size/results.json`; see
[Measure binary size](windows-native-renderer.md#measure-binary-size).
