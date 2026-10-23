# Vello backend performance: the record of the work (2026-10-19 to 2026-10-20)

This document is the technical record of the effort to make the Vello backend (`src/Vello/FerroUI.Vello/`) as fast as the Skia backend. It covers the method, every change that was made and measured, what the measures say about where the time goes, the experiments that were not kept, and the designs of the work that is left. It is written so that the work can be taken up again without the people or the sessions that did it.

`vello-backend.md` is the design document of the backend; its sections 14 and 15 hold the round-by-round tables in full. This document summarises them, adds what is not there (the cost model, the experiments not kept, the open designs, the tools), and is the place to start.

## 1. Status

The work is concluded. Skia stays the default backend: its speed is enough for now.

- **Merged.** Rounds 1 to 5 are on `main`: #263, #264, #265, #266 and #267.
- **Experimental round.** It is in #268 (branch `vello-renderer-experimental`):
  - a depth buffer for the hybrid renderer;
  - the Metal queues ordered by a shared event;
  - fragments in the GPU mode;
  - the design of retained texture layers.
- **What the backend uses.** The renderer crates are the published ones (`vello_cpu`, `vello_gpu`, `vello` from crates.io); nothing in the repository patches them (section 8.1).

Where the backend stands against Skia, with the latest measures (milliseconds a frame, medians):

| Platform and surface | Scenario | Skia | Vello | Faster |
|---|---|---|---|---|
| macOS, Metal, M3 Pro, 2560 by 1600 | pointer across buttons (partial redraw) | 0.60 to 1.14 | 0.73 (hybrid) | Vello or even |
| | effects page, every box / one box | 9.75 / 3.86 | 7.65 / 3.13 (hybrid) | Vello |
| | scrolling 20 px a frame (full redraw) | 0.92 to 1.59 | 1.73 to 1.80 (hybrid) | Skia |
| | text page drawn again | 0.69 | 1.22 to 1.26 (hybrid) | Skia |
| | catalog tour, second round (total ms) | 466 to 483 | 853 (hybrid) | Skia |
| macOS, software (raster surface) | every scenario but one (a tie) | 1.48 to 77.38 | 0.56 to 20.12 (CPU) | Vello |
| Ubuntu 24.04 VM, OpenGL (GLX on virgl) | catalog smoke run | 1.71 to 2.55 | 2.54 to 2.74 (hybrid) | about even |
| Ubuntu 24.04 VM, software | catalog smoke run | 14.49 | 13.82 (CPU) | Vello |
| Windows 11 VM, ANGLE on Direct3D 11 | catalog smoke run | 7.01 to 7.46 | 4.33 to 4.73 (hybrid) | Vello |
| Windows 11 VM, software | catalog smoke run | 7.50 | 5.82 (CPU) | Vello |

What remains slower is a full redraw on a real GPU: scrolling, a page that is drawn again, the tour. Section 7 explains why, and section 9 has the designs that would close it.

## 2. Goal and decisions

| Date | Decision |
|---|---|
| 2026-10-19 | Goal: the three modes of the backend (CPU, hybrid, GPU) as fast as Skia or faster on macOS, Windows and Linux; analyse first. |
| 2026-10-20 | Finish and merge the open rounds, then make experimental renderer changes. Skia must not regress, and the public API of the port stays stable. Experimental work goes into a draft pull request. |
| 2026-10-20 | Changes to the Metal path of the backend (option 2): made. The GPU mode left as it was (option 3), until the next decision. |
| 2026-10-20 | **No patched Vello.** The published crates are used as they are; what changes is how the backend and the compositor drive them. The patched crates of section 8.1 were removed. |
| 2026-10-20 | Build the way Xilem keeps the drawing of its widgets (cached scene fragments) in the GPU mode and measure it against Skia and the hybrid mode. If it wins, it becomes the default on GPUs with compute shaders. It did not win (section 5.6). |
| 2026-10-20 | Stop: document the retained texture layers, merge, conclude. |

## 3. How a frame reaches the screen

### 3.1 The three modes

| Mode | Renderer | Where the coverage of a path is computed | Where it is composed |
|---|---|---|---|
| CPU | `vello_cpu` (sparse strips), multithreaded for targets of 512 by 512 or more | processor | processor |
| hybrid | `vello_gpu` (sparse strips; called `vello_hybrid` before 0.3) | processor, while the scene is recorded (strips) | GPU (vertex and fragment shaders) |
| GPU | `vello` (classic, compute shaders) | GPU | GPU |

