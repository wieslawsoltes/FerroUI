# ProGPU render backend (#369)

A render backend of FerroUI on the native C++ engine of ProGPU
([wieslawsoltes/ProGPU](https://github.com/wieslawsoltes/ProGPU), MIT, pinned at
`v0.1.0-preview.70`): its WebGPU renderer with all its WGSL shaders (vector, path and glyph
rasterization, images, layers, masks, blend modes, effects), its text stack (fonts and the system
font catalog, fallback, Unicode bidi and line breaking, the OpenType shaper) and its retained
scene model, on the desktop (Metal, Direct3D 12, Vulkan) and in the browser on WebGPU. The goal is
a retained GPU mode as fast as the engine allows, measured against the Skia and Vello backends.

Status: P1 to P3 are in the branch. The backend draws shapes, paths, brushes, clips, opacity,
layers and analytic shadows into offscreen targets and into the Metal drawables of macOS windows,
behind `use_progpu()`, which also binds the text services of the backend (section 2.4: font
manager, typefaces and the shaper of the engine); glyph runs, images from files and effects are
drawn as nothing and said once in the log (section 2.7). The owner's request of 2026-11-05: top priority; the PR is open from the start;
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

**The WebGPU implementation is wgpu 30, linked statically.** The engine is built from the ProGPU
sources as its standard-WebGPU target (`progpu_native_dawn`). It calls WebGPU only through a
dispatch table of procedures resolved at engine creation (`resolve_proc`), never through link-time
symbols. FerroUI resolves them from wgpu-native compiled as a Rust dependency: the fork
`FerroUI/wgpu-native`, branch `progpu/wgpu-30` (wgpu-core 30.0.1, the same wgpu as the
`wgpu` crate of the Vello backend). The fork adds what the engine needs:
- real futures with `wgpuInstanceWaitAny`, which the engine uses to poll its submissions without
  waiting;
- a complete `wgpuGetProcAddress` and `wgpuBufferGetMapState`;
- native-object interop on Metal: `wgpuInstanceCreateDeviceFromMetal(instance, device, queue,
  descriptor)` and `wgpuDeviceCreateTextureFromMetal(device, texture, descriptor)`.

`progpu-native-sys` links the engine archives and wgpu-native; nothing is loaded or downloaded at
run time. Its tests render retained scenes, analytic shadows and path measures on that device.

**Window surfaces** keep FerroUI's cooperative resize, with no stretched or stale frame while a
window is resized: the platform presents its own frames, and the backend draws into them, as the
Vello backend does (`src/Vello/FerroUI.Vello/gpu/metal.rs`). On macOS:
- the device is made on the platform's `MTLDevice` with `wgpuInstanceCreateDeviceFromMetal`;
- each frame's drawable texture is wrapped with `wgpuDeviceCreateTextureFromMetal`, and the engine
  renders into its view;
- the platform presents the drawable.

The queue is the one difference from Vello: with the platform's queue created with room for 4096
command buffers instead of Metal's default 64, the device is created on the platform's own queue.
The presentation is then committed after the frame on the same queue, with no shared event (Vello
needs one because it renders on a second queue). Windows (D3D12) and Linux (Vulkan) get
interop functions of the same shape in the fork (step P8).

**Dawn instead of wgpu.** The same engine archive runs on Dawn: the two `webgpu.h` revisions differ
only by three enum values the engine does not use, and the engine resolves its procedures at run
time. A host that already has a Dawn device, such as WebScene (whose GPU provider ABI 2 hands out
its instance, a procedure lookup and canvas textures), creates the engine on that device with
Dawn's procedures. Two things are still to do before both can live in one process:
- the fork's C functions must stop being exported globally, so that they cannot interpose Dawn's
  functions of the same names (a risk on Linux);
- with Dawn on macOS, presenting through the platform's layer needs a shared event, as Dawn cannot
  adopt the platform's device and queue.

**Sharing GPU content without fences.** Content shared on one device and one queue needs no fence:
the queue orders the producer's submission before the consumer's, and the implementation tracks
resource usage between them. The engine draws such textures directly, without a copy
(`progpu_native_engine_bind_scene_external_images`). The owner of a borrowed texture recycles it
once `progpu_native_engine_poll_submission` reports, without waiting, that the reading submission
completed.
- WebScene on ProGPU is fence-free when the engine runs on WebScene's Dawn device.
- With FerroUI on wgpu and WebScene on Dawn, the two share zero-copy textures, and the GPU orders
  them with shared events (`MTLSharedEvent`, D3D12 fences, Vulkan semaphores). The GPU waits; the
  CPU never does.
- Vello and ProGPU on one wgpu device would need the `wgpu` crate's core device and texture
  identifiers, which the crate does not expose. They share textures through the Metal interop
  instead.

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

`use_progpu()` binds the text services of the backend with the render interface, so an application
names no text subsystem: the font manager and the glyph typefaces come with the rendering
subsystem, the shaper as the text shaping subsystem named last. All of them stand on the text stack
of the engine; no other font or shaping library is linked.

| Service | File | On what |
|---|---|---|
| Faces and the catalog | `font.rs` | The font C ABI of the bindings (`progpu_native_sys::font`): the system font catalog of the engine (families, styles, matching a family at a style, the fallback face of a character for a language) and faces made of font bytes; face metrics, names, tables, the variation instance of a style, and unhinted glyph outlines in design units from the parsers of the engine (TrueType, CFF, variations). No hinting provider is linked. |
| `IFontManagerImpl` | `font_manager.rs` | The catalog: installed families, the default family (the first the catalog has of a short list), a typeface of a family at a style with the simulations of what the face cannot give (bold from weight 600 for a face under it, oblique for an upright face; a variable face is set to the weight and the slant instead), the fallback of a character (`try_match_character`), and a typeface of the bytes of a stream (embedded fonts, `ferres://` assets). |
| `IPlatformTypeface` | `typeface.rs` | A face at one style: its tables (copied to their reader), its stream, the normalized variation coordinates of its instance. |
| `ITextShaperImpl` | `text_shaper.rs` | The OpenType shaper of the engine. A typeface holds one text context of the engine; a text is shaped as scalars with up to five scalars of the text before and after it as context, so a slice of a word joins to its surroundings; the script is found by the engine, the direction comes from the bidi level, the language from the culture, the features and the letter spacing from the options. A trailing line break is shaped as a zero width space (a pair `\r\n` as one), a tab as a space of the tab width. A typeface of another font manager is shaped from the bytes of its stream. |
| `IGlyphRunImpl` | `glyph_run.rs` | The glyphs of a run with their positions, the face and its instance, and the bounds of the ink (the design box of the face at each glyph, widened for the simulations). The geometry of a run (`build_glyph_run_geometry`) is the outlines of its glyphs. |

**The catalog or font bytes.** Both: the engine exposes its own system font catalog, and a face can
be made of bytes FerroUI brings. The catalog is the one the engine already has (it reads the font
folders of the system, the names and the coverage of each face, and matches families, styles and
characters), so exposing it costs a thin C layer and gives the backend system fonts and character
fallback without a platform font API in FerroUI. Bytes are needed anyway for embedded fonts. The
fallback itemization of a text stays in FerroUI: its text layout asks the font manager for the
face of each character the current face lacks, which the catalog answers.

**Checked against the reference shaper.** The tests shape the same texts with the shaper of the
engine and with HarfBuzz (a development dependency of the tests only) and compare glyph ids,
clusters, advances and offsets within 0.011 px at 20 px: Latin with kerning, ligatures and
contextual alternates, combining marks, characters outside the basic plane, Arabic with marks
(right to left, in visual order), Hebrew with points, Japanese, a variable face at an instance, and
a slice of an Arabic word with its surroundings. One difference was found and fixed in the engine:
a contextual rule that matched without a nested lookup (an exception rule) did not end its lookup,
so a later rule still applied (the colon between a lower case letter and a capital in Inter).

**Known differences.**
- Clusters are whole graphemes: a joiner (U+200C, U+200D) in a text belongs to the cluster before
  it, where HarfBuzz keeps it a cluster of its own. This is why a trailing line break is shaped as
  a zero width space and not as a joiner.
- The advances the text layout reads from a `GlyphTypeface` come from the `hmtx` table; a variable
  face at an instance is shaped with its varied advances, and the two differ where the layout uses
  the metrics of the typeface instead of the shaped run.
- A text context copies the bytes of its font: a face of a large system collection that is used
  for shaping costs its file size in memory once.
- The catalog is read once for the process: fonts installed later are not seen.
- `get_intersections` of a glyph run answers nothing, so an underline does not skip the ink.
- A typeface of another font manager is shaped, but its glyph run draws nothing and has no outline
  geometry.

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

### 2.7 The platform render interface (P3)

What the backend implements today, and the rules it follows.

**Devices and engines.** A render context owns a device and the engines on it (`device.rs`,
`gpu.rs`). The offscreen context has a device of its own (the default adapter); the context of a
window surface on macOS is made on the Metal device and the command queue of the platform
(`wgpuInstanceCreateDeviceFromMetal`). An engine has one retained scene and one target format, so a
device keeps one engine per format it draws to (RGBA8 for offscreen targets, BGRA8 for drawables)
and a target sends its scene before it renders. The engine guards against concurrent entry with
the thread that owns it; the compositor renders from the render thread and, during a synchronous
resize, from the UI thread, always under the lock of the render loop, so the backend adopts the
engine on the thread that enters next (`progpu_native_engine_adopt_owner_thread`).

**Render targets** (`targets.rs`, `bitmap.rs`, `layer.rs`):

| Target | How a frame is drawn |
|---|---|
| Metal surface of a window | The platform owns and presents the drawables. A frame begins a session of the surface, wraps the texture of the drawable (`wgpuDeviceCreateTextureFromMetal`), the engine renders the scene into it, the wrapper is released and ending the session presents. The texture is the size of the surface at that moment, so a resized window is drawn at its new size in the frame that presents it: no stretched and no stale frame. A drawable holds nothing of the frame before, so every frame draws the whole scene. |
| Framebuffer of a surface | The frame is kept in a texture of the device and drawn over (`PRESERVE_TARGET`); the pixels are read back into the locked framebuffer (RGBA8888 and BGRA8888). |
| Render target bitmap, layer | A texture of the device; a drawing context draws over what the bitmap holds; reading pixels reads the texture back. A layer drawn into another target is an external image of the scene, without a copy. |
| Context without a device | Never ready; what is drawn is drawn as nothing. |

**The scene of a frame** (`scene.rs`, `drawing_context.rs`). A drawing context records into the
scene builder of the engine through its C ABI in the bindings; ending the frame sends the stream (`update_scene`) and renders it.
The rules of the engine the recorder keeps:
- a scene identifier keeps its stream generation strictly increasing; a resource keeps its
  identifier, and its generation changes only with its content;
- resource identifiers ascend in the order of the resource table;
- an external image is bound under the identity the builder gave it, and the bound table is sorted
  by identifier.

**Brushes.** The engine evaluates a brush in the coordinates of the shape or path it fills, before
the transform of the state. A brush record carries the rows that map those coordinates into the
space of the brush, so the backend passes the inverse of the transforms of the brush alone: the
relative transform (on the box of the fill as a unit square) and then the absolute transform about
the transform origin.

**The end of a thread.** In a build with debug assertions the WebGPU implementation keeps the
state of one of its locks in a thread-local, and releasing a device needs that lock. The objects of
a render context are released when the thread-locals of its thread are destroyed, in an order the
backend does not choose; a release after the lock state is gone would abort the process. The
backend therefore never calls the WebGPU implementation while the locals of the calling thread are
being destroyed: the device, its engines and its textures are then left to the end of the process
(`device.rs`, `thread_is_ending`). The detection is a thread-local mark registered after the first
call into the implementation on the thread, which the reverse order of destruction puts before the
lock state. Objects released while the thread runs are released as usual.

**Not drawn yet, said once in the log** (area `ProGPU`, `notes.rs`): the glyphs of text (text is
shaped and laid out by the services of section 2.4; the glyph runs are not drawn yet), bitmaps
decoded from files or streams, bitmaps drawn under an opacity mask, inset shadows and the shadows of rectangles with different corner radii, and
a clear under a rounded or geometry clip. Content under an opacity mask is drawn without the mask.
The backend refuses compositing layers with a blend mode (`try_push_compositing_layer` answers
no); blend modes are step P5. A frame that fails is logged every time and leaves the target as it
was. None of these crashes.

**Known limits of the step.**
- A layer drawn into a target and drawn into again before the target renders shows its later
  content: the scene holds the texture, not a copy.
- The window target draws the whole scene every frame and the whole stream is sent every frame;
  P6 changes both.
- The headless frame benchmarks are not in this branch; the test of the compositor on an offscreen
  target of the backend (`platform_tests.rs`) stands in for them.

Tests: `cargo test -p ferroui-progpu` renders through the public contracts and reads pixels back
(clears, rectangles, rounded rectangles, ellipses, paths, strokes, gradients, clips, opacity,
transforms, layers, the framebuffer target, the Metal texture target on a device made on a Metal
device, a compositor frame, the safe failures, the end of a thread). The example
`progpu_window` opens a window through the application builder with `use_progpu()`.

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

What the work on P1 to P3 and the reading of the engine for P4 found, beyond the list above:

- **One scene and one target format per engine.** Every target of a device shares the retained
  scene of the engine of its format, so a target sends its stream before it renders and two
  windows on one device replace each other's scene. Retained mode (P6) needs a scene per target:
  several retained scenes in one engine, or engines that share the pipelines and caches of a
  device.
- **No update smaller than a stream.** `progpu_native_engine_update_scene` replaces the retained
  snapshot and releases the render bundle of the scene; there is no call that changes one state
  record (a transform, an opacity, a clip) or the records of one visual.
- **Glyph outlines without the hinting provider** (done). The exported C ABI hands outlines out
  only through the hinted entry points, which need a prepared FreeType. The bindings now carry a
  font C ABI over the parsers of the text module (`progpu_native_sys::font`): unhinted outlines in
  design units, face and glyph metrics, names, tables and variation instances.
- **Fonts are bytes** (done). The same font C ABI exposes the system font catalog (families,
  matching, the fallback face of a character) beside faces made of bytes.
- **The glyph cache is per frame payload.** The glyph atlas is kept while the outline array and
  the segment arena of a frame are byte-equal to those of the frame before; a frame with another
  set of glyphs rasterizes all its outlines again. Retained text needs an atlas keyed by outline,
  scale and phase that keeps entries across frames.
- **Text quality.** Coverage is 8x8 supersampled on the GPU into an R8 atlas and sampled with a
  linear filter; the subpixel phase is horizontal only and quarter-pixel; the subpixel text mode
  samples the grayscale atlas three times and is not a filtered LCD raster (the separate RGB glyph
  run is one, for opaque targets and whole-pixel positions only); the coverage transfer is a fixed
  gamma and contrast or the GDI+ grayscale contrast table. Parity with the text of the macOS
  platform (unhinted outlines, stem darkening, linear blending of coverage) needs a transfer for
  that, and a comparison against the Skia backend on real text, which has not been made.

These land in ProGPU as their own pull requests, each with the tests of the engine; the FerroUI
backend starts on the existing C ABI with a small C++ shim over the builder (step P2) and moves to
the Rust writer as it lands, keeping the shim as the differential reference.

### 3.1 The WebGPU implementation

The engine runs on any WebGPU implementation of the standard `webgpu.h` whose procedures the host
resolves (`progpu_native_dawn_engine_create`); it links no WebGPU library.

- **wgpu 30 (the default).** wgpu-native of the fork `FerroUI/wgpu-native`, branch
  `progpu/wgpu-30` (webgpu-headers `673658bc`), compiled as a Rust dependency of
  `progpu-native-sys` and linked statically. It shares wgpu-core 30 with the Vello backend, and the
  linker keeps only what the program reaches. Section 2.2 lists what the fork adds.
- **Dawn (optional).** The engine archive is unchanged; the host resolves Dawn's procedures and
  hands it a Dawn device. Dawn's headers (`01addc4b`) differ from the fork's by three enum values
  the engine does not use. This is the way to share a device with a host that has its own Dawn,
  such as WebScene (section 2.2).

In the browser the implementation is the browser's (`navigator.gpu`); nothing is shipped. Speed,
features and size are measured with wgpu first (section 4).

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

| Step | Content | Done when | Status |
|---|---|---|---|
| P0 | This plan | merged with the draft PR | done |
| P1 | `ferroui-progpu`: the SDK package fetched and verified at build time (version and hashes pinned), linked; the engine created headless; a test renders a cleared target and reads it back | the headless test passes on macOS | done (the engine is built from the submodule instead of fetched packages) |
| P2 | Drawing: rectangles, rounded rectangles, ellipses, lines, paths (fills and strokes, dashes), solid and gradient brushes, transforms, clips, opacity, layers and blend modes, through the C++ shim over the builder; the render tests of the drawing contracts against the CPU references | the drawing contract tests of the other backends pass | done for shapes, paths, brushes, clips, opacity, layers and analytic shadows, through the C ABI over the scene builder in the bindings; blend modes and masks move to P5 |
| P3 | Windows: the wgpu-native surface on macOS (`CAMetalLayer`), cooperative resize; `use_progpu()`; the ControlCatalog runs | catalog screenshots | done on macOS: offscreen, framebuffer and Metal window targets, `use_progpu()`, a window example with shapes (section 2.7); the ControlCatalog needs text (P4) |
| P4 | Text: font manager, shaper, glyph runs, system fonts and fallback | the text tests of the backends; the text pages of the catalog | in progress: the font manager on the catalog of the engine, typefaces, the shaper of the engine checked against HarfBuzz, glyph runs with bounds and outline geometry (section 2.4); drawing the glyphs is next |
| P5 | Images and bitmaps, render target bitmaps, writeable bitmaps, effects (blur, drop shadow, the custom effects of #455 where the engine has them), acrylic | the effect and image tests | open (render target bitmaps, layers and writeable pixel bitmaps are in P3) |
| P6 | Retained mode: fragments as encoded records, state records, generations, cached layers, damage | stable frames submit without re-encoding; catalog measurements | open |
| P7 | ProGPU bindings: `progpu-native-sys`, `progpu-native` with the Rust writer and the differential test, the C ABI additions (font catalog, delta update, span bundles, surface helpers), in the ProGPU repository | merged in ProGPU; the backend on them | in progress: `progpu-native-sys` (the engine built from the sources, wgpu-native linked statically, a C ABI over the scene builder), the Geometry module with its C ABI, analytic drop shadows, owner-thread adoption; the Rust writer and the other C ABI additions are open |
| P8 | Windows (Direct3D 12) and Linux (Vulkan) surfaces, in the VMs | catalog smoke runs | open |
| P9 | Browser: the `WebGPU` mode of `ferroui-browser` on the Emscripten build of the engine | the browser catalog on WebGPU | open |
| P10 | Measurements and size (section 4), the documentation, the NOTICE files | the tables of section 6 | open |

## 6. Results

None yet.

## 7. Licensing

ProGPU is MIT (Copyright (c) 2026 Wiesław Šoltés). The SDK package ships the notices of its
optional static dependencies (FreeType, ICU) and the browser build those of Emscripten and
Emdawnwebgpu; the NOTICE files of `ferroui-progpu` and of the browser build carry them. ProGPU's
Unicode data tables need the Unicode license notice, which is checked in P10.
