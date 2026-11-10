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
manager, typefaces and the shaper of the engine) and draws text from a glyph atlas that is kept
across frames, matched by measurement to the text of the Skia backend on macOS (section 2.4.1);
images from files and effects are drawn as nothing and said once in the log (section 2.7). The owner's request of 2026-11-05: top priority; the PR is open from the start;
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
needs one because it renders on a second queue). The capacity is needed, not a margin: wgpu keeps
two Metal command buffers for each render pass of a command encoder (the pass and the resource
transitions before it) until the frame is submitted, and a queue that has given out all of its
buffers blocks the thread that asks for one more. A translucent group is two passes today (its
content, then the composite on its parent), so a frame of the gauntlet with 20 groups (43 passes,
86 command buffers) stopped for ever on a queue of the default capacity, while one with 8 groups
(19 passes) did not. The macOS platform (`native/FerroUI.Native/src/OSX/metal.mm`) and the
offscreen Metal device (`ferroui_metal::offscreen`) create their queues with 4096, which is also
the number of open command buffers wgpu allows a device: about 1000 groups in one frame. The
retained path (section 2.3) is to render the contents of the groups before one pass over the
target, which removes the composite passes. Windows (D3D12) and Linux (Vulkan) get interop
functions of the same shape in the fork (step P8).

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
and draws it again while the content does not change (`IDrawingFragment`,
`IDrawingContextImplWithFragments`). The retained path makes that fragment a piece of the scene the
engine keeps compiled on the GPU, so that a frame costs what changed in it. This section is the
design of that path (step P6). It needs changes in the engine itself, beyond the bindings, and is
written down for review before any of it is built.

#### 2.3.1 What a frame costs today

Read from the engine and the backend as they are (nothing here is a timing):

| Where | What happens every frame in which anything changed |
|---|---|
| Backend | The commands of every visual are replayed into the drawing context: one call into the builder for every record, the paths prepared again, the glyph runs placed again. The backend keeps no fragments. |
| `progpu_native_engine_update_scene` | Seven passes over the whole stream: a byte comparison with the retained snapshot, the validation walk, the comparison of the generations with the old stream, a copy, a hash of the bytes, the content hashes of every family, and the release of every render bundle. |
| `progpu_native_engine_render_scene` | Two preflight walks over every command. A family page (analytic, path, glyph, image, brush, text style) is keyed by the hash of its family: one changed record or one changed state compiles every record of the family again on the CPU and writes its whole buffer from offset 0. The transform, the opacity and the clip of a state are applied on the CPU while the records are compiled, so a moved visual is a changed family. Every render bundle is recorded again. |
| Translucent groups | A layer ends the current pass, renders its content in a pass of its own and opens the parent again to composite: two passes for each group and one for the root (section 2.2). |
| Cached layers | At most 16, keyed by revisions the caller names. A hit skips the GPU replay of the content; the commands inside are still walked and compiled on the CPU. |
| Scenes | One retained scene for each engine: a window and the layers drawn through the same engine replace each other's snapshot, so the comparison at the head of the update never matches. |

A frame in which nothing changed is already cheap (the bundles are replayed). A frame in which one
visual moved costs the same as a frame in which every visual changed.

#### 2.3.2 What changes in a gauntlet frame

Counted on the window target with the bookkeeping of the compositor (a visual whose kept
fragment the compositor offers to draw again has the content of the frame before), every tenth
frame of frames 10 to 120 of the scene, the first two frames left out. The scene sets the width of
its content in every frame, so a large part of it is laid out and drawn anew by design:

| In one frame | tier 5 | tier 16 |
|---|---|---|
| Visuals drawn | 784 to 798 | 8,951 to 10,364 |
| content changed, or drawn for the first time | 289 to 318 | 3,634 to 3,858 |
| content unchanged, at the same place | 43 to 73 | 180 to 958 |
| content unchanged, moved by whole device pixels | 255 to 310 | 3,247 to 5,047 |
| content unchanged, moved by a fraction of a pixel | 6 | 18 |
| content unchanged, rotated or scaled | 138 to 140 | 1,052 to 1,084 |
| Draw commands | 955 to 975 | 11,107 to 13,176 |
| of visuals whose content changed | 460 to 495 (48 to 51 %) | 5,480 to 6,488 (47 to 51 %) |
| of unchanged visuals at the same place or moved by whole pixels | 34 to 37 % | 39 to 45 % |

Three things follow for the design:

1. Scrolling lands on whole device pixels: of the visuals that only moved, all but 18 (tier 16)
   moved by whole pixels. An offset in whole pixels is enough to keep them.
2. Keeping fragments removes at most about two fifths of the draw commands of a gauntlet frame
   from the encoding and the compiling. Half of the commands belong to content that is new in the
   frame, and a tenth to the rotating panels. The retained path is therefore necessary and not
   sufficient for this scene: what a **changed** visual costs (its encoding in the backend, its
   compilation in the engine, and the passes over the whole stream that come with any change
   today) decides the other half.
3. The second frame of the scene, before the animation starts, draws 10,064 visuals at tier 16
   with every fragment unchanged at the same place: the case every static window is.

#### 2.3.3 Spans, placements and scenes

Three objects are added to the engine. The stream format of the records does not change: a span is
made of the same pointer-free commands and resources as a scene stream.

- **Scene.** The engine keeps one retained scene for every scene identifier it was given, instead
  of one in all, each with its own order, spans and bundles; `progpu_native_scene_frame::scene_id`
  selects it. A scene is released by a call, and the scenes together stay inside a byte budget the
  engine reports. The family buffers, the pipelines, the glyph atlas and the path coverage cache
  stay shared by the scenes of an engine.
- **Span.** The records of one fragment, under an identifier and a generation the backend chooses.
  The engine validates a span when it arrives, compiles it once for each family into ranges of the
  family buffers, and keeps those ranges while the generation stays. The buffers become chunks of
  a fixed size with ranges handed out from them, so a new or changed span uploads its own ranges
  and nothing else moves; a removed span returns its ranges.
- **Placement.** What a frame says about a span without changing it: an offset in whole device
  pixels, the clip of its ancestors (a rectangle and, for rounded and rotated clips, the analytic
  mask chain of section 2.7) and the group it is drawn into. Placements live in a table on the GPU
  (a storage buffer, one record for each span of the order); every vertex and instance of a span
  carries the slot of its placement, and the shaders add the offset after the brush coordinates
  are taken and before the mask is evaluated. A span that moved is one record written.

