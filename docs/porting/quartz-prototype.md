# macOS Quartz prototype

For the latest contract audit and size checkpoint, see [Quartz/Skia parity](quartz-skia-parity.md)
and [its recorded results](quartz-parity-results.json). Measurements below retain their stated
prototype checkpoints. Current [performance and shipping-size measurements](quartz-optimization-round4-results.md)
cover the optimized shared macOS/iOS renderer.

The opt-in `ferroui-quartz` backend uses macOS Core Text, Core Graphics geometry and ImageIO,
with **direct Metal GPU drawing** selected by `quartz-metal`. Common paints use batched Metal render passes; GPU coverage masks and compute kernels
handle paths, clips, blend modes and effects. Native glyph inputs, images, opacity groups
and presentation remain GPU textures. It bundles neither Skia/HarfBuzz nor wgpu/Vello.
The earlier CPU-only `quartz` configuration remains available as a reference.

The shared backend now also supports [native iOS rendering](ios-quartz.md) on iOS 16+.
The macOS configuration requires macOS 13+ and a shared-event-capable Metal device. Tier 2
RGBA8 read/write textures use the original fast path; other devices use separate GPU
destination textures without CPU framebuffer transfers. It fails if GPU creation is unavailable;
Metal mode does not silently switch to CPU framebuffer rendering. HelloWorld is the primary
binary size benchmark; ControlCatalog supplies broader rendering smoke coverage because its
many embedded assets and 200+ pages obscure the size of a minimal application.

## Build and run

```sh
cargo build --locked --profile dist -p hello-world --no-default-features --features quartz-metal
FERROUI_SMOKE_EXIT_MS=3000 ./target/dist/hello-world
```

HelloWorld and ControlCatalog select `FerroNativeRenderingMode::Metal` automatically with
`quartz-metal`. Select `quartz` instead for the historical Core Graphics CPU renderer.
Both backends use the same one-window XAML application, text, trimmed WinUI theme and `dist`
profile. Theme trimming remains enabled in both builds. Panic unwinding also remains enabled;
`dist-abort` is not used for this comparison. The default build continues to use Skia/HarfBuzz.

To run the native ControlCatalog prototype, use:

```sh
cargo run -p control-catalog-desktop --no-default-features --features quartz-metal
```

The catalog retains its WinUI theme, pages, embedded pictures and fonts.

For a screenshot smoke run:

```sh
FERROUI_SMOKE_SCREENSHOTS=target/quartz-prototype/screenshots \
FERROUI_SMOKE_SCREENSHOT_PAGES=Home,TextBlock,TextBox,Image,TabbedPage \
FERROUI_SMOKE_PAGES=1500 \
  cargo run -p control-catalog-desktop --no-default-features --features quartz-metal
```

## Measure binary size

```sh
python3 scripts/measure-quartz-prototype.py
```

The script builds HelloWorld from the same source twice with the same profile, switching only the `skia` and
`quartz-metal` features. It copies both executables to `target/quartz-prototype`, records exact file
sizes, gzip sizes, SHA-256 checksums, dynamic libraries and the compiler/OS/architecture in
`sizes.json`. It also checks the native build's normal dependency graph for Skia, HarfBuzz,
Vello, wgpu and ANGLE, and saves that graph. The checkout's patched Skia needs Ninja and the
Xcode command-line tools to build from source.

Use `--package control-catalog-desktop` for the corresponding catalog comparison.

The primary comparison is of complete HelloWorld executables, with the same theme trimming.
Apple's shared frameworks are prerequisites supplied by the operating system, and add no
application-shipped framework files. The gzip result describes compression of the executable,
not a signed installer or application bundle. The two renderers do not yet have feature parity.

## Metal GPU HelloWorld results

Measured after the gap-closure implementation on 7 November 2026, macOS 27.0.1 ARM64 with
rustc 1.99.1. All builds use `dist` (fat LTO, one codegen unit, stripped symbols, panic unwinding)
and the same HelloWorld XAML and trimmed WinUI theme. The generated theme-parts manifests
have identical SHA-256 `20b22207c88715089b4a6ee004e758eebefd2c288ead7bc723f3f445399299fd`.
MB means 1,000,000 bytes.