The sparse-strips renderers have no way to keep the strips of a scene: `vello_gpu` and `vello_cpu` removed their recordings in 0.0.8. Classic `vello` can append one scene to another with a transform (`Scene::append`).

### 3.2 Windows and frames

- **The kept window frame.** A window is drawn into a frame the backend keeps (`gpu/window_frame.rs`, `WindowFrames`), and each frame of the platform gets a copy of it. The compositor draws only what changed (its dirty rectangles): the render target says it retains the previous frame and can be drawn into directly.
- **The window device, `create_gpu` (`gpu/mod.rs`).** It takes, in this order:
  - the Metal device of the platform (macOS);
  - a hardware device of the backend's own on Vulkan (Linux) or Direct3D 12 (Windows), with a `wgpu` surface made from the handle of the window (`gpu/window_surface.rs`);
  - `wgpu` on the OpenGL context of the platform: ANGLE, GLX or EGL (`gpu/gl_context_gpu.rs`, round 5);
  - on Linux, OpenGL through EGL on a context of its own.

  Software adapters (lavapipe, WARP) are refused unless `FERROUI_VELLO_ALLOW_SOFTWARE_ADAPTER=1`: the CPU mode is faster there.
- **Without a device.** A window is drawn in the CPU mode into a framebuffer that keeps its frame (`framebuffer_render_target.rs`), or presented through the OpenGL context of the platform (`gl_window.rs`, `VelloGlWindows`).
- **On macOS.** The backend's device is a `wgpu` device made over the Metal device of the platform, with a command queue of its own (`gpu/metal.rs`). The platform presents on its own queue.

## 4. Method

### 4.1 Benchmarks

The frame benchmarks of the catalog (`samples/ControlCatalog/tests/frame_benchmark.rs`) run in an optimised build:

- the buttons page with the pointer moving across the buttons (a partial redraw);
- the same page scrolled 20 pixels a frame (a full redraw);
- the text page drawn again;
- the effects page, with every box and with one box drawn again;
- two rounds of a tour of every page.

The surface is Metal and not on screen, 2560 by 1600 pixels (scaling 2), on an Apple M3 Pro. The raster surface measures software.

- **Two figures a frame.** One with the device waited for, as a test needs. The other without the wait, which is what a window on screen pays on the processor: the surface counts its waits.
- **Variables.**
  - `FERROUI_TEST_RENDERER`: `skia`, `vello-hybrid`, `vello-gpu`, `vello-cpu`.
  - `FERROUI_BENCH_SURFACE=metal`, and `FERROUI_BENCH_SCALING=2`.
  - `FERROUI_VELLO_PERF=<ms>` prints the phases of a frame of the backend.
  - `FERROUI_FRAME_TIMES=1` prints the frame times of every composition target, for any backend: the measure of a window on screen.

### 4.2 Rules that the measures taught

- **Run the test binary, not `cargo test`.** `cargo test` builds the sample crates again through their build scripts on every run (four minutes each). That load made the first Skia baseline two times too slow, and the comparisons of the first tables had to be corrected (`vello-backend.md`, section 14.5).
- **One build, every renderer in one session, Skia first and last.** The two Skia runs show how much the machine drifted; they differed by up to a factor of 1.7 on partial redraws in some sessions, which is the noise of a comparison.
- **An A/B of two builds runs both binaries in one session.** Copy the binary of the first build aside before building the second (`scripts/vello-perf/bench-bin.sh`).
- **Pixels before speed.** Every change was checked with the comparison harness (`tests/FerroUI.RenderBackends.Comparison`). It draws 32 scenes in every mode and compares them with Skia and with the CPU mode within bounds per scene. It caught one real bug in the experiments (section 8.1).

### 4.3 Profiling

- **The processor.** `/usr/bin/sample` of a running benchmark, summed per function for the thread of the test (`scripts/vello-perf/inclusive.py`).
- **The GPU.** A Metal System Trace (`xcrun xctrace record --template 'Metal System Trace'`), its `metal-gpu-intervals` table exported and summed per encoder (`scripts/vello-perf/gpu_intervals.py`; the commands are in the script).

### 4.4 Linux and Windows

These were measured in Parallels virtual machines on the same Mac, one machine at a time:

- Ubuntu 24.04 ARM64: OpenGL accelerated by the host through virgl; Vulkan is only lavapipe.
- Windows 11 ARM64: Direct3D 11 accelerated, which ANGLE uses; Direct3D 12 is only WARP.

The measure is a smoke run of the catalog that shows every page for 150 ms with `FERROUI_FRAME_TIMES=1`, at a scaling of 2 (`scripts/vello-perf/vm-linux/`, `scripts/vello-perf/vm-windows/`, with `scripts/windows/vm-sync.sh` for the sources).

Parallels has no GPU driver that gives these machines hardware Vulkan or Direct3D 12, and writing one is not feasible. A machine with a real GPU was not available, so the paths of Vulkan and Direct3D 12 on hardware are built but not measured.

## 5. Changes, round by round

Every round is a pull request with its measures; `vello-backend.md` has the full tables.

### 5.1 Round 1 (#263): the ways the renderers have for rectangles and text

1. **Rectangles and rectangle clips go the renderers' own way** (`fill_rect`, `push_clip_rect`), without strips. An aliased rectangle is snapped to whole pixels, which the anti-aliased way paints exactly.
2. **The hybrid mode draws text from the glyph atlas of its renderer.** A glyph is rasterized once at a quarter of a pixel across, as Skia draws text.
3. **The CPU mode keeps its render contexts and the renderer's resources** from scene to scene (`RendererPool`).
4. **A kept target that is drawn again in part is rendered there only** (CPU mode, `retain_target` with an offset).
5. **The CPU frame of a window is uploaded only where it changed.**

Hybrid, pointer across the buttons: 1.05 to 0.91 ms. CPU mode: 4.26 to 0.55 ms.

### 5.2 Round 2 (#264): software framebuffers, the thread pool, shadow pictures

1. **The framebuffer of a window keeps its frame** in the renderer's format. Only what changed is rendered, and it is converted to the platform's BGRA where it changed.
2. **The thread pool of `vello_cpu`** renders scenes of 512 by 512 pixels or more.
3. **The picture of a box shadow without a closed form is kept** and reused (`ShadowPictures`, 32 MB at most).

Result: the CPU mode in software became faster than Skia raster in every scenario but one tie. For example, the tour went from 3136 to 1390 ms against Skia's 1716, and the effects page from 38.5 to 20.4 ms against 77.2.

### 5.3 Round 3 (#265): the GPU mode

1. **The GPU mode draws its filter scenes on the device with the hybrid renderer** instead of on the processor.
2. **A kept target of the GPU mode is rendered only where it is cleared:** a scene of the rectangle around the cleared rectangles, copied into place.
3. **The benchmarks print the time without waiting for the device.**

GPU mode, pointer across the buttons: 1.97 to 1.04 ms. Tour: 1375 to 1089 ms.

### 5.4 Round 4 (#266): the windows of Linux and Windows

1. **No panic on a graphics context the backend cannot draw with.**
2. **A device of the backend's own** (Vulkan, or OpenGL through EGL, on Linux; Direct3D 12 on Windows), with a `wgpu` surface made from the handle of the window.
3. **Swap chains are retired outside the context of the platform:** Mesa refuses EGL while a GLX context is current.
4. **The X11 render window is sized to its top level:** a stale child window left a black band at the bottom.
5. **`FERROUI_FRAME_TIMES`.**
6. **The CPU mode is presented through the OpenGL context of the platform.** On Windows, a GDI framebuffer does not show in a window that uses composition.
7. **A TreeDataGrid fix found by the tour.**

### 5.5 Round 5 (#267): the hybrid mode on the OpenGL context of the platform

`wgpu` on OpenGL is made over the existing context (`hal::gles::Adapter::new_external`). The frame is drawn into a texture of that context and blitted into the framebuffer of the platform's surface.

- On the Windows virtual machine (ANGLE) the hybrid mode became faster than Skia: a median of 4.33 to 4.73 ms against 7.01 to 7.46.
- On the Linux virtual machine (virgl) it is about even.
- The first frame on an OpenGL context takes up to 1.2 s, while the driver compiles the renderer's shaders.

### 5.6 Experimental round (#268): stock Vello, driven the way it is fast

1. **A depth buffer for the hybrid renderer.**
   - `vello_gpu::Renderer::render` takes a depth view; with it, opaque content is drawn front to back and covered pixels are skipped.
   - One depth texture per target size is kept.
   - Scrolling went from 2.33 to 1.73 ms, the text page from 1.39 to 1.22 ms (together with item 2).