The **order** of a scene is its list of entries: a span with its placement, the push and the pop of
a group (the existing layer descriptor), a clear. A frame sends what changed:

```c
/* One call a frame for a target. The delta is pointer-free, like a scene stream. */
progpu_native_status progpu_native_engine_update_scene_delta(
    progpu_native_engine* engine,
    const uint8_t* delta, size_t delta_size,
    progpu_native_scene_delta_metrics* metrics);

/* delta: header { magic, version, scene_id, base_generation, generation, operation table }
 * operations:
 *   PUT_SPAN      { span_id, generation, stream bytes }     validated and compiled once
 *   REMOVE_SPAN   { span_id }
 *   PUT_RESOURCE  { resource_id, generation, kind, bytes }  shared by the spans of the scene
 *   REMOVE_RESOURCE { resource_id }
 *   SET_ORDER     { entries[] }      sent when visuals were added, removed or reordered
 *   SET_PLACEMENTS{ first, count, placements[] }            ranges of the placement table
 */
progpu_native_status progpu_native_engine_release_scene(progpu_native_engine*, uint64_t scene_id);
```

`progpu_native_engine_render_scene` then draws the order: consecutive spans of one family whose
ranges are adjacent in a chunk stay one draw, as consecutive commands do today; a render bundle
covers a run of draws with one group and one mask, and is recorded again only when a span of the
run changed its ranges or the order changed. Scissor rectangles are set between bundles from the
placements, so a clip that moved records nothing. `progpu_native_engine_update_scene` with a whole
stream stays as it is, for callers that send whole streams and as the reference of the tests.

Resources named by identifier remove what a frame repeats for them today: a glyph outline is a
resource under (typeface, glyph), sent once, so the engine no longer hashes the outline bytes of
every glyph of a frame to find its atlas tile; an image is a resource under the identifier and the
version of its bitmap; brushes stay inside the span that uses them.

What a **changed** span costs is the other half of a gauntlet frame (section 2.3.2), and the same
objects carry it: a new span is validated and compiled alone (no pass over the rest of the scene,
no other family record compiled again, no bundle of another run recorded again), its glyphs find
their tiles by identifier, and its ranges are the only bytes uploaded. The backend's share of a
changed visual is the builder call for every record; the Rust stream writer of section 3 removes
that boundary and is independent of the engine work.

#### 2.3.4 What a placement may change

The rule that keeps the pixels: **a span that is drawn again without being compiled again gives
the bytes it would give compiled at the new place.** That holds for an offset in whole device
pixels and for a changed clip, and for nothing else the engine has today:

| Change of a visual | What happens |
|---|---|
| Moved by whole device pixels (scrolling on the pixel grid, a parent that moved) | one placement record |
| Clip of an ancestor changed | one placement record |
| Moved by a fraction of a pixel | the span is compiled again at the new phase: glyph tiles and path coverage are rasters made for a scale and a subpixel phase (section 2.4.1), and sampling a raster made for another phase would blur it |
| Rotated or scaled | the span is encoded and compiled again; only this span |
| Content changed | the fragment is dropped by the compositor; a new span |

Analytic shapes, images and gradients could take a whole affine transform in the placement,
because their shaders are evaluated for each pixel. That is left out of the first version: the rule
above is one test (the same scene drawn from moved spans and from a whole stream must read back
the same bytes), and text, which the owner puts first, cannot take part in more than the offset.

#### 2.3.5 Groups before the root pass

Independent of the spans, and the first thing to build because it removes a limit the gauntlet
already reaches (section 2.2): the content of every group that does not read its backdrop is
rendered before the root pass begins.

- The groups of a frame are ordered by nesting depth. The groups of one depth share a **group
  page**: one texture, each group a rectangle of it sized to its bounds in device pixels, packed
  on shelves; a second page is opened when the first is full. One pass draws the content of all
  groups of a depth, the deepest depth first, with the viewport and the scissor of each group's
  rectangle.
- The pass of the parent (the root, or the page of the depth above) draws each group as one quad
  sampling its rectangle, with the opacity and the blend of the group, at the group's place in
  the order.
- A frame is then 1 + (the greatest nesting depth) passes, whatever the number of groups, instead
  of 1 + 2 x (the number of groups); the root target is no longer loaded and stored for each
  group. The command buffers of a frame no longer grow with the groups.
- Groups that read their backdrop, groups with effects and advanced blends keep today's path.
- The managed implementation is the reference for the order: it prepares its layer textures in a
  pass before the geometry of the frame "so a nested offscreen submission never splits the main
  pass" (`EnsureLayerTexture`, `PreparePictureLayerCaches`).

The stream and the C ABI do not change for this part; the pixels must not change either (the tests
of the layers and of section 2.7 read back the same bytes before and after).

A cached group (a visual with a cache mode) is then a group whose rectangle is kept from frame to
frame: its span generations are its content revision, so a hit skips the CPU work for its content
as well, which the cached layers of today do not.

#### 2.3.6 The backend

- `ProGpuDrawingContext` implements `IDrawingContextImplWithFragments`.
  `begin_fragment` starts a span in the builder and sets the clips of the ancestors aside: they
  belong to the placement, not to the span. What is drawn until `end_fragment` is recorded as
  today, in device pixels at the transform of the visual, and also drawn.
- The fragment (`ProGpuFragment`) keeps the identifier and the generation of the span, the
  transform it was recorded at, the options it was recorded under (antialiasing, text mode) and
  the bitmaps it samples.
- `draw_fragment` accepts when the options are the same, the linear part of the transform is the
  same and the translation differs by whole device pixels; it then adds an order entry with a
  placement and encodes nothing. Otherwise it returns `false` and the compositor draws the visual
  again into a new fragment, which replaces the span.
- A fragment that is dropped (the compositor drops it when the commands or a resource of them
  change, or the visual goes away) queues `REMOVE_SPAN` for the next delta of its scene.
- Fragments that sample a render target (a layer, a render target bitmap) are not kept in the
  first version: the texture of such an image is bound for each frame with its generation.
- Every target (a window, a layer, an offscreen frame) has its own scene identifier and sends
  deltas to it. A layer that is not drawn into in a frame sends nothing and is sampled as it is.
- The dirty rectangles of the compositor become `DAMAGE_RECT` with `PRESERVE_TARGET` on the frame;
  spans whose bounds lie outside the damage are left out of the draws of the frame.

