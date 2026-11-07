# macOS Quartz prototype

The opt-in `ferroui-quartz` backend uses macOS Core Text, Core Graphics geometry and ImageIO,
with **direct Metal GPU drawing** selected by `quartz-metal`. Paths and ordinary glyph outlines
are rasterized by Metal compute shaders; images, clips, opacity groups, retained layers and
window presentation remain GPU textures. It bundles neither Skia/HarfBuzz nor wgpu/Vello.
The earlier CPU-only `quartz` configuration remains available as a reference.

This is an experimental macOS 13+ prototype. It requires Metal RGBA8 read/write texture tier 2
and a shared-event-capable device. It fails explicitly if the GPU requirements are unavailable;
Metal mode does not silently switch to CPU framebuffer rendering. HelloWorld is the primary
binary size benchmark; ControlCatalog supplies broader rendering smoke coverage because its
many embedded assets and 200+ pages obscure the size of a minimal application.

For parallel Windows implementation, see the [DirectWrite/Direct2D handover](windows-native-renderer-handover.md).

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

These measurements precede the active gap-closure work below; the final comparison will be rerun
after those changes are validated.

Measured on 7 November 2026, macOS 27.0.1 ARM64 with rustc 1.99.1. Both builds use
`dist` (fat LTO, one codegen unit, stripped symbols, panic unwinding). All three generated
HelloWorld theme-parts manifests have SHA-256
`20b22207c88715089b4a6ee004e758eebefd2c288ead7bc723f3f445399299fd`.
MB means 1,000,000 bytes; only the backend feature differs.

| Backend | Executable bytes | MB | gzip bytes |
|---|---:|---:|---:|
| Skia/HarfBuzz, Metal presentation | 28,282,256 | 28.28 | 10,801,706 |
| Native Metal/Core Text | 23,466,800 | 23.47 | 8,583,627 |
| Reduction | 4,815,456 | 4.82 | 2,218,079 |

The GPU prototype is **17.03 percent smaller**, retaining GPU rendering. This saves almost
all of the earlier CPU prototype's reduction; the Metal implementation adds 66,800 bytes
relative to that historical result. The remaining executable still includes Ferro's shared
framework, property/control system, compiled XAML and trimmed theme. Removing the rendering
and shaping dependencies alone does not remove those costs.

Thirteen Metal backend tests pass, including imported-frame validation, color/orientation
conversion, completion-driven retirement, and the reflected TextBox border gradient. The CPU
reference also passes nine regression tests. Both GPU HelloWorld executables exit through the smoke
timer and render an 800 by 480 PNG. The native build's normal dependency graph excludes
Skia, HarfBuzz, Vello, wgpu and ANGLE.

A native run without a screenshot recorded one GPU surface, 17 compute dispatches, two
presentations, **zero readbacks, zero uploads and zero GPU errors**. Its screenshot run recorded
one expected readback. The Metal catalog's debug build rendered Home, TextBlock, TextBox, Image
and TabbedPage overview pages and saved ten immediate/composed PNGs without render-loop errors.
Home, text, image and navigation content were visually reviewed. This is overview coverage,
not validation of every catalog demo or interaction.

Artifacts are under `target/quartz-prototype`: `hello-world-skia`, `hello-world-quartz-metal`,
`hello-metal.png`, `sizes.json`, `native-dependencies.txt` and `metal-debug-screenshots/`.
Exact bytes, hashes, commands, validation and historical CPU results are retained in
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

## Prototype coverage and limits

The first implementation covers solid and basic linear/radial brushes, paths, geometry booleans,
rectangles with separate corner radii, ellipses, clipping, opacity groups and masks, standard
composition blend modes, glyph drawing, bitmap drawing and readback, PNG/JPEG encoding, and
Core Text system/embedded fonts and shaping. It uses standard premultiplied BGRA bitmap storage;
RGBA input can be converted when loading images or presenting framebuffers.

Conic gradients, repeating gradients/tiles, image/scene-brush transforms, elliptical radial gradients,
inset shadows, custom effects/shaders, external GPU image types beyond IOSurface, advanced text rendering options,
font simulations/variation selection, shaping context across styled run boundaries and exact underline ink skipping need further work.
Some standard effects may be omitted by Ferro's optional-feature fallback. Unimplemented conic-brush
and inset-shadow paths log a diagnostic once per feature. Writable bitmap requests in unsupported
formats fail explicitly. This configuration is for evaluating the native approach, not for
shipping arbitrary existing Ferro applications unchanged.

The historical `quartz` reference runs drawing and framebuffer copies on the CPU. The
`quartz-metal` configuration uses private RGBA8 GPU textures and a fullscreen Metal render pass
to the native drawable. CPU readback occurs only for an explicit bitmap lock, encoding or
screenshot. Image decoding and color-glyph rasterization produce CPU image inputs which are
uploaded to textures. Shaping and path preparation run on the CPU, as they do in many GPU UIs.
Binary size results do not establish frame-rate, startup or memory parity with Metal/Skia.

## Metal rendering design