| Backend | Executable bytes | MB | gzip bytes |
|---|---:|---:|---:|
| Skia/HarfBuzz, Metal presentation | 28,282,256 | 28.28 | 10,801,627 |
| Native Metal/Core Text | 23,569,440 | 23.57 | 8,626,238 |
| Native Metal/Core Text + runtime WGSL compiler | 24,992,992 | 24.99 | 9,270,066 |

The minimal GPU backend saves **4,712,816 bytes (16.66%)**. Enabling runtime WGSL effects costs
1,423,552 bytes over that configuration and still saves **3,289,264 bytes (11.63%)** relative
to Skia. ControlCatalog enables this optional compiler; size-first HelloWorld omits it.
Measure it separately with:

```sh
python3 scripts/measure-quartz-prototype.py --native-backend quartz-runtime-shaders \
  --output target/quartz-prototype/runtime-shaders
```

Both native feature graphs exclude Skia, HarfBuzz, Vello, wgpu and ANGLE. Naga is absent from
the minimal build and present only in the runtime-shader configuration. Apple's shared
frameworks add no application-shipped framework files. Ferro's property/control system,
compiled XAML and trimmed theme remain part of each executable; the saving is not a claim
about installer size or the percentage achievable in other applications.

The completed validation includes 42 Metal backend tests, 12 CPU-reference tests, 10 shared
context-manager tests and 13 native Metal-device tests. All three release HelloWorld binaries
rendered and exited successfully. Each native run without screenshot export recorded two GPU
surfaces, 18 compute dispatches, two presentations, zero readbacks, one startup image upload
and zero GPU errors. A screenshot run added the expected single readback. Native image/glyph
inputs may upload pixels; the renderer does not use a CPU canvas for GPU presentation.

The final catalog comparison captures 15 pages in separate processes, with both immediate
and composed output, against the original Skia catalog at the same display scaling. The
native runs report zero completed GPU errors. Visual review covers text/border placement,
brushes, shadows, acrylic, color/image effects and runtime shaders. This establishes the named
regressions and overview coverage, not every interaction on the catalog's 200+ pages or
pixel-identical Skia output.

Artifacts are under `target/quartz-prototype/goal-complete`: the three `hello-world-*`
executables, `hello-*.png`, `sizes.json`, `runtime-shaders/sizes.json`, test/media logs,
`catalog/`, `skia-catalog/` and `validation-audit.json`. Exact bytes, hashes, source fingerprints,
commands, validation and historical results are retained in
[quartz-prototype-results.json](quartz-prototype-results.json).

## Historical CPU-only HelloWorld results

Measured on 7 November 2026, macOS 27.0.1, ARM64, rustc 1.99.1. Both executables were built
from the same prototype working tree on `97653e5036db`, using `dist` with unwinding and symbol
stripping. The generated theme-parts manifests are byte-for-byte identical. MB means 1,000,000
bytes. These results apply to the existing HelloWorld sample with its trimmed WinUI theme.

| Backend | Executable bytes | MB | gzip bytes |
|---|---:|---:|---:|
| Skia and HarfBuzz | 28,282,256 | 28.28 | 10,801,706 |
| Quartz and Core Text | 23,400,000 | 23.40 | 8,553,561 |
| Reduction | 4,882,256 | 4.88 | 2,248,145 |

The native executable is **17.26 percent smaller**. The result does not imply complete rendering
parity or the same percentage saving in applications with other features. The normal native
dependency graph excludes Skia, HarfBuzz, Vello, wgpu and ANGLE. Both executables rendered
HelloWorld, saved an 800 by 480 screenshot, closed through the smoke timer and exited with code
zero. The screenshot paths are `target/quartz-prototype/hello-skia.png` and `hello-quartz.png`.

