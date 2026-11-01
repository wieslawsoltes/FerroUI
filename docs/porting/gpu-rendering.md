# GPU rendering of the effects and rendering features

The rule ([PORTING-GUIDE.md](PORTING-GUIDE.md), ground rule 6): every backend draws every
rendering feature on the GPU, validated by tests; the CPU path of a feature is the reference and
the fallback of raster surfaces only. A frame that changes positions, uniforms or animated values
makes no new bitmap or texture.

This page is the audit of the effects and rendering features against that rule: for each path, how
each backend draws it, what is wrong with it, and the test that covers it. It is kept up to date by
the pull requests that change a path.

## Backends and modes

- **Skia** draws through Ganesh (OpenGL, WebGL2 in the browser, Vulkan, Metal on iOS), Graphite
  (Metal on macOS) or the raster backend (CPU surfaces, render target bitmaps, the render tests).
  Runtime effects (SkSL) run on all three.
- **Vello** has four modes: the CPU renderer (`vello_cpu`), the hybrid renderer (`vello_hybrid`:
  CPU coarse rasterization, GPU fine rasterization), the GPU renderer (`vello` over wgpu) and
  the WebGL2 renderer of the browser. The renderers have layers with blend modes, clips,
  gradients, images and (CPU and hybrid) filter layers; none has a hook for a custom shader.

## Status

Status: **GPU** (drawn by the GPU, nothing uploaded or read back per frame), **cached** (an image
made once and kept while its inputs stay the same), **CPU fallback** (the CPU path draws it on this
backend and uploads the result: a violation of the rule), **gap** (what the backend draws differs
from the reference, or nothing is drawn).