2. **The Metal queues are ordered by a shared event.**
   - The device's queue signals an `MTLSharedEvent` behind the frame. A command buffer of the platform's queue waits for that value ahead of the presentation.
   - The processor no longer waits until the frame is scheduled (`waitUntilScheduled` is the fallback).
3. **Fragments, in the GPU mode.**
   - `IDrawingContextImplWithFragments` (`FerroUI.Base`, `platform/i_drawing_context_impl_with_fragments.rs`) is an optional extension of the drawing context, reached like the effects one, with a default of `None`.
   - The compositor keeps the fragment of each draw-list visual and drops it when the commands or their resources change (`server_composition_draw_list_visual.rs`).
   - The GPU mode keeps a `vello::Scene` per visual and appends it whole, as Xilem keeps the drawing of its widgets.
   - A fragment is drawn again only at the transform it was made at moved by whole pixels, with the same opacity and options.
   - It is not kept when it paints with an image or blends with what lies outside it (`drawing_context_impl/fragments.rs`, `scene/vello_gpu_scene_sink.rs`; tests in `gpu/fragment_tests.rs`).

| Scenario (total / without the wait) | Skia | GPU mode, before 3 | GPU mode, after 3 | hybrid, after 1 and 2 |
|---|---|---|---|---|
| pointer across buttons | 0.64 / 0.16 | 1.11 / 0.36 | 1.12 / 0.34 | 0.74 / 0.30 |
| scrolling 20 px a frame | 0.92 / 0.34 | 2.51 / 0.71 | 2.26 / 0.48 | 1.80 / 1.10 |
| text page drawn again | 0.69 / 0.20 | 2.00 / 0.57 | 1.99 / 0.56 | 1.26 / 0.45 |
| catalog tour, second round (total ms) | 483 | 1150 | 1150 | 853 |

The fragments take a third off the GPU mode's processor time on a scrolled frame, with the same pixels. The GPU mode is bound by its GPU, though (section 7.2), so it did not become the default; the hybrid mode stays the faster one.

## 6. The comparison harness and the tests

- **The comparison harness** (`tests/FerroUI.RenderBackends.Comparison`) draws 32 scenes, among them shapes, text, effects and codecs, in Skia and in every Vello mode. It fails when a mode differs beyond its bound: a share of the pixels beyond a tolerance of 32 levels, against Skia and against the CPU mode (`modes.rs`, `BOUNDS`). `FERROUI_VELLO_TEST_DUMP=<dir>` writes what each mode drew, as raw RGBA.
- **At the end of the work:** Base 4379 tests, Vello 521, harness 32, all passing on macOS.

## 7. Where the time goes: the cost model

### 7.1 The hybrid mode on a full redraw

A scrolled frame of the buttons page at 2560 by 1600 (round 5 code, before the depth buffer), from `sample`, in samples of the render of a frame:

| Part | Share |
|---|---|
| waiting for the GPU (the benchmark waits; a window does not) | 48 % |
| recording the scene, of which: | 36 % |
| - strips of paths (`StripGenerator::generate_with_clip`) | 20 % |
| - rectangles, borders and their strokes (`draw_rectangle`, `stroke_path`) | in the strips above |
| - glyph runs (from the atlas) | 9 % |
| - clips | 3 % |
| the render call (scheduling, encoding, uploads) | 10 % |

On the GPU, from a Metal trace of the same frames: the render pass of `vello_gpu` takes about 0.6 to 0.7 ms a frame, the copy of the frame into the drawable 0.2 ms, and the clear 0.06 ms. Skia's whole GPU frame is about 0.6 to 0.75 ms. Skia copies its frame into the drawable as well (through its layer), so the copy is no disadvantage of Vello's.

What this means:

- **The processor is the gap.** On a full redraw the hybrid mode spends about 2.5 times Skia's processor time: the sparse-strips renderer computes the coverage of every path on the processor while the scene is recorded, and Skia only records commands for its GPU.
- **The work is repeated for content that only moved.** The compositor replays the draw list of every visual into a new scene each frame, which is how a drawing context is driven and suits Skia. For Vello it is the worst case: a scrolled frame recomputes the coverage of content whose pixels only moved.
- **The renderer has no way to keep it.** `vello_gpu` has no recordings, so the strips cannot be kept by the renderer; they can only be avoided by keeping their result, a texture (section 9.1), or computed in parallel (section 9.2).

