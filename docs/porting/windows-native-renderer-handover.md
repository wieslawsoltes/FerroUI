# Windows native renderer handover: DirectWrite + Direct2D + Direct3D 11

## Assignment

Implement an **opt-in, GPU-rendered Windows backend** that uses the operating system's
DirectWrite for fonts/shaping, Direct2D for 2D rendering/effects, Direct3D 11/DXGI for GPU
resources and presentation, and Windows Imaging Component (WIC) for image codecs. The purpose
is a smaller shipped application, with no bundled Skia, HarfBuzz or ANGLE in the native build.
Keep existing Skia builds working and the default unchanged.

This is implementation work, not just an investigation. Start with HelloWorld, then reach
ControlCatalog coverage, including GPU video/camera interop. Do not stop after replacing the
text shaper or after drawing text into a CPU framebuffer. Measure both complete executables
and any additional shipped DLLs/assets before claiming a size reduction.

Branch the Windows work from `main` once this prototype has merged, for example
`render/windows-native-renderer`. The macOS prototype continues in its own branch; keep Windows
changes isolated and coordinate edits to shared feature and configuration files. No Windows
implementation is part of this document.

## Background and current branch

Ferro is a Rust port of Avalonia. The original investigation compared GPUI's platform backends
and two native-text prototypes written for Avalonia:

- [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui)
- [DirectWrite.Avalonia](https://github.com/wieslawsoltes/DirectWrite.Avalonia)
- [CoreText.Avalonia](https://github.com/wieslawsoltes/CoreText.Avalonia)

Those are references for API selection, ownership and shaping behavior, not drop-in Rust
implementations. Confirm their current source and license before reusing code. GPUI's Windows
approach is useful evidence for DirectWrite plus native Direct3D; it is not a complete
Direct2D implementation to copy. Direct2D is the proposed Ferro-specific route to broader
Avalonia-style drawing/effects coverage with system components.

The investigation started from `main` at
`97653e5036db8c25baf51fc38da08b4a468e7696`. This branch adds the macOS prototype and shared
backend-selection changes. Read [quartz-prototype.md](quartz-prototype.md) and
[quartz-prototype-results.json](quartz-prototype-results.json) for architecture and evidence.

The last completed, **historical** macOS `dist` comparison was HelloWorld at 28,282,256 bytes
with Skia/HarfBuzz and 23,466,800 bytes with native Metal/Core Text (17.03% smaller). Those are
not Windows measurements and predate current gap-closure work. Do not project that percentage
onto Windows. ControlCatalog has many assets and over 200 pages, so it is a coverage target;
HelloWorld is the primary size benchmark.

The macOS gap goal remains active. This snapshot contains GPU effects, gradients, image/scene
tiling, shadows, lighting, acrylic, pixel operations, GPU media imports, command batching and
texture reuse, plus recent shader/text/synchronization work. Some of that is still under
validation. It is experimental reference code, not a declaration of feature parity. A real
camera was unavailable on the Mac: only a GPU-generated, explicitly synthetic camera fixture
has been verified through its preview path. Windows needs its own hardware checks.

## Read these repository contracts first

Paths below are relative to the checkout root.

| Area | Starting points |
|---|---|
| Renderer and context contracts | `src/FerroUI.Base/platform/i_platform_render_interface.rs`, `i_drawing_context_impl.rs`, bitmap/geometry/glyph-run interfaces in the same directory |
| Font and shaping contracts | `src/FerroUI.Base/platform/i_font_manager_impl.rs`, `i_text_shaper_impl.rs`; `src/FerroUI.Base/media/i_platform_typeface.rs`; `media/text_formatting/text_shaper_options.rs` |
| Existing native adapter | `src/Quartz/FerroUI.Quartz/{lib.rs,fonts.rs,drawing.rs,bitmap.rs,geometry.rs,metal.rs,external_objects.rs}` |
| Native effects and regression cases | `src/Quartz/FerroUI.Quartz/{effects.rs,tiles.rs,custom_shaders.rs,effects_tests.rs,external_objects_tests.rs,tests.rs}` |
| Desktop selection | `src/FerroUI.Desktop/{Cargo.toml,app_builder_desktop_extensions.rs}` |
| Win32 options/graphics | `src/Windows/FerroUI.Win32/{Cargo.toml,win32_platform.rs,win32_platform_options.rs}` |
| Existing D3D texture sessions | `src/Windows/FerroUI.Win32/direct_x/i_direct3_d11_texture_platform_surface.rs` |
| DXGI presentation | `src/Windows/FerroUI.Win32/direct_x/{dxgi_connection.rs,dxgi_render_target.rs,dxgi_swapchain_window.rs}` |
| DirectComposition surfaces | `src/Windows/FerroUI.Win32/d_composition/` |
| Windows.UI.Composition surfaces/effects | `src/Windows/FerroUI.Win32/win_rt/composition/`, including `d2d_effects.rs` |
| Surface text settings | `src/Windows/FerroUI.Win32/win32_text_rendering.rs`; `docs/porting/text-rendering.md` |
| D3D COM bindings | `src/Windows/FerroUI.Win32/direct_x/{directx.idl,direct_x_unmanaged_methods.rs}`; `ferroui-microcom` / `microcom-codegen` |
| Windows media | `src/FerroUI.Media.Windows/{lib.rs,native.rs,native/media.cpp}`, `src/FerroUI.Media.Video.Windows/`, `src/FerroUI.Media.Camera.Windows/` |
| Samples and smoke runners | `samples/HelloWorld/{Cargo.toml,program.rs}`, `samples/ControlCatalog.Desktop/{Cargo.toml,program.rs,media_samples.rs}` |
| Baseline render implementations | `src/Skia/FerroUI.Skia/`; `src/Vello/FerroUI.Vello/gpu/` for effects and external-object contracts |
| Size methodology | `docs/binary-size.md`, `scripts/measure-quartz-prototype.py` |

The existing DXGI implementation is coupled to ANGLE/EGL. A file named DXGI or Direct3D does
**not** establish an ANGLE-free device/presentation path. Trace actual constructors and
feature discovery. The D3D texture-surface contracts already accept a graphics context and
D3D device, which is a useful integration seam; verify their implementations can serve a
native device. Extend the native seam or add a native swapchain target where required.

## Proposed design

### Crate, feature selection and device ownership

Create a Windows-only renderer crate, suggested `src/Windows/FerroUI.Direct2D` /
`ferroui-direct2d`, registered in the workspace. Implement the existing renderer/font/shaper
contracts and a builder extension such as `.use_direct2d()`. Keep window/input/accessibility
in the existing Win32 backend.

Add a desktop feature such as `direct2d` and sample feature forwarding. Include it in
`use_platform_detect()` on Windows, register the DirectWrite shaper instead of HarfBuzz,
and extend the compile-time backend-selection guard. Keep `skia` as the default. Decide and
document behavior if multiple backend features are enabled; selecting a native renderer must
not leave an unused bundled default renderer in the measured build.

Today the desktop Windows dependency explicitly enables the Win32 `angle` feature. Remove
that unconditional requirement for native builds: declare the Win32 dependency without
default features and forward ANGLE from the appropriate existing Skia feature/configuration.
Audit transitive dependencies in the catalog, headless/VNC and media crates as well. Preserve
the old Windows defaults and rendering-mode order for existing applications.

Add a native D3D11 graphics mode/device provider to Win32. Create a hardware D3D11 device
with BGRA interop support; retain the DXGI adapter identity, immediate-context ownership,
Direct2D device/context and generation together. If media needs video support or multithread
protection, reconcile those creation flags and context rules with the existing Windows media
implementation. Share the actual device/adapter with presentation and media where possible.
Do not acquire D3D11 indirectly from an ANGLE display for this backend.

Use Direct2D device contexts targeting DXGI surfaces from the D3D11 texture session. Account
for session size, offset, scaling and lifetime. Flush/end drawing before presenting or handing
a texture to another API. Support resize, occlusion/minimize, surface replacement and retained
frame contracts. Tie device-dependent caches to device generation; handle
`D2DERR_RECREATE_TARGET` and DXGI device removal/reset coherently.

Propose and verify the supported Windows/SDK baseline on the Windows machine. Query newer
DirectWrite/Direct2D interfaces and capabilities at runtime; distinguish required operations
from optional improvements. WARP may be an explicit diagnostic fallback, but it does not count
as hardware GPU verification and must not silently satisfy the GPU requirement.

### Fonts and shaping

Use system DirectWrite font discovery/fallback and custom in-memory font loaders. The loader,
font bytes, collection and font-face lifetimes must outlive all users; handle COM apartment
and render/UI thread rules explicitly. Preserve missing-family failure instead of accepting
silent substitution when Ferro expects to try the next family.

Implement text shaping using script analysis and glyph mapping/placement (for example
`IDWriteTextAnalyzer`), while preserving Ferro's formatter, run boundaries and bidi levels.
Do not introduce a second complete paragraph layout pipeline with different metrics.
Correctly handle UTF-16 clusters/surrogates, RTL, combining marks, ligatures, tabs, newlines,
letter spacing, locale, OpenType features and joining context across sliced/styled runs.
Keep cluster indices local to the supplied `ReadOnlyMemory<u16>` range.

Render supplied glyph IDs, advances and offsets with Direct2D/DirectWrite. Cover embedded
fonts, color emoji, simulations, variable font axes where exposed, glyph bounds/outlines,
underline ink skipping and scaled/rotated text. Honor the platform's rendering parameters
and Ferro's text options. ClearType is only appropriate on suitable opaque targets; use a
correct grayscale policy for transparency/effect layers. Test fallback behavior and settings
changes rather than assuming a fixed RGB LCD surface.

### Drawing, images and effects

Map the existing 2D contract to Direct2D: paths and geometry operations, fills/strokes/dashes,
transforms, rounded corners, clips, masks, opacity groups and offscreen snapshots. Use WIC
for decode/encode; decoded image uploads and explicit screenshots are distinct from ordinary
GPU rendering. Images and render-target snapshots must retain independent content/lifetimes.

Implement linear/radial/conic gradients, brush transforms/origins, pad/repeat/reflect spread,
image and scene brush layout/tiling/mirroring, sampling modes and premultiplied-alpha blending.
Do not assume the platform has a direct native brush for every operation: a small GPU shader
for a missing primitive is acceptable. Avoid importing an entire portable renderer to fill
one gap. The macOS TextBox bug was caused by ignoring a reflected gradient brush transform;
include its top-versus-bottom border regression at scaled DPI.

Reuse Ferro's shared composition/effect graph evaluation and implement its backend layer
operations. Cover blur, drop and inset/outset box shadows (spread and caster exclusion), color
matrices, convolution, blend/pixel operations, luminance-to-alpha, lighting, acrylic/backdrop
and redirected visuals. Inspect the shared renderer before duplicating graph traversal.
Use built-in D2D effects where semantics match, and small D3D/D2D custom effects where needed.

Custom runtime shader support should be an explicit design decision: the existing descriptions
carry SkSL and WGSL, not HLSL. Investigate WGSL-to-HLSL translation and compiler/runtime
availability. Keep optional compiler dependencies separate from a size-first native app;
record their size cost. The Quartz crate now has an optional `runtime-shaders` WGSL-to-MSL
adapter, and ControlCatalog enables it to exercise shader demos. Unsupported shader programs
must produce a clear diagnostic, not a false claim of rendering parity.

### GPU video/camera and external textures

Reuse the Windows media providers and shared `FrameLease`/`GpuCompletion` contracts. Inspect
`src/FerroUI.Media.Windows/native/media.cpp` for current D3D11 video-device requirements,
BGRA conversion, resource views, array slices and completion queries. Replace ANGLE-dependent
interop discovery with a native D3D feature where necessary; do not rewrite the media UI.

Import the producer's D3D texture/shared handle, validate adapter LUID, generation, dimensions,
format, color/alpha, orientation and synchronization. Keep conversion/composition on the GPU.
A GPU copy into renderer-owned storage is acceptable; CPU readback/map/staging and upload of
preview frames is not. Retain the producer lease through actual GPU completion; submission
alone does not permit decoder/camera buffer reuse. Safely retire after navigation, stop,
source replacement, cancellation and device loss. Implement and advertise only the external
handle and semaphore/keyed-mutex/fence contracts actually supported.

Verify both video and camera in ControlCatalog. If no physical camera exists, say so and
separately test an explicitly synthetic GPU source; do not label that physical capture.
Video/camera decode/import/convert/draw must add zero CPU pixel-transfer counters, while
explicit screenshot/photo/export operations may intentionally read back and must be counted.

## Suggested delivery sequence

1. Build and run the untouched Windows Skia HelloWorld and catalog; save baseline artifacts,
   runtime/backend/device details and size/dependency evidence before changing feature wiring.
2. Land the optional native crate, native D3D11 mode, DirectWrite shaper and GPU HelloWorld.
   Verify hardware device/presentation, text and resize; measure initial size.
3. Reach basic ControlCatalog drawing/text/image coverage, then brush/effect/composition parity.
   Port meaningful native regression fixtures rather than testing only API construction.
4. Implement external texture synchronization and media lifetime handling; run video/camera
   navigation and retirement tests, including failures and device invalidation.
5. Validate broader catalog pages against Skia, complete resource/device recovery tests, and
   publish final size and coverage results with commands and artifact hashes.

Keep Windows implementation changes in the new crate and Win32 paths where practical. Shared
feature files will overlap with the ongoing macOS work: inspect the latest source branch
before integration. Do not overwrite Quartz files as a shortcut to implement Windows.

## Validation and measurements

The following native feature names are **proposed**, not commands that work in this snapshot.
In PowerShell, once implemented:

```powershell
cargo build --locked --profile dist -p hello-world --no-default-features --features skia
Copy-Item target/dist/hello-world.exe target/hello-world-skia.exe
cargo build --locked --profile dist -p hello-world --no-default-features --features direct2d
Copy-Item target/dist/hello-world.exe target/hello-world-direct2d.exe
cargo tree --locked -p hello-world --no-default-features --features direct2d -e normal
cargo run --locked -p control-catalog-desktop --no-default-features --features direct2d
```

Set the native Win32 rendering mode in the sample; merely changing its feature is insufficient
if startup still requests ANGLE. Use one source revision, architecture/toolchain, `dist`
profile, panic mode, stripped-symbol policy, XAML/theme trimming and sample content for both
measurements. Preserve each executable before the other build overwrites it. If extending the
Python measurement script, remove its macOS/`otool` assumptions, support `.exe`, inspect DLL
imports, and report total deployed files as well as executable/gzip bytes. Separate runtime
shader-compiler configurations. Record SHA-256, OS/GPU/driver, commands, timings and dependency
graphs. Include Skia, HarfBuzz, ANGLE/mozangle, wgpu and Vello in the native graph audit.

Use the existing catalog smoke runner's page selection and immediate/composed screenshots.
Start with Home, TextBlock, TextBox, Image, TabbedPage, then Visuals, Composition brushes,
Shadows and lights, Effect brushes, acrylic/blend effects and custom shader demos. Check
actual page header names in the current catalog. Test 100%, fractional and 200% DPI, light/dark
themes, opacity/clip/transforms, resize/minimize and multi-monitor changes where available.
Store side-by-side evidence and explain meaningful differences; screenshots alone do not
prove GPU residency or correct lifetime/synchronization.

Use CPU-reference comparisons already in Ferro for color matrices, convolution, lighting,
pixel operations and shader effects. Add tests for RTL/context, font fallback, embedded fonts,
color glyphs, gradient reflection, sampling, clipping/effect bounds, snapshot independence,
lease retirement and resource reuse. Check the D3D debug layer for hazards/leaks on test runs;
do not ship its overhead in size benchmarks. Confirm the old Skia backend still runs and
non-Windows builds keep working. Cross-compilation checks cannot replace Windows runtime tests.

Completion means: a usable opt-in hardware-GPU native renderer; reproducible Windows
before/after sizes; broad, documented catalog coverage; zero CPU media-frame transfers;
clean dependency separation; and an honest list of remaining platform/hardware limitations.
Do not mark unsupported operations or untested devices as implemented.

## Primary API references

- [Microsoft: Direct2D devices and device contexts](https://learn.microsoft.com/en-us/windows/win32/direct2d/devices-and-device-contexts)
  describes creating the D3D11/DXGI/Direct2D device chain with BGRA interop support.
- [Microsoft: Direct2D device-context quickstart](https://learn.microsoft.com/en-us/windows/win32/direct2d/direct2d-quickstart-with-device-context)
  shows targeting DXGI surfaces.
- [Microsoft: IDWriteTextAnalyzer::GetGlyphs](https://learn.microsoft.com/en-us/windows/win32/api/dwrite/nf-dwrite-idwritetextanalyzer-getglyphs)
  documents text-to-glyph mapping and cluster/shaping outputs. Check its companion placement
  and script-analysis APIs for the full shaping contract.
- [Microsoft: glyphs and glyph runs](https://github.com/MicrosoftDocs/win32/blob/docs/desktop-src/DirectWrite/glyphs-and-glyph-runs.md)
  covers custom glyph rendering and native outlines.

Check installed Windows SDK headers and current Microsoft documentation for exact interface
versions, OS requirements and synchronization semantics before implementation.

## Handover snapshot checks

The completed macOS gap implementation passes 42 Metal tests, 12 CPU-reference tests,
10 shared context-manager tests and 13 native Metal-device tests. Fifteen original Skia and
Quartz Metal catalog pages were compared using both drawing paths. Video and synthetic
camera presentation pass without CPU media-frame transfers; physical camera capture is
unverified because no webcam is available. Current HelloWorld saves 16.66% with native GPU
rendering, or 11.63% with the optional runtime WGSL compiler enabled. See
[the macOS report](quartz-prototype.md) and [recorded results](quartz-prototype-results.json)
for exact feature graphs, measurements, evidence and hardware limits. These checks do not
establish Windows compatibility; Windows needs its own runtime and size validation.


## Latest macOS contract audit

The follow-up [Quartz/Skia audit](quartz-skia-parity.md) and
[recorded results](quartz-parity-results.json) supersede the earlier macOS size checkpoint
above. The audit adds native writable pixel/alpha formats, exact lock behavior, PNG settings,
geometry edge cases, isolated clip coverage, bitmap-only blend state and ranged OpenType
features. It also catches the extra coordinate flip that inverted standalone hinted and color
glyph images. These are useful DirectWrite/Direct2D acceptance cases; preserve native text
quality instead of matching Skia's text pixels.

The backend-neutral **Renderer parity** catalog page exercises geometry, writable inputs, mixed
scripts, RTL selection, replacement, undo and text hit testing. Run its **Run text checks**
action on Windows, or set `FERROUI_SMOKE_RENDERER_PARITY=1` for the catalog smoke runner. The
macOS evidence is 58 Metal tests, 24 CPU tests and 16 pages captured through both drawing
paths on each backend; it does not establish Windows compatibility. Physical webcam capture
still needs hardware.