Eight backend tests pass: UTF-16 subrange/RTL shaping, visible glyph drawing, bold system-font
fallback, color glyph ink bounds, geometry holes, clipping/opacity/snapshots, PNG/DPI round trips,
and recovery after a singular transform. The native catalog also saved both immediate and
composed screenshots of five overview pages (Home, TextBlock, TextBox, Image and TabbedPage)
without render-loop errors. This smoke run does not establish that every demo or editing
interaction works.

Exact commands, checksums, dynamic library references and source hashes are in
[the recorded results](quartz-prototype-results.json). The current compiler reports existing
framework deprecation/unused-code warnings. The native backend itself passes its formatting
check and tests.

## Backend integration

`ferroui-desktop` retains its default `skia` feature and adds `quartz` and `quartz-metal`. The two samples use explicit
path dependencies with default features disabled because Cargo workspace dependency inheritance
would otherwise re-enable the default Skia feature. Applications enabling both features retain
Skia's dependencies; a small native build must disable default features.

The backend implements the existing platform render, drawing, geometry, bitmap, font manager
and text shaping contracts. Font tables still feed FerroUI's own `GlyphTypeface` implementation,
and layout, bidi processing and text editing remain in FerroUI. The native shaper produces
Ferro's existing UTF-16 cluster indices. It checks each Core Text run's font identity before
accepting glyph IDs, so glyphs from an unexpected fallback face are not drawn with another font.

The C ABI bridge owns retained framework references. Rust owners release them when their last
reference is dropped. Immutable CTFont objects can cross threads; mutable drawing surfaces are
accessed under a mutex. Bitmap locks use owned staging pixels. The CPU reference copies into a
locked platform framebuffer; Metal mode presents directly through GPU textures.

Core Graphics provides path boolean operations, stroke outlines and curve flattening. In the
CPU reference configuration it also provides drawing. In Metal mode, the GPU implements filling,
clipping, gradients, transparency layers and ordinary glyph drawing. ImageIO decodes and encodes images. The small `kurbo`
dependency provides Bezier command storage, SVG arc conversion and path measurements; the backend
does not link a second rasterizer or shaping engine. Region decomposition and path measurement
reuse the existing Vello backend's engine-independent helper algorithms.

The catalog's FluidNavBar uses Ferro geometry and drawing contracts in both configurations.
It retains its animated dip, buttons and separately trimmed icon contours without a Skia canvas
lease. Headless/VNC retain their default HarfBuzz behavior, with a feature to keep an explicitly
configured text shaper in a native-only build.

## Native GPU coverage and platform policy

`quartz-metal` implements paths, geometry booleans/strokes, separate corner radii, clips,
opacity/masks and retained GPU snapshots; linear/conic/focal elliptical radial gradients with
transforms and pad/repeat/reflect spread; image/scene brushes with layout and mirrored tiles;
and nearest, bilinear, mipmapped and cubic image sampling. Box shadows include spread,
inset/outset ordering and caster exclusion. Bitmap storage is premultiplied RGBA on Metal;
the public readable interface converts explicitly to BGRA. ImageIO handles PNG/JPEG codecs.

GPU filter/layer operations cover Gaussian blur, drop shadow, luminance-to-alpha, blending,
color matrices, multiply factors, convolution, pixel-operation folds and the shared material/
lighting contract. Acrylic tint/luminosity/noise stays on the GPU; the shared render walk supplies
backdrop content. Direct custom-effect graphs use Ferro's shared composition painter. Runtime
WGSL effects use an optional Naga-to-MSL adapter and native Metal pipelines; they require
`runtime-shaders` on the crate, or `quartz-runtime-shaders` on Desktop/HelloWorld. ControlCatalog
turns that option on with its `quartz-metal` feature. Size-first applications can omit it.
The normal native dependency graph contains no Skia/HarfBuzz, wgpu, Vello or ANGLE.

