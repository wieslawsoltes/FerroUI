# ProGPU render backend (#369)

A render backend of FerroUI on the native C++ engine of ProGPU
([wieslawsoltes/ProGPU](https://github.com/wieslawsoltes/ProGPU), MIT, pinned at
`v0.1.0-preview.70`): its WebGPU renderer with all its WGSL shaders (vector, path and glyph
rasterization, images, layers, masks, blend modes, effects), its text stack (fonts and the system
font catalog, fallback, Unicode bidi and line breaking, the OpenType shaper) and its retained
scene model, on the desktop (Metal, Direct3D 12, Vulkan) and in the browser on WebGPU. The goal is
a retained GPU mode as fast as the engine allows, measured against the Skia and Vello backends.

Status: plan (P0). The owner's request of 2026-11-05: top priority; the PR is open from the start;
performance and published size are measured locally against Skia (no new workflows), with
screenshots of the ControlCatalog.

## 1. What ProGPU provides

| Part | Where (ProGPU) | Form |
|---|---|---|
| Engine: device, pipelines, batching, submission | `src/ProGPU.Native/src/Backend` | C ABI `progpu_native.h` (ABI 5) |
| Retained scene: a pointer-free, versioned byte stream (header, resources, commands, arena) | `src/ProGPU.Native/src/Scene` | the stream format of `progpu_native.h`; `progpu_native_engine_update_scene`, `progpu_native_engine_render_scene` |
| Scene builder that writes the stream | `src/Scene/Builder` | C++ only (`progpu::native::semantic_scene_builder`, `progpu_native_scene_builder.hpp`) |
| Text: fonts, outlines, variations, color glyphs, bidi (UAX 9), line breaks (UAX 14), shaping (GSUB/GPOS, script shapers; checked against the HarfBuzz corpus), layout | `src/Text` | C ABI for contexts, shaping, bidi, breaks, layout, outline capture; C++ only for the system font catalog and fallback itemization |
| Device ABIs | | wgpu-native (May 2024), Dawn (WebScene provider), browser (Emdawnwebgpu) |
| Prebuilt SDK | NuGet `ProGPU.Backend.Native` and `ProGPU.Native.Sdk` (`0.1.0-preview.70`) | headers, static libraries, CMake config, runtimes for win/osx/linux x64/arm64 |

The engine retains the scene, not the caller's objects: the caller sends a stream when its
generation changes and renders it every frame. A stable frame copies nothing, rasterizes nothing and
uploads nothing; it replays recorded render bundles into the target pass. Unchanged draw families
keep their GPU pages across a new stream (content hashes per family). Layers with
`LAYER_CACHE_CONTENT` keep their pixels while their content revision holds, so a change of opacity
or clip does not draw the subtree again; `LAYER_CACHE_LOCAL_SPACE` places a cached layer with a
separate state record. `DAMAGE_RECT` with `PRESERVE_TARGET` limits a frame to a region.

## 2. Design in FerroUI

### 2.1 The crates

- `ferroui-progpu` (`src/ProGPU/FerroUI.ProGPU`): the backend, beside `ferroui-skia` and
  `ferroui-vello`, with the same contracts: `IPlatformRenderInterface`, the drawing context with
  fragments (`IDrawingContextImplWithFragments`), geometries, bitmaps, render targets for windows and
  offscreen, `IFontManagerImpl`, `ITextShaperImpl`, `IGlyphTypeface`, and the
  `use_progpu()` application extension.
- The Rust bindings of the engine live in the ProGPU repository (section 3). ProGPU is a git
  submodule of FerroUI (`third_party/ProGPU`, owner's decision of 2026-11-05, outside the workspace
  and the paths of the upstream-name check), and the workspace takes `progpu-native-sys` from it by
  path: its build script builds the engine from the submodule's sources with CMake (37 seconds on
  six cores, incremental after that), so FerroUI runs the engine of the pinned ProGPU commit,
  including engine changes not yet in a ProGPU package, and FerroUI does not keep a second copy of
  the C ABI. Every checkout of the FerroUI workflows takes the submodule (`submodules: true`, not
  recursive: ProGPU's own submodules are never fetched). The published crate of the bindings falls
  back to the pinned SDK packages.

### 2.2 The device and the surfaces

ProGPU takes its device from a C WebGPU library of a matching ABI: the `wgpu` crate of the Vello
backend cannot be handed over. Headless rendering and offscreen targets use the wgpu-native runtime
(the pinned May-2024 ABI), which the engine's source build links.

Window surfaces must keep FerroUI's cooperative resize (no stretched or stale frame while a window
is resized): the platform presents its own frames, inside the window's Core Animation transaction on
macOS, and the backend draws into them. wgpu-native's C ABI can neither import a platform texture
nor present inside a transaction (its Metal presentation option is internal to wgpu-hal), so the
window path is the engine's **Dawn target**, as in ProGPU's own macOS host
(`DawnMetalRenderTarget`): each frame's `CAMetalDrawable` texture is IOSurface-backed and is
imported once per swapchain IOSurface as Dawn shared texture memory; the engine renders into it
between `BeginAccess` and `EndAccess` (synchronized with a Metal shared event), and the platform's
render session presents it. The native Metal surface of FerroUI (`ferroui-native`, `metal.rs`:
`IMetalPlatformSurface`, drawables of its own `CAMetalLayer`) already has that shape. Windows uses
the same pattern with Dawn's D3D12 shared textures and the platform's swap chain, Linux Dawn's
Vulkan path (step P8). The Dawn runtime is the provider package ProGPU's Dawn backend resolves
(`progpu_native_dawn_engine_create` with a host resolver); its size is measured with the others.
### 2.3 Retained GPU mode

