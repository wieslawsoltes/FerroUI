# Windows TDG renderer performance parity — 2026-11-08

The [2026-11-09 main-integration follow-up](windows-native-renderer-main-followup.md)
contains newer resource/state optimizations and fresh measurements after merging main.

**Native beats Skia in all twelve measured workloads.** Across five paired runs,
native GPU-drained frame times are 7.9–19.9% lower. Every individual pair also
has a native/Skia mean ratio below 1 for all twelve cases; the closest is 0.9993.
The geometric mean speedup is 1.17×. This completes the performance goal for
these recorded-path TDG workloads on this machine.

This follows the [initial baseline](windows-tdg-renderer-benchmarks.md) and
[first optimization](windows-tdg-renderer-optimization.md). The initial native
implementation took 4.2–9.4× the Skia time. The first optimization reduced that
to 1.4–2.2×. The remaining cost came mainly from the bridge and text submission,
and did not require lowering text quality or changing the benchmark workload.

## Final measurements

Production/benchmark source: `1cdef8437a98cefa7394bea41b824e27cb5889c1`.
Release executable SHA-256:
`C0C7C53C77E7331B7AF2BA57866783C1334E8CDE4444850EC403D535DA024F4B`.

Same Intel i5-12400F, NVIDIA GTX 1660 Ti, Windows 11 Pro build 26200,
driver 32.0.16.1088 and Rust 1.93.0 (`x86_64-pc-windows-msvc`) as the earlier
reports. Both backends use hardware rendering on adapter LUID
`00000000:000102c7`. One release executable contains both backends.

Each of five repetitions runs **Skia first, native second**, with three warm-up
and fifteen measured batches of twenty frames per workload. The table combines
all 75 measured batch samples per backend and workload, with sample standard
deviation. No samples or repetitions are discarded. Times include scrolling,
layout, complete render-data recording, replay, submission and GPU completion.
No compilation, other benchmark, profiling or capture export runs concurrently.
The JSON separates recording/replay phases: native replay enqueues commands,
whose execution during context disposal is included in submission and total
time. GPU-drained total time is the cross-backend comparison metric.

| Scenario | Source | Skia ms/frame ± SD | Native ms/frame ± SD | Native / Skia | Time reduction |
|---|---|---:|---:|---:|---:|
| Stationary redraw | Classic | 0.396 ± 0.026 | 0.331 ± 0.015 | 0.837 | 16.3% |
| Vertical, 1 px | Classic | 0.445 ± 0.022 | 0.410 ± 0.051 | 0.921 | 7.9% |
| Vertical, 1 row | Classic | 0.605 ± 0.064 | 0.535 ± 0.062 | 0.885 | 11.5% |
| Horizontal, 1 px | Classic | 0.437 ± 0.028 | 0.390 ± 0.052 | 0.891 | 10.9% |
| Horizontal, column | Classic | 1.673 ± 0.065 | 1.468 ± 0.045 | 0.878 | 12.2% |
| Horizontal, far column | Classic | 1.739 ± 0.123 | 1.514 ± 0.148 | 0.871 | 12.9% |
| Stationary redraw | Neutral | 0.404 ± 0.027 | 0.324 ± 0.017 | 0.801 | 19.9% |
| Vertical, 1 px | Neutral | 0.466 ± 0.044 | 0.378 ± 0.024 | 0.812 | 18.8% |
| Vertical, 1 row | Neutral | 0.629 ± 0.073 | 0.517 ± 0.027 | 0.823 | 17.7% |
| Horizontal, 1 px | Neutral | 0.447 ± 0.033 | 0.367 ± 0.024 | 0.822 | 17.8% |
| Horizontal, column | Neutral | 1.765 ± 0.143 | 1.522 ± 0.067 | 0.862 | 13.8% |
| Horizontal, far column | Neutral | 1.797 ± 0.075 | 1.558 ± 0.057 | 0.867 | 13.3% |

The viewport remains 800×500 physical pixels at scale 1, with Segoe UI 12,
24-pixel rows, real text shaping and the same classic/neutral virtualization
fixtures and scroll scripts. GPU work is drained once per batch, with each
Skia frame flushed. There is no presentation/vsync, retained-frame reuse or
dirty-region shortcut. These measure offscreen full redraws, not displayed
catalog FPS or every possible scene/hardware configuration. Timed native runs
assert that backend-error and readback counters do not increase.

[Raw evidence](benchmarks/windows-tdg-2026-11-08/parity/) includes all ten timing
JSON/log pairs, source/binary identity, pooled statistics and individual-pair
ratios in `summary.json`, plus separate profiling, builds, tests and catalog
captures. Fixed Skia-first ordering follows the requested procedure; these
results do not estimate ordering effects independently.

## What closed the gap

- Common rectangular clips, restores and solid glyph draws cross the Rust/C++
  boundary as bounded command batches. Commands retain their glyph resources.
  Snapshots, effects, image interop, other draws and drawing-session completion
  flush pending work in order; automatic capacity flushes preserve clip scopes.
- Immutable positioned runs retain native glyph arrays and the font face.
  Natural advances are used only when float/double accumulation reproduces
  every original position exactly. Other layouts keep explicit positioning.
- Rectangular text clips stay deferred while Direct2D's actual raster ink
  bounds prove that the entire draw fits inside their integer-pixel interior.
  Partial clips materialize normally. A bounded, device-generation-aware cache
  reuses bounds across integer translations. Fractional clip fringes, rotated
  masks, mixed stacks and snapshots retain their semantics.