Core Text supplies system/embedded fonts, fallback, family styles, localized shaping and OpenType
features. UTF-16 slices retain their surrounding joining context with local cluster indices.
System UI family descriptors preserve all nine standard weights (100–900) instead of collapsing
them to regular/bold. Simulated bold/oblique outlines and underline ink intervals are supported. The public
`QuartzFontManager::create_variable_typeface_from_stream` extension selects validated OpenType
variation axes; Ferro's generic font-manager trait does not expose arbitrary axis coordinates.
Normal glyphs use GPU outline coverage. Explicit light/strong hinting uses bounded, DPI-aware
Core Text glyph-image inputs, cached independently of brush tint and composed on the GPU.
Color glyph inputs use the same cache; non-solid text brushes preserve their intrinsic colors
while applying brush opacity and alpha through a GPU mask. Baseline alignment and alias/antialias selection apply
to solid and outline-brush text; native text uses grayscale coverage, including when an
application asks for LCD subpixel antialiasing. This follows this backend's macOS policy for
transparency and avoids promising Skia's exact edge weights or hinting behavior.

The historical CPU-only `quartz` configuration is a reference, not the GPU coverage target.
It still has a narrower brush/effect implementation. Both configurations fail explicitly for
unsupported writable formats rather than accepting invalid pixel layouts. Metal mode uses
no CPU canvas or framebuffer presentation; CPU transfers are explicit bitmap lock/export/
screenshot operations and decoded/native glyph image inputs. Video/camera frames stay on the GPU.

The supported device is FerroNative's system-default GPU on macOS 13+, with RGBA8 read/write
tier 2, shared events and system MPS support. A different presentation device is rejected before
cross-device texture use. Arbitrary adapter switching, Intel/eGPU behavior and physical GPU
removal are hardware-unverified on this Apple Silicon Mac. No webcam is available. These are
explicit platform/hardware limits; the implementation does not silently fall back to CPU rendering.
Full wide-gamut/HDR color-management fidelity and pixel-identical Skia text are not claimed.

## Metal rendering design

The `metal` feature links Apple's Metal and Metal Performance Shaders frameworks through a small
Objective-C bridge. It compiles embedded MSL through the system compiler at first use. Offscreen
surfaces use private BGRA8 textures on Apple GPUs and RGBA8 elsewhere. Compatible
platform drawables are rendered directly with non-retained frame contents; other targets keep
a private frame. A renderer lock and ordered queue serialize drawing, copies, filters and
external-event waits/signals. The platform queue is shared when possible; transitions and
presentation across different queues retain shared-event ordering. Normal presentation does
not transfer pixels or block the CPU. See the [batching and native-text design](quartz-metal-optimization.md).

Core Graphics prepares retained native paths at quarter-device-pixel flattening tolerance. Winding
coverage uses four samples per pixel; aliased mode uses one. Paths with at least 32 edges use
8-pixel spatial row bins, cached within a 16 MB budget. Glyph outlines are cached per Core Text
font, and glyph-image inputs are capped at 16 MB. Metal commands share batches capped at 256
operations and an estimated 64 MB of newly allocated render storage; presentation, explicit
readback and producer-frame completion flush them. A 64 MB texture pool shares storage owners
with snapshots and layer parents, preventing reuse while another bitmap aliases the texture.
Storage owners carry the renderer generation, preventing an old owner's late release from
recycling storage into a recovered queue.
An isolated regression checks fewer than eight submissions and at most two fresh texture
allocations for 64 repeated surfaces, then checks snapshot contents after pool reuse.

MPS implements isotropic Gaussian blur; a separable GPU pass handles general affine blur axes.
Other effects use small compute kernels. Sampling generates/caches mip chains entirely on the
GPU and invalidates them on writes; high-quality magnification uses Mitchell reconstruction.
Native glyph input generation uses Core Text, while tint, clips and composition remain GPU work.

Completed command-buffer errors invalidate the renderer generation. The shared context manager
now recreates a lost backend even if its platform device is still healthy; Quartz starts a new
queue/event, clears caches and rejects old-generation imports. Late failures from an old queue
cannot poison its replacement. Indeterminate producer reads retain their quarantine semantics;
loss is not a completion proof. Healthy context disposal submits remaining UI commands without
waiting on the CPU. This policy has fault-injection and context-manager tests; physical driver
reset/removal remains unverified. Device access revoked by the OS may still require application
restart; a new queue does not claim to restore access the OS has revoked.

