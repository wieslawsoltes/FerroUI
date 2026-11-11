# Native Windows renderer

`ferroui-direct2d` is an opt-in Windows backend. It uses a hardware D3D11
device (feature level 11.0 or later), DXGI flip-model presentation, Direct2D drawing, DirectWrite font
discovery/shaping, and WIC image codecs. The default desktop backend remains
Skia. The native feature graph excludes Skia, HarfBuzz, ANGLE, Vello and wgpu.

## Build and run

Use a Windows MSVC Rust toolchain and the Visual Studio C++ and Windows SDK
components. The SDK's `fxc.exe` compiles the built-in effect kernels at build
time; `FXC` can specify its location. No shader compiler is deployed for those
kernels.

The required DirectWrite factory and in-memory font loader set the native API
baseline at Windows 10 Creators Update or later. The current hardware runs
were on Windows 11 with SDK 10.0.26100.0; earlier supported Windows versions
have not been exercised. See Microsoft's
[in-memory font API documentation](https://github.com/MicrosoftDocs/win32/blob/docs/desktop-src/DirectWrite/custom-font-sets-win10.md).

```powershell
cargo run -p hello-world --no-default-features --features direct2d
cargo run -p control-catalog-desktop --no-default-features --features direct2d
```

An application can select `.use_direct2d()` explicitly, or enable
`ferroui-desktop/direct2d` and use platform detection. Select one renderer:
enabling both the Windows native and Skia desktop features is rejected at build
time. Win32 window management, input and accessibility continue to use the
existing platform implementation.

Runtime WGSL effects are optional and add Naga and a runtime dependency on the
Windows `d3dcompiler_47.dll`:

```powershell
cargo run -p control-catalog-desktop --no-default-features --features direct2d-runtime-shaders
```

The size-oriented native build omits that feature. Built-in gradients, shadows,
blur, acrylic, lighting, convolution and composition pixel operations do not
require it. Custom shader effects without the feature report their unsupported
status rather than silently invoking a CPU shader implementation.

## Ownership and rendering

The renderer owns one hardware D3D11 device with BGRA and video support and
multithread protection. There is no WARP fallback. Direct2D contexts target
textures on that device; the window target copies into a premultiplied-alpha
composition swap chain attached to its HWND through DirectComposition and
presents through DXGI. Offscreen layers and snapshots remain GPU resources.
Snapshots copy their content so later drawing and texture reuse cannot alter
them. Temporary Direct2D frames are pooled within a bounded 64 MiB/16-frame
budget. Device replacement clears the pool and changes the device generation;
stale contexts and imports are rejected, while decoded/uploaded application
images can reconstruct their GPU resources.

Every native object and pending media read retains the graphics factories and
their MTA usage. Windows releases Rust thread-local caches after C++ static
destructors; dropping the singleton owner therefore stops pool reuse but keeps
the factories alive until the last native resource is released. The SDK harness
explicitly tests that order, including final factory destruction.
During process detach, remaining Rust TLS resources skip COM calls because
system graphics DLL teardown may already have run. The OS reclaims those final
process resources; ordinary window closure and media retirement release them
normally while the process is running.

The compositor already applies render scaling. Its drawing context therefore
does not apply bitmap DPI a second time. Immediate render-target drawing still
uses bitmap DPI. Tests compare both paths at 100%, 125%, 150% and 200%.

Lighting uses the shared `EffectRasterRegion` calculation to round outwards,
clip the scratch texture to the viewport, and map cropped pixel coordinates
back into logical effect space. Direct2D and Quartz use the cropped texture;
Vello uses the same calculation for its already bounded lighting pass. Skia
already evaluates lighting over the drawn rectangle. Shared composition logic
also folds ambient/distant lighting into a constant color for every backend.
Native procedural lighting dispatches directly into a cropped GPU image and
draws that image into the parent. It does not snapshot the parent or allocate
an intermediate render target. Lighting that multiplies existing pixels keeps
its input-dependent filter path.

Pointwise composition effect graphs now render in a brush-sized target in the
shared painter. Their blend, noise, mask and color-matrix intermediates inherit
that size on every backend with layer-effect support. Backdrops, blur, shadow
and convolution keep the original canvas. Stroke/glyph coverage for these
graphs uses a bounded target too, and publishes its bounds before allocating
its paint and mask layers. Direct2D passes completed internal layer images to
their parents without copying them; their targets cannot be reused until the
root EndDraw has submitted every read. Public snapshots remain independent.
Color-matrix and luminance layers attach their native effect to the completed
child image in the parent graph. They submit the child once, without taking a
snapshot and redrawing the filtered pixels into the child. This preserves the
parent's output clip and group opacity; the SDK regression compares straight
and premultiplied alpha with the previous intermediate-image path.
Finished image/noise surfaces cache their immutable snapshot until the next
write or drawing operation. These changes address the remaining window-sized
work and repeated flush/copy operations in Reveal beyond its lighting pass.
The shared coverage painter marks its bounded context so nested effect brushes
can reuse that target. Translucent wrappers retain their isolated opacity path.
Temporary surfaces must retain their image storage after disposal while queued
draws reference it; Skia uses an owned GPU surface rather than a borrowed
session framebuffer for these targets.

The shared graph painter also reuses a fresh transparent parent layer for a
mask, blend or composite's initial isolation. It tracks the first source
separately: later source-over siblings still isolate their output so their
masks cannot erase earlier content. This removes redundant submissions in
Reveal while keeping the optimization available to every backend.

`FERROUI_D3D_PROFILE=1` reports native submission/presentation timing and a
breakdown for submissions above 16 ms. The breakdown timings overlap (for
example, snapshots contain EndDraw), so they must not be added together.
Use `FERROUI_FRAME_TIMES=1` for the compositor's frame summary. Debug catalog
measurements are useful for finding work, but are not release FPS claims;
`--profile fast-release` is available for an optimized interactive trial.

An earlier optimized interactive Reveal trial on the GTX 1660 Ti at 175% DPI used
identical pointer sweeps in a 1100 by 800 logical-pixel window, without
screenshot capture during the measurement. The 32-frame samples changed from
20.591 ms median / 22.730 ms p95 before these optimizations to 15.253 ms median /
23.442 ms p95 after them. Typical frame time improved by about 26%, but the
remaining spikes exceed the 16.7 ms budget; this is not a stable-60-fps claim.

The final shared isolation change was measured separately against its immediate
predecessor, with compilation idle, one active catalog window, the same window
size and pointer sweeps, and no screenshots during measurement. The two-sweep
samples were 31 frames before and 29 after: median 15.403 to 14.243 ms, p95
18.351 to 16.407 ms. That is another 7.5% reduction in typical frame time.
The final maximum was 19.433 ms, so occasional missed 60 Hz frame budgets
remain. These small diagnostic samples do not establish sustained frame rate.

DirectWrite script analysis sees the surrounding UTF-16 paragraph, including
formatter slice context, while excluding unrelated lines of large text buffers.
The resulting clusters are relative to the requested slice.
Embedded font bytes have a COM owner retained by the font file. Variable axes,
font fallback, OpenType features, glyph advances and positioned glyph drawing
use DirectWrite. Long script runs use bounded calls and commit only prefixes
marked `canBreakShapingAfter` by DirectWrite, preserving ligatures, combining
clusters and RTL order. Native input limits can be smaller than the 16-bit
cluster-map range, so a rejected large call is retried with less text.
An indivisible shaping span that exceeds the native limit without any safe
split still reports an error. Tests cover long Latin/combining paragraphs,
Arabic paragraphs and surrogate pairs.

OpenType feature ranges use owner-text UTF-16 coordinates, with exclusive ends,
matching the HarfBuzz adapter. Overriding one feature preserves the standard
ligature, contextual-ligature, contextual-alternate and kerning defaults from
[WinUI typography](https://github.com/microsoft/microsoft-ui-xaml/blob/74c2cab127943565ca2a278cc8f64f472209dd14/dxaml/xcp/core/text/common/Typography.cpp).
Context outside a formatter item participates in joining analysis but cannot
consume that item's characters into an external ligature. Direct immediate
custom drawing uses the fallback color of a XAML composition brush for fills
and opacity masks, including its opacity.

## Video, camera and shared textures

The Windows media providers accept either their existing OpenGL producer or a
native D3D11 producer. The native path gives Media Foundation the renderer's
device. Decoded frames retain their native texture and array slice; the renderer
imports them directly and performs GPU copies/conversion. Imported composition
handles retain the original `FrameLease`, including its budget accounting.

A producer lease is retired only after the GPU event query completes. A failure
after submission quarantines the read until completion or confirmed device
removal. NT and legacy DXGI shared handles, keyed mutexes, and D3D11 fence waits
and signals are supported when the corresponding device interfaces are
available. Preview does not map frame pixels or upload a CPU copy. Explicit
image export/readback APIs remain available separately.

The catalog's normal `direct2d` feature uses physical camera discovery. The
optional test feature below enables an explicitly selected GPU-generated
synthetic camera, not a substitute claim of physical camera validation:

```powershell
$env:FERROUI_SMOKE_MEDIA='1'
$env:FERROUI_SMOKE_MEDIA_CAMERA='1'
$env:FERROUI_SMOKE_MEDIA_SYNTHETIC_CAMERA='1'
cargo run -p control-catalog-desktop --no-default-features --features direct2d-media-test-fixtures
```

## Validation and measurements

```powershell
cargo test -p ferroui-direct2d --lib --features runtime-shaders
cargo test -p ferroui-direct2d --lib reveal_lighting_dispatch_work -- --ignored --nocapture --test-threads=1
cargo check -p ferroui-vello --features gpu
./scripts/measure-windows-native.ps1
# Also measure the optional native runtime shader compiler:
./scripts/measure-windows-native.ps1 -IncludeRuntimeShaders
```

The renderer tests compare pixels with shared CPU references, including
transformed effect bounds, gradients/tiling, convolution, composition blend
operations, lighting, masks, acrylic, sampling, glyph rendering and image-codec
round trips. They also check Arabic formatter-slice context, embedded-font
lifetime, variable fonts, surrogate clusters, snapshot independence and DPI.

`native_tests.cpp` is an additional Windows SDK harness which includes the
adapter directly. Compile with the build-generated `effects_shader.h`, the SDK
libraries used by `build.rs`, and optionally `FERRO_RUNTIME_SHADERS`. With
`FERROUI_D3D_DEBUG=1`, it checks D3D11 debug messages, array-slice pixels,
producer completion, NT/legacy handles, keyed mutexes, timeline fences,
runtime shader coordinates and hardware-device replacement.

`FERROUI_D3D_STATS=1` prints catalog counters in this order: surfaces, path
draws, image draws, presents, explicit readbacks, ordinary image uploads,
texture imports, GPU copies, errors, completed media reads, compute dispatches,
compute pixels. Media smoke counters distinguish asset uploads from video or
camera transfers. `FERROUI_SMOKE_SCREENSHOTS` requests explicit diagnostic
readbacks and should not be used for a zero-readback preview measurement.

The size script builds HelloWorld with identical `dist` profiles and preserves
both deployments, hashes, dependency graphs and DLL imports. It records the
source revision, OS, GPU and Rust version. All shipped application files count;
Windows/VC runtime prerequisites are listed as system imports. Historical
prototype sizes are not final measurements of this implementation.
The audit script requires PowerShell 7.2 or later. It requires one clean source
revision for the complete comparison, records compiler build retries, and
audits the runtime-shader configuration separately when requested.

The final clean-source comparison is recorded in
[windows-native-renderer-results.json](windows-native-renderer-results.json),
at source revision `2ae89e43d432901090af2446924a4e3e7de7c08e`, using Rust
1.93.0, x86_64 MSVC and the `dist` profile:

| HelloWorld configuration | EXE / total deployed bytes | Gzip bytes | Smaller than Skia |
|---|---:|---:|---:|
| Skia / HarfBuzz | 42,464,768 | 15,558,431 | — |
| Native, built-in shaders | 28,883,968 | 10,124,067 | 31.98% |
| Native, optional runtime shaders | 30,820,864 | 10,887,986 | 27.42% |

Each deployment contains one EXE and no additional application DLLs/assets.
Windows and installed VC runtime DLLs are prerequisites, listed with versions
in the report; they are excluded from both deployment totals. The optional
runtime compiler also dynamically loads the system `d3dcompiler_47.dll`.
All three preserved deployments were launched from their own directories and
exited successfully. Native startup used hardware D3D11 with zero backend
errors/readbacks. All three builds succeeded on their first attempt.

Final validation and artifact hashes are recorded in
[windows-native-renderer-validation.json](windows-native-renderer-validation.json).
The native Rust suite passes 38 tests (one separate lighting benchmark is
ignored by the normal run); all 586 shared composition tests pass. The SDK
harness reports zero D3D debug warnings/errors. Default Skia desktop builds
and runtime capture runs pass on Windows, and the latest default Linux desktop
passes `cargo check` in Ubuntu/WSL with Skia, HarfBuzz and X11 enabled.

Paired native/Skia catalog captures at 175% DPI cover 18 dark-theme pages:
Home, TextBlock, TextBox, Image, TabbedPage, Visuals, Composition brushes,
Shadows and lights, Effect brushes, Acrylic brush, Blend and colour effects,
Lighting, Reveal, Colour effects, Image effects, Shader effects, Interactive
effects and Page transitions. Seven light-theme captures cover Home,
TextBlock, Image, Composition brushes, Acrylic brush, Reveal and Shader
effects. The native runs report zero backend errors. The comparison shows
consistent layout/effect behavior, with font antialiasing and blur rounding
differences; animated samples can differ with capture time. TextBox and
TabbedPage here are overview galleries, not interactive coverage of every
subpage. A separate native TextBox trial exercised embedded font styles,
Arabic/mixed-direction editing, selection, emoji deletion and undo.

The latest normal catalog video run passes playback, shared-texture import,
navigation retirement and QR decoding with zero CPU frame transfers. Physical
discovery reports zero cameras; an explicit capture request therefore times
out as unavailable. Physical capture remains unverified. The separate,
explicitly synthetic GPU-camera fixture passes preview and navigation
retirement with zero CPU frame transfers, and is not enabled in the user trial.

Current hardware validation uses Windows, an NVIDIA GeForce GTX 1660 Ti, and
hardware D3D11 feature level 11.1. The native SDK harness passes with zero
debug-layer errors/warnings. Catalog video and synthetic-camera playback pass
with zero CPU frame transfers and full lease retirement after navigation.
Physical camera capture, macOS execution of the shared Quartz change, and full
catalog parity remain validation items; this backend is experimental.

The effect architecture was checked against the MIT-licensed
[WinUI Reveal implementation](https://github.com/microsoft/microsoft-ui-xaml/blob/78c72df1a5e6df181ce2d86a8a8cdff53be5ef22/controls/dev/Materials/Reveal/RevealBrush.cpp).
The native adapter uses the existing Ferro rendering contracts and shared
composition effect semantics.

### Video delivery pacing follow-up

The Windows importer previously spent separate presentation ticks creating an
import, polling it and delivering its ready frame. Immediately completed
imports now finish in one tick; genuinely asynchronous work and producer fences
still defer delivery without blocking the UI. This applies to both Windows
providers (Direct2D and ANGLE/Skia).

The shared video control polls at 16 ms while active, retaining its roughly
one-second command-settling allowance. The shared texture presenter permits two
immutable updates in flight, skips duplicate identities and retires temporary
imports in the same render batch as their surface update. The acquired bitmap
and GPU reader retain their own leases. The existing three-frame storage budget
still applies. Geometry changes are queued with their frame; unchanged frames
reuse the existing geometry. These presenter/control changes apply to every
rendering backend, including camera previews.

The native composition swap chain presents with sync interval zero, allowing
DWM to compose the latest complete image without an additional blocking vblank.
Tearing flags remain disabled. Media Foundation read-ahead now applies video
and audio backpressure independently, so a full video queue cannot prevent the
audio clock from receiving data.

Validation on the GTX 1660 Ti includes three import-state tests, six shared
media-control tests, twelve video-control tests and a composition test proving
same-batch import disposal preserves the surface bitmap. An explicit hardware
test consumes and draws the bundled 24 fps video through Direct2D at 24.20 fps
over five seconds, disposes temporary imports before drawing and verifies full
lease retirement. The native SDK harness reports zero D3D debug warnings/errors,
including its resize/minimize/device-replacement checks.

The final foreground native fast-release catalog sustained 24.05 newly imported
frames/s over 8.109 seconds from that 24 fps sample, with zero CPU frame
transfers, zero backend errors and successful navigation retirement/QR decoding.
This measures admitted frame delivery, not physical display scanout. The default
Skia debug catalog also passes playback, retirement and zero-CPU-transfer smoke
checks; its unoptimized build is not a matched renderer throughput comparison.
The final normal native catalog also passes interactive Play, Pause, paused
seek, resume and Stop checks; Stop restores the first frame at 0:00.
Set `FERROUI_SMOKE_MEDIA=1` and `FERROUI_SMOKE_MEDIA_VIDEO_MS=5000` to repeat the
sustained-delivery observation. Local logs are under `target/native-evidence/`
with the `video-pacing-` prefix. Physical camera capture remains unverified.

### Acrylic intermediate target bounds

Effect brushes with an in-app backdrop and hard-border Gaussian blur now use
bounded temporary targets in the shared compositor. The target encloses the
brush plus the blur's sampling reach, transformed into device pixels and rounded
outward. Nested hard blurs accumulate their padding. This keeps the acrylic
luminosity/colour blend layers and snapshots at card size instead of window size,
including on backends whose internal save layers use their entire target.

Backdrop replay shifts its render-root transform and clip into that temporary
target. Dependency regions are translated back to the original device
coordinates, so later changes behind an acrylic card still invalidate the right
area. Nested backdrop renders carry the same origin through their replay.
Pointwise effects retain their existing fresh-layer reuse. Soft-border blurs,
shadows, convolutions and host-backdrop destination clearing keep their existing
path. A temporary context without the required effect support also falls back.

All 589 shared composition tests pass, including fractional-DPI blur padding,
nested backdrop replay and original-device dependency coordinates. The default
Skia catalog builds and captures the acrylic page successfully. Local evidence
for this follow-up uses the `target/native-evidence/acrylic-` prefix.

Native fast-release slider trials on the GTX 1660 Ti at 125% DPI, with the
catalog maximized on the same 1024-by-768 display, reduce average render work
from 44.730 ms to 22.682 ms in representative 60-frame acrylic samples (about
49%). These are submission-work measurements, not display-frame-rate claims;
work remains above a 16.7 ms budget. The display resolution changed after this
comparison, so subsequent profile records are not part of that pair. Native
before/after captures at 1005-by-699 pixels preserve the card layout and backdrop
appearance, and the native runs report zero backend errors. Both ends of the
tint slider were exercised. The first optimized rebuild encountered a compiler
access violation; the retry completed successfully. The default Skia build and
capture also pass with the final source.

### Upstream and investigation integration

The Windows branch incorporates upstream main `6836f7a1e` and the investigation
branch `dff710746`, including its updated Windows handover and Quartz contract
audit. The two shared drawing conflicts retained both custom-effect graph
handling and the new scene-3D operation. Windows-specific rendering, text, media
and bounded-acrylic changes remain in place. This integration does not add a
Direct2D implementation of the optional scene-3D drawing interface.

The new Renderer parity catalog page exposes geometry acceptance cases that
needed Windows corrections: fill-only curves contribute to bounds, containment
distinguishes fully-inside/fully-contains from partial overlap, and a stroke
broken by an unstroked segment closes to the original figure start. These use
the same shared geometry contract as the newly audited Quartz implementation.
Writable inputs now retain BGRA8888, RGBA8888 or RGB565 CPU memory with the
requested alpha convention, using separate premultiplied BGRA GPU storage.
Unchanged locks preserve bytes/version and do not read back or upload pixels.
Drawing, resizing and shader inputs consume that GPU storage directly. The
geometry export also discards hollow boolean-boundary figures instead of
mistaking them for filled intersections. The native suite passes 42 tests (one
lighting benchmark remains separately ignored), and all 590 shared composition
tests pass. The broader encoding cases in the macOS audit still require a
Windows audit; macOS results are not Windows certification. Earlier
size reports identify their historical source revisions and are not measurements
of this integration.

The merged native catalog passes the Renderer parity text smoke check (RTL
selection, hit testing, replacement and undo) and its geometry/writable-input
construction checks, with zero backend errors. Both native and default Skia
catalog compilation checks pass. Evidence uses the `merge-` prefix under
`target/native-evidence/`.

The investigation branch's subsequent iOS/Quartz commit `58460442c` is also
merged. It does not change the Windows renderer, shared Base/Controls code or
the TreeDataGrid benchmark source measured below.

### TreeDataGrid GPU measurements

The [2026-11-10 desktop gauntlet comparison](windows-gauntlet-renderer-benchmarks.md)
after merging main at `adaa37363` shows a different workload profile: native
takes 42% longer per animated gauntlet callback, while delivering 2.94× Skia's
callback throughput on the still wall. Those desktop timings include scheduling
and are distinct from the offscreen TDG measurements below. The report reviews
the open performance stack (#676–#679), retains raw samples and records the
actual clamped Windows viewport.

The [2026-11-09 follow-up](windows-native-renderer-main-followup.md) merges
`main` at `f91b679fe` and adds bounded positioned-run reuse, repeated-use
admission, deferred native transform updates and safe overlap-scan shortcuts.
Five fresh paired runs give 10.7–26.9% lower native mean times than Skia across
all twelve workloads; ten improve over the merged native baseline. All 55
native tests, identical TDG captures, catalog text/DPI checks and video/shared
textures pass. The report retains variability, individual-pair limits and raw evidence.

The latest [performance parity report](windows-tdg-renderer-parity.md) beats Skia
in all twelve matched recorded-path workloads: five complete Skia-first/native-second
pairs give 7.9–19.9% lower native GPU-drained frame times. Batched native commands
and glyph draws, deferred safe text clips and guarded DirectWrite simple shaping
close the remaining gap. All 54 native tests, catalog text/DPI/effects captures,
24 fps video/shared textures and synthetic GPU camera checks pass. Physical
camera capture remains untested because this machine has no camera. Raw samples,
source/binary identities and a repeatable comparison runner accompany the report.

Earlier stages remain documented for comparison:

The [2026-11-08 comparison](windows-tdg-renderer-benchmarks.md) runs Skia first,
then native, on the same hardware and optimized executable. Native is slower
in all twelve recorded full-redraw workloads: 3.650–8.693 ms/frame versus
0.399–1.790 ms/frame, or 4.2–9.4× the Skia time. The separate immediate drawing
path is substantially slower again and exposes unnecessary opacity-1 layers.
These are offscreen GPU-drained redraw measurements, not displayed catalog FPS.
Raw samples, variability, commands and source/binary identities accompany the
report. Functional acceptance does not establish performance parity.

The [optimization follow-up](windows-tdg-renderer-optimization.md) fixes repeated
geometry clipping of rectangles, per-command device queries and eager glyph
outlines, and reuses font metrics, brushes and the analyzer. Native recorded
redraws are 3.0–4.4× faster than that baseline, with 45 native tests and 33 shared
drawing-context tests passing. Native still takes 1.4–2.2× the Skia time in the
same workloads. The shared immediate renderer also skips opacity-1 groups for
all backends. Final catalog parity and video/shared-texture checks pass.