- Compatible monochrome text under integer translation is combined into one
  native glyph draw. Ink must not overlap between runs, source-over blending
  and exact positions are required, and font, em size, color, antialiasing and
  applied clips must agree. Clip changes, incompatible text and capacity limits
  flush the group. Color glyphs and other transforms keep individual draws.
- Short shaping results use a bounded 1 MiB/2,048-entry per-face LRU. Keys include
  the full UTF-16 owner, slice, locale, size, direction and every feature range.
  Reusable short-input buffers and one metrics call per glyph run further reduce
  allocations and bridge calls.
- On cache misses, a narrow simple-text path asks DirectWrite to certify the
  **entire owner**. It requires a whole-input simple result, printable ASCII,
  English locale, no feature overrides/RTL, a static unsimulated face and a
  power-of-two design em. Only then are nominal glyphs and integral design
  advances used. All other inputs use full script analysis, shaping and placement.

Microsoft documents [GetTextComplexity](https://learn.microsoft.com/en-us/windows/win32/api/dwrite_1/nf-dwrite_1-idwritetextanalyzer1-gettextcomplexity)
as the test for whether full shaping is needed; its classification covers
standard OpenType features as well as complex scripts. The implementation
requires `textLengthRead == owner length`, rather than accepting a simple prefix.
Native [glyph-run world bounds](https://learn.microsoft.com/en-us/windows/win32/api/d2d1_1/nf-d2d1_1-id2d1devicecontext-getglyphrunworldbounds)
provide transformed raster bounds for safe clip containment.

Separate diagnostic profiles use five measured/two warm-up batches of twenty
frames. Classic stationary redraw now makes **one core native glyph draw for
189 text runs**, with one applied rectangular clip. Core glyph-call CPU time
is 1.59 µs/frame; the enclosing per-run processing timer is 31.69 µs/frame.
Classic row scrolling shapes 8.94 fresh runs/frame: its shaping-cache-miss timer
is 5.95 µs/frame, and total shaping is 9.85 µs/frame. These optional, nested CPU
timers explain the change; the uninstrumented GPU-drained table establishes
performance. Profiling now exposes twenty slots, including cache and core-draw
counts. GPU cost is still included in the primary timings.

The multithreaded Direct2D factory, serialized native bridge and D3D11 thread
protection remain enabled. The native catalog dependency graph still excludes
Skia, HarfBuzz and ANGLE; the comparison executable intentionally includes both
renderers. These changes stay within the native backend; the earlier shared
opacity-1 optimization remains available to all backends.

## Correctness and media validation

- **54 native tests pass**, with one opt-in lighting benchmark ignored. New
  regressions compare certified shaping against an independent uncached full
  DirectWrite reference, and combined draws against individual native draws.
  Coverage includes fractional font sizes/DPI, clips, aliasing, overlapping ink,
  transparency, owner slices, features, RTL, color text, concurrent targets,
  temporary run lifetimes and automatic batch-capacity flushing. Existing
  geometry, effect, lighting, acrylic, bitmap and text tests also pass.
- The native TDG validation PNG remains byte-identical to the earlier native
  capture: SHA-256
  `E74F66241E77FCF794361D6D091ED721F24D9E5744C68B3A8B2BD4EB3F448087`.
- The native catalog builds and passes its Renderer parity smoke at desktop
  scale 1.75: native text quality, RTL selection, hit testing, replacement and
  undo. Renderer parity, Reveal and Acrylic brush captures were inspected.
- Sustained native video delivers **123 frames in 5,103 ms (24.10 fps)** for the
  bundled 24 fps clip. Shared textures, GPU imports/conversions, navigation
  retirement and QR decoding pass, with zero CPU pixel transfers.
- The opt-in synthetic Media Foundation GPU camera passes capture at 640×480,
  30 fps, shared-texture import and navigation retirement, with zero CPU pixel
  transfers. Physical discovery finds no camera on this machine, so physical
  capture remains untested. The normal catalog was rebuilt without camera fixtures.

The shared drawing-context suite passed its 33 tests in the preceding phase;
this follow-up changes no shared production code. No text-quality substitution,
CPU preview transfer or effect removal is used to obtain these measurements.

## Reproduce

Build once, then use the same executable for every timed pair. Substitute the
benchmark executable path printed by Cargo if its hash suffix differs:

```powershell
cargo test --locked --release -p ferroui-treedatagrid-benchmarks --no-default-features --features renderer-benchmarks --lib --no-run --target-dir target/tdg-renderer-release
./scripts/compare-windows-tdg-renderers.ps1 -Executable target/tdg-renderer-release/release/deps/ferroui_treedatagrid_benchmarks-b0697de91f0ec0ec.exe
```

The runner records five complete pairs, clears profiling/capture flags, checks
that the executable hash stays fixed, and preserves raw JSON/logs. Avoid other
compilation or GPU workloads while it runs. Functional checks:

```powershell
cargo test --locked -p ferroui-direct2d --features runtime-shaders --lib -- --test-threads=1
cargo build --locked -p control-catalog-desktop --no-default-features --features direct2d-runtime-shaders
```

Catalog smoke commands and the explicit synthetic-camera feature are documented
in [the native renderer guide](windows-native-renderer.md).