The renderer remains experimental: four-sample coverage, full-canvas effect intermediates and
runtime MSL compilation leave performance/quality tradeoffs. The tests prove specific resource,
ordering and rendering contracts, not production frame-rate or memory parity with Skia.

Set `FERROUI_METAL_STATS=1` for HelloWorld to print cumulative surfaces, paint/image
operations, presentations, readbacks, uploads and completed-command errors at shutdown.
Paint/image counts precede batching and are not physical GPU dispatch counts.
The diagnostic waits for submitted work to finish, but reads no pixels. A normal smoke run
must have positive compute/presentation counts, zero readbacks and zero errors. A screenshot
run deliberately reads back its offscreen output. These counters distinguish actual GPU
rendering from CPU drawing uploaded for presentation.

## Video, camera and the TextBox border correction

`quartz-metal` now exposes Ferro's external-objects feature through both its optional features
and its public compositor feature map. It delegates owning IOSurface handle wrapping and
Metal texture import to FerroNative. Imported frames validate device identity, renderer
generation, dimensions, storage format, alpha and color encoding. Supported inputs are
BGRA8/RGBA8 with premultiplied or opaque sRGB, and opaque full-range Rec.709. Unsupported
formats fail explicitly. Timeline-event snapshots run wait/copy/signal on the renderer queue
and own independent GPU pixels. The `SharedMetalImage` extension also imports a retained native
Metal texture through the shared-context-image contract without an IOSurface or CPU copy.
Binary semaphores, keyed mutexes and automatic synchronization are not advertised Metal
contracts; producers use shared timeline events or immutable frame leases with GPU completion.

The first draw of an imported frame converts it into a private GPU texture using a Metal
compute pass. That pass handles BGRA/RGBA sampling, bottom-up storage, opaque alpha and
Rec.709-to-sRGB conversion. Subsequent draws reuse the converted texture. The producer's
CoreVideo allocation remains leased until Metal's completion handler runs. Navigation away
retires outstanding reads and releases the frame budget. No CPU mapping, readback or upload
of video/camera pixels is used. A failed completion quarantines the producer lease.

The catalog's bundled video was played through at least five advancing GPU conversions.
A synthetic camera using GPU-generated immutable CoreVideo IOSurfaces exercised the actual
CameraPreview, camera session, mailbox, native importer and compositor at 640 by 480 / 30 fps.
Both media flows reported zero CPU pixel transfer counters and retired all frames on navigation.
The global Metal counters were read at startup, after video and after camera: readbacks stayed
at seven and uploads at fourteen, with zero GPU errors throughout. Those existing transfers
occurred before media playback; media added neither readbacks nor uploads. The later QR image
demo adds unrelated image processing.

**A physical camera preview has not been verified:** the test Mac has no camera (it reports
zero devices with permission granted). The synthetic test establishes the
camera presentation path, not AVFoundation hardware acquisition. The fixture is explicit,
development-only and absent from ordinary `quartz-metal` and HelloWorld builds:

```sh
FERROUI_SMOKE_MEDIA=1 FERROUI_SMOKE_MEDIA_CAMERA=1 \
FERROUI_SMOKE_MEDIA_SYNTHETIC_CAMERA=1 FERROUI_METAL_STATS=1 \
  cargo run -p control-catalog-desktop --no-default-features --features quartz-media-test-fixtures
```

With a webcam attached, use `--features quartz-metal` and omit
`FERROUI_SMOKE_MEDIA_SYNTHETIC_CAMERA` to test actual capture. For video plus camera discovery
without acquiring hardware, set only `FERROUI_SMOKE_MEDIA=1`. Evidence logs are under
`target/quartz-prototype/goal-complete/` and structured results are recorded in the results JSON.

