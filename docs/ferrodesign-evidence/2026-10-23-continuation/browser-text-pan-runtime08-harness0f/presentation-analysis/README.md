# Where the WebGL2 text-pan settling interval was spent

This is a reanalysis of the preserved **runtime `08e29e4f` / harness `0f073278`** trace, not a new execution. The six traced motions consistently include this page's submitted surface in a Chromium display swap lasting **629.323–735.739 ms**. The settled callback follows that swap's end by **1.050–7.026 ms**. This locates the dominant recorded late delay at the browser display-swap boundary; the trace does not identify the underlying driver, worker or host-scheduling cause.

The run used headless Chromium 156.0.8078.4, a 1600 × 1100 canvas at DPR 1, and ANGLE Vulkan SwiftShader on a shared host. It does not establish hardware-GPU throughput or current FerroUI 0.4 performance. The complete attempt exited 1 because Software2D's strict console assertion found the missing root favicon; the separate WebGL2 trace completed. Those outcomes remain unchanged.

## Two distinct sets of observations

The **36 normal motions** have dispatch-to-observed-native-frame p95 **26.774 ms**, observed native-presentation p95 **118.863 ms**, and settled p95 **905.030 ms**. Their medians are 15.237, 77.606 and 775.314 ms respectively. These are three different milestones. The six later diagnostic motions below never enter those statistics.

The application increments its frame counter near the start of `CanvasHost` rendering, before recording the frame's drawing commands (`samples/FerroDesign/crates/ferrodesign-app/src/canvas.rs`, around line 1941). The WebGL platform session increments `presentedFrames` in `GlSession::dispose`, after the backend's submission returns (`src/Browser/FerroUI.Browser/rendering/browser_web_gl_render_target.rs`, lines 154–169). Neither counter is a fence for Chromium's final display swap. The benchmark's settling milestone waits for two subsequent animation callbacks, which makes the later scheduling delay observable.

All durations below are milliseconds. “Input” is the actual trace capture marker, not the Node dispatch clock. “Main overlap” is the union of recorded renderer-main-thread complete slices between native presentation and settling; nested slices are counted once.

| Motion | FerroUI callback wall / thread CPU | Input → native presentation | Matched swap wall / thread CPU | Main overlap | Input → settled | Settled after swap end |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 54.144 / 54.128 | 70.425 | 702.129 / 1.771 | 4.815 | 777.493 | 1.220 |
| 1 | 49.806 / 49.326 | 66.622 | 706.275 / 1.596 | 3.793 | 779.185 | 1.050 |
| 2 | 42.545 / 42.146 | 59.540 | 735.739 / 2.223 | 4.149 | 800.286 | 1.680 |
| 3 | 47.308 / 47.071 | 64.239 | 732.411 / 1.958 | 4.819 | 800.591 | 1.680 |
| 4 | 44.260 / 44.274 | 60.974 | 629.323 / 1.598 | 4.455 | 699.784 | 7.026 |
| 5 | 47.857 / 47.849 | 64.643 | 697.287 / 6.206 | 8.361 | 767.321 | 2.561 |

The callbacks are `FunctionCall` records in `/current/ferroui.js`, line 1, column 24984, on renderer main thread `144761:144761`. Inspection of the executed asset locates the `TimerHelper.JsExportOnAnimationFrame` animation loop there. Its full 35,910-byte asset has SHA-256 `40f2c812b4ee1c8fa3ebbc1e0ff453256cc08f8a73fa5bbc01b96ab9bd2e2a20`. This separate 42.545–54.144 ms callback is real main-thread work. The trace lacks Rust/WASM call stacks to divide it into scene replay, text work, raster preparation or backend submission.

## Exact association and limits

[analysis.json](analysis.json) preserves all six submitted surface IDs, aggregation display IDs, trace-marker timestamps, swap start/end timestamps, post-submit/finish events, thread durations and throttle instants. Frame IDs are strings because their integers exceed JavaScript's exact-number range.