| Path | Skia (Ganesh, Graphite, raster) | Vello (CPU, hybrid, GPU, WebGL2) | Tests |
|---|---|---|---|
| Scene lighting (Reveal backgrounds and borders) | GPU: the lighting shader (#456) | Hybrid and GPU modes: GPU, a render pass of FerroUI evaluates the lights into a pooled texture of the device the scene paints with (#459); CPU mode: the CPU light map (the reference); WebGL2: CPU fallback (gap: no device textures there yet) | `composition_effect_tests.rs` (exact light, CPU light map, no image of pixels per frame), `gpu/metal/tests.rs` (Graphite), `composition_winui_material_tests.rs` (no bitmap per frame), `src/Vello/FerroUI.Vello/gpu/effect_pass_tests.rs` (hybrid and GPU modes: the exact light, no allocation over steady frames) |
| Lit visuals (`push_multiply_layer` with a light map) | GPU: the lighting shader and the multiply blender (#456) | CPU fallback: the content drawn by the CPU renderer and multiplied on the processor | `composition_effect_tests.rs`, `composition_winui_lighting_tests.rs` |
| Pixel operations (the blend modes the layers lack, the arithmetic composite of two images, `MaskInvert`) | GPU: layers with runtime blenders, Skia's arithmetic blender, the `Dissolve` noise shader (#456) | CPU fallback: each source into a render target bitmap, read back, combined, uploaded | `composition_effect_tests.rs` (each operation against `PixelOperation::apply_to_premultiplied_rgba8`), `composition_winui_lighting_tests.rs` |
| `push_blend_layer` | GPU: a save layer with the blend mode | GPU: a layer of the scene | `layer_effect_tests.rs` of both backends |
| `push_color_matrix_layer` (colour matrix, saturation, tint, invert, grayscale, luminance to alpha, the arithmetic composite with a colour, the shadow effect) | GPU: a colour filter of a save layer | CPU fallback: the content drawn by the CPU renderer, the matrix applied on the processor, uploaded | `layer_effect_tests.rs` |
| `Visual.Effect`: blur, drop shadow, their transitions | GPU: an image filter of a save layer | GPU where the scene has filter layers and no clip is open; CPU fallback otherwise (the GPU renderer and WebGL2 have no filter layers) | `src/Vello/FerroUI.Vello/effect_tests.rs`, `tests/FerroUI.RenderTests/media/effect_tests.rs`, `tests/FerroUI.RenderBackends.Comparison/effect_tests.rs` |
| Luminance to alpha (`ImmutableLuminanceToAlphaEffect`) | GPU: a colour filter | CPU fallback | `tests/FerroUI.RenderTests/media/effect_tests.rs` |
| Box shadows, outer | GPU: a blurred draw (image filter) outside a clip of the box; the clip of a rounded box under a rotation, or with radii Ganesh cannot clip analytically, is a coverage shader (#457; it was a software mask of Ganesh, see the next section) | GPU: a closed-form blurred rounded rectangle where it agrees with the Gaussian; cached otherwise (the picture of the blurred shape, made by the CPU renderer, kept per shape) | `tests/FerroUI.RenderTests/media/box_shadow_tests.rs`, `coverage_clip_tests.rs` (`the_shadows_of_rotated_cards_keep_their_clips`) |
| Box shadows, inset | as the outer ones | as the outer ones | the same |
| Clips: rectangles, rounded rectangles | GPU: analytic clips; a rounded rectangle under a rotation or with complex radii is a coverage shader on GPU surfaces (#457) | GPU: clip layers of the scene | `tests/FerroUI.RenderTests/geometry_clipping_tests.rs`, `coverage_clip_tests.rs` (exact coverage), `gpu/metal/tests.rs` (Graphite, no clip left to the GPU's clip stack over moving frames) |
| Clips: geometry (`push_geometry_clip`) | GPU for convex paths of a few edges and with MSAA or stencil; a software mask of Ganesh otherwise (gap) | GPU: clip layers | `tests/FerroUI.RenderTests/geometry_clipping_tests.rs` |
| Opacity masks (`push_opacity_mask`), the alpha masks of composition (`CompositionMaskBrush`, the masks of shapes, images and text) | GPU: a save layer and a destination-in draw of the mask brush | GPU: a layer with a destination-in blend | `tests/FerroUI.RenderTests/opacity_mask_tests.rs` |
| `GetAlphaMask` | made on request (a snapshot), not per frame | the same | - (no pixel test) |
| `ThemeShadow`, the composition `DropShadow` | GPU: blurred layers; the colour of a drop shadow is a small bitmap made once per colour (cached) | the same through the Vello effects (GPU or CPU fallback as `Visual.Effect`) | `tests/FerroUI.RenderTests/composition/winui_shadow_tests.rs`, `composition_theme_shadow_tests.rs`, `composition_shadow_tests.rs` |
| Acrylic and backdrop brushes, Mica, desktop acrylic | GPU: the backdrop drawn again under a blur image filter, the tint and luminosity layers; the noise texture is decoded once (cached) | the backdrop through the Vello effects (GPU or CPU fallback); the noise image cached | `composition_winui_material_tests.rs` |
| Reveal focus and border lights | as the scene lighting | as the scene lighting | as the scene lighting |
| Connected-animation cross-fade | GPU: the frozen image of the source (made once, at `prepare_to_animate`) drawn with an animated opacity | the same | - (no pixel test) |
| Noise and tiled-surface brushes | GPU: an image shader with repeat tiling; the image is made once | GPU: a tiled image | `a_border_effect_tiles_a_surface_brush` |
| Image brushes, `draw_bitmap` | GPU: an image shader or image draw; a raster image is uploaded once and kept by the resource cache of the GPU context while it fits the budget (128 MiB, next section) | GPU: an image of the scene, uploaded once per image | `tests/FerroUI.RenderTests/media/image_brush_tests.rs`, `tile_brush_tests.rs` |
| Render target bitmaps (`RenderTargetBitmap`) | made by the application, not per frame | the same | - |
| The visual cache (`CacheMode`) | a GPU layer kept while the content is the same | the same | - (no pixel test) |

## The software masks of Ganesh (the A8 uploads)

While scrolling the Ferro Store on WebGL2, single-channel textures of 322 by 408, 605 by 433, 322 by
269 and 605 by 210 pixels were uploaded through `make_bmp_proxy` and
`createNonMippedProxyFromBitmap`, more each time content was revealed. Those are Ganesh's software
masks (`GrSWMaskHelper`): when Ganesh cannot draw a clip or a coverage analytically, and the
render target has neither MSAA nor a stencil it may use for an anti-aliased clip, it rasterizes the
shape on the CPU into an A8 bitmap and uploads it. The store's cards are rotated
(`RotateTransform`) frames with `ClipToBounds` and rounded corners and an outer box shadow: each
card clips to a rounded rectangle under a rotation (the `ClipToBounds`) and to the outside of one
(the box shadow keeps the shadow out of the box), and Ganesh's analytic clips of rounded rectangles
are in device space only, so both are software masks, made again whenever the card moves on the
screen (a scroll moves every card).

The fix (#457): on GPU surfaces such a clip is a clip shader, an SkSL coverage of the rounded
rectangle in the coordinates of the drawing (`composition_effects::rounded_rect_coverage`: the
distance to the edge, or to first order to the arc of a corner, in device pixels through the
inverse transpose of the transform, ramped over a pixel as Ganesh's analytic clips are), which
Ganesh and Graphite apply per pixel without a mask. Measured on the Store a second time, a 322 by
408 mask was still uploaded on each scroll frame with upright cards: the card and the picture in it
both clip to rounded corners and share their right edge, and Ganesh merges the two clip elements
into one rounded rectangle with different corners, which none of its analytic effects takes (it
also stops at four analytic elements a draw). So a rounded clip is an element of the clip stack
only when it is upright, its radii are not complex and no other rounded element is in the stack
(the drawing context tracks them by save count); otherwise it is a clip shader. Single upright
clips, the common case, stay elements: a clip shader makes the programs of the draws under it
programs of their own (with every rounded clip a shader the Store linked 29 programs on its first
scroll instead of 4).

The raster backend keeps Skia's clip. Against the exact coverage of each pixel (16 by 16 samples)
the coverage shader is within 14 levels at the edge and 2 to 4 on average; Skia's raster clip,
which flattens the arcs, is 8 to 13 off on average (`coverage_clip_tests.rs`). A clip of a geometry
(`push_geometry_clip`) that is not a convex path of a few edges stays a software mask on Ganesh
without MSAA (gap).

**Programs.** The GPU links the program of a draw the first time it draws it; on WebGL ANGLE
translates and links it, tens to hundreds of milliseconds each on a cold cache (a 737 ms frame was
measured on the first scroll of the Store; the SkSL compiles themselves take about a millisecond).
A GPU render target prewarms the runtime effects with its second frame
(`DrawingContextImpl::prewarm_runtime_effects`): every SkSL program compiles and each effect is
drawn once, in the forms the drawing context draws it, into a corner of the target that is cleared
afterwards; the first paint is not delayed (prewarming in the first frame cost about 1.2 s of long
tasks on a cold cache). On the Store no lighting, blender or noise program is linked during the
first scroll afterwards; the draws of content under a coverage clip (an image, a circle) still
make programs of their own when first seen, as any new kind of draw does in Ganesh.

## The image textures of Ganesh (the RGBA uploads)

Ganesh uploads a raster image (`ImmutableBitmap`) to a texture when it is first drawn and keeps the
texture in the resource cache of the context, keyed by the image, until the cache needs the room.
`SkiaOptions::max_gpu_resource_size_bytes` is the budget of that cache. Upstream sets 29.5 MB
(`1024 * 600 * 4 * 12`), which the port kept until 0.4.0. The Ferro Store's Discover page draws
four screenshots of 1.3 to 6.1 MB as textures, plus icons, layers and the glyph atlas: more than the budget, so
the textures of the screenshots that scrolled out of view were evicted and uploaded again when they
scrolled back in. Measured in the browser (WebGL2, a 375 by 812 viewport, two passes down and up the
page after a warm-up pass): 52 texture uploads of 23 MB per two passes, all through
`GrMakeCachedBitmapProxyView` from `ImmutableBitmap::draw`.

The default is now 128 MiB (`DEFAULT_MAX_GPU_RESOURCE_SIZE_BYTES`), the budget of the image
textures of Vello (`scene/image_textures.rs`): the same passes make no upload, and the frame times
stay at the display's interval (99th percentile 10.3 ms at 120 Hz). An application that has to
hold less sets the option lower. Skia's own default (`None`) is 256 MiB.

## The iOS simulator: no destination reads in shaders

Skia turns framebuffer fetch on for the Metal devices of the Apple GPU family (`MtlCaps`: `fFBFetchSupport` when the family is Apple, in Graphite and in Ganesh), and the simulator's Metal device reports that family: it is the GPU of the Mac. The simulator's Metal compiler rejects framebuffer fetch ("reading from a rendertarget is not supported"), so a draw whose shader reads its destination gets no pipeline, and Graphite then fails on it (a null pipeline in `QueueManager::addRecording`, a crash of the application). The runtime blenders of the multiply layer of the lights and of the pixel operations read their destination, and since the GPU route of those effects (#456) the prewarm of a context's second frame draws each of them: every application crashed in the simulator after a few frames.

On a GPU surface of the iOS simulator (`target_os = "ios"` with `target_abi = "sim"`) `push_lighting_multiply_layer` and `push_pixel_operation` report that they did not draw, so the compositor draws these effects with its CPU path, and the prewarm skips them. Devices are not affected: their GPUs have framebuffer fetch. Measured on the iOS 26.4 simulator (iPhone 17 Pro, `ios_view`): before, the application crashed with the pipeline errors on its third frame; after, a tap and two rotations, no pipeline error and no crash.

What remains for the simulator: a blend layer with a blend mode that reads its destination (`push_blend_layer` with multiply, screen and the other non-coefficient modes) has the same failure there; no application of the repository uses one in the simulator.

## Vello: what is missing and the GPU route

vello's renderers cannot express the lighting, the colour matrix, the multiply layer or the pixel
operations: they compose layers with the blend modes of the Compositing and Blending
specification, and have no colour matrix, no blend that multiplies and keeps the alpha below, and
no hook for a custom shader. The GPU renderer and WebGL2 have no filter layers either. Today those
paths draw their content with the CPU renderer and upload the result on every frame, in every
mode; this is the gap the rule closes.

The route, in pull requests stacked on the Skia ones:

1. **A pass FerroUI owns** (`src/Vello/FerroUI.Vello/gpu/effect_passes.rs`, #459). A wgpu render
   pass with a WGSL port of the programs of `ferroui-skia/composition_effects.rs`, compiled once
   per device, with the parameters as uniforms, writes a texture of the device from a pool (sizes
   rounded up to 32 pixels, reused once the scene that painted with a texture was rendered); the
   scene paints with it through the `VelloSceneTexture` route, one texel per device pixel.
   Done for the scene lighting (`draw_lighting`) in the hybrid and GPU modes.
2. **Layers with an effect** (next): the multiply layer of lit visuals, the colour matrix layers
   and the pixel operations need the content of the layer as an input. The content is recorded
   into a scene of the device (`create_filter_scene`, as the blur of the hybrid mode does),
   rendered into a pooled texture (`render_to_texture`), and a pass reads it (and, for the multiply
   layer, evaluates the lights) into another texture the outer scene paints with. Ordering is
   that of the queue: the content, the pass, then the frame that paints with the result. The
   blur and drop shadow of the GPU mode (no filter layers in `vello`) take the same route with a
   two-pass Gaussian.
3. **WebGL2.** The same programs in GLSL ES 3.0 on the WebGL2 context the renderer uses, with
   framebuffer textures in place of wgpu textures (`hybrid-webgl` has no device textures yet: the
   sink has to learn to paint with a framebuffer texture first).
4. **The CPU renderer** keeps the CPU path: it is the reference.
5. **Validation:** each pass against the exact result or the CPU reference in each GPU mode
   (`gpu/effect_pass_tests.rs`), `tests/FerroUI.RenderBackends.Comparison` scenes against Skia, and
   the allocation counter of the pool, which stays constant over steady frames.
