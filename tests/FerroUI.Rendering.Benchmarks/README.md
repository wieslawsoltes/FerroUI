# Common rendering and text benchmarks

This executable measures shared workloads through `IPlatformRenderInterface`,
`IDrawingContextImpl`, `IFontManagerImpl` and `ITextShaperImpl`. Backend adapters select
implementations; workload and measurement code contain no backend-specific drawing paths.
It is a measurement harness, not a renderer dependency of applications.

## Build and run

```sh
cargo build --locked --release -p ferroui-rendering-benchmarks --features vello

target/release/rendering-benchmarks --list
python3 scripts/rendering-perf/compare.py --backends skia,quartz \
  --trials 6 --samples 120 --warmup 20 --other-work-paused \
  --out target/rendering-perf/macos
```

Build once before timing; the runner launches the prebuilt executable directly in a fresh
process for each backend/trial. It alternates forward and reverse orders, records machine
and source/binary/font identities, and snapshots background CPU/load before and after each
process. Keep other workloads paused if possible. No renderer or OS process is stopped by
the runner. Raw samples, individual run logs, trial medians/ranges, p95/p99 and comparisons
remain in the output directory. Summary figures are medians of the per-process medians;
p95 figures are medians of the per-process p95 values, not pooled percentiles.

Default build features include Skia and Quartz. The optional `vello` feature adds all three
Vello modes; it is not required for the Skia/Quartz comparison. Disabled or unsupported
adapters fail explicitly instead of silently selecting another renderer.

| Adapter | Rendering | Fonts | Shaping |
|---|---|---|---|
| `skia` | Skia Graphite on Metal | Skia | HarfBuzz |
| `quartz` | Quartz Metal | Core Text | Core Text |
| `quartz-harfbuzz` | Quartz Metal | Core Text | HarfBuzz |
| `vello-cpu` | Vello CPU canvas, uploaded/presented with Metal | Vello | HarfBuzz |
| `vello-hybrid` | Vello hybrid, Metal | Vello | HarfBuzz |
| `vello-gpu` | Vello compute, Metal | Vello | HarfBuzz |

`quartz-harfbuzz` is a diagnostic pairing: it keeps the native font manager and glyph
rasterizer while changing the shaper, to separate shaping cost from drawing cost. It needs
HarfBuzz enabled through `skia` or `vello`. A Core Text shaper cannot consume a Skia/Vello
platform typeface, so unsupported Cartesian combinations are not advertised.

## Measurement contract

The initial platform adapter is macOS Metal: the same device, offscreen surface,
1280×800 logical canvas and 2× scaling (2560×1600 physical pixels). Other GPU platforms
need an equivalent offscreen platform surface; they must not silently measure raster instead.
The reusable Metal surface lives in `ferroui-metal` under the opt-in `offscreen` feature;
Vello retains a compatibility re-export for existing tests.

Every frame clears and redraws the scene, changes positions where applicable, disposes its
drawing context, and waits for GPU completion. The surface waits on the platform command
queue; Quartz's presentation signals/waits connect its renderer queue to that queue. Vello's
adapter additionally drains its shared device. GPU completion fences must succeed.

* `completed`: CPU recording, queueing, GPU execution and completion wait, per serial frame.
* `cpu_before_wait`: the same wall-clock frame minus the explicitly measured completion
  fence. Includes preemption/driver overhead; it is not a hardware CPU timer or an
  independently measured asynchronous submission benchmark.
* `wait`: time spent waiting for completion, not GPU timestamp duration.
* No display refresh, vsync or input-to-photon measurement is involved. Do not label reciprocal
  throughput as presented FPS.

One separate validation frame per scene is read back outside the timed intervals. The harness
requires a red marker and substantial non-background pixels **outside** that marker. Timed
frames use private GPU textures without readback. Errors, dropped frames, unavailable targets
or lost contexts fail the process. `--capture true` writes BGRA validation frames for visual
inspection; do not require identical text pixels between backends.

## Workloads

All backends receive the same rectangles, curves, bitmaps, shadows, opacity layers, fixed-font
Latin/Arabic/Hebrew strings, em sizes, direction and glyph-run placement. Rendering includes
warm cached glyph runs and GPU resources. Separate shaping workloads use preloaded font files
and UTF-16 input; 32 calls per sample reduce timer overhead. Font matching, font loading and
glyph-run construction are outside these warmed measurements. No system fallback fonts enter
the fixed-font cases. Input/font files and glyph counts are recorded for review.

Light hinting and grayscale antialiasing are requested through the common API. Native Core Text
appearance remains the text-quality reference; an adapter is not made faster by disabling its
hinting or forcing Skia-identical rasterization. A speed comparison alone is not quality parity.

This suite measures backend primitives and shaping, not complete application startup, control
layout, retained dirty-region rendering, scrolling or catalog navigation. The catalog's existing
frame benchmarks reuse `Backend` for their service setup and accept `FERROUI_TEST_RENDERER=quartz`
on a Metal surface, enabling a separate end-to-end benchmark layer with the same adapters.

## Extending

Add an adapter to `Backend::new`/`available`, with its renderer, font manager and compatible
shaper. The scene definitions stay unchanged. Report genuine unsupported capabilities rather
than replacing a scene with a cheaper one. For another platform, add a shared platform surface
with a successful completion fence and an untimed validation readback. Keep renderer-specific
synchronization in the surface adapter, not in the workload body.

For focused profiling, pass `--workload text-latin` (or another render/`shape-*`
row) directly to the binary. The default runs every unchanged workload. The
comparison runner records backend source identities and rejects changes to
the source or binary during a timed session.


To compare an optimization with a preserved executable from an earlier revision,
add `--baseline-binary PATH --baseline-source-commit COMMIT`. The runner interleaves
`before-*` and `after-*` for every selected adapter, reverses the whole order on
alternate trials, records both executable hashes and rejects executable/current-source
changes during timing. Build both binaries before running. The supplied baseline commit
is provenance supplied by the caller; retain its original build/source manifest as well.
This avoids comparing a new revision only with measurements from a different load session.