Until the delta call exists the same fragments work against the whole-stream call: the builder
keeps the bytes of a span and writes them into the stream again without the backend encoding
them (a call of the builder C ABI in the bindings). That removes the backend's share of an
unchanged visual and is the differential reference afterwards: a frame built from kept spans and
the same frame built from commands must be the same stream.

#### 2.3.7 Order of the work, and what each part is checked against

The order follows the profile of the gauntlet at tier 16 on this backend (2.6 to 2.9 frames a
second before any of this; section 6.2 has the numbers after each part):

- The render thread is busy for all of a frame (about 340 ms); the UI thread for 7 percent. The
  walk of the compositor is 9 percent of the render thread, the backend's glyph and geometry
  calls 7 percent together. The encoding of the backend is not what a frame waits for.
- 62 percent of the render thread is the byte-by-byte hash of the engine, and nearly all of that
  is glyph outlines: every glyph run carries the outlines of its glyphs through the stream, and
  the engine hashes their bytes four times a frame (the content hashes of the stream, the
  identity of every glyph resource, the identity of every raster, the payload hash of the glyph
  family).
- After that: recording the render bundle 4.5 percent, submitting and polling the queue 4.6,
  finishing the command encoder 3.6, dropping the trackers of the bundle 3.5, building glyph
  resources 2.9, copying and clearing memory about 6.

| Step | Part | Where | Checked by |
|---|---|---|---|
| 1 | Glyph outlines by identifier (section 2.3.8) and the word-wise hash of the content identities (section 2.3.9): built | engine, builder, bindings, backend | the engine tests of the registry and of keyed runs (the same pixels as the same run with its outlines in the stream); the text tests of the backend; the gauntlet frames of section 6.2 unchanged to the byte; frames a second and render thread time at tier 16 (section 6.2) |
| 2 | The bundles and draws of section 2.3.10, in three parts: spans as command lists with bundles kept by content (built), fewer draws, `update_scene` with changed content only (R4) | engine | the engine tests of the lists, the replay and the kept bundles; a scene drawn again, sent again with other colours and sent again with other draws reads back the bytes of a new engine; the gauntlet frames of section 6.2 unchanged to the byte; draws a frame, frames a second and render thread time at tier 16 (section 6.2) |
| R1 | Groups before the root pass | engine (`Scene/` render execution, layer resources) | the layer and clip tests read back the same bytes; pass and command-buffer counts of a frame with many groups; the gauntlet at tier 16 on a queue of the default capacity |
| R3 | Scenes per target | engine C ABI | two targets on one engine keep their scenes: the second frame of each is a snapshot hit |
| R4 | Spans compiled once, chunked buffers, the placement table | engine (family page compilers, shaders, bundle recording) | read-back equality of moved spans against whole streams for every family, with clips and masks; upload bytes of a frame with one changed span (the metrics of the engine report them) |
| R5 | The delta call | engine C ABI, bindings, backend | the same frames through deltas and through whole streams; validation errors of a bad span leave the scene as it was |
| R6 | The other resources by identifier (paths, images), damage, cached groups | engine, backend | partial redraw scenarios of section 4 |
| R2 | Fragments in the backend against the whole-stream call | backend; builder span calls in the bindings | stream equality with and without kept spans; the backend suite |

Half of the content of a gauntlet frame is new (section 2.3.2), so after step 1 the cost of a
changed visual is the likely next item: the seven passes of `update_scene` over the whole stream
and the pages compiled again from offset 0. R4 and R5 stay designed and are not built before a
profile asks for them. R2 is skipped until a profile shows that the encoding of the backend
matters; today the UI thread and the walk of the compositor together are under a tenth of a
frame. The rotated text of section 2.3.11 is a matter of quality, not of speed, and is scheduled
apart.

Every engine part is a pull request of its own in the ProGPU repository, stacked on the branch
of the bindings, with the tests of the engine; none of them is a change of the bindings alone.
The frame times are measured after each part with the gauntlet and the stage capture of the
engine (`PROGPU_NATIVE_SCENE_FRAME_CAPTURE_CPU_STAGES`: preflight, resources, encode, flush,
finalize), by the procedure of section 4.

#### 2.3.8 Glyph outlines by identifier

**The rule.** A frame does not carry and does not hash the outline of a glyph the engine already
has. The work of a frame for text is a few words for every distinct glyph of a run and one
instance for every glyph drawn; the bytes of an outline are read when it is registered and when
a raster of it is made, never otherwise.

**Where the key lives.** The outline of a glyph is a resource of the engine, registered once
under a key the caller chooses; a glyph run in the stream names its outlines by key.

```c
typedef struct progpu_native_glyph_outline_key {
    uint64_t face;     /* the face at one variation instance, chosen by the caller */
    uint32_t glyph;    /* the glyph of the face */
    uint32_t variant;  /* what else changes the outline: the weight step, a simulated oblique */
} progpu_native_glyph_outline_key;

typedef struct progpu_native_registered_glyph_outline {
    progpu_native_glyph_outline_key key;
    uint64_t segment_offset;   /* into the segments of the call */
    uint64_t segment_count;
    float min_x, min_y, max_x, max_y;
} progpu_native_registered_glyph_outline;

progpu_native_status progpu_native_engine_register_glyph_outlines(
    progpu_native_engine*, const progpu_native_registered_glyph_outline* outlines, size_t outline_count,
    const progpu_native_glyph_segment* segments, size_t segment_count);
progpu_native_status progpu_native_engine_release_glyph_face(progpu_native_engine*, uint64_t face);

/* The outline table of a keyed glyph run in the stream (24 bytes an entry). */
typedef struct progpu_native_scene_glyph_key {
    progpu_native_glyph_outline_key outline;
    float raster_scale;
    float subpixel_x;
} progpu_native_scene_glyph_key;
```

- An outline is immutable under its key, so there is no revision. Registering a key again with
  the same content does nothing; with other content it is refused. The engine validates the
  segments once, at registration, and keeps its own copy.
- The raster scale and the subpixel phase are not part of the registration: one outline serves
  every size and phase. They stay with the run, in its key table.
- A keyed run is a glyph resource of the stream with the flag
  `PROGPU_NATIVE_SCENE_GLYPH_OUTLINE_KEYS`: its payload is the key table, it has no segments.
  The positioned glyphs are what they are today; `outline_index` is the index into the table.
  The builder gets `add_glyph_keys` beside `add_glyph_outlines`; a run with its outlines in the
  stream stays valid and is how one-off outlines are drawn (section 2.3.11).
