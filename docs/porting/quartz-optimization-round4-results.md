# Opacity command recording results

These historical measurements predate the upstream rebase and privacy history rewrite. Superseded branch commit IDs, local device identifiers and background process names are omitted from the published evidence. They do not measure the rebased code.

Measured 2026-11-09 on Apple M4, 24 GB RAM, macOS 27.2. Implementation `pre-rewrite revision` is compared with the preserved [third-pass executable](quartz-optimization-round3-results.md), source `pre-rewrite revision`. The [JSON report](quartz-optimization-round4-results.json) records binary/source/font identities, trial medians/ranges, percentiles and background load. Raw distributions and BGRA frames remain in `target/rendering-perf/macos-opt4-final`.

**Opacity CPU-before-wait time falls 4.3%** (0.1156 → 0.1106 ms). Completed frame time changes from 0.6916 to 0.6553 ms (5.2% lower), but trial ranges overlap substantially and the unchanged Skia control varies much more. This does not establish a reliable frame-time improvement under steadier load. The native executable has **no size increase**, and all eight native validation frames remain byte-identical.

The Skia gap remains open: opacity is 0.6553 ms native versus 0.4255 ms Skia in the new executable (54.0% higher). This pass reduces redundant command recording; it does not establish parity or a reliable closure of the completed-frame gap.

## Implementation

Compact solid/image commands previously also appended the entire 208-byte generic QParams structure simply to derive a draw count. An explicit count now follows recording, ordered drawable batches, prepared glyph replay, merge limits and device recovery. Generic coverage/path paints retain their full inputs and edge offsets. Axis-aligned solid rectangles now derive their exact bounds from two transformed corners and omit unused generic edge-data allocation/copies, including fractional rectangles. The compact shader retains its original four-sample coverage. Atlas clears use 32-byte compact rectangle vertices, replacing 208-byte generic vertices; their constant transparent fragment shader and blending remain unchanged.

For the opacity workload’s 96 child rectangles, 24 composites and validation marker, this avoids 25,168 bytes of redundant generic recording plus 4,224 bytes of clear vertex input per frame. This is a reduction in transient recorded bytes, not a measured process-memory saving. Core Text raster inputs, font matching, coverage, sampling, and every Metal shader are unchanged from the prior native reference.