The compositor of FerroUI already keeps a backend fragment of the content of every draw-list visual
and draws it again while the content does not change (`IDrawingFragment`). The backend makes the
fragment the encoded records of the visual (its commands, its resources by stable id) in the stream
format, so a frame is put together without drawing the content again:

1. **Per-visual fragments.** A visual whose render data did not change contributes its fragment as
   it is: a copy of its encoded records, no tessellation, no shaping, no allocation.
2. **Transforms in state records.** The transform, opacity and clip of a visual are its state record
   (`progpu_native_scene_state`), not baked into its draws, so moving, scrolling or fading a visual
   changes one record.
3. **Generations.** The frame stream gets a new generation only when a fragment or a state changed;
   a frame without changes calls `progpu_native_engine_render_scene` only (render bundles replayed).
4. **Cached layers.** Visuals with a cache mode, and subtrees that change rarely under changing
   parents, are pushed as `LAYER_CACHE_CONTENT` layers with the revision of their content, placed in
   local space; scrolling a list moves cached layers instead of drawing their rows again.
5. **Damage.** The dirty rects of the compositor become `DAMAGE_RECT` with `PRESERVE_TARGET`.
6. **Text.** Glyph outlines are resources by typeface and glyph id, rasterized on the GPU into the
   engine's atlas once; a glyph run is a positioned-glyph record.

What the engine does not do yet, and section 3 adds: replacing one resource or one state of a page
without sending the whole stream (today every stream change re-records the full-scene render
bundle, and a family page with several resources is uploaded again as a whole).

### 2.4 Text

`IFontManagerImpl` on ProGPU's system font catalog (families, styles, matching, character fallback)
and font loading from bytes (embedded fonts, `ferres://` assets); `ITextShaperImpl` on the text
context of each typeface with its fallback faces, the shaping request of ProGPU (scripts, language,
direction, features, variations), and its bidi and line breaking where the text layout of FerroUI
asks the platform. Glyph metrics and outlines come from the same faces, so layout and rendering use
one font stack.

### 2.5 Geometries and bitmaps

The backend uses ProGPU's own native code and shares nothing with the Vello or Skia backends.
ProGPU's WinUI and Avalonia backends draw with its managed stack; this backend is the first UI
backend on the native engine, so the parts the managed stack has and the engine lacks are added to
the engine (decided by the owner, 2026-11-05):

- **A native Geometry module** in `src/ProGPU.Native` (`src/Geometry`): the path, Boolean and stroke
  code the engine's geometry entry points use today, moved out of the Direct2D compatibility layer
  into a neutral module the Direct2D layer then uses too; with path measures (contour length, the
  point and tangent at a distance, segments) and the widening of a pen into its outline (caps,
  joins, miter limit, dashes), ported from ProGPU's managed `PathMeasure` and
  `StrokePathGeometry`. Its C ABI answers every query of `IGeometryImpl`: bounds and stroke bounds,
  length, point and tangent at a distance, segment, fill and stroke containment, intersection and
  Boolean combination, widening.
