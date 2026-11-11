# Windows TDG renderer optimization — 2026-11-08

Historical first optimization. The [parity follow-up](windows-tdg-renderer-parity.md)
subsequently beats Skia in all twelve measured workloads.

**Native recorded redraws are now 3.0–4.4× faster than the baseline**, reducing
mean time by 67–77% across all twelve workloads. Native still takes 1.4–2.2×
the Skia time on this machine. The main regressions are fixed; these results do
not establish complete performance parity.

This follows the [initial Skia/native comparison](windows-tdg-renderer-benchmarks.md).
The baseline showed a 4.2–9.4× native deficit in recorded offscreen full redraws.
It did not show that Direct2D itself must be slower: the backend was doing
avoidable work around the Windows drawing APIs.

## Profile and changes

Optional CPU timers (`FERROUI_D3D_PROFILE=1`) measure fourteen operation groups.
They are disabled for final performance measurements. The diagnostic baseline
uses two warm-up and five measured batches of twenty frames. Profile counts
include all 140 frames; timers can nest and are not GPU timestamps.

For classic stationary redraw, native performs 404 geometry clips and 2,597
driver device-health checks per frame. Applying clips takes 1.242 ms/frame;
device checks take 0.657 ms; creating clip/draw paths takes another 0.285 ms.
Drawing all 189 text blocks itself takes only 0.117 ms in the glyph draw timer.
This identifies substantial CPU overhead outside glyph rasterization.

Classic column scrolling additionally builds and exports 895 glyph outlines per
frame on average. Native outline extraction takes 1.080 ms and path export
another 0.378 ms, before all Rust path processing. Those outlines were built
even for ordinary solid-color text rendered directly by DirectWrite.

The changes address these costs:

- Rectangular clips use `PushAxisAlignedClip` when the device transform preserves
  axis alignment. Rotated/skewed rectangles retain geometry masks. Mixed clip
  stacks pop in reverse order and are restored around snapshots, effects and
  opacity layers. Clipped compute bounds include both clip types.
- Bitmap device health is checked when opening a drawing session and outside
  active drawing, instead of before every state/draw call. Native frame access
  still validates device generation; `EndDraw` records failures so the next
  session can recreate the target.
- Glyph outlines are cached lazily per run. Normal solid text uses native glyph
  drawing and metrics. Geometry brushes, text geometry and decoration
  intersections obtain outlines on demand. Synthetic bold/oblique still build
  outlines for accurate modified ink bounds.
- The shared immediate renderer skips opacity-1 scopes, benefiting all backends.
  Nontrivial group opacity remains unchanged. Production render-data recording
  already had this optimization.
- Font faces keep a bounded cache of design-unit glyph metrics (at most 4,096
  entries per immutable face, with each variation instance owning its cache).
  Runs at different em sizes share those metrics. Drawing frames reuse a solid
  brush, and serialized shaping calls reuse the DirectWrite text analyzer.