Whole-page initialization and a separately collected atlas-clear/solid stream were also measured. They did not reliably improve completed frame time, so neither is present in the final implementation. The existing atlas ownership, ordered dependency drains, pool leases, texture budgets and completion behavior are retained. The [design](quartz-metal-optimization.md#compact-command-recording-and-layer-clears) describes the final implementation.

## Completed frame time

Milliseconds per completed frame; shaping rows are milliseconds per call. The unchanged Skia control is included for both executables to expose machine-load variation. All trial results are retained, including apparent regressions.

| Workload | Previous native | New native | Change | Previous-binary Skia | New-binary Skia |
|---|---:|---:|---:|---:|---:|
| rounded-rects | 0.6600 | 0.6783 | +2.8% | 0.6721 | 0.7154 |
| geometry | 0.8220 | 0.8344 | +1.5% | 0.7144 | 0.6498 |
| bitmaps | 0.5998 | 0.5139 | -14.3% | 0.5569 | 0.5294 |
| text-latin | 0.9942 | 0.9240 | -7.1% | 0.8405 | 0.6207 |
| text-arabic | 0.8481 | 0.8005 | -5.6% | 0.7705 | 0.6722 |
| text-hebrew | 0.8173 | 0.7541 | -7.7% | 0.7568 | 0.6648 |
| shadows | 0.9153 | 0.9084 | -0.8% | 9.9817 | 6.9330 |
| opacity-layers | 0.6916 | 0.6553 | -5.2% | 0.6665 | 0.4255 |
| shape-latin | 0.0103 | 0.0104 | +1.4% | 0.0106 | 0.0084 |
| shape-arabic | 0.0118 | 0.0118 | +0.4% | 0.0046 | 0.0044 |
| shape-hebrew | 0.0070 | 0.0069 | -1.3% | 0.0018 | 0.0018 |

Geometry and rounded-rectangle medians increase slightly in this session, while native bitmap/text medians decrease. Shaping and Skia code are unchanged, yet some of their medians move substantially. These results do not justify a broad rendering speedup claim. The practical result is smaller recording inputs with passing pixel/contract checks and unchanged shipping size; a stable frame-time win remains to be demonstrated.

## Method and limits

Two prebuilt release executables, Skia and Quartz in each; six fresh processes per adapter/revision, alternating forward/reverse order. Each case uses 30 warmup frames and 150 samples at 2560×1600 physical pixels (2×), fixed font bytes, identical workload/glyph counts, Light hinting and grayscale AA. Every timed frame waits for successful GPU completion. The baseline executable hash matches the prior committed report. Both binaries and all recorded sources were unchanged during timing, and the checkout was clean.

The user authorized background load to be recorded. This chat ran no builds, tests, simulators or image processing during formal timing. External application and OS activity remained. Opacity native per-trial medians span 0.5221–1.1561 ms before and 0.5515–1.1318 ms after. The source-identical Skia control changes from 0.6665 to 0.4255 ms between executables (36.2% lower). This large control difference limits attribution to the code change; workload code is identical, but background load and whole-binary effects are not isolated. CPU-before-wait is wall time minus the explicit fence wait, including preemption/driver work, not a hardware CPU timer. The reported CPU reduction therefore also needs confirmation under steadier load.

All 24 runs validated: 28,800 timed rendering frames and 345,600 shaping calls, with zero native renderer readbacks and zero GPU errors. Validation readbacks occur outside timing. This is warmed offscreen throughput, not displayed FPS, input latency, startup, scrolling, process memory, Intel/eGPU results or physical iOS performance.

## Quality and platform validation

All 73 Quartz tests pass in normal Metal mode and all 73 pass with separate destination textures forced. The two new regressions verify compact clear regions against independent textures after page poisoning/reuse and dependencies, and verify all 8,200 compact commands across recording/merge boundaries. Existing native font, packed-mask, effects, media, clipping, snapshot, allocation/recovery and format tests pass.

All eight benchmark validation frames match the previous native executable byte-for-byte, including opacity and Latin/Arabic/Hebrew text. Native text remains the quality reference. No CPU framebuffer path, shaping-result cache, additional rendering/text dependency, quality downgrade or shader change is introduced.

The macOS native catalog ran Home, TextBlock, TextBox, Image, Visuals, Composition brushes, Shadows and lights, and Shader effects: 16 framework/composed captures at 2200×1600, zero GPU errors in all eight processes. The Visuals composed frame, TextBox and Image captures were visually inspected. Capture readbacks are outside benchmark timing. Artifacts: `target/quartz-opt4-catalog-final`; logs: `target/quartz-opt4-profile/catalog-{build,capture}-final.log`.

The shared renderer also passes all 41 smoke checks on the iPhone Air / iOS 27.0 simulator with `quartz-runtime-shaders`, with zero renderer readbacks and GPU errors. TextBlock, TextBox, Image and Visuals were selected, captured and visually inspected; the catalog reports zero renderer readbacks and GPU errors. The initial final-page screenshot raced the app exit and captured SpringBoard. A repeat with `SIMCTL_CHILD_FERROUI_SMOKE_EXIT_MS=45000` retained the final page until capture and corrected it without a code change. Logs are `target/quartz-opt4-profile/ios-smoke-final.log` and `ios-catalog-corrected.log`; all five screenshots are in `target/quartz-opt4-ios-final`. Test apps were uninstalled and the owned simulator shut down. These checks cover selected pages on a simulator, not exhaustive parity or physical iOS performance.

## Shipping binary size

HelloWorld, same UI/theme, lock/compiler, `dist` profile (fat LTO, one codegen unit, stripped symbols, panic abort):

| Backend / revision | Executable bytes |
|---|---:|
| Skia | 17,839,632 |
| Native, previous | 13,310,848 |
| Native, current | 13,310,848 |

Native size changes by **0 bytes** and remains **4,528,784 bytes (25.39%) smaller than Skia**. The rebuilt Skia executable is byte-identical to the preceding checkpoint. The minimal native dependency graph excludes Skia, HarfBuzz, Vello, wgpu, ANGLE and optional Naga; catalog validation includes runtime shaders. This measures executable bytes, not app bundles or resident memory. Commands, linked libraries, source/binary identities and previous artifact identity are retained in JSON. Artifacts: `target/quartz-opt4-size-final`.

## Reproduce

For a new comparison, set `BASELINE_COMMIT` to the public commit used to build your preserved baseline executable. Historical source revisions are omitted from this branch.

```sh
cargo build --locked --release -p ferroui-rendering-benchmarks --features vello
python3 scripts/rendering-perf/compare.py \
  --backends skia,quartz \
  --baseline-binary target/quartz-opt4-profile/rendering-benchmarks-before \
  --baseline-source-commit "$BASELINE_COMMIT" \
  --trials 6 --samples 150 --warmup 30 --scale 2 \
  --out target/rendering-perf/macos-opt4-final
python3 scripts/measure-quartz-prototype.py --output target/quartz-opt4-size-final
```

Preserve the earlier executable and its original build manifest before rebuilding. Run size/platform validation after formal timing. The next useful step is a steadier-load completion/queue profile with Metal GPU counters, then an isolated change that lowers GPU/queue time; the rejected atlas stream should not be revived without that evidence.