The WinUI TextBox border brush reflects a two-unit gradient around its vertical center using
`ScaleY=-1`. Ignoring that brush transform placed the highlight at the top. Gradient painting
now applies relative and absolute brush transforms around the declared origin, independently
of the geometry clip and visual transform. A high-DPI translated regression test and catalog
screenshots confirm that the highlight appears at the bottom. Image/scene-brush transforms
are also covered by the completed GPU brush regressions.

## GPUI findings and platform recommendations

The inspected Zed revision is `b5cd310b3b54dfc659fcc06453e6358843dfe216`. GPUI has split its
platform implementations out of the core `gpui` crate. On Apple, its
[renderer](https://github.com/zed-industries/zed/blob/b5cd310b3b54dfc659fcc06453e6358843dfe216/crates/gpui_apple/src/metal_renderer.rs)
uses Metal with its own shaders, atlases and primitive batching; its
[text system](https://github.com/zed-industries/zed/blob/b5cd310b3b54dfc659fcc06453e6358843dfe216/crates/gpui_apple/src/text_system.rs)
uses Core Text and system glyph rasterization. Metal supplies the GPU API, not a complete
Avalonia-compatible 2D renderer. Ferro still needs an implementation of its richer drawing
contracts. GPUI's dependency approach is applicable; copying only its platform API selection
would not implement Ferro's paths, opacity masks and brush semantics.

On Windows GPUI uses a
[Direct3D renderer](https://github.com/zed-industries/zed/blob/b5cd310b3b54dfc659fcc06453e6358843dfe216/crates/gpui_windows/src/directx_renderer.rs)
and DirectWrite text. For Ferro, Direct2D + DirectWrite is the next strong native-backend
candidate: unlike Metal, Direct2D exposes many required 2D drawing operations directly.
The supplied [CoreText.Avalonia](https://github.com/wieslawsoltes/CoreText.Avalonia/tree/b0e7af21ec8e13303055227911074f832958aea5)
and [DirectWrite.Avalonia](https://github.com/wieslawsoltes/DirectWrite.Avalonia/tree/d0e283f9970f550316316bec9fa7e5438ada4c40)
prototypes are useful references for the font/shaper contract, and the latter includes a
Direct2D prototype. Keep these platform implementations behind Ferro's existing interfaces;
keep Skia as the portable default until native coverage and performance are established.

Linux does not offer a universally installed GPU 2D API equivalent to Direct2D. GPUI's
[Linux text system](https://github.com/zed-industries/zed/blob/b5cd310b3b54dfc659fcc06453e6358843dfe216/crates/gpui_linux/src/linux/text_system.rs)
and [wgpu text adapter](https://github.com/zed-industries/zed/blob/b5cd310b3b54dfc659fcc06453e6358843dfe216/crates/gpui_wgpu/src/cosmic_text_system.rs)
retain a Rust text stack. A Linux native option would need to choose explicit distribution
prerequisites (for example system Pango/fontconfig/FreeType plus a GPU renderer) and report
those external runtime dependencies separately from executable size. It should be a later
experiment, not assumed to offer the same deployment benefits as Windows or Apple platforms.

iOS is a plausible follow-on for this Metal/Core Text design, but this prototype is macOS-only:
Ferro's iOS surface/device bridge and framework availability must be adapted and tested.
Android likewise requires its own shaping and rendering adapter; system Vulkan/OpenGL ES
provide GPU access but not Ferro's complete 2D API. Investigate public NDK text APIs and their
minimum Android versions before promising removal of bundled text libraries. GPUI's inspected
platform code does not establish a complete Android solution.

Next steps are to measure startup/frame time/memory, broaden catalog interaction and device
coverage, introduce Windows Direct2D/DirectWrite with the same feature selection, then assess
iOS reuse and separate Linux/Android prototypes. Keep benchmark applications,
theme settings and release profiles identical for each backend, and publish executable size,
compressed size, startup, frame time and memory measurements with rendering coverage.

## Completed gap-closure validation

The requested macOS GPU gap categories are implemented and validated for the supported device
policy described above. The tests assert rendering behavior and ownership contracts:

| Area | Validation |
|---|---|
| Effects/composition | GPU blur with transformed axes and parent output clips; shadow/luminance; matrix, convolution, factors, lighting and pixel operations against shared references; acrylic; direct custom graphs; runtime shader reference pixels and GPU image children |
| Brushes/sampling | Conic/focal elliptical gradients, repeat/reflect and transforms; image/scene layout and mirrored tiles; nearest/bilinear, mip invalidation and cubic premultiplied coverage |
| Shadows/render options | Inset/outset spread and ordering; geometry/text aliasing; merged/restored options and baseline alignment |
| Text/fonts | UTF-16/RTL/contextual joining; fallback and all system UI weights; variable axes, simulations, underline ink; DPI-aware hinted glyph caching; color glyphs with non-solid brush alpha |
| External textures | Native shared-image import, timeline wait/copy/signal and snapshot independence; device/generation validation; completion-driven media retirement and failure quarantine |
| Performance/recovery | Spatial bins, bounded batching/cache/pool reuse and aliased snapshots; lost backend recreation with a healthy platform device; late old-generation completion and texture-owner release |

The final native catalog and original Skia catalog were captured for Home, TextBlock, TextBox,
Image, Visuals, Composition brushes, Shadows and lights, Effect brushes, Acrylic brush, Blend
and colour effects, Colour effects, Image effects, Shader effects, Interactive effects and
Page transitions. Each backend has 30 PNGs covering the two drawing paths. Animations and
native font metrics can differ; these captures do not establish universal rendering parity.

Video playback and the explicit GPU IOSurface synthetic camera test both passed with zero CPU
media pixel transfers. Normal camera discovery reports zero cameras with permission granted.
Physical camera capture, Intel/eGPU behavior and actual device removal remain hardware-unverified.
The historical CPU renderer remains narrower. These limits do not imply CPU fallback in Metal
mode, and the tests do not claim production frame-rate or full HDR/color-management parity.

## Capture validation correctly

The catalog's multi-page smoke runner can advance navigation before an asynchronous composed
snapshot consumes its server scene. A filename can then describe the previous page while the
pixels show the next. For backend comparisons, capture one page per process:

```sh
python3 scripts/capture-quartz-comparison.py \
  --binary target/quartz-prototype/catalog-stock \
  --output target/quartz-prototype/goal-complete/skia-catalog --macos-bundle \
  --expected-pixel-size 2200x1600
python3 scripts/capture-quartz-comparison.py \
  --binary target/quartz-prototype/goal-complete/control-catalog-quartz-metal \
  --output target/quartz-prototype/goal-complete/catalog --macos-bundle \
  --window-position 2150,100 --expected-pixel-size 2200x1600
```

The script captures both immediate and composed paths, preserving per-page logs and a manifest.
`--macos-bundle` copies the executable unchanged into a temporary app bundle declaring
`NSHighResolutionCapable`. On this mixed-DPI setup, the original Skia captures rendered at 2×;
the native captures were pinned to desktop position 2150,100 on the 2× display. Adapt that
position and expected PNG size to your display setup. `--window-position` needs the updated
catalog's `FERROUI_SMOKE_WINDOW_POSITION` hook; the original catalog predates that hook.
`--expected-pixel-size` fails if a process ends up at a different pixel density. A successful
process exit alone does not establish that screenshots are comparable.
The original multi-page captures remain diagnostic artifacts, not authoritative page comparisons.
Compare static content within each named page; animated samples and native font metrics can
differ in timing and layout. Direct bitmap captures and live composed output are separate paths.

## Common macOS performance benchmark

The [2026-11-08 performance report](quartz-performance.md) compares Skia, Quartz, native
rendering with HarfBuzz, and all Vello modes through common rendering/text contracts. It records
large warmed full-redraw gaps for native dense text and opacity layers; functional coverage and
small binary size do not establish performance parity. Core Text remains the quality reference.