The rectangular-clip choice follows Microsoft's
[Direct2D layer guidance](https://learn.microsoft.com/en-us/windows/win32/direct2d/direct2d-layers-overview).
DirectWrite exposes [native glyph drawing and outline access separately](https://github.com/MicrosoftDocs/win32/blob/docs/desktop-src/DirectWrite/glyphs-and-glyph-runs.md);
requesting outlines is unnecessary for the normal native glyph draw path.

## Final measured results

Source: `3080d57d9d71ac435255dd04f1ad1c3c6a4ded4a` (first optimization commit
`493b3dbee`, resource-reuse follow-up `3080d57d9`). Executable SHA-256:
`470E4C024B6BFE3DA89B3C1E558D95E7B2271E970396F76318BA37554E3D998F`.

Same GTX 1660 Ti, adapter LUID `00000000:000102c7`, Windows/driver/toolchain,
800×500 viewport, fonts, data and scroll scripts as the baseline. The final
release executable contains both renderers. Skia ran first, native second,
with three warm-up and fifteen measured twenty-frame batches per case. No
profiling, capture exports or concurrent compilation ran during these timings.
All twelve workloads completed for both backends without native timed readbacks
or renderer errors. The raw samples contain layout, submission and GPU-drained
times, dispersion and realized-cell counts.

| Scenario | Source | Baseline native ms | Final native ms | Final Skia ms | Native speedup | Native / Skia |
|---|---|---:|---:|---:|---:|---:|
| Stationary redraw | Classic | 3.666 | 0.844 ± 0.088 | 0.395 ± 0.022 | 4.35× | 2.13× |
| Vertical, 1 px | Classic | 3.802 | 1.041 ± 0.180 | 0.479 ± 0.079 | 3.65× | 2.17× |
| Vertical, 1 row | Classic | 4.201 | 1.053 ± 0.098 | 0.657 ± 0.082 | 3.99× | 1.60× |
| Horizontal, 1 px | Classic | 3.650 | 0.861 ± 0.044 | 0.437 ± 0.047 | 4.24× | 1.97× |
| Horizontal, column | Classic | 8.333 | 2.335 ± 0.048 | 1.686 ± 0.102 | 3.57× | 1.38× |
| Horizontal, far column | Classic | 8.693 | 2.417 ± 0.089 | 1.644 ± 0.030 | 3.60× | 1.47× |
| Stationary redraw | Neutral | 3.749 | 0.858 ± 0.024 | 0.393 ± 0.020 | 4.37× | 2.18× |
| Vertical, 1 px | Neutral | 4.089 | 0.943 ± 0.052 | 0.456 ± 0.030 | 4.34× | 2.07× |
| Vertical, 1 row | Neutral | 3.871 | 1.102 ± 0.051 | 0.651 ± 0.027 | 3.51× | 1.69× |
| Horizontal, 1 px | Neutral | 3.683 | 0.933 ± 0.068 | 0.482 ± 0.037 | 3.95× | 1.93× |
| Horizontal, column | Neutral | 7.403 | 2.446 ± 0.129 | 1.768 ± 0.108 | 3.03× | 1.38× |
| Horizontal, far column | Neutral | 8.615 | 2.546 ± 0.054 | 1.755 ± 0.039 | 3.38× | 1.45× |

Values are mean ± sample standard deviation in ms/frame; deviations describe
twenty-frame batch means. Baseline variability is in the initial report. These
are single ordered comparisons on one machine, not randomized repeated trials.

The follow-up instrumented run confirms the intended work reduction. Classic
stationary device checks fall from 2,597 to **one per frame**, taking 0.0004 ms;
clip application falls from 1.242 to **0.131 ms/frame**; path creation falls from
406 to **two paths/frame**. Column scrolling now extracts/exports **zero glyph
outlines**. Its metric-query timer falls from 0.201 to **0.029 ms/frame** after
the bounded design-metric cache. The original glyph count and text content are
preserved. The native TDG validation PNG remains byte-for-byte identical to
the [baseline image](benchmarks/windows-tdg-2026-11-08/recorded-direct2d-validation.png),
SHA-256 `E74F66241E77FCF794361D6D091ED721F24D9E5744C68B3A8B2BD4EB3F448087`.

The separate immediate-path comparison also completes all twelve cases on both
backends. Classic native stationary redraw falls from **125.338 to 0.818 ms**
after eliminating opacity-1 groups and the native overhead above. This is a
diagnostic of visual-to-bitmap drawing, not a 153× catalog-frame-rate claim.
The last neutral immediate far-column case has higher variability (3.451 ±
0.401 ms); the complete raw distribution is retained rather than discarded.

Raw evidence:

- Recorded [Skia JSON](benchmarks/windows-tdg-2026-11-08/optimized/recorded-skia.json) and
  [native JSON](benchmarks/windows-tdg-2026-11-08/optimized/recorded-direct2d.json),
  with [Skia](benchmarks/windows-tdg-2026-11-08/optimized/recorded-skia.log) and
  [native](benchmarks/windows-tdg-2026-11-08/optimized/recorded-direct2d.log) completion logs.
- Immediate [Skia JSON](benchmarks/windows-tdg-2026-11-08/optimized/immediate-skia.json) and
  [native JSON](benchmarks/windows-tdg-2026-11-08/optimized/immediate-direct2d.json),
  with [Skia](benchmarks/windows-tdg-2026-11-08/optimized/immediate-skia.log) and
  [native](benchmarks/windows-tdg-2026-11-08/optimized/immediate-direct2d.log) completion logs.
- Instrumented profiles [before](benchmarks/windows-tdg-2026-11-08/optimized/profile-before.json)
  and [after](benchmarks/windows-tdg-2026-11-08/optimized/profile-after.json).
  The before profile probes baseline production behavior plus the instrumentation;
  the after profile uses the final source. Both include warm-up frames. Slot
  definitions are documented by `ferroui_direct2d::profiling_statistics`.

Reproduce using the [runner instructions](../../tests/FerroUI.TreeDataGrid.Benchmarks/README.md#windows-gpu-renderer-comparison),
with the same `15`/`3`/`20` counts and `FERROUI_TDG_DRAW_PATH=recorded`.
Run Skia then native in separate processes from the prebuilt executable. The
immediate comparison uses the same settings with `FERROUI_TDG_DRAW_PATH=immediate`.
Profiling is a separate run: `FERROUI_D3D_PROFILE=1`, five measured and two warm-up
batches of twenty frames. Keep it disabled for the renderer comparison.

The remaining gap is concentrated in submission and real-font layout rather
than the batch completion wait. Final profile examples include roughly 0.13 ms
applying clips and 0.16 ms drawing glyphs in stationary redraw, and 0.45 ms
shaping in column scrolling. The timers do not cover every Rust/driver operation
and can nest; they do not fully attribute the remaining difference. Reaching
parity requires further profiling of command submission, clipping/state changes
and shaping/layout rather than reinstating the removed work or bypassing text
quality and feature support.

## Correctness and limits

The Windows suite passes 45 tests, with the lighting benchmark separately
ignored. New coverage exercises mixed rectangle/geometry clips through opacity
layers, active snapshots, scaling, reflection and rotation, and exercises
decoration intersections and synthetic-font bounds after lazy outline creation.
The shared drawing-context suite passes all 33 tests, including nontrivial group
opacity and visual text/render options. A failed drawing-session regression
also verifies that the next session recreates its lost target.

The final catalog binary passes native [renderer parity](benchmarks/windows-tdg-2026-11-08/optimized/catalog-parity.log)
and [media/shared-texture smoke checks](benchmarks/windows-tdg-2026-11-08/optimized/catalog-media.log).
RTL selection, replacement, undo, hit testing, geometry and writable inputs pass.
Video delivers 122 newly imported frames over 5,101 ms (23.91/sec), with zero CPU
pixel transfers; navigation retirement and QR decoding pass. Camera discovery
reports zero attached cameras, so this run does not certify hardware capture.
The [native catalog capture](benchmarks/windows-tdg-2026-11-08/optimized/catalog-parity.png)
was visually checked. Smoke stderr contains only the hardware-adapter startup
line on both runs.
The fractional rectangle API and geometry-mask rasterizer produce slightly
different antialias fringe coverage (the fixture observed a maximum difference
of 9/255). The regression test confines any difference above 2/255 to the
one-pixel clip fringe and limits the maximum to 16/255; interiors match.

The comparison remains a full redraw into an 800×500 offscreen GPU target at
scale 1, with actual platform fonts and shaping. It includes layout, recording,
submission and batch GPU completion, but excludes retained-scene reuse, dirty
regions, swapchain presentation and vsync. These are not catalog FPS figures.