### 7.2 The GPU mode

The processor time of the GPU mode is low (0.48 to 0.71 ms on a scrolled frame), and it is bound by its GPU. From a Metal trace of the scrolled frames:

- about 0.85 ms a frame of compute passes of classic `vello`, which rasterizes the whole target every frame;
- about 0.3 ms of copies: the region of a kept frame, and the frame into the drawable with the alpha premultiplied (the renderer writes straight alpha into a storage texture).

Even without the copies, the compute passes alone take longer than Skia's whole GPU frame. Fragments (section 5.6) do not change this.

### 7.3 The CPU mode

In software the CPU mode wins: it renders what changed, on several threads, and converts only what changed. Where it was presented through ANGLE (before round 5) it lost to Skia on the GPU, by the upload of what changed (1.2 ms a frame) and the present.

## 8. Experiments not kept

### 8.1 Analytic rounded rectangles in a patched `vello_gpu`

- **What was built.** Copies of `vello_common`, `vello_gpu` and `vello_gpu_shaders` 0.3.0 in the repository, used through `[patch.crates-io]`.
- **The new paint.** A rounded rectangle with circular corners, or the ring between two of them (a border), drawn as rectangle instances whose coverage the fragment shader computes from the signed distance to the shape:
  - `coverage = clamp(0.5 - distance, 0, 1)`, exact for straight edges;
  - for a ring, the outer coverage minus the inner.
- **How it was wired.**
  - It is a paint type of its own (6) with five texels of encoded data: the outer and inner rectangles, their radii, and the color.
  - It was carried by a field of `EncodedBlurredRoundedRectangle`, because a new variant of `EncodedPaint` would have broken the exhaustive matches of `vello_cpu`.
  - A WESL helper (`rounded_rect.wesl`) evaluates it.
  - `Scene::fill_rounded_rect_analytic` covers the inside of a fill with a plain solid rectangle, leaves the inside of a ring out, and draws four bands around it.
- **When the backend used it.** For a solid paint with anti-aliased edges under an axis-aligned transform, for fills and for the strokes of `draw_rectangle`. A stroke is a ring:
  - the outer radius is `r + t/2` (0 for a square corner with a miter join, `t/2` with a round join);
  - the inner radius is `max(r - t/2, 0)`.
- **Result.** The comparison harness caught a bug: radii had been clamped to half the shorter side, where the contract allows a radius up to the side as long as the two radii on a side fit. After the fix the hybrid mode drew the rounded-rectangle scene within 0.005 % of the CPU mode.
- **Why it was dropped.** It was not benchmarked: the decision was made not to patch Vello. The commits are kept on a local branch only (`backup/vello-patched-crates`, commit `0fe3689a`), not pushed.
- **Further.** It remains a candidate to propose to the Vello project as a feature of `vello_gpu`. With it in a release, the backend would get it with published crates.

### 8.2 A BGRA frame copied by a blit on macOS

The idea was to keep the window frame in the drawable's format, so that the copy into the drawable becomes a blit on the platform's queue. It is not possible: the platform's `CAMetalLayer` is framebuffer-only (`framebufferOnly` is `YES`), so its drawables cannot be the destination of a blit. A render-pass copy is needed, and it costs the same in either format. The shared-event ordering of section 5.6 kept the part of the idea that gains.

### 8.3 Bands as full-size scenes with a clip

A full redraw built by several threads, each recording the whole frame into a full-size scene clipped to its band, gains nothing: `vello_common` culls a path only to the size of its scene (`make_tiles_analytic_aa` with the width and height of the strip generator), not to a clip rectangle. Every band would pay for the whole path. Bands have to be scenes of the size of the band (section 9.2).

### 8.4 Fragments as the default on GPUs with compute shaders

See section 5.6: they work, but the GPU mode stays bound by its GPU.

## 9. Open work and designs

### 9.1 Retained texture layers in the compositor (designed, not built)

**What it is for.** In a scrolled frame nothing changes but the offset of the content, and the hybrid mode computes the coverage of all of it again. A retained layer draws such a subtree once into a texture of the device. In the frames after, it composes the texture at the subtree's new place: for `vello_gpu` that is one rectangle painted with an image, and no strips. Only what changed inside the subtree, and what scrolls into view, is drawn again. It is the way Graphite keeps the tiles of its documents, applied to visuals.