- The keys are resolved when the scene is rendered, every frame. A key that is not registered
  (or whose face was released) fails the frame with `INVALID_ARGUMENT`; nothing is drawn in its
  place. Whether an engine holds an outline is not a property of a stream, so `update_scene`
  takes a stream with any well-formed key.
- Only a plain glyph run takes a keyed resource. The colour (RGB) and painted glyph draws keep
  resources that carry their outlines; the builder and the validation of the stream refuse a
  keyed resource there.

Why not the two alternatives. A handle given out by the engine would be shorter, but a stream
with handles means nothing without the engine that gave them, and a stale handle would draw
another glyph without an error; a key that is unknown is an error. A key on every glyph instance
would repeat sixteen bytes for every repeated glyph and put the lookup on the instance; the
table keeps it on the distinct glyph. A registration for every scale and phase would multiply
the registrations by the sizes in use without saving a lookup.

**What the atlas key is.** For a registered outline the tile of the atlas is found by
(registry serial, raster scale, subpixel phase): three words, compared as words. The registry
gives every outline a serial counted from one that is never given a second time, and a released
face number is refused afterwards, so a serial stands for the outline bytes and the bounds for
as long as the engine lives. For an outline that comes in the stream the tile is found
as today, by the content: bounds, scale, phase and the segment bytes, hashed word-wise and
compared on a hit. Both kinds of tile live in the one atlas and are dropped together when it
starts again.

**In the engine.**

| Piece | What it does |
|---|---|
| The registry (`glyph_outline_registry`) | key to serial to entry; an entry owns its segments and its bounds. A call registers all of its outlines or none. A released face frees its entries; their serials name nothing afterwards. The registry needs no device and outlives a device loss. |
| Validation of the stream, preflight | a keyed resource is checked for its size and for finite, positive scales and legal phases; its outline count is the table length and its segment count is zero. |
| Content hashes of `update_scene` | cover the key table (24 bytes for a distinct glyph of a run) and the positioned glyphs. No outline bytes. |
| The glyph page of `render_scene` | resolves every key once for a run and maps (entry, scale, phase) to one outline of the frame, so a glyph that 700 runs use is one outline of the frame, not 700. The identity of a keyed resource for sharing is its table. The page keeps, beside its outlines, the entry each one came from. |
| `render_glyphs` | an outline with an entry is not validated again and not hashed: its tile is looked up by the three words. Only an outline without a tile has its segments read, to be rasterized; the segments uploaded for that are the segments of the outlines to rasterize, not of every outline of the frame. The comparison with the payload of the frame before uses the outline records and the serials instead of segment bytes. The payload hash (a diagnostic) covers the instances and the outline records. |

**In the backend.** The outline cache of a face gives every outline its key when it is made
(`face` is a number of the face at one variation instance, counted for the process and never
given a second time; `variant` is the weight step and the oblique bit; the sheared outline of a
simulated oblique is cached like the others). An outline that a run names for the first time is
appended to a log of the process (`outline_log.rs`); an engine registers what the log gained
since it read it last before it takes a stream (4,096 outlines a call at most), so a second
engine or an offscreen target learns the same outlines without the runs knowing of engines, and
an engine made later reads the log from its start. A face that is dropped takes its outlines
out of the log and appends its release. A run keeps its key table with its placement and sends
24 bytes for a distinct glyph instead of its outline.

- A scene holds the faces its glyph runs name until its next generation begins, so a release
  never reaches an engine before the scene that draws the face was rendered. A scene the engine
  keeps is not rendered again after its builder began the next generation.
- When the engine refuses a registration call, its outlines are registered one at a time and
  the refusal is noted once; a frame that draws an outline the engine did not take fails, as any
  frame with an unknown key.
- What this costs in memory: the log holds every outline that runs named of the faces that
  are alive (the same allocations the faces cache), every engine holds its own copy of them, and
  a dropped variation instance leaves one small release entry in the log and one face number in
  the released set of every engine. The atlas keeps the tiles of a released face until it
  starts again.

**What stays content-hashed.** The stream and its families (the retained snapshot of
`update_scene`), outlines that come in the stream (rotated text, section 2.3.11; hosts that do
not register), colour glyph bitmaps, paths, images and the family pages. Section 2.3.9 is about
their hash; spans (R4, R5) are what later stops unchanged content from being read at all.

#### 2.3.9 The hash of the content identities

The content identities of the engine (the stream, the families, glyph and raster identities,
payload hashes) are FNV-1a over single bytes (`append_fnv1a64`): one multiplication for a byte.
They become a word-wise 64-bit hash: eight bytes a step, a multiplication and a fold for a
word, the length and the last bytes mixed in at the end of a range. It is an identity inside one
process, not a format.

Checked before the change, over the engine, its tests, the managed code and the documents:

- No test compares a value of this hash with a constant. The engine tests check that it is
  deterministic and that it is equal between two paths of the engine. The constants in the text
  and font tests belong to other loops with another start value.
- No value is written to a file or read from one, and no header describes the algorithm.
- The values are visible outside in one place: the `payload_hash` of the frame metrics (scene,
  geometry, path, glyph, image; asked for with a capture flag). They change. One diagnostic
  program of the package consumer computes the geometry payload hash again and prints both
  beside each other without comparing, and one diagnostic note quotes a printed value; both are
  stated in the pull request instead of being changed silently.
- The MIL module has its own copy of the loop and keeps it.

Because the new hash is not the same for a range hashed in one call and in two, an identity
must be made by one piece of code; the callers are read for that when they are moved.

#### 2.3.10 Bundles and draws

What is left of the profile after the hash is the cost of recording. The trace of the engine
(`PROGPU_NATIVE_TRACE_SCENE_ENCODE=1`) for a gauntlet frame at tier 16 after step 1:

| A frame at tier 16 | After step 1 |
|---|---|
| Commands / draws / changes of family | 11,600 to 13,000 / 8,750 to 9,760 / 6,620 to 7,310 |
| Spans / render bundles created and finished | 9,230 to 9,350 / 9,130 to 9,240 |
| Finishing the bundle encoders | 16.7 to 17.1 ms |
| Creating the encoders, recording the draws, releasing the encoders | 1.0, 0.8, 0.24 ms |
| Mask bindings (all for rounded clips) | 717 to 822, 5.7 to 6.4 ms |
| The phases: resources, prepare, bundles, replay, flush | 6.9, 4.3, 26.8, 0.45, 29.7 ms |