The `metal` feature of `ferroui-quartz` links Apple's Metal framework and a small Objective-C
bridge. It compiles the embedded Metal shader source through the system compiler at first use.
The C ABI is also used for immutable paths/fonts and explicit pixel transfers. Each surface
owns a private RGBA8 texture. GPU resources are serialized through one renderer lock and one
command queue; the Rust bitmap mutex guards ownership and drawing access. The presentation
queue waits on an `MTLSharedEvent` signaled by the drawing queue before presenting. This avoids
CPU pixel transfer and avoids filling the native platform queue with per-draw command buffers.

Core Text supplies glyph outlines and shaping. Core Graphics transforms, strokes and flattens
paths at a quarter-device-pixel tolerance. The compute kernel evaluates winding and four
coverage samples per pixel within the path's bounding box, applies a GPU clip mask and blends
into the destination texture. Gradients are evaluated in inverse-transformed logical coordinates.
Snapshots, resize operations, image draws and layer composition use GPU copies/sampling.
Color glyphs without outlines use small Core Text bitmap inputs, uploaded separately.

This deliberately small renderer establishes feasibility, not production performance. The
coverage algorithm examines every edge for each covered pixel, and the prototype submits a
command buffer per operation. Production work should add spatial edge bins, batching, cached
glyph/path coverage, resource pools and precompiled shaders. Antialiasing is a basic four-sample
approximation. Full color-management fidelity needs validation. Radial gradients and shadow blur are approximate; all the other coverage gaps
listed above still apply. Arbitrary GPU effects, device recovery, multiple GPUs and additional
external texture types require separate work. Validate on Intel and Apple Silicon before widening support.

Set `FERROUI_METAL_STATS=1` for HelloWorld to print cumulative surfaces, compute dispatches,
image dispatches, presentations, readbacks, uploads and completed-command errors at shutdown.
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
formats fail explicitly. Arbitrary shared-image snapshots and generic semaphore snapshots
remain unsupported; media uses immutable frame leases and actual GPU completion.

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
at seven and uploads at eleven, with zero GPU errors throughout. Those existing transfers
occurred before media playback; media added neither readbacks nor uploads. The later QR image
demo adds unrelated image processing.

**A physical camera preview has not been verified:** this Mac reports zero cameras, permission
is granted, and the user confirmed no webcam is available. The synthetic test establishes the
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
`target/quartz-prototype/media-interop/` and structured results are recorded in the results JSON.

The WinUI TextBox border brush reflects a two-unit gradient around its vertical center using
`ScaleY=-1`. Ignoring that brush transform placed the highlight at the top. Gradient painting
now applies relative and absolute brush transforms around the declared origin, independently
of the geometry clip and visual transform. A high-DPI translated regression test and catalog
screenshots confirm that the highlight appears at the bottom. Image/scene-brush transforms
still require further work.

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

Recommended stages are: measure and validate this macOS GPU option; optimize and close the
listed rendering gaps; introduce Windows Direct2D/DirectWrite with the same feature selection;
then assess iOS reuse and separate Linux/Android prototypes. Keep benchmark applications,
theme settings and release profiles identical for each backend, and publish executable size,
compressed size, startup, frame time and memory measurements with rendering coverage.

## Active gap-closure goal

The remaining-gap implementation goal is active. The historical measurements above and the
recorded JSON remain the last completed benchmark, not measurements of this work in progress.

Implemented in the current working tree (coverage is being expanded and revalidated):

- GPU Gaussian blur through the system Metal Performance Shaders framework, including a general
  affine separable pass, drop-shadow and luminance-to-alpha effects.
- Layer blend, color matrix, multiply-factor and convolution effects. Matrix and convolution
  results are checked against Ferro's shared pixel references.
- Conic gradients, repeat/reflect spread, elliptical/focal radial gradients and gradient transforms.
- GPU intermediate tiles shared by image and scene brushes, including mirror modes, source/destination
  layout and brush transforms. Pixel fixtures cover image tiling and transformed clipping.
- Aliased/antialiased geometry, nearest/bilinear bitmap sampling, merged/restored rendering options,
  text antialias selection and baseline alignment.
- GPU inset/outset box shadows with spread, caster exclusion and correct background/inset/stroke order.

The Visuals catalog page now shows its Gaussian-blurred LayerVisual, with immediate and composed
screenshots under `target/quartz-prototype/goal-effects-visuals/` and zero completed GPU errors.
Additional catalog comparisons and coverage tests are still in progress.

The current work also includes GPU lighting, pixel-operation folding, acrylic material drawing,
GPU mipmap/cubic sampling, bounded command batching and texture pooling, and an optional
`runtime-shaders` WGSL-to-MSL adapter. ControlCatalog enables the shader adapter; size-first
HelloWorld does not. Font-family enumeration, variable-font axis selection, outline simulations,
contextual shaping and underline ink skipping have regression fixtures. Generic imported-image
timeline snapshots are being validated on the renderer queue.

Remaining goal work includes final catalog comparisons and media regressions for these changes,
text/rendering policy refinement, device recovery/selection robustness, and repeated HelloWorld
size measurements. Do not treat the historical size table as a benchmark of this snapshot.
Physical camera capture remains hardware-unverified because no webcam is available.