**What exists.** The bitmap cache of a visual (`ServerCompositionVisualCache`, the cache mode of the contract) already does most of it:

- it renders the subtree into an offscreen layer of the render interface (`create_offscreen_render_target`);
- it is the dirty rectangle collector of its subtree in the update pass (`push_cache_if_needed`). A change inside the subtree re-renders only that part of the layer, and a change of the transform of the visual itself dirties only the target;
- it composes the layer with `draw_bitmap`, snapped to the pixels of the device.

**What has to be added.**

1. **A layer that is not asked for.**
   - **Gaining one.** A visual gets a retained layer from the compositor itself, not from its cache mode, when its own transform changed in two frames in a row while nothing else in its subtree did: the content of a scroll viewer, a panel that slides.
   - **Losing it.** It loses the layer when its subtree changes in most frames, after some frames without a move, or when the budget of the layers (memory of the device) is spent.
   - **Use.** The visual's `cache()` returns the explicit cache or this one, so the update and render passes take both the same way.
2. **The visible part and a margin.**
   - **Why.** The bitmap cache renders the whole bounds of the subtree, clamped to the largest texture, which loses resolution; the content of a long list would be a texture of tens of megabytes.
   - **What a retained layer holds.** The bounds of the subtree within the clip it is drawn in, widened by a margin (a viewport in the direction of the move), in the pixels of the device.
   - **Moving the layer.** When the visible part leaves the layer, the layer moves: what it still holds is copied to its new place on the device (`copy_texture_region`), and the strip that came into view is drawn as a dirty rectangle of the cache.
3. **The same pixels.**
   - **When a layer is reused.** Only when the transform of the visual to the device is the one it was rendered at, moved by whole pixels. Another scale or a fraction of a pixel renders it again, so text, snapped rectangles and flattened curves are where they would be.
   - **Why the pixels match.** Source-over composition is associative, so the pixels are those of drawing the subtree directly, but for the rounding of the layer (one level of 255).
   - **When a subtree is not retained:**
     - the opacity at the visual is not 1, because the drawing context multiplies opacity into each paint, and a group opacity on the layer would compose overlapping shapes differently;
     - something in it is composed with what lies below it in a mode other than source-over;
     - an effect or an opacity mask is pushed at the visual itself.
4. **Only where it gains.**
   - The render interface says whether it wants retained layers, as an optional feature of its context, asked for as the other optional features are.
   - The Vello backend gives it in the hybrid and CPU modes.
   - The Skia backend gives it only if a measure shows that it gains: it composes its own layers on the GPU already, and a texture per subtree costs memory and a copy.

**How to measure it.**

- The frame benchmarks (scrolling, the pointer across the buttons, the tour), in one session with Skia first and last.
- The comparison harness, with a scene drawn through a retained layer against the same scene drawn directly, at the place it was rendered at and moved by whole pixels.

**Expected effect.** The scrolled frame would cost the backgrounds of the ancestors (a few rectangles) and one image rectangle, plus a strip of new content every few frames, instead of the coverage of the whole page. That is the frame where the hybrid mode is now about 1.9 times Skia.

### 9.2 Full redraws built in bands on several threads (designed, not built)

For new content, which no layer holds (a page that is shown, the tour):

1. **Record.** The hybrid sink records the calls of the drawing context into a list that threads can share: paths, paints, clips, layers, glyph runs, with owned data.
2. **Replay in parallel.** Each worker replays the list into a `vello_gpu` scene of the size of its band, translated by the band's offset. The scene culls what lies outside the band (section 8.3).
3. **Render into band textures.** Each band is rendered into a texture of the band size. The band textures are the kept frame of the window, and the copy into the drawable (needed anyway, section 7.1) is made band by band. This adds no GPU copy: `vello_gpu` renders into a whole texture view, with no offset.
4. **Text.** Each band has a renderer with its own glyph atlas, or the glyph runs are recorded on one thread.

**Expected effect.** About half of the processor time of a full redraw is the recording of the scene. With four bands the processor time of a full redraw would come near Skia's; the GPU time stays.

### 9.3 Smaller exact changes (not built)