- A geometry of the backend is its path in the engine's segment records
  (`progpu_native_path_segment`) with its fill rule, so drawing it copies the records into the
  scene, and a combined geometry draws through the scene's Boolean nodes.

Bitmaps decode with ProGPU's native image module: PNG today, and **JPEG (baseline and
progressive), GIF and BMP decoders** added beside it, bounded and allocation-checked like the PNG
decoder, with a C ABI (decided by the owner, 2026-11-05). The backend links no other decoder. The
scene holds a bitmap as an image resource by stable id; a `WriteableBitmap` updates its resource in
place (`update_rgba8_image`, same id, a new generation), so a changed bitmap uploads its pixels and
nothing else.

### 2.6 Browser

A new rendering mode `WebGPU` of `ferroui-browser` (FerroUI's browser target is
`wasm32-unknown-emscripten`): the ProGPU engine compiled with Emscripten 4.0.18 and Emdawnwebgpu,
the device requested by the page (`navigator.gpu`, high-performance adapter) before the module
starts, the surface of the canvas of the view, and the same retained mode as the desktop. It sits
beside the WebGL2 and 2D modes, which stay.

## 3. Bindings in the ProGPU repository

The C++ builder never crosses the C ABI, and a call across the boundary for every command would
cost more than the work of most commands. The fastest Rust integration keeps the boundary to a few
calls a frame:

1. **`progpu-native-sys`**: the C ABI (`progpu_native*.h`) as Rust declarations, generated with
   `bindgen` and checked in, with a test that the layouts (sizes, offsets) match the headers, and a
   `build.rs` that builds the engine from the ProGPU checkout it is part of (the submodule), or,
   for the published crate, takes the SDK packages (static libraries and the runtime) of the same
   version.
2. **`progpu-native`**: a safe crate with
   - a **Rust stream writer**, the counterpart of `semantic_scene_builder`: it writes the
     pointer-free records directly into a reused buffer (no FFI per command, no allocation in a
     stable frame); a differential test builds the same scenes with the C++ builder and the Rust
     writer and requires identical bytes, and the engine's validator checks every stream in tests;
   - the engine, the external-image bindings, submissions and device loss as owned types;
   - the text contexts, shaping, bidi, breaks and layout over caller-owned buffers (the C ABI is
     already allocation-free).
3. **C ABI additions** to the engine:
   - the system font catalog and fallback itemization (C++ only today);
   - a **delta update**: replace resources and state records by id in a retained scene without
     sending the stream again, keeping the render bundles of the unchanged spans (the
     "family-granular retained update" the baseline lists as not implemented);
   - render bundles per span or per cached layer instead of one for the scene, so a change of one
     visual records one span again;
   - surface helpers for the native handles (creating and configuring a WebGPU surface for a
     `CAMetalLayer`, an `HWND`, an X11 window, a Wayland surface);
   - the C ABI of the native Geometry module (section 2.5).
4. **Image decoders** in the native image module beside PNG: JPEG (baseline and progressive), GIF and
   BMP, bounded and allocation-checked as the PNG decoder.

These land in ProGPU as their own pull requests, each with the tests of the engine; the FerroUI
backend starts on the existing C ABI with a small C++ shim over the builder (step P2) and moves to
the Rust writer as it lands, keeping the shim as the differential reference.

### 3.1 The WebGPU implementation

The engine takes a device of a C WebGPU implementation of its pinned ABI. The `wgpu` crate of the
Vello backend exposes no `webgpu.h` handles, so it cannot be handed over; the two that fit are:

- **Dawn** (the implementation of Chrome): the Dawn build of the engine links no WebGPU library and
  resolves every procedure through the resolver the host gives
  (`progpu_native_dawn_engine_create`), so FerroUI supplies the Dawn of the pinned revision
  (Dawn `710c330`, `webgpu-headers` `01addc4b`). It is the same API family as the browser path and
  has the most mature Direct3D 12, Metal and Vulkan backends; it is the target of the desktop
  backend.
- **wgpu-native** (May-2024 ABI): the default build of the engine is linked against it; the
  runtime is the pinned package (`Silk.NET.WebGPU.Native.WGPU` 2.23.0), or built from the pinned
  commit. The first steps run on it, and it stays a supported implementation.

Both are measured (section 4) for speed, features and size before the default is chosen per
platform. In the browser the implementation is the browser's (`navigator.gpu`); nothing is shipped.