A span ends whenever the scissor, the target layer, the mask or the use of depth changes, and
nearly every visual of the scene has a clip of its own: a bundle holds one draw. That decides
the three parts:

**Part 1, built: spans as command lists, bundles kept by content.** Keeping one bundle per span
across frames would not pay with one draw in a bundle: a bundle that is found again still costs
its lookup, one `ExecuteBundles` call and the state of the pass set again after it, more than
the draw it holds. So:

- A 2D span records its draws as a list of 64-bit command words (set pipeline, set bind group,
  set vertex buffer, set index buffer, draw, draw indexed) instead of a bundle. Nothing of wgpu
  is called for a span until the pass replays its list, and the replay sets only the state that
  differs from what the pass holds. A list holds one reference on every object it names.
- A span with at least eight draws gets a bundle made from its list, when the compile before
  met the same list or when the scene is drawn again unchanged. The bundle is kept by the
  content of the list (the hash of its words and the target format, compared word for word)
  across `update_scene`; what a completed compile did not ask for is released at its end. Eight
  follows from the cost above and is not tuned by measurement; no span of the gauntlet reaches
  it, so kept bundles are covered by the tests of the engine only.
- A list names objects and ranges, not what the objects hold. An update that changes only the
  content of buffers (a colour, a position within the same ranges) leaves a list equal, and its
  span goes on by the kept bundle. A list binds whole buffers and leaves the ranges to its
  draws, so content added or removed elsewhere leaves the lists of the other spans alone.
- 3D spans keep their immediate bundles.

**Part 2: fewer draws.** What splits the draws of the gauntlet, in this order: the scissor of
every visual, the alternation of families (a rectangle, then its text, then the next
rectangle), and the mask binding of a rounded clip. The engine already merges neighbouring
draws of one family in a span when their ranges follow each other, so the work is to let more
draws be neighbours.

**Part 3: `update_scene` with changed content only** is R4 of section 2.3.7.

#### 2.3.11 Text under a rotation or a skew, and simulated bold

**Rotated and skewed text.** Today a run under a rotation, a skew or an uneven scale is
rasterized upright at the scale of its transform and drawn through the transform with a linear
filter. That is visibly rougher than the reference (the rotated cards of the gauntlet, section
6.2) and does not meet the goal. The glyph and the path family of the engine both rasterize
coverage in the axes of the outline they are given; neither has a mode of its own for the device
orientation. The fix therefore gives the engine the outline in device orientation:

- The backend transforms the weighted outline of a glyph by the linear part of the device
  transform (rotation, skew and scale together), with the phase of its place in quarter pixels
  in both directions, and sends it as an outline of the stream at raster scale 1. The weight is
  applied before the transform, in the axes of the glyph, as for upright text.
- The glyph is drawn at a whole device pixel under the identity state, texel for texel, exactly
  as upright text is. Nothing is resampled.
- The tiles are found by content, so a rotated label that stands still is rasterized once. Text
  that turns in every frame is rasterized in every frame, and its outline bytes travel in every
  frame: that is what a rotation costs in a direct-mask text renderer as well.
- The atlas must not be emptied by turning text. Its tiles are packed on shelves and the atlas
  starts again when it is full; tiles of one-off outlines go to a region of their own that
  starts again without touching the region of the registered glyphs.
- Above the size where the mask of a glyph no longer fits a tile of the atlas, the run is drawn
  as a path fill of its outlines.

Acceptance: the difference image against the Skia backend on the rotated cards of the gauntlet
and on a rotated and a skewed line of the text scene of section 6.1, with the numbers of the
upright lines as the bar (no visibly rougher edges; mean and counts above 8 and above 32 of 255
in the range of the upright text).

**Simulated bold.** The rule is the same as for the weight of text: match what the platform text
shows, by measurement, or state where and why it differs. Today it is two strikes of the
weighted outline 1/24 em apart, 1.19 to 1.42 times the ink of the reference, and the reference
showed no emboldening on those lines. The work: establish what the reference draws for a face
with the bold simulation (the outline widening it asks for, and whether its rasterizer applies
it for this face and size), measure it on stems and rounds as the weight was measured, and
either grow the outline by that measure with `embolden_outline` (one outline, one registered
variant, no second strike) or document the exact difference in section 2.4.1.

#### 2.3.12 What this design does not settle

- **Draw calls.** Spans compiled in order are adjacent and stay merged; after many replacements
  the ranges of neighbours are apart and a run falls into more draws. Compacting a chunk (moving
  ranges and their index bases) is the answer if the counts show it; it is not in the first
  version.
- **Fractional movement.** A visual that moves by fractions of a pixel in every frame gains
  nothing from its span. The gauntlet scrolls on whole device pixels (section 2.3.2); a scene
  that does not would need the layout to land on pixels, which is not a matter of the backend.