- **Skipping a rounded clip that contains its content.** A clip whose content lies inside its inner rectangle clips nothing; skipping it saves its strips and the intersection of every path with it. The compositor knows the bounds of the subtree.
- **Filling a border as a ring.** A uniform border is stroked: the outline of the stroke is computed (`kurbo::stroke`) and then filled. Filling the ring between the two rounded rectangles directly (section 8.1 has the geometry) skips the outline.
- **Fewer copies in the GPU mode.** On a full redraw, the region of a kept frame is rendered into a texture of its own and copied back. When the region is the whole frame, it could be rendered into the frame directly.

### 9.4 Not verified

- **Hardware Vulkan and Direct3D 12.** Built and tested in software only; no machine with a real GPU was available (section 4.4).
- **Wayland.** Windows on Wayland hand out no native handle; the OpenGL presenter of round 4 draws its EGL surfaces.
- **The first frame on an OpenGL context** (shader compilation, up to 1.2 s): a pipeline cache was not tried. `vello_gpu` 0.3.0 takes none; the repository of the Vello project has added one since.

## 10. Reference

### 10.1 Pull requests

| PR | Content |
|---|---|
| #263 | Round 1: rectangles and clips, text from the glyph atlas, CPU contexts kept, partial renders, partial uploads |
| #264 | Round 2: framebuffers that keep their frame, the thread pool, shadow pictures |
| #265 | Round 3: the GPU mode (filter scenes on the device, bounded scenes), the benchmark's measure without waiting |
| #266 | Round 4: the windows of Linux and Windows on a device of the backend's own; the OpenGL presenter of the CPU mode |
| #267 | Round 5: the hybrid mode on the OpenGL context of the platform |
| #268 | Experimental round: depth buffer, shared-event ordering, GPU-mode fragments; this document |

### 10.2 Files

| Area | Files |
|---|---|
| Sinks of the modes | `scene/vello_cpu_scene_sink.rs`, `scene/vello_hybrid_scene_sink.rs`, `scene/vello_gpu_scene_sink.rs`, `scene/i_vello_scene_sink.rs` |
| Drawing context | `drawing_context_impl.rs`, `drawing_context_impl/` (`effects.rs`, `box_shadows.rs`, `fragments.rs`) |
| Devices and windows | `gpu/mod.rs` (`create_gpu`), `gpu/metal.rs`, `gpu/window_surface.rs`, `gpu/window_frame.rs`, `gpu/gl_context_gpu.rs`, `gl_window.rs`, `gpu/device_surface_render_target.rs`, `gpu/vello_wgpu_device.rs` |
| Software framebuffers | `framebuffer_render_target.rs`, `surface_render_target.rs`, `helpers/pixel_format_helper.rs` |
| Benchmark surface | `gpu/metal_offscreen.rs`, `samples/ControlCatalog/tests/frame_benchmark.rs` |
| Compositor | `src/FerroUI.Base/rendering/composition/server/` (`server_composition_target.rs`, `server_composition_draw_list_visual.rs`, `server_composition_visual/`) |
| Harness | `tests/FerroUI.RenderBackends.Comparison/` (`modes.rs`, `scenes.rs`) |
| Tools | `scripts/vello-perf/` (its README) |

### 10.3 Environment variables

| Variable | Effect |
|---|---|
| `FERROUI_TEST_RENDERER` | Renderer of the catalog tests and benchmarks: `skia`, `vello-hybrid`, `vello-gpu`, `vello-cpu` |
| `FERROUI_BENCH_SURFACE`, `FERROUI_BENCH_SCALING` | Surface of the benchmarks (`metal`, else raster) and its scaling |
| `FERROUI_RENDERER` | Backend of the desktop catalog: `vello`, `vello-hybrid`, `vello-gpu`, `vello-cpu`; Skia otherwise |
| `FERROUI_VELLO_PERF=<ms>` | Phases of the frames of the backend; frames longer than the value printed on their own |
| `FERROUI_FRAME_TIMES=1` | Frame times of every composition target (any backend) |
| `FERROUI_VELLO_ALLOW_SOFTWARE_ADAPTER=1` | Lets the GPU modes use lavapipe or WARP |
| `FERROUI_VELLO_TEST_DUMP=<dir>` | The comparison harness writes what each mode drew |
| `FERROUI_VELLO_REQUIRE_GPU=1` | The harness fails when a GPU mode has no adapter |
| `FERROUI_SMOKE_PAGES`, `FERROUI_SMOKE_EXIT_MS`, `FERROUI_SMOKE_RENDERING` | Smoke runs of the desktop catalog: page time, exit, the platform's rendering mode |
