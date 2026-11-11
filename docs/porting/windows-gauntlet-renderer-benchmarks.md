# Windows gauntlet: Skia and native — 2026-11-10

**Native is slower on the animated gauntlet and faster on the still wall.**
Across three matched desktop runs per backend/scenario, gauntlet tier 16 takes
136.36 ms per callback on Skia and 193.67 ms on native. Native takes 42.0% longer
per callback (29.6% lower throughput). Still wall tier 20 takes 79.13 ms on Skia
and 26.95 ms on native, giving native 2.94× the callback throughput.

These are animation-callback intervals in the real desktop application. They
include UI work, compositor backpressure, rendering and Windows scheduling.
Physical presentation FPS and isolated GPU draw time are not independently
measured. The earlier TDG results measured a different, offscreen GPU-drained
workload; they did not establish performance parity for this animated scene.

## Merge and open performance PRs

Merged the latest fetched `main`,
`adaa37363d7b08a3efb568848b4eb7cf040bb343` (#674, text-layout caching), in
`d6350e344ff2a3ab69bfda9744402155e0cc3f07`. Conflicts were resolved using main's
newer coherent Quartz/iOS implementation and results, retaining the Windows
Direct2D integration and historical Windows handover. Both workspace members
were retained where main's new resource-benchmark crate overlapped Direct2D.
The merge passes all 55 native renderer tests. Apple runtime validation was
not performed on this Windows machine.

The open gauntlet performance stack was reviewed separately, through its PR
metadata, changed-file lists and relevant code patches. All four PRs were open
drafts when inspected. Their changes are **not applied to these binaries**.

| PR | Change | Relevance on Windows |
|---|---|---|
| [#676](https://github.com/wieslawsoltes/FerroUI/pull/676) | Inline drawing-state storage and copied compositor readback values | Shared work can benefit Skia and native. This readback is UI state, not GPU pixel transfer. |
| [#677](https://github.com/wieslawsoltes/FerroUI/pull/677) | Reuse server render-data buffers, inline observer copies, retain solid-fill paint | Buffer/observer work is shared; paint reuse is Skia-specific. |
| [#678](https://github.com/wieslawsoltes/FerroUI/pull/678) | Keep composition visuals when rows detach and recycle | Shared UI/server work, relevant to the gauntlet's scrolling/recycling. |
| [#679](https://github.com/wieslawsoltes/FerroUI/pull/679) | Earlier batch acceptance and frame scheduling, plus banded frame pipelining | Scheduling changes are shared. The GPU pipeline is implemented in `SkiaMetalGpu`/Graphite, not Windows ANGLE or Direct2D. Its reported 60 fps macOS result is not a Windows result. |

The PRs form a stack: #676 → #677 → #678 → #679. Their exact observed head/base
SHAs and changed paths are retained in `open-performance-prs.json`. Applying
that stack would require another matched Windows comparison; its published
M3 Pro measurements do not quantify the benefit to either Windows backend.

## Method

Benchmark/harness source: `66de41368b49dd0e1c080e34599eb0ca9bf4080f`.
Both release builds use that source and lockfile, with separate backend
features: default Skia/ANGLE, and
`--no-default-features --features direct2d-runtime-shaders`.
The normal native dependency graph contains no Skia, HarfBuzz or ANGLE stack.

The original headless `frame_gate` uses a **raster Skia framebuffer**. It was
not used as a purported native GPU comparison. The desktop program now has
an opt-in fixed-frame mode and native Windows feature selection. Normal
non-measurement platform defaults are preserved.

- Intel i5-12400F, NVIDIA GTX 1660 Ti, driver 32.0.16.1088, Windows 11 Pro
  build 26200, Rust 1.93.0. Both GPU backends use adapter LUID
  `00000000:000102c7`; Skia/ANGLE explicitly selects the NVIDIA adapter.
- The desktop is currently **1024×768 at 60 Hz**, with render scaling 1.25.
  Windows clamps the requested 1280×820 logical window to a client area of
  **822.4×594.4 logical points (1028×743 pixels)**. Every run records that same
  client area and scaling. These dimensions differ from the macOS PR setup.
- The same Roboto Regular/Bold files are loaded into both backends. Their
  hashes are recorded; themes, scene data and per-frame updates are identical.
- Each repetition runs both scenarios with **Skia first, then native**. There
  are three repetitions, 60 warm-up scene frames and 240 measured frames per
  process. Every measured range is scene frames 61–300: 720 intervals per
  backend/scenario. There are no discarded primary runs or samples.
- Pointer-over counts are zero in all twelve runs. No input automation,
  compilation or diagnostic profiling runs during the measured intervals.
  Per-second callback-rate progress output is enabled equally for both.
- Compositor snapshots are taken after timing stops, on the last repetition,
  at scene frame 300. These snapshots were inspected; no byte-identical
  cross-backend text-rasterization claim is made.

## Results

The averages pool all 720 intervals per backend/scenario. The uncertainty
column is sample standard deviation of the **three run means**, rather than
variation between different animated scene frames.

| Scenario | Skia mean ms ± run SD | Native mean ms ± run SD | Skia callback fps | Native callback fps | Native / Skia time |
|---|---:|---:|---:|---:|---:|
| Gauntlet, tier 16 | 136.36 ± 4.16 | 193.67 ± 2.88 | 7.33 | 5.16 | 1.420 |
| Still wall, tier 20 | 79.13 ± 0.06 | 26.95 ± 0.58 | 12.64 | 37.10 | 0.341 |

| Repetition | Gauntlet Skia / native callback fps | Still wall Skia / native callback fps |
|---|---:|---:|
| 1 | 7.49 / 5.11 | 12.63 / 36.23 |
| 2 | 7.44 / 5.25 | 12.63 / 37.35 |
| 3 | 7.09 / 5.13 | 12.65 / 37.76 |

All native runs report zero renderer errors and zero pixel readbacks before
the optional capture. Their native present-call counters reach 300, including
warm-up. Those counters validate native window activity; they are not an
independent count of physically displayed refreshes for both backends.

## Separate native diagnostic

A short, instrumented native gauntlet run (3 warm-up/12 measured frames) was
performed after the primary runs. It is retained separately and **not pooled**
with the performance table. It uses an earlier scene range and profiling
overhead, so its frame time is not a substitute for the primary mean.

Its window profile reports 283.9 ms average rendering/submission work, 0.15 ms
presentation wait and about 760.5 GPU copies per window frame. Representative
slow-frame logs contain 76–94 ms in `EndDraw`, 29–37 ms in snapshots and
31–42 ms in effect-kernel scopes. The CPU timers nest and can include driver
waits; summing them would double-count work, and they are not GPU timestamps.

This points the next investigation toward native layer/snapshot/effect and
primitive submission work, rather than assuming DXGI presentation wait explains
the gap. The shared PRs may reduce surrounding costs, but no Windows speedup
from their unmerged changes has been measured here.

## Evidence and reproduction

[Raw evidence](benchmarks/windows-gauntlet-2026-11-10/) contains all twelve
primary JSON/log pairs, full callback samples, source/binary/font identities,
hardware metadata, PR snapshots, compositor captures, native validation and
separate profiling. Executable SHA-256 hashes are in `measured/run.json`.

Build once per backend and keep distinct executable copies:

```powershell
cargo build --locked --release -p gauntlet-benchmark --target-dir target/gauntlet-skia
cargo build --locked --release -p gauntlet-benchmark --no-default-features --features direct2d-runtime-shaders --target-dir target/gauntlet-native
./scripts/compare-windows-gauntlet-renderers.ps1 -SkiaExecutable target/gauntlet-skia/release/perf-compare-ferroui.exe -NativeExecutable target/gauntlet-native/release/perf-compare-ferroui.exe
```

The runner checks immutable executable hashes, frame counts, renderer errors,
viewport, scaling and matching pointer conditions. See the
[sample README](../../samples/GauntletBenchmark/README.md#windows-gpu-backend-comparison)
for individual measurement/capture environment variables.
