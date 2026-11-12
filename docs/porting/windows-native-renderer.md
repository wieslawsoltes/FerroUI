# Windows native renderer

`ferroui-direct2d` (`src/Windows/FerroUI.Direct2D`) is an opt-in render backend for Windows that
draws with the graphics stack of the system: Direct2D on a hardware Direct3D 11 device, DirectWrite
for fonts and shaping, the Windows Imaging Component (WIC) for image files, and a DXGI swap chain
shown through DirectComposition. A build that selects it contains no Skia, no HarfBuzz and no ANGLE.
Skia remains the default renderer of every desktop build; nothing changes for an application that
does not ask for this backend.

Choose it when:

- the size of the executable matters: the renderer, the shaper and the image codecs come from
  Windows and are not linked into the program (see
  [Measure binary size](#measure-binary-size));
- text should be shaped and rasterized by DirectWrite, with the font fallback of the system;
- video or camera frames should stay on the Direct3D 11 device that draws the window.

Keep Skia when:

- the application has to start without a hardware GPU (the native renderer refuses the software
  rasterizer and has no CPU fallback);
- it needs subpixel (ClearType) text, the blur, acrylic or Mica window backdrops, or rendering that
  matches the other platforms pixel for pixel;
- it uses custom shader effects and cannot take the optional runtime shader compiler
  (see [Effects and custom shaders](#effects-and-custom-shaders)).

Measurements against Skia on ANGLE are in
[windows-native-renderer-benchmarks.md](windows-native-renderer-benchmarks.md).

## Requirements

To build:

- A Windows target with the MSVC C++ toolchain. The backend has a C++17 translation unit that the
  `cc` crate compiles.
- The Windows SDK with the shader compiler `fxc.exe`. The build script takes the newest
  `Windows Kits\10\bin\<version>\x64\fxc.exe` under the 32-bit program files directory. Set the
  environment variable `FXC` to the full path of the compiler when the SDK is somewhere else.
  Without a compiler the build stops with a message that names `FXC`.

To run:

- Windows 10 version 1709 or later.
- A hardware Direct3D 11 device of feature level 11_0 or 11_1. The built-in effect kernels are
  Shader Model 5 compute shaders. The device is created with the hardware driver type only: WARP
  never satisfies the request, so a machine or a session without a hardware adapter cannot start
  the backend.

`use_direct2d()` checks both at startup and stops the program with a message that states the
requirement when the device cannot be created.

## Build and run

The samples take the backend with their feature `direct2d`. The default features select Skia, so
they have to be turned off: enabling `skia` and `direct2d` together on Windows is a compile error.

```powershell
cargo run -p hello-world --no-default-features --features direct2d
cargo run -p control-catalog-desktop --no-default-features --features direct2d
```

| Package | Feature | Adds |
| --- | --- | --- |
| `ferroui-desktop` | `direct2d` | The backend, and the Win32 windowing backend in its Direct3D 11 mode (`ferroui-win32/native-d3d11`) without ANGLE |
| `ferroui-desktop` | `direct2d-runtime-shaders` | `direct2d` and the runtime shader compiler (`ferroui-direct2d/runtime-shaders`) |
| `hello-world` | `direct2d` | The backend through `ferroui-desktop` |
| `hello-world` | `direct2d-runtime-shaders` | `direct2d` and the runtime shader compiler |
| `control-catalog-desktop` | `direct2d` | The backend, and video and camera frames on its Direct3D 11 device |
| `control-catalog-desktop` | `direct2d-runtime-shaders` | `direct2d` and the runtime shader compiler, for the pages with custom shader effects |
| `control-catalog-desktop` | `direct2d-media-test-fixtures` | `direct2d` and the synthetic camera of the media smoke run |
| `ferroui-direct2d` | `runtime-shaders` | The WGSL to HLSL translation of custom shader effects |
| `ferroui-direct2d` | `media-controls` | `D3D11TextureImporter`, which the media controls use to draw Direct3D 11 frames |

In an application, turn off the default features of `ferroui-desktop` and enable `direct2d`:

```powershell
cargo add ferroui-desktop --no-default-features --features direct2d
```

`use_platform_detect()` then loads the Win32 windowing backend and registers the renderer and the
text shaper; no other code changes. An application that uses custom shader effects enables
`direct2d-runtime-shaders` instead of `direct2d`.

An application that builds its `AppBuilder` by hand calls
`Direct2DApplicationExtensions::use_direct2d()`, which registers the rendering subsystem `Direct2D`
and the text shaping subsystem `DirectWrite`. The windows must then come from `ferroui-win32` with
its feature `native-d3d11`, whose default platform options select the rendering mode
`Win32RenderingMode::Direct3D11`: in that mode the windowing backend creates no EGL or OpenGL
context and leaves the device and the swap chain to the renderer.

## Architecture

The backend has two layers.

`native.cpp` holds everything that talks to COM: the factories, the Direct3D device, drawing,
text, images, effects, the swap chain and the imported textures. It exports a C ABI (functions
named `d2_*`) and is compiled by `build.rs` together with the header that `fxc.exe` generates from
`effects.hlsl`. One recursive mutex serializes every entry point, so the Rust side may call from
the UI thread and from the render thread.

The Rust modules implement the rendering interfaces of the framework on top of that ABI:

| Module | Role |
| --- | --- |
| `lib.rs` | `Direct2DRenderInterface` (the platform render interface), `use_direct2d()`, the device accessors and the statistics |
| `presentation.rs` | The render interface context, window render targets and offscreen targets |
| `drawing.rs` | The drawing context: brushes, pens, clips, opacity, layers, blend modes, glyph runs |
| `effects.rs`, `tiles.rs` | Effects, layer effects, box shadows, the acrylic material and tiled brushes on the GPU |
| `custom_shaders.rs` | Custom shader effects (feature `runtime-shaders`) |
| `bitmap.rs`, `writeable_bitmap.rs` | GPU bitmaps, decoding and encoding, writable bitmaps |
| `geometry.rs`, `path_measure.rs`, `region.rs` | Geometries, path measurement and regions |
| `fonts.rs` | `Direct2DFontManager` and `DirectWriteTextShaper` |
| `external_objects.rs` | Import of shared Direct3D 11 textures for video, camera and other producers |
| `ffi.rs` | The declarations of the C ABI |

The drawing context does not cross the boundary once per primitive. It records draw commands and
passes them to the C++ layer in batches, at the end of a drawing session or when 4,096 commands
have collected.

### Device

The engine owns one Direct3D 11 device for the process, created with BGRA and video support and
with multithread protection on, and one Direct2D device on top of it. Every surface is a
premultiplied BGRA texture drawn through a Direct2D device context in pixel units. The render
interface reports premultiplied alpha and `BGRA8888` as its default formats.

The caches of the device are bounded:

| Cache | Bound |
| --- | --- |
| Pooled frame textures | 16 textures and 64 MiB |
| Glyph atlas | 256 x 256 pages, 64 pages (16 MiB), least recently used first out |
| Box shadows | 512 entries and 32 MiB |
| Shaping results, per font | 2,048 entries and 1 MiB; runs of up to 256 UTF-16 units with up to 16 features |
| Glyph metrics, per font | 4,096 entries |

### Presentation

A window render target needs the `HWND` of the window. In the Direct3D 11 mode the windowing
backend creates the window without a redirection bitmap, and the renderer attaches a
DirectComposition target to it. The swap chain is a composition swap chain with two BGRA buffers,
the flip-sequential model and premultiplied alpha, so a transparent window is composed by the
system without a copy through GDI. It is created on the first present and resized when the client
size changes.

Each window keeps one retained root texture. A frame is drawn into it, copied to the back buffer
on the GPU and presented with a sync interval of zero; the compositor of the framework paces the
frames. The retained texture lets a frame redraw the dirty regions only. A present is skipped
while the system reports the window as occluded.

Offscreen render targets are textures of up to 16,384 pixels on a side.

### Device loss and recovery

The engine counts device generations. When Direct3D reports the device as removed, the engine
creates a new device before it releases the old one, clears the glyph atlas, the box shadow cache,
the frame pool and the compiled shader objects, and increments the generation. While no hardware
adapter is available the old state stays in place and the next check tries again.

- A window target whose generation is stale reports itself as corrupted, and the compositor
  recreates it.
- Bitmaps check their generation at the start of a drawing session. A bitmap that was decoded from
  a file or created from pixels is recreated from those bytes.
- Compiled custom shader pipelines are cached per generation and rebuilt on demand.
- A leased video or camera texture of an earlier generation is refused; the producer supplies a
  new frame on the new device.

`ferroui_direct2d::device_generation()` returns the current generation to code that shares the
device.

### Text

`Direct2DFontManager` enumerates the system font collection, matches typefaces by family, weight,
style and stretch, loads fonts from streams through an in-memory font loader, applies variable
font axes, and asks DirectWrite for the fallback font of a code point. The default family is
Segoe UI.

`DirectWriteTextShaper` shapes with the DirectWrite text analyzer: script analysis, glyph mapping
with the OpenType features and their ranges, then glyph placement. The shaper passes the text of
the whole paragraph together with the range of the run, so that context at the edges of a run is
available. A run is shaped in windows of at most 65,535 UTF-16 units. A window that the analyzer
rejects is halved, and a window that does not end the run is cut back to a position where the
analyzer allows shaping to break, so that the remainder is shaped with the next window.

Glyph runs are drawn by Direct2D with grayscale antialiasing, or aliased when the text rendering
mode asks for it. Small opaque runs without colour glyphs that repeat from frame to frame are
served from the glyph atlas.

### Images

WIC decodes image files and streams into premultiplied BGRA textures and encodes bitmaps as PNG
or JPEG. Bitmaps created from pixels and writable bitmaps accept `BGRA8888`, `RGBA8888` and
`RGB565`; other formats are refused. A bitmap created from pixels is converted to premultiplied
BGRA once. A writable bitmap keeps its pixels in CPU memory in the declared format and uploads
them to its texture only when a lock has changed them. Snapshots of a surface are copies on the
GPU.

### Effects and custom shaders

No effect reads pixels back to the CPU. The backend implements the effect interfaces of the
drawing context with the built-in effects of Direct2D (blur, colour matrix, composite, blend,
convolution, arithmetic composite, table transfer, affine transform) and with one compute shader,
`effects.hlsl`, whose kernels cover what Direct2D has no effect for: multiply, convolution, lighting
and pixel operation layers, box shadows and conic gradients. `build.rs` compiles that shader, so a
normal build needs no shader compiler at run time.

Covered are blur and drop shadow effects, luminance to alpha, blend, colour matrix, multiply,
lighting, pixel operation and convolution layers, outer and inset box shadows (cached per shape and
parameters) and the acrylic material.

Custom shader effects carry WGSL source. They need the feature `runtime-shaders`, which adds the
`naga` crate to translate WGSL to HLSL (Shader Model 5) and compiles the result with
`d3dcompiler_47.dll` from the system directory; the program ships no copy of that library. The
uniform block has to be a global named `uniforms` whose members match the uniform layout of the
effect, and the entry points are `vs_main` and `fs_main`. Pipelines are cached per effect type. A
shader that fails to translate or compile is reported once on standard error and its content is
drawn unchanged.

Without `runtime-shaders` a custom shader effect is reported once as unhandled and its content is
drawn without the effect.

A layer pushed on a Direct2D drawing context is a texture of the size of the whole target, so the
graph of a composition effect brush (a backdrop material, a mask, a blur under a small element)
would cost window-sized textures for every node. The drawing context therefore answers `true` to
`IDrawingContextImpl::prefers_bounded_effect_targets()`: the compositor evaluates such a graph in
scratch targets of the size of the brush and draws the result from there. The other backends
answer `false`, the default, and draw the graph in place as before.

### Media and external textures

The render interface context exposes the external objects feature of the compositor for Direct3D 11
textures: shared NT handles, legacy global shared handles, and textures of the renderer's own
device handed over as a lease. Access is synchronized with a keyed mutex or, where the device
supports it, with a fence. An imported frame must come from the adapter of the renderer and belong
to the current device generation, may be `RGBA8` or `BGRA8` up to 8,192 pixels on a side, and must
not have straight alpha. Colour conversion and flipping happen on the GPU, and the lease of a frame
ends when a GPU event query confirms that the conversion has completed.

`ferroui-media-windows` has a matching feature `direct2d`:
`WindowsFrameProducer::from_direct2d` makes Media Foundation decode onto the device of the
renderer (`ferroui_direct2d::native_device()`), so video and camera frames reach the screen
without a copy through system memory. The control catalog uses it for its video and camera pages
when built with `direct2d`.

## Environment variables

The diagnostic variables are read by presence; any value turns them on.

| Variable | Read by | Effect |
| --- | --- | --- |
| `FXC` | `build.rs` | Full path of `fxc.exe`, when the Windows SDK is not in its default location |
| `FERROUI_D3D_STATS` | the engine, HelloWorld, the control catalog | The engine names the adapter and the feature level on standard error. The samples print the renderer statistics when they exit, and the catalog media smoke run prints `CATALOG_GPU_CHECKPOINT` lines |
| `FERROUI_D3D_PROFILE` | the engine | Names the adapter, collects CPU timings, and prints a frame profile per window every 60 frames and a line for every submission above 16 ms. Read once when the process starts |
| `FERROUI_D3D_DEBUG` | the engine | Names the adapter and creates the Direct3D device with the debug layer and the Direct2D factory with debug information. The debug layer of the Windows SDK has to be installed |
| `FERROUI_D3D_SHADER_DUMP` | `custom_shaders.rs` | A directory that receives the translated HLSL of each custom shader effect as `<effect name>.hlsl` (feature `runtime-shaders`) |

The statistics are twelve counters, in this order: surface allocations, path draws, image draws,
presents, explicit pixel readbacks, image uploads, texture imports, GPU copies, errors, completed
media reads, compute dispatches, compute pixels. `ferroui_direct2d::statistics()` returns them.
Every error of the C++ layer is also printed on standard error with the prefix `Direct2D:`.

```powershell
$env:FERROUI_D3D_STATS = '1'
$env:FERROUI_SMOKE_EXIT_MS = '3000'
cargo run -p hello-world --no-default-features --features direct2d
```

The timings of `FERROUI_D3D_PROFILE` are CPU times of calls into the system. They can nest and are
not GPU timestamps, and collecting them costs time: leave the variable unset when measuring.

The media smoke run of the control catalog (`FERROUI_SMOKE_MEDIA=1` and its companions, listed in
`samples/ControlCatalog.Desktop/media_samples.rs`) has two additions for this backend:

| Variable | Effect |
| --- | --- |
| `FERROUI_SMOKE_MEDIA_VIDEO_MS` | Observes video delivery for that many milliseconds (at most 30,000) after the warm-up and prints `CATALOG_VIDEO_DELIVERY` with the frames, the time and the rate |
| `FERROUI_SMOKE_MEDIA_SYNTHETIC_CAMERA` | With the value `1` and the feature `direct2d-media-test-fixtures`, the camera page uses a synthetic camera on the device of the renderer |

`FERROUI_SMOKE_RENDERING` and `FERROUI_SMOKE_COMPOSITION` select rendering and composition modes
of the Skia build; leave them unset in a native build.

## Feature coverage and differences from Skia

The backend implements the drawing context of the framework on the GPU: solid, gradient
(linear, radial and conic), image and scene brushes, pens with dashes, caps and joins, geometry and
rectangular clips, opacity and opacity masks, layers, the blend modes of the framework, regions,
individual rounded rectangles, compositing layers, bitmaps, render target bitmaps, writable
bitmaps, glyph runs, effects and the external textures above.

It differs from the Skia backend in these points:

- **No software rendering.** There is neither a WARP nor a CPU path. The default Win32 platform
  options of a native build contain the Direct3D 11 mode alone.
- **Text.** Shaping and rasterization are DirectWrite's, not HarfBuzz's and Skia's, so glyph
  positions and coverage can differ by fractions of a pixel. Text is antialiased in grayscale or
  aliased; there is no subpixel antialiasing.
- **Windows are always composed with alpha.** The window has no redirection bitmap and the swap
  chain is premultiplied. The windowing backend reports the transparency level `Transparent` as
  supported and `None`, `Blur`, `AcrylicBlur` and `Mica` as unsupported in this mode.
- **Custom shader effects** need the feature `runtime-shaders`; otherwise their content is drawn
  unchanged.
- **Pixel formats.** Bitmaps accept `BGRA8888`, `RGBA8888` and `RGB565` only. Creating a bitmap
  from pixels of another format, or from a buffer that is too small for its stride and size, is a
  programming error and stops the program with a message.
- **Bitmap blending modes.** `Source` and `Plus` are blends of the bitmap draw itself; the other
  modes are composed through a layer that holds the bitmap. The modes that change the destination
  outside the bitmap (`SourceIn`, `SourceOut`, `DestinationIn`, `DestinationAtop`) end at the
  axis-aligned bounds of the destination rectangle, which is exact without rotation or skew only.
- **Text with a gradient or an image brush** is filled through the outlines of the glyphs, not
  through the glyph atlas.
- **Typeface attributes.** A typeface of an embedded font reports the weight, style and stretch
  its font declares; a typeface of a system font reports the values it was asked for. The bold
  and oblique simulations of a nearest match are not applied.
- **Encoding.** The JPEG quality is honoured; the compression level of a PNG is not.
- **Not implemented.** 3D scenes (`Viewport3D`), which are drawn by the Skia and Vello backends
  only, and the optional drawing context features that have a default in the platform interface.
  The effect graph form of a custom layer effect is drawn without the effect.
- **Objects of another backend.** A bitmap or a geometry that another backend created cannot be
  drawn, and a brush of an unknown kind is skipped, as is an opacity mask without a brush the
  backend can resolve. Each such case is reported once on standard error.

## Tests

The tests need a Windows machine with a hardware GPU. They create the real device and compare
pixels read back from GPU surfaces.

```powershell
cargo test -p ferroui-direct2d
cargo test -p ferroui-direct2d --features runtime-shaders
cargo test -p ferroui-direct2d -- --ignored reveal_lighting_dispatch_work_is_bounded_by_the_control
cargo test -p ferroui-desktop --no-default-features --features direct2d
cargo test -p ferroui-media-windows --features direct2d
```

The first command runs the drawing, text, bitmap and effect tests. The second adds the test that
translates and compiles every shader program of the effects library. The third runs the one test
that is ignored by default because it measures the amount of compute work. The fourth checks that
platform detection selects Direct2D and DirectWrite without HarfBuzz, and the fifth checks the
Direct3D 11 frame producer of the media backend.

`native_tests.cpp` in the crate directory is a separate harness for the C++ layer. It includes
`native.cpp`, has its own `main` and runs the engine under the Direct3D debug layer. Cargo does
not build it: compile it with the MSVC compiler, with the generated `effects_shader.h` on the
include path and the libraries that `build.rs` links.

For a check in a real window, run a sample with `FERROUI_D3D_STATS` set and read the statistics
line it prints on exit: the error counter is the ninth value and the readback counter the fifth,
and both stay at zero in a normal run.

```powershell
$env:FERROUI_D3D_STATS = '1'
$env:FERROUI_SMOKE_EXIT_MS = '20000'
$env:FERROUI_SMOKE_PAGES = '150'
cargo run -p control-catalog-desktop --no-default-features --features direct2d-runtime-shaders
```

## Measure binary size

`scripts/measure-windows-native.ps1` builds HelloWorld with the Skia backend and with the native
backend in the `dist` profile from one clean, committed revision, and writes the sizes to
`target/windows-native-size/results.json`. It needs PowerShell 7.2 or later and `dumpbin.exe` of
the MSVC tools.

```powershell
./scripts/measure-windows-native.ps1 -IncludeRuntimeShaders
```

For each build the script records the size of the executable and of its gzip compression, checks
with `cargo tree` that a native build depends on none of the bundled renderers, and checks with
`dumpbin` that the executable imports system libraries only. The results are in
[windows-native-renderer-benchmarks.md](windows-native-renderer-benchmarks.md#binary-size).