### 3.2 Binary size

Measured on macOS arm64 (preview.70): the engine's runtime library is 38.9 MB stripped (15.4 MB
gzipped), of which 33.1 MB is one symbol, `progpu_edit_icu_data`, the complete ICU data set that the
engine links for the word boundaries of text editing; the code is 3.4 MB, the Unicode normalization
data 0.45 MB, the WGSL shaders about 0.6 MB. wgpu-native is 4.1 MB stripped.

The size is reduced by correct construction, never by turning a feature off or trimming data by
hand; every change keeps the behaviour and its tests:

1. **ICU data built for what the engine uses.** ICU's data filter (`ICU_DATA_FILTER_FILE`, the
   supported build input) generates the data set of the services the engine calls: the word-break
   rules and their dictionaries. The engine's ICU build at its pin produces it, and the engine's
   edit-word-boundary tests are its acceptance. Beside it, the boundary service becomes a provider
   of the C ABI, so a host that already ships a Unicode segmenter (FerroUI's ICU4X segmenter)
   provides the boundaries and the application carries one segmentation implementation, not two;
   the engine's own provider remains the default.
2. **The engine as a static archive** of the SDK, linked into the application with link-time
   optimization and dead-code stripping, so the families an application does not call (3D, the
   Direct2D compatibility layer, MIL, hit-test queries) are not in its binary: the linker removes
   unreferenced code, nothing is disabled.
3. **One copy of each shared component.** The engine's text, shaping and font code is FerroUI's
   text stack with this backend (section 2.4), not a second stack beside Skia's; an application
   built with ProGPU does not link Skia.
4. **Shaders as WGSL text** (about 0.6 MB, smaller compressed), compiled by the WebGPU
   implementation; in the browser the browser compiles them, so the module carries only the text.
5. **Fonts** from the system catalog; none are bundled by default.

The bindings and SDK changes these need are part of step P7.

### 3.3 Lessons from GPUI

GPUI (the UI framework of Zed; read 2026-11-05 at `crates/gpui`, `gpui_apple`, `gpui_windows`,
`gpui_wgpu`, `gpui_web`) and what it changes here:

- **Analytic shadows.** GPUI draws the blurred shadow of a rounded rectangle in closed form in the
  fragment shader (the Gaussian integral through an `erf` approximation, drop and inset), one
  instanced quad per shadow, with no offscreen layer and no blur pass. ProGPU has drop shadows only
  as a group effect (a layer, two separable Gaussian passes, a compose). FerroUI's themes put a
  `BoxShadow` on most raised surfaces (buttons, flyouts, cards, menus), so the engine gets an
  analytic rounded-rectangle shadow primitive (drop and inset, per-corner radii, spread), in the
  analytic primitive family with its batching, validated against the group effect's blur. The
  group effect stays for shadows of arbitrary content.
- **A small, fixed primitive vocabulary, batched.** GPUI's scene has eight primitive kinds (shadows,
  quads with per-corner radii and borders, paths, underlines, monochrome, subpixel and polychrome
  sprites, surfaces), each an instance record, sorted by draw order and texture and drawn in a few
  instanced batches. The backend maps borders, rounded rectangles and ellipses to ProGPU's analytic
  primitives (never to paths) so the engine batches them the same way.
- **Frame reuse.** GPUI's cached views replay the previous frame's primitive range of a view that
  was not notified, but the scene is still sorted and uploaded every frame. ProGPU's retained scene
  goes further (content-hashed pages, replayed render bundles, nothing uploaded in a stable frame),
  so the plan keeps it; the lesson kept is the granularity: one retained fragment per visual
  boundary, invalidated by the visual's own change.
- **Text and binary size.** GPUI ships no shaper, segmenter or Unicode data: it uses the system text
  stacks (CoreText, DirectWrite, Canvas 2D in the browser) and cosmic-text on Linux. The owner keeps
  ProGPU's own text stack, so the size lesson applies to its data: the ICU data of the edit word
  classifier (33.1 MB of the 38.9 MB engine) is filtered to the break rules and the one admitted
  dictionary it uses (section 3.2), not replaced.