[analyze.py](analyze.py) finds this renderer's `STEP_SUBMIT_COMPOSITOR_FRAME`, joins its surface ID to `STEP_SURFACE_AGGREGATION`, then joins that display ID to `STEP_BUFFER_SWAP_POST_SUBMIT` and `STEP_FINISH_BUFFER_SWAP`. The matched post-submit slice is enclosed by `SkiaOutputSurfaceImplOnGpu::SwapBuffers` on `CrGpuMain` (`144304:144304`). Thus the long swaps include this page's actual frames; the association is stronger than temporal proximity alone. Each aggregation also includes another static surface, so it does not prove this page exclusively caused the wait.

Each aggregation-to-swap-end interval contains two `Swap throttled` instants on `VizCompositorThread` (`144304:144325`), both reporting one pending swap and a maximum of one. In motion 3, the first instant is **210 µs before** the long swap slice starts, after that motion's display aggregation. The JSON preserves this negative offset rather than placing every throttle inside the slice.

The long swap's recorded `tdur` is only **1.596–6.206 ms** on `CrGpuMain`; that is not total SwiftShader worker CPU or hardware GPU time. Likewise, main-thread slice coverage is an observed wall-clock union, not CPU utilization. The small late main-thread overlap, explicit swap throttling and close swap-end/settled timing support **display-swap backpressure as the dominant recorded late-stage wait**. They cannot distinguish unrecorded worker execution, device/driver waiting or shared-host scheduling inside it.

`SkiaOutputSurfaceImplOnGpu` is Chromium's own compositor event name. It does **not** indicate an application Skia dependency: this application uses FerroUI Vello, and the selected native browser path is the Vello WebGL renderer.

## What changed in the later framework, and what did not

The JSON verifies exact blob and SHA-256 equality between runtime `08e29e4f530c0ae65f6e2ac072f381605e23ed0c` and source checkpoint **`c549f6bcafe614a6373c743f5efbf2a3cb0eccd2`** for nine relevant files: the browser animation loop/timer, WebGL platform session and statistics, Vello WebGL renderer/sink and platform interface, the application canvas, and the sample manifest. Complete lockfile entries are also identical for `vello_gpu` 0.3.0, `vello_common` 0.3.0, `vello_gpu_shaders` 0.3.0, `glifo` 0.4.0, `peniko` 0.6.1, `web-sys` 0.3.106 and `js-sys` 0.3.106.

The merged drawing-fragment optimization's `keeps_fragments()` selects `VelloRenderingMode::Gpu`; this browser's WebGL sink reports `Hybrid`. The new hybrid depth-buffer reuse belongs to the separate wgpu hybrid sink, and Metal queue-ordering changes belong to the Metal path. They do not replace the unchanged WebGL path examined here. Browser rasterizer dependencies already use optimization level 3 in the preserved sample manifest.

There is still potential application/backend work to profile: this WebGL path creates a fresh scene per drawing context, reports `previous_frame_is_retained: false`, and replays a full surface while preserving renderer/resources caches. Zero application prepare/rebuild counters therefore do not mean zero backend work. The measured callback should be profiled before assigning its cost to text or choosing a cache change. These are source observations and a profiling direction, not a demonstrated optimization or a fix for the long browser swap.

Other framework composition code did change during the 0.4 integration. The narrow source equality above does not make the whole current runtime identical. A separately labeled current-runtime browser measurement must determine whether this host/display delay persists.

## Reproduction and integrity

With Python 3.11+ and the named git objects available, run from any directory:

```sh
python /path/to/FerroUI/docs/ferrodesign-evidence/2026-10-23-continuation/browser-text-pan-runtime08-harness0f/presentation-analysis/analyze.py > /tmp/presentation-analysis.json
```

The script verifies the archived and decompressed hashes against [the parent provenance](../provenance.json), derives the six matches and interval unions, checks source/dependency equality, and emits the preserved JSON without invoking a browser, compiler or network API. The input trace is 2,739,494 bytes with SHA-256 `5fb1aefbc1c5a239892f608e6cc19a7caf6386f0800fb18917b0ecfdc96074b4`. Its recorded analysis reports no truncation or data loss. Actual trace marker timestamps are used for these intervals; the original page/trace alignment and its 0.2 ms uncertainty remain in the JSON. Incomplete begin-only slices are excluded from complete-slice unions. Source hashes, dependency checksums, input hashes and the derivation-script hash are all retained in [analysis.json](analysis.json).