- **Rotating content.** A rotated span is encoded and compiled again in every frame of the
  rotation (a tenth of the gauntlet's commands), and with section 2.3.11 its glyphs are
  rasterized again as well.
- **Memory.** The registered outlines, compiled spans, the chunks and the group pages are
  budgets beside the existing ones; the numbers are chosen with the catalog and the gauntlet and
  reported by the engine. Registered outlines leave with their face; nothing else evicts them.
- **The generated managed contract.** The new records of `progpu_native.h` carry the
  annotations of the managed generator; the generated file is not produced or checked here.
- **The browser.** A storage buffer read in the vertex stage is part of WebGPU; the compatibility
  mode without it is not a target of this backend (section 2.6).

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
| `IGlyphRunImpl` | `glyph_run.rs` | The glyphs of a run with their positions, the face and its instance, and the bounds of the ink (the design box of the face at each glyph, widened for the simulations and the text weight). The geometry of a run (`build_glyph_run_geometry`) is the outlines of its glyphs. A run keeps what the engine draws it from (outlines with scale and phase, places) while its placement stays. |

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

### 2.4.1 Drawing glyphs

**The atlas.** The engine rasterizes glyph coverage on the GPU into one R8 atlas and keeps it
across frames. A tile is found by the exact identity of its raster: the outline (which stands for
the face, its variation instance, the glyph, the text weight and a simulated oblique), the raster
scale and the horizontal subpixel phase. The backend registers every outline with the engine once
and a frame names it by key (section 2.3.8), so the identity is three words and no outline byte
travels with a frame or is hashed for a glyph the atlas holds. An outline that is in the atlas is not rasterized again
and keeps its tile, so a frame with one new glyph rasterizes that glyph only (tested in the engine
and in `text_draw_tests.rs`: the raster count of a second frame with one more glyph is one). The
atlas grows by doubling its edge up to the texture limit of the device, copying its content; when
it is full at its largest size it starts again with the glyphs of the current frame. Nothing is
evicted one by one yet. The transfer from coverage to alpha is applied when a glyph is drawn, not
when it is rasterized, so one tile serves every colour.

**Placement.** Upright text at one scale is drawn texel for texel: every glyph lies on a whole
device pixel, the quarter pixel its origin falls on is the phase its raster is made for, and the
baseline is on a whole pixel (the Skia backend snaps the baseline the same way). The atlas is
sampled at texel centres there, where the linear filter of the engine returns the texel unchanged,
so nothing is blurred. Text under a rotation, a skew or an uneven scale is rasterized at
the scale of its transform, in steps of an eighth of a doubling so an animated scale does not fill
the atlas, and drawn through the transform with linear filtering.

**The weight of text.** The text the Skia backend draws on macOS (unhinted, subpixel positioned,
greyscale antialiasing) is heavier than the outlines of the font. Measured on stems, bars,
diagonals and rounds from 8 to 96 px an em, the ink is the outline grown by a box: every edge lies
0.0054 em further out sideways, an edge that faces up lies twice as far up, and an edge that faces
down stays where the outline has it; from about 54 px an em the growth levels off at 0.29 px. The
engine grows outlines by such a box exactly (`embolden_outline` of the text module: the Minkowski
sum of the contour and a box with four free sides, traced as their convolution; curves are cut at
their turning points and each piece moves to the corner of the box its normal faces, so curves
stay curves and right angles stay right angles). The backend asks for the grown outline once for a
glyph and a weight and caches it; aliased text and outline geometry keep the plain outlines. No
algorithm of another renderer is used: the growth is what the measurement shows, and the measured
numbers are the constants.

**The coverage transfer.** Greyscale text is drawn with alpha = 1 - (1 - coverage)^e. The exponent
depends on the lightness of the text colour, in four steps (0.94, 1.16, 1.47, 1.81 from dark to
light, by a quarter of red and three quarters of green): the reference gets heavier the lighter
its colour is. The steps were measured on 25 colours.

**The quality check.** The example `progpu_text_quality` draws one text scene with the Skia
backend (CPU raster surface) and with this backend into offscreen targets, twice with this
backend (shaped by the engine and shaped by HarfBuzz, to separate shaping from rasterization), and
writes the images, difference images, enlarged crops of the small sizes and the numbers. The scene:
Latin regular and bold at 9 to 32 px (Inter), Arabic, Hebrew and Japanese at several sizes, an
accent colour, simulated bold and oblique, dark on light and light on dark. Results are in
section 6.

**Where the text of this backend is worse or different.**
- Edges are placed in eighths of a pixel in both directions: the rasterizer of the engine takes 8
  sample rows with 8 evenly spaced columns each, so the coverage of an edge that is nearly
  vertical or horizontal moves in steps of 32 of 255. This is the largest part of the remaining
  difference (about 8 of 255 on edge pixels, most visible at thin horizontal strokes: Japanese at
  12 px is the worst line of the scene). Staggering the sample columns from row to row would give
  64 distinct horizontal positions at the same cost; the vertical direction needs more rows or an
  analytic coverage. The shader is shared with the managed backend of ProGPU, so the change is a
  proposal, not part of this step.
- Small round glyphs (8 to 20 px) come out 3 to 5 percent heavier than the reference; straight
  and diagonal strokes are within 2 percent.
- Simulated bold is two strikes of the weighted outline and is much heavier than the reference
  (ink 1.19 to 1.42 times). The reference image shows no emboldening on those lines although the
  Skia backend asks for it; which of the two is intended was not decided here.
- Simulated oblique is 1 to 2 percent lighter than the reference.
- Only the horizontal phase is kept (quarter pixels); a glyph is up to an eighth of a pixel from
  its exact place horizontally, and vertically it is on the snapped baseline.
- The colour rule of the transfer is sure for greys and for the tested accent colours; saturated
  blue and magenta were measured as dark, which was not examined further.
- Subpixel (LCD) text is drawn as greyscale. Text filled with a gradient or an image is a path
  fill of the plain outlines (not weighted). Text under a rotation is filtered linearly and its
  raster scale is quantized; section 2.3.11 is the design that replaces this.
- The reference is the CPU rasterizer of Skia, not its GPU text path.

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
decoder, with a C ABI (decided by the owner, 2026-11-05). The backend links no other decoder.
Done: PNG files and streams, through the image C ABI of the bindings
(`progpu_native_sys::image`: the size of an image, then its pixels as straight RGBA8 into a
buffer of the caller), decoded at their size, to a width or to a height, and resized. Open: the
other three decoders (a file of another format is refused as unsupported) and encoding (the image
module has no encoder, so `save` of a bitmap is unsupported). The
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

**Clips.** A clip costs a texture only when its shape needs one:

| Clip | How it is recorded |
|---|---|
| Rectangle, under a transform that keeps it axis aligned | the clip rectangle of the state (a scissor) |
| Rectangle or rounded rectangle under any other transform (a rotation, a skew) | an analytic mask of the state: a rounded rectangle with its own transform, evaluated in the shader, no texture |
| Two to four such clips inside each other | one chain of analytic masks (`ANALYTIC_CHAIN`), still no texture |
| A fifth analytic clip, or an analytic clip inside a geometry clip | a layer with the new mask; the content inside it is drawn into the layer |
| Geometry clip | a mask texture of the size of the target, generated from the path, within the budget of the engine (256 MB a frame); a frame over the budget fails and is logged |

A clip without an area (an empty rectangle, a geometry with empty bounds, a transform that is not
invertible) clips everything; a path without an area is not drawn. Both would be refused by the
scene builder as invalid records.

**The end of a thread.** In a build with debug assertions the WebGPU implementation keeps the
state of one of its locks in a thread-local, and releasing a device needs that lock. The objects of
a render context are released when the thread-locals of its thread are destroyed, in an order the
backend does not choose; a release after the lock state is gone would abort the process. The
backend therefore never calls the WebGPU implementation while the locals of the calling thread are
being destroyed: the device, its engines and its textures are then left to the end of the process
(`device.rs`, `thread_is_ending`). The detection is a thread-local mark registered after the first
call into the implementation on the thread, which the reverse order of destruction puts before the
lock state. Objects released while the thread runs are released as usual.

**Not drawn yet, said once in the log** (area `ProGPU`, `notes.rs`): bitmaps drawn under an
opacity mask, inset shadows and the shadows of rectangles with different corner radii, and a clear
under a rotated, rounded or geometry clip. Content under an opacity mask is drawn without the
mask. The shadow of a translucent element under such a clip is drawn whole, also under the
element (the state has one mask, and the clip keeps it). Image files other than PNG are refused
when they are loaded, and bitmaps are not encoded (section 2.5).
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
device, a compositor frame, the safe failures, the end of a thread, PNG files, rotated and nested
clips, a frame of 256 doubly clipped cells, a frame of 256 translucent groups on the queue of the
platform). The example
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
   - a **delta update**: spans, resources and placements replaced by identifier in a retained
     scene without sending the stream again, a retained scene for each target, and render bundles
     that are recorded again only for the run a change is in (the "family-granular retained
     update" the baseline lists as not implemented); the design is section 2.3;
   - the content of translucent groups rendered before the root pass (section 2.3.5);
   - surface helpers for the native handles (creating and configuring a WebGPU surface for a
     `CAMetalLayer`, an `HWND`, an X11 window, a Wayland surface);
   - the C ABI of the native Geometry module (section 2.5).
4. **Image decoders** in the native image module beside PNG: JPEG (baseline and progressive), GIF and
   BMP, bounded and allocation-checked as the PNG decoder. The C ABI of the PNG decoder is in the
   bindings (`progpu_native_image_png_read_size`, `progpu_native_image_png_decode_rgba`).

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
- **The glyph cache was per frame payload** (done). The glyph atlas was kept only while the
  outline array and the segment arena of a frame were byte-equal to those of the frame before.
  The scene path now keeps an atlas cache by raster identity (outline bytes, bounds, scale, phase)
  across frames (section 2.4.1). A frame still hashes the outline bytes of its glyphs to find
  their tiles; glyph resources named by id (P6) remove that.
- **Text quality** (matched by measurement; section 2.4.1 and section 6). Coverage is 8x8
  supersampled on the GPU into an R8 atlas; the subpixel phase is horizontal only and
  quarter-pixel; the subpixel text mode samples the grayscale atlas three times and is not a
  filtered LCD raster (the separate RGB glyph run is one, for opaque targets and whole-pixel
  positions only). Added for the match: text style transfers 14 to 78 (an exponent given
  directly) and outlines grown by a box (`embolden_outline`, exported through the font C ABI as
  `progpu_native_font_outline_embolden`); the backend places upright text texel for texel. Open:
  the regular 8x8 sample grid (edges in eighths of a pixel).

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
| P4 | Text: font manager, shaper, glyph runs, system fonts and fallback | the text tests of the backends; the text pages of the catalog | done on macOS: the font manager on the catalog of the engine, typefaces, the shaper of the engine checked against HarfBuzz, glyph runs with bounds and outline geometry (section 2.4), glyphs drawn from the persistent atlas at the measured weight of the platform text (section 2.4.1, numbers in section 6); open: the sample grid of the rasterizer, LCD text, `get_intersections` |
| P5 | Images and bitmaps, render target bitmaps, writeable bitmaps, effects (blur, drop shadow, the custom effects of #455 where the engine has them), acrylic | the effect and image tests | in progress: PNG files and streams decode through the image module (section 2.5); render target bitmaps, layers and writeable pixel bitmaps are in P3; open: JPEG, GIF and BMP, encoding, effects |
| P6 | Retained mode: fragments as spans of a retained scene, placements, scenes per target, groups before the root pass, damage | stable frames submit without re-encoding; catalog measurements | in progress, in the order of section 2.3.7: glyph outlines by identifier and the word-wise content hash are built (section 2.3.8), then spans as command lists with bundles kept by content (section 2.3.10); numbers in section 6.2 |
| P7 | ProGPU bindings: `progpu-native-sys`, `progpu-native` with the Rust writer and the differential test, the C ABI additions (font catalog, delta update, span bundles, surface helpers), in the ProGPU repository | merged in ProGPU; the backend on them | in progress: `progpu-native-sys` (the engine built from the sources, wgpu-native linked statically, a C ABI over the scene builder), the Geometry module with its C ABI, analytic drop shadows, owner-thread adoption; the Rust writer and the other C ABI additions are open |
| P8 | Windows (Direct3D 12) and Linux (Vulkan) surfaces, in the VMs | catalog smoke runs | open |
| P9 | Browser: the `WebGPU` mode of `ferroui-browser` on the Emscripten build of the engine | the browser catalog on WebGPU | open |
| P10 | Measurements and size (section 4), the documentation, the NOTICE files | the tables of section 6 | open |

## 6. Results

### 6.1 Text against the Skia backend (macOS)

`cargo run -p ferroui-progpu --example progpu_text_quality -- <folder>` writes the two images, the
difference image and `text-quality.txt`. Differences are per channel of 255 over the whole panel
(text is about 18 percent of its pixels); a text pixel is one that is not background in one of the
two images; the ink ratio is the summed coverage of this backend over that of Skia.

| Panel (real faces) | Max | Mean (R, G, B) | Above 8 | Above 32 | Text pixels: mean | Text pixels: above 8 | Ink ratio |
|---|---|---|---|---|---|---|---|
| Dark on light, plain outlines and the fixed transfer (before the match) | 169 | 3.21, 3.08, 2.99 | 9.82% | | | | 0.899 |
| Light on dark, before the match | 179 | 5.90, 5.96, 6.06 | 11.39% | | | | 0.784 |
| Dark on light | 102 | 1.42, 1.38, 1.34 | 6.49% | 0.55% | 7.80, 7.56, 7.37 | 35.7% | 1.002 |
| Light on dark | 117 | 1.13, 1.16, 1.19 | 5.07% | 0.45% | 6.23, 6.35, 6.53 | 27.9% | 1.005 |
| Both | 117 | 1.28, 1.27, 1.27 | 5.78% | 0.50% | 7.01, 6.95, 6.95 | 31.8% | 1.004 |

By line the ink ratio is 0.986 to 1.020 for every real face and size (Latin regular and bold 9 to
32 px, Arabic 11 to 28, Hebrew 11 to 20, Japanese 12 to 24, the accent colour). The mean
difference on text pixels is 4 to 10 of 255 except Japanese at 12 px (19 to 20, maximum 117: thin
horizontal strokes between sample rows). The lines of simulated styles are counted apart:
simulated bold is 1.19 to 1.42 times the ink of the reference (mean on text pixels 35 to 47),
simulated oblique 0.982 to 0.996. Shaping with HarfBuzz instead of the engine gives the same
numbers, so the difference is rasterization, not shaping. The centroids of the lines agree within
0.01 px in both directions at every size.

At the size of the screen the two renderings are not told apart by eye in the enlarged crops of the
9 to 14 px lines; the difference image shows thin outlines at glyph edges, strongest at the
Japanese 12 px line and the simulated bold lines.

### 6.2 The gauntlet against the Skia backend (macOS)

The gauntlet sample runs on this backend when it is built with `--features progpu` and started
with `FERROUI_RENDERER=progpu`; the switch is in the harness part of the sample
(`samples/GauntletBenchmark/program.rs`), the scene is untouched:

```
cargo build --release -p gauntlet-benchmark --features progpu
FERROUI_RENDERER=progpu PERF_TIER=16 PERF_FONTS=<folder of Roboto-Regular.ttf and Roboto-Bold.ttf> \
    target/release/perf-compare-ferroui
```

The same binary without `FERROUI_RENDERER` runs on Skia. The window was captured on both backends
at the frozen frame 90 (`PERF_FREEZE=90`), 2560 x 1704 device pixels, title bar included.
Differences are per channel of 255 over the whole capture:

| Tier | Max (R, G, B) | Mean (R, G, B) | Above 8 | Above 32 | Translucent groups in the frame |
|---|---|---|---|---|---|
| 5 | 122, 118, 110 | 0.42, 0.42, 0.40 | 1.79% | 0.15% | 3 to 4 |
| 16 | 121, 107, 109 | 0.59, 0.61, 0.60 | 2.87% | 0.26% | 56 |

No frame failed and the backend reported nothing that it left out, at either tier. Every element
of the scene is there on both: cards with shadows, avatars under rounded clips, the tilting
badges, sparklines with gradient fills, progress bars, the rotating panels with their clipped
cells. The difference image shows thin outlines: glyph edges, the edges of the sparklines and of
rounded shapes, the edges of the rotated cells. What is worse on this backend, seen in enlarged
crops: text under a rotation (the titles and numbers of the rotating panels) has rougher edges
than on Skia, because a glyph is rasterized upright and resampled (section 2.4.1); upright text
and the shapes are not told apart by eye.

**Frame rate at tier 16** (the animated scene, the same window, a quiet machine; frames a second
as the sample reports them once a second, the render thread from a 10 s sample of the process).
The target is the cap of 60.

| State of the backend | Frames a second | Render thread busy | Render thread a frame | Captures at the frozen frame |
|---|---|---|---|---|
| Before the retained work (outlines in every frame, byte-wise hash) | 2.6 to 2.9 | 99% | about 340 ms | the reference of the table above |
| Step 1: glyph outlines by identifier, word-wise hash | 8.0 to 10.7 (mean 9.0 over two runs) | 96% | about 107 ms | the same bytes as before at tier 5 and at tier 16 |
| Step 2, part 1: spans as command lists (section 2.3.10) | 11.5 to 15.9 (mean 13.1) | 94% | about 72 ms | the same bytes as before at tier 5 and at tier 16 |

What a frame of the engine holds and costs (the counts of the backend, `FERROUI_PROGPU_PERF`,
means over 30 frames; the animation changes the amount of content, so every number is a range):

| A frame at tier 16 | After step 1 | After step 2, part 1 |
|---|---|---|
| Draws | 8,750 to 9,760 | 8,670 to 9,750 |
| Render bundles created | 9,130 to 9,240 | 0 |
| `update_scene` | 6.6 to 8.4 ms | 6.3 to 7.5 ms |
| `render_scene` | 63 to 78 ms | 32 to 38 ms |
| of it: preflight / resources / encode / flush | 3.4 to 4.3 / 5.5 to 7.5 / 27.9 to 33.2 / 26.4 to 33.3 ms | 3.3 to 4.0 / 5.8 to 7.1 / 10.6 to 12.3 / 12.4 to 14.9 ms |
| Stream / geometry upload / texture upload | 13.4 to 16.0 / 8.0 to 9.5 / 3.4 to 4.3 MB | 12.9 to 15.8 / 7.8 to 9.4 / 3.3 to 4.2 MB |

Where the render thread was after step 1, in percent of its samples (inclusive): rendering the
scene in the engine 63, of which recording the render bundle 15, submitting the queue 15,
finishing the command encoder 11, dropping the trackers of the bundle 10; the walk of the
compositor with the backend's recording 19 (glyph runs 6, geometries 8); `update_scene` 7; the
content hash 6 (it was 61); the glyph page of the engine 3. The UI thread was busy for 23 percent
of the time.

Where the render thread is after part 1 of step 2, in percent of its samples (inclusive):
rendering the scene in the engine 46 (finishing the command encoder 11, the mask bindings of
rounded clips 7.5, submitting 5, texture upload 3, the glyph page 2); the walk of the compositor
with the backend's recording 28 (geometries 12, of which widening strokes 5 and path bounds 3;
glyph runs 6); `update_scene` 9 (the content hash 4); the bounds of the visuals before the walk
6. The UI thread is busy for 34 percent of the time. The draws are as many as before; they are
part 2. A frame of 60 a second has 16.7 ms, and the walk of the compositor with `update_scene`
alone is about 26 ms: the cap is not reached by the engine side alone, the scene has to stop
being recorded and sent whole (R2, R4, R5 of section 2.3.7).

**The counts of a frame.** `FERROUI_PROGPU_PERF=<frames>` makes the backend print, every so many
frames of a window, the means of what its scenes held and cost: scenes, commands, draws, changes
of family, submissions, the bytes of the stream and of the uploads, and the times of
`update_scene` and `render_scene` with the stages of the engine. It is off by default and then
costs one load of a flag a frame.

## 7. Licensing

ProGPU is MIT (Copyright (c) 2026 Wiesław Šoltés). The SDK package ships the notices of its
optional static dependencies (FreeType, ICU) and the browser build those of Emscripten and
Emdawnwebgpu; the NOTICE files of `ferroui-progpu` and of the browser build carry them. ProGPU's
Unicode data tables need the Unicode license notice, which is checked in P10.