- **GPU APIs.** GPUI uses Metal on Apple platforms and Direct3D 11 on Windows directly, and wgpu only
  on Linux and the web, with a WebGL2 variant of its shaders (instance records in a texture instead
  of storage buffers). ProGPU stays on WebGPU everywhere (the owner's request); a WebGL2 fallback of
  the browser build for browsers without WebGPU is an open question for the owner, not planned.

## 4. Measurements

All measured locally (the owner, 2026-11-05: no new workflows), on the same machine and commit for
every backend, medians of repeated runs:

- **TreeDataGrid**: `tests/FerroUI.TreeDataGrid.Benchmarks` (the paired and revision scroll
  comparisons, first layout) with Skia, Vello and ProGPU.
- **Rendering benchmarks**: the rendering, text and compositor groups of `tests/FerroUI.Benchmarks`.
- **Catalog scenarios** of `docs/porting/vello-performance.md`: pointer across buttons (partial
  redraw), the effects page, scrolling 20 px a frame, the text page drawn again, the catalog tour;
  milliseconds a frame (CPU encode and submission, and GPU where the API reports it).
- **Published size**: the stripped executables of the HelloWorld samples, `hello_window`, the WinUI
  gallery and the ControlCatalog with Skia and with ProGPU (the release and the dist profile;
  ProGPU's runtime libraries counted), and the browser module (`.wasm` and its script, raw and
  compressed) of the WebGL2 Skia build and the WebGPU ProGPU build.
- **Screenshots** of the ControlCatalog pages with each backend, side by side.
- **Against GPUI** (the owner, 2026-11-05), built locally from a pinned Zed commit:
  - the size of a hello world in the browser: FerroUI's HelloWorld (WebGPU ProGPU build, and the
    WebGL2 Skia build) against GPUI's hello world example built with `gpui_web` (its wgpu
    renderer), the `.wasm` and its script, raw and compressed;
  - the TreeDataGrid against GPUI's table counterpart (its `uniform_list` table example with
    the same row count and columns): first layout, scrolling a page a frame and frame times,
    on the desktop (FerroUI with ProGPU against GPUI's Metal renderer) and in the browser
    (FerroUI's WebGPU build against GPUI's web build).

The tables go into section 6 and the pull request as each step lands.

## 5. Steps

| Step | Content | Done when |
|---|---|---|
| P0 | This plan | merged with the draft PR |
| P1 | `ferroui-progpu`: the SDK package fetched and verified at build time (version and hashes pinned), linked; the engine created headless; a test renders a cleared target and reads it back | the headless test passes on macOS |
| P2 | Drawing: rectangles, rounded rectangles, ellipses, lines, paths (fills and strokes, dashes), solid and gradient brushes, transforms, clips, opacity, layers and blend modes, through the C++ shim over the builder; the render tests of the drawing contracts against the CPU references | the drawing contract tests of the other backends pass |
| P3 | Windows: the wgpu-native surface on macOS (`CAMetalLayer`), cooperative resize; `use_progpu()`; the ControlCatalog runs | catalog screenshots |
| P4 | Text: font manager, shaper, glyph runs, system fonts and fallback | the text tests of the backends; the text pages of the catalog |
| P5 | Images and bitmaps, render target bitmaps, writeable bitmaps, effects (blur, drop shadow, the custom effects of #455 where the engine has them), acrylic | the effect and image tests |
| P6 | Retained mode: fragments as encoded records, state records, generations, cached layers, damage | stable frames submit without re-encoding; catalog measurements |
| P7 | ProGPU bindings: `progpu-native-sys`, `progpu-native` with the Rust writer and the differential test, the C ABI additions (font catalog, delta update, span bundles, surface helpers), in the ProGPU repository | merged in ProGPU; the backend on them |
| P8 | Windows (Direct3D 12) and Linux (Vulkan) surfaces, in the VMs | catalog smoke runs |
| P9 | Browser: the `WebGPU` mode of `ferroui-browser` on the Emscripten build of the engine | the browser catalog on WebGPU |
| P10 | Measurements and size (section 4), the documentation, the NOTICE files | the tables of section 6 |

## 6. Results

None yet.

## 7. Licensing

ProGPU is MIT (Copyright (c) 2026 Wiesław Šoltés). The SDK package ships the notices of its
optional static dependencies (FreeType, ICU) and the browser build those of Emscripten and
Emdawnwebgpu; the NOTICE files of `ferroui-progpu` and of the browser build carry them. ProGPU's
Unicode data tables need the Unicode license notice, which is checked in P10.
