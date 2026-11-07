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
| Lit visuals (`push_lighting_multiply_layer`, `push_multiply_layer` with a light map) | GPU: the lighting shader and the multiply blender (#456) | Hybrid and GPU modes: GPU, the content recorded into a scene of the device and the light multiply pass of FerroUI evaluates the lights per pixel times the content (#455 E3, the route's step 2); the multiply layer of a light map (more than 8 point and spot lights) and the CPU mode: the content drawn by the CPU renderer and multiplied on the processor (the reference); WebGL2: CPU fallback | `composition_effect_tests.rs`, `composition_winui_lighting_tests.rs`, `the_multiply_layer_of_lit_visuals_matches_the_exact_light_in_the_gpu_modes` (`gpu/effect_pass_tests.rs`) |
| Pixel operations (the blend modes the layers lack, the arithmetic composite of two images, `MaskInvert`) | GPU: layers with runtime blenders, Skia's arithmetic blender, the `Dissolve` noise shader (#456) | Hybrid and GPU modes: GPU, each source recorded into a scene of the device and rendered into a pooled texture, the pixel operation pass of FerroUI combines it into what the sources before made (#455 E3); CPU mode and WebGL2: CPU fallback (each source into a render target bitmap, read back, combined, uploaded by the compositor) | `composition_effect_tests.rs` (each operation against `PixelOperation::apply_to_premultiplied_rgba8`), `composition_winui_lighting_tests.rs`, `the_pixel_operations_match_their_cpu_reference_in_the_gpu_modes` (`gpu/effect_pass_tests.rs`) |
| `push_blend_layer` | GPU: a save layer with the blend mode | GPU: a layer of the scene | `layer_effect_tests.rs` of both backends |
| `push_color_matrix_layer` (colour matrix, saturation, tint, invert, grayscale, sepia, linear transfer, luminance to alpha, the arithmetic composite with a colour, the shadow effect) | GPU: a colour filter of a save layer | Hybrid and GPU modes: GPU, the content recorded into a scene of the device, rendered into a pooled texture, the colour matrix pass of FerroUI writes another the scene paints with (#455 E3); CPU mode: the matrix applied on the processor (the reference); WebGL2: CPU fallback | `layer_effect_tests.rs`, `the_layer_passes_match_their_cpu_references_in_the_gpu_modes`, `steady_frames_of_the_layers_allocate_no_texture` (`gpu/effect_pass_tests.rs`) |
| `push_convolve_matrix_layer` (Win2D's `ConvolveMatrixEffect`, #455 E3) | GPU: a matrix convolution image filter of a save layer (a runtime effect of Skia on every surface) | Hybrid and GPU modes: GPU, the convolution pass of FerroUI on the route of the colour matrix; CPU mode: `ConvolveMatrix::apply_to_premultiplied_rgba8` (the reference); WebGL2: CPU fallback | `the_convolution_layer_matches_its_cpu_reference` (`ferroui-skia/tests.rs`, `ferroui-vello/effect_tests.rs`), `the_layer_passes_match_their_cpu_references_in_the_gpu_modes` (`gpu/effect_pass_tests.rs`), `media/convolve_matrix.rs` |
| `Visual.Effect`: blur, drop shadow, their transitions | GPU: an image filter of a save layer | Hybrid mode: GPU, a filter layer of the scene where no clip is open, a filter scene of the device (the hybrid renderer) under a clip; GPU mode: GPU in a build with the hybrid renderer (the default of the platforms), a filter scene of the device the scene paints with; the textures of those scenes come from the pool of `gpu/effect_passes.rs` (#455 E3: no texture per frame); CPU fallback in a build of the GPU renderer alone and on WebGL2 (no filter layers) | `src/Vello/FerroUI.Vello/effect_tests.rs`, `tests/FerroUI.RenderTests/media/effect_tests.rs`, `tests/FerroUI.RenderBackends.Comparison/effect_tests.rs` |
| Custom effects of `Visual.Effect` with a graph (`CustomEffect::graph`, #455 E2 and E3, [custom-effects.md](custom-effects.md)): the library's tint, grayscale, sepia, saturation, brightness and contrast, invert (colour matrix layers), glow (a drop shadow), sharpen and edge detect (convolution layers) | as its nodes: the colour matrix layers GPU (a colour filter of a save layer), the convolutions GPU (the matrix convolution image filter), the glow GPU (the drop shadow image filter); the graph loaded once per type and prewarmed, a frame changes only the values of its members | as its nodes: in the hybrid and GPU modes the colour matrix and convolution layers GPU (the passes of the route's step 2), the glow GPU as the blur; the CPU mode by the references; WebGL2: CPU fallback | `the_colour_effects_match_their_cpu_references`, `the_glow_matches_its_cpu_reference`, `the_image_effects_match_their_cpu_references`, `the_graph_effects_match_their_cpu_references_at_twice_the_scale` (`ferroui-skia/tests.rs`, `ferroui-vello/effect_tests.rs`), `graphite_draws_the_library_effects_as_their_cpu_references`, `graphite_draws_the_library_at_twice_the_scale_as_the_cpu_references` (`gpu/metal/tests.rs`), `the_library_effects_match_their_cpu_references_in_the_gpu_modes`, `the_library_effects_match_their_cpu_references_at_twice_the_scale_in_the_gpu_modes` (`ferroui-vello/gpu/effect_pass_tests.rs`, no layer drawn on the processor), `composition_custom_effect_tests.rs`; across the backends, the steady state and the prewarm: the row of #455 E9 below |
| Custom effects of `Visual.Effect` that are runtime shaders (`CustomEffect::shader`, #455 E4 and E7, [custom-effects.md](custom-effects.md)): the library's pixelate (content-sampling), scan lines and spotlight (overlays), and the interactive effects: pointer spotlight and reactive grid (overlays), water ripple and burning flame (content-sampling) | GPU: the SkSL program compiled once per thread and type, its uniforms checked once against the parameters; a content-sampling program is the runtime shader image filter of a save layer (the content a texture, never read back), an overlay drawn over the content inside the layer with its blend mode; the layer in whole device pixels; registered for the prewarm; a frame changes only the uniforms. A program that fails is reported once and draws the content unchanged | Hybrid and GPU modes: GPU (#455 E5), the WGSL of the description wrapped into the module of a pass (`ShaderEffectDescription::wgsl_module`), checked by naga against the uniform convention once per type and compiled once per device and type; the content recorded into a scene of the device over the area of the effect and rendered into a pooled texture, the pass writes another the scene paints with (an overlay composed over the content with its blend mode in the pass); a frame writes only the uniforms. A module that fails is reported once and draws the content unchanged. CPU mode: the CPU reference of the effect (the content drawn by the CPU renderer over the area, the reference applied on the processor); an application's effect without one draws its content unchanged, reported once. WebGL2: CPU fallback (the CPU reference): its renderer has no device textures yet (route step 3); the GLSL ES 3.0 of each module is ready, translated by naga at build time (`ferroui_build::shader_effects`), not drawn yet | `the_shader_effects_match_their_cpu_references` and the fallbacks (`ferroui-skia/tests.rs`), `graphite_draws_the_library_shader_effects_as_their_cpu_references` (`gpu/metal/tests.rs`), `custom_shader_effects.rs`, `the_library_shader_effects_match_their_cpu_references_in_the_gpu_modes`, `a_shader_that_fails_draws_unchanged_and_reports_once_in_the_gpu_modes`, `steady_frames_of_the_shader_effects_allocate_no_texture`, `the_shader_modules_of_the_library_are_checked_by_naga` (`ferroui-vello/gpu/effect_pass_tests.rs`), `the_shader_effects_match_their_cpu_references` and the fallback (`ferroui-vello/effect_tests.rs`), `the_library_shader_effects_translate_to_glsl_es` (`ferroui-build/shader_effects.rs`), `composition_custom_effect_tests.rs`; the interactive effects: `the_interactive_effects_match_their_cpu_references`, `frames_of_the_interactive_effects_compile_their_programs_once` (`ferroui-skia/tests.rs`), `graphite_draws_the_interactive_effects_as_their_cpu_references` (`gpu/metal/tests.rs`), `the_interactive_effects_match_their_cpu_references_in_the_gpu_modes`, `frames_of_the_interactive_effects_compile_their_pipelines_once` (`ferroui-vello/gpu/effect_pass_tests.rs`), `the_interactive_effects_match_their_cpu_references` (`ferroui-vello/effect_tests.rs`), `input/effect_input_tests.rs` |
| Images of runtime shader effects (`EffectParameterValue::Image`, the children of an overlay, #455 E6, [custom-effects.md](custom-effects.md)): the library's dissolve and mask transitions | GPU: each image a child image shader of the overlay's program, stretched over the layer and sampled by the nearest texel; the Skia image of an image made once per thread and version of its bitmap, kept by the identity of its handle; Graphite uploads it once (`drawable_image`, counted by `upload_count`), Ganesh caches its upload by the identity of the image, raster draws it as it is; frames that change only uniforms upload nothing; when the last handle is dropped the next drawing context drops the Skia image and the flush the texture. A content-sampling program takes no images (skia-safe 0.153 binds no setter for the children of a runtime shader image filter's builder): refused once, the content unchanged | Hybrid and GPU modes: GPU, each image uploaded once per device into a texture of the device (its premultiplied pixels, read once by the handle), bound with the content in the pass (`sample_<name>` of the module), kept by the identity of the handle on the render thread and released, with the bind groups that hold it, by the next drawing context or pass after the last handle is dropped; frames that change only uniforms upload nothing and allocate nothing. CPU mode: the CPU reference samples the pixels of the images. WebGL2: CPU fallback (the CPU reference); the GLSL ES of the module binds the images as textures | `the_page_transitions_match_their_cpu_references`, `steady_frames_of_the_page_transitions_make_their_images_once`, `the_images_of_a_dropped_transition_are_released` (`ferroui-skia/tests.rs`), `graphite_draws_the_page_transitions_from_textures_uploaded_once` (`gpu/metal/tests.rs`), `the_page_transitions_match_their_cpu_references_in_the_gpu_modes`, `the_images_of_the_page_transitions_are_uploaded_once_and_released`, `the_modules_of_the_page_transitions_are_checked_by_naga` (`ferroui-vello/gpu/effect_pass_tests.rs`), `the_page_transitions_draw_their_cpu_references` (`ferroui-vello/effect_tests.rs`), `the_images_of_the_page_transitions_are_textures_of_the_glsl_es` (`ferroui-build/shader_effects.rs`), `effect_image_tests.rs` |
| Custom effects FerroUI ships, validated across the backends, in the steady state and prewarmed (#455 E9, [custom-effects.md](custom-effects.md), section 3.8): every effect of the library (the graph effects, the runtime shaders, the page transitions with their images, the interactive effects) | Raster at 96 and 192 DPI and Graphite at scales 1 and 2 against the CPU references; frames of the whole library that change only parameters compile no SkSL program and make no image again (`runtime_effects::compilation_count`, `custom_shader_effects::effect_images_made`); the prewarm of a composition target draws each type once offscreen after the first frame at the value it registers, which compiles every program the first frame of the library needs (raster, and through the prewarm recorder of a Graphite render target); Ganesh takes the same programs (the prewarm on the context of the frames), without a pixel test of its own | CPU, hybrid and GPU modes at 96 and 192 DPI (scales 1 and 2) against the CPU references; in the hybrid and GPU modes frames of the whole library compile no pipeline, allocate no texture of the pool and upload no image once the first frames did (`effect_passes::shader_pipelines_created`, `texture_allocations`, `image_uploads`), nothing drawn on the processor; no prewarm (the render targets of Vello keep the default of `IRenderTarget::prewarm`): the first frame that draws a shader effect type makes its pipeline, once per device, and no frame after it makes one, in either mode; WebGL2: the CPU references (route step 3) | `every_custom_effect_matches_its_cpu_reference_in_every_backend_and_mode`, `the_backends_draw_every_custom_effect_alike` (the comparison scenes: Skia raster against each mode of Vello), `a_neutral_custom_effect_draws_the_content_unchanged_in_every_backend_and_mode` (`tests/FerroUI.RenderBackends.Comparison/custom_effect_tests.rs`, scenes in `custom_effect_scenes.rs`); `frames_of_the_library_compile_no_program_after_the_first`, `the_prewarm_of_the_library_compiles_what_its_first_frame_draws` (`ferroui-skia/tests.rs`); `a_graphite_render_target_prewarms_the_library_ahead_of_the_frame_that_shows_it` (`gpu/metal/tests.rs`); `frames_of_the_whole_library_compile_and_allocate_nothing_in_the_gpu_modes`, `the_first_frame_of_the_library_makes_one_pipeline_per_shader_type` (`ferroui-vello/gpu/effect_pass_tests.rs`); `a_target_prewarms_the_custom_effects_of_the_library_after_its_first_frame` (`compositor_tests.rs`) |
| Custom effects of `Visual.Effect` without a graph or a shader (`ImmutableCustomEffect`, #455, [custom-effects.md](custom-effects.md)) | gap: no factory; the content is drawn unchanged in a layer without a filter, reported once | the same | `custom_effect_without_a_factory_draws_unchanged` (`ferroui-skia/tests.rs`, `ferroui-vello/effect_tests.rs`), `composition_custom_effect_tests.rs` |
| Luminance to alpha (`ImmutableLuminanceToAlphaEffect`) | GPU: a colour filter | Hybrid and GPU modes: GPU, the colour matrix pass on the route of the layers (#455 E3); CPU mode and WebGL2: on the processor | `tests/FerroUI.RenderTests/media/effect_tests.rs`, `the_layer_passes_match_their_cpu_references_in_the_gpu_modes` (`gpu/effect_pass_tests.rs`) |
| Box shadows, outer | GPU: a blurred draw (image filter) outside a clip of the box; the clip of a rounded box under a rotation, or with radii Ganesh cannot clip analytically, is a coverage shader (#457; it was a software mask of Ganesh, see the next section) | GPU: a closed-form blurred rounded rectangle where it agrees with the Gaussian; cached otherwise (the picture of the blurred shape, made by the CPU renderer, kept per shape) | `tests/FerroUI.RenderTests/media/box_shadow_tests.rs`, `coverage_clip_tests.rs` (`the_shadows_of_rotated_cards_keep_their_clips`) |
| Box shadows, inset | as the outer ones | as the outer ones | the same |
| Clips: rectangles, rounded rectangles | GPU: analytic clips; a rounded rectangle under a rotation or with complex radii is a coverage shader on GPU surfaces (#457) | GPU: clip layers of the scene | `tests/FerroUI.RenderTests/geometry_clipping_tests.rs`, `coverage_clip_tests.rs` (exact coverage), `gpu/metal/tests.rs` (Graphite, no clip left to the GPU's clip stack over moving frames) |
| Clips: geometry (`push_geometry_clip`) | GPU for convex paths of a few edges and with MSAA or stencil; a software mask of Ganesh otherwise (gap) | GPU: clip layers | `tests/FerroUI.RenderTests/geometry_clipping_tests.rs` |
| Opacity masks (`push_opacity_mask`), the alpha masks of composition (`CompositionMaskBrush`, the masks of shapes, images and text) | GPU: a save layer and a destination-in draw of the mask brush | GPU: a layer with a destination-in blend | `tests/FerroUI.RenderTests/opacity_mask_tests.rs` |
| `GetAlphaMask` | made on request (a snapshot), not per frame | the same | - (no pixel test) |
| `ThemeShadow`, the composition `DropShadow` | GPU: blurred layers; the colour of a drop shadow is a small bitmap made once per colour (cached) | the same through the Vello effects (GPU or CPU fallback as `Visual.Effect`) | `tests/FerroUI.RenderTests/composition/winui_shadow_tests.rs`, `composition_theme_shadow_tests.rs`, `composition_shadow_tests.rs` |
| Acrylic and backdrop brushes, Mica, desktop acrylic | GPU: the backdrop drawn again under a blur image filter, the tint and luminosity layers; the noise texture is decoded once (cached); the pipelines are prewarmed after the first frame (#437, "Pipelines made ahead of their first draw") | the backdrop through the Vello effects (GPU or CPU fallback); the noise image cached | `composition_winui_material_tests.rs` |
| Reveal focus and border lights | as the scene lighting | as the scene lighting | as the scene lighting |
| Connected-animation cross-fade | GPU: the frozen image of the source (made once, at `prepare_to_animate`) drawn with an animated opacity | the same | - (no pixel test) |
| Noise and tiled-surface brushes | GPU: an image shader with repeat tiling; the image is made once | GPU: a tiled image | `a_border_effect_tiles_a_surface_brush` |
| Image brushes, `draw_bitmap` | GPU: an image shader or image draw; a raster image is uploaded once and kept by the resource cache of the GPU context while it fits the budget (128 MiB, next section) | GPU: an image of the scene, uploaded once per image | `tests/FerroUI.RenderTests/media/image_brush_tests.rs`, `tile_brush_tests.rs` |
| Render target bitmaps (`RenderTargetBitmap`) | made by the application, not per frame | the same | - |
| The visual cache (`CacheMode`) | a GPU layer kept while the content is the same | the same | - (no pixel test) |
| 3D scenes (Viewport3D) (#608, [media3d.md](media3d.md)) | gap since D3, the CPU reference everywhere (feature `media3d`, on with the facade's `media3d`): the scene is rasterized by `ferroui-render3d::cpu` at the resolution of the device and drawn as a raster image (`drawable_image`, so Graphite uploads it once), kept while the snapshot, the size, the scale and the edge mode are unchanged. On raster and headless surfaces that is the defined path; with a GPU context (Graphite, Ganesh GL, ANGLE, WebGL2, Vulkan) it is the counted last resort (`ferroui_render3d::scenes_drawn_on_cpu`, the first one logged). Without the feature nothing is drawn and the scene is counted (`rendering::scene3d::scenes_not_drawn`). Planned: the Metal executor on Graphite (D5) and the GL executor on Ganesh GL, ANGLE, Android and WebGL2 (D8), both from the one WGSL source translated by naga at build time with the uniform layout of `ferroui_render3d::uniforms`, into a pooled offscreen texture with private depth and MSAA drawn as an image, the output reused while the scene is unchanged; the Vulkan executor (D9) | gap since D3, the CPU reference in every mode (feature `media3d`): the image painted as an image of the scene, the same image (one upload) while the scene is unchanged; the CPU mode is the reference, the hybrid and GPU modes count each scene (`scenes_drawn_on_cpu`). Planned: the wgpu executor in the hybrid and GPU modes (D4, Metal first, then Vulkan, DX12 and GLES in D9), the output painted with `paint_device_layer`; WebGL2: the CPU reference until route step 3 | D3: `tests/FerroUI.RenderBackends.Comparison/scene3d_tests.rs`: the 11 scenes against their CPU reference and their golden properties in every backend and mode at 1x and 2x, Skia raster against each Vello mode, the kept output; the unit tests of `ferroui-render3d` (clipping, the fill rule, depth, interpolation, lighting, the layouts). Planned: the steady-state and compile-once tests of D4 and D5 (media3d.md, section 4) |

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

## Pipelines made ahead of their first draw (#437)

**The stall.** A GPU backend makes the pipeline (the program) of a kind of draw the first time it draws it. On macOS with Skia Graphite and a cold Metal shader cache (the first run of a freshly built or copied binary), the first frame that drew an `AcrylicBrush` (the NavigationView pane, flyouts, menus, the CommandBar) stalled 160 to 455 ms; `sample` showed the time in `skgpu::graphite::SharedContext::findOrCreateGraphicsPipeline`. The acrylic of WinUI is an effect graph: a hard-edged blur of the backdrop, a luminosity blend and a colour blend layer, the noise at 2 %, and with a change of the material policy a cross-fade (a `Plus` layer): several pipelines, each compiled on the render thread inside that frame. Graphite compiles the pipelines of a recording when the recording is made (`Recorder::snap`, `DrawPass::prepareResources`) on the thread that snaps, and keeps them in the global cache of the shared context, keyed by the render step, the paint (shader graph and blend) and the format and sample count of the target (`MtlCaps::makeGraphicsPipelineKey`; load and store operations are not part of the key).

**What the binding offers.** skia-safe 0.153 binds Graphite's `Context` (`make_recorder`, `insert_recording`, `submit`), `Recorder` (`snap`), `Recording`, the surfaces and images, and an empty `ContextOptions` (`src/gpu/graphite/context.rs`, `recorder.rs`, `context_options.rs` of the crate). It binds neither the precompile API (`PrecompileContext`, `PaintOptions`, `include/gpu/graphite/precompile/` exist in the Skia sources of skia-bindings but have no glue in `src/graphite.cpp`), nor the fields of `ContextOptions` (an `SkExecutor` for threaded compiles, the `PipelineCachingCallback`, `PersistentPipelineStorage`). `Context` and `Recorder` are neither `Send` nor `Sync`; a `Recording` is `Send`. For Ganesh, `GrContextOptions::PersistentCache` is not bound either (`src/gpu/ganesh/context_options.rs`: "TODO: PersistentCache"); `DirectContext::precompile_shader` is, but without the cache there is nothing to feed it. So the route is drawing the effects once, ahead of the frames that show them.

**The mechanism.**

- *What is prewarmed.* A theme registers the effect graphs it draws (`ferroui_base::rendering::composition::effects::register_effect_prewarm`, the list of the process, loaded as an effect factory loads an effect). The WinUI theme registers the in-app acrylic when it is created, and `AcrylicBrush` when the first brush is created (`AcrylicBrush::register_effect_prewarm`, names `ACRYLIC_PREWARM_EFFECTS`): the translucent material and the same with the cross-fade. The opaque material and the window acrylic are colour fills and the noise, which these draw too. Each registration of a new name is a new generation of the list. The custom effects of `Visual.Effect` (#455) register themselves: a type with a graph registers its graph, drawn at the values of its description around the image (a colour matrix layer that is the identity there is not drawn: a colour matrix layer is one program whatever its matrix), and a runtime shader registers the value of its default, pushed as an effect of the drawing context around the image, both under `custom-effect:<NAME>` when the type is first loaded (the library when the registry of the custom effects is first read).
- *When and how it is drawn.* After a frame, a composition target (`ServerCompositionTarget::prewarm_effects`) takes the graphs registered since the generation it, or another target of its compositor at the same scaling, prewarmed, and hands them to its render target (`IRenderTarget::prewarm`), which gives it a drawing context of the kind its frames draw with, over an offscreen surface of the kind its frames draw the scene onto. The target paints each graph once through the painting of the effect brushes (`paint_effect_graph_for_prewarm`: a transient `ServerCompositionEffectBrush`, the values of the description, the source parameters drawn as the noise image of the materials, decoded as the images of the application are), at the scaling of the scene, in a scene of 128 logical units: the drawing context makes the layers, image filters, blends and draws a brush of the graph makes, so the pipelines are those of the frames. The first frame is not delayed; a target that does not prewarm is not asked again for that generation. `FERROUI_EFFECT_PREWARM=0` turns this off (for a measure).
- *Skia, Graphite.* `SkiaGpuRenderTarget::prewarm` asks the context for a prewarm (`ISkiaGrContext::begin_prewarm`): `GraphiteGrContext` makes a second recorder (`Context::make_recorder`) and returns a context over it (`GraphitePrewarmContext`, `gpu/graphite/graphite_prewarm.rs`). The surface of the prewarm has the colour type of the layers the composition target draws the scene onto (`SurfaceRenderTarget`: the native colour type, premultiplied, no colour space), so the keys match those of the frames. The render thread draws (about a millisecond: drawing records commands); the surfaces are released, which hands their draws to the recorder; then the recorder moves to a thread of its own (`ferroui-graphite-prewarm`), which snaps it inside an autorelease pool: the pipelines compile there, into the global cache. The recording is dropped, never inserted: nothing of it reaches the GPU or the screen. A recorder made by `Context::makeRecorder` has a resource provider of its own and shares only the shared context with the context of the frames, whose pipeline cache and pipeline manager lock what several threads use; Skia asserts only that two threads do not use one recorder at once, so the move is sound once the render thread is done with it (the `unsafe impl Send` of `PrewarmCompile` states this). A frame that needs a pipeline the thread is still compiling finds its task in the pipeline manager and waits for it rather than compiling it again.
- *Skia, Ganesh* (OpenGL, WebGL2, Vulkan, Direct3D, Metal on iOS). The context has no second recorder: the prewarm draws onto an offscreen surface of the context of the frames and flushes it, so the programs are linked on the render thread between the first two frames, the slot of the prewarm of the runtime effects (see "The software masks of Ganesh"). On a GL window surface (bottom-left origin) a program that depends on the origin may still be linked again by the first frame that draws it there.
- *Vello.* vello 0.11 compiles all its pipelines when its renderer is made (`Renderer::new`, `full_shaders`), and vello_gpu 0.3 (the hybrid renderer) makes its render pipelines when its programs are made; wgpu on Metal compiles a pipeline when it is created. The render targets of Vello keep the default of `IRenderTarget::prewarm` (nothing to prewarm for the acrylic). The pipelines FerroUI makes lazily are the passes of `gpu/effect_passes.rs`: the lighting pass on the first frame that draws scene lighting, each pass of the layers with an effect on the first frame that draws such a layer (not acrylic), and the pass of a shader effect type on the first frame that draws it (once per device and type: `the_first_frame_of_the_library_makes_one_pipeline_per_shader_type`). A prewarm of these would need a render target of Vello to draw onto a scene of its device offscreen and submit it, which none does yet: the pipelines of a shader effect are one render pipeline each, and their compile time has not been measured.
- *Raster.* The raster render targets keep the default (`false`): nothing is drawn. Nor does a GPU render target in the iOS simulator, whose Metal compiler rejects the destination reads of the luminosity and colour blends (previous section).
- *The option.* `SkiaOptions::prewarm_effect_pipelines` (on by default) turns the prewarm of the Skia render targets off.

**Measuring.** `FERROUI_SLOW_FRAME_MS=<ms>` prints each frame of a composition target that takes at least that long (`slow frame of composition target <id>: frame <n> took <t> ms`); `FERROUI_FRAME_TIMES=1` prints the summary of the frames when the window closes. With either, the prewarm prints `effect prewarm of composition target <id>: <n> effects drawn offscreen in <t> ms` (the render thread) and, on Graphite, `graphite pipeline prewarm: recorded in <t> ms on the render thread, compiled in <t> ms on a thread of its own`. A cold Metal shader cache is the first run of a binary under a new name (copy the binary to a fresh file name for each cold run), or remove the folder of the binary under `$(getconf DARWIN_USER_CACHE_DIR)com.apple.metal/`. The page "Acrylic brush" of the control catalog shows `AcrylicBrush` cards; the frame that first shows the page is the one that stalled:

```text
cp <target>/fast-release/control-catalog-desktop /tmp/cc-cold-1
FERROUI_EFFECT_PREWARM=0 FERROUI_SLOW_FRAME_MS=25 FERROUI_FRAME_TIMES=1 \
  FERROUI_SMOKE_SCREENSHOTS=/tmp/cc-shots FERROUI_SMOKE_SCREENSHOT_PAGES="Acrylic brush" /tmp/cc-cold-1   # before
cp <target>/fast-release/control-catalog-desktop /tmp/cc-cold-2
FERROUI_SLOW_FRAME_MS=25 FERROUI_FRAME_TIMES=1 \
  FERROUI_SMOKE_SCREENSHOTS=/tmp/cc-shots FERROUI_SMOKE_SCREENSHOT_PAGES="Acrylic brush" /tmp/cc-cold-2   # after
```

Run each binary a second time for the warm cache. The page "Acrylic" draws `ExperimentalAcrylicBorder` (the material of the drawing context, `draw_rectangle_with_material`), which is not an effect graph and is not in the list yet.

## Vello: what is missing and the GPU route

vello's renderers cannot express the lighting, the colour matrix, the multiply layer, the pixel
operations or a convolution: they compose layers with the blend modes of the Compositing and
Blending specification, and have no colour matrix, no blend that multiplies and keeps the alpha
below, and no hook for a custom shader. The GPU renderer and WebGL2 have no filter layers either.
Those paths drew their content with the CPU renderer and uploaded the result on every frame, in
every mode; steps 1 and 2 of the route close the gap in the hybrid and GPU modes, step 3 is
WebGL2.

The route, in pull requests stacked on the Skia ones:

1. **A pass FerroUI owns** (`src/Vello/FerroUI.Vello/gpu/effect_passes.rs`, #459). A wgpu render
   pass with a WGSL port of the programs of `ferroui-skia/composition_effects.rs`, compiled once
   per device, with the parameters as uniforms, writes a texture of the device from a pool (sizes
   rounded up to 32 pixels, reused once the scene that painted with a texture was rendered); the
   scene paints with it through the `VelloSceneTexture` route, one texel per device pixel.
   Done for the scene lighting (`draw_lighting`) in the hybrid and GPU modes.
2. **Layers with an effect** (#455 E3, done in the hybrid and GPU modes). The multiply layer of
   lit visuals, the colour matrix layers, the pixel operations, the matrix convolution and the
   luminance mask need the content of the layer as an input. The drawing context
   (`drawing_context_impl/effects.rs`) records the content into a scene of the device (a scene of
   the hybrid renderer of a pooled size, as the filter scenes of the blur are), renders it into a
   texture of the pool (`render_to_texture`), and a pass of `gpu/effect_passes.rs` reads it into
   another texture of the pool: the colour matrix in both alpha modes, the light multiply (the
   light math of the lighting pass, evaluated per pixel, times the content), the pixel operation
   of two sources (each next source combined into what the sources before made), the
   convolution. The scene the layer was pushed in paints the result over the bounds of the layer,
   one texel per pixel. Ordering is that of the queue: the content is submitted, then the pass,
   then the frame that paints with the result. A texture is released once the pass that reads
   it, or the scene that paints with it, is submitted (the hybrid sink holds the textures it
   paints with until it is dropped, as the GPU sink does), so a texture of the pool is never
   written before what reads it ran; the pool's allocation counter stays constant over steady
   frames (`steady_frames_of_the_layers_allocate_no_texture`). Each program is compiled once per
   device and thread, its uniforms written for each pass, its bind groups kept by the identities
   of the pooled textures. The scenes of the blur and the drop shadow drawn on the device take
   their textures from the same pool. A layer inside a scene of the CPU renderer, the multiply
   layer of a light map (more than 8 lights) and a build of the GPU renderer alone keep the CPU
   path. The blur and drop shadow of the GPU mode were already drawn on the device in a build
   with the hybrid renderer (its filter layers in a scene of the device the GPU scene paints
   with), so the two-pass Gaussian of the plan is not needed there; it remains for a build of the
   GPU renderer alone, which draws them on the processor.
3. **WebGL2** (next). The same programs in GLSL ES 3.0 on the WebGL2 context the renderer uses,
   with framebuffer textures in place of wgpu textures (`hybrid-webgl` has no device textures yet:
   the sink has to learn to paint with a framebuffer texture first). Until then WebGL2 draws the
   layers of step 2 with the CPU path, as before E3, and the shader effects of #455 by their CPU
   references. The GLSL ES 3.0 of a shader effect's pass is ready: `ferroui-build` translates the
   WGSL module of its description with naga at build time (`ferroui_build::shader_effects`,
   decision 2 of custom-effects.md), so the browser module will carry no translator; the renderer
   does not consume it yet.
4. **The CPU renderer** keeps the CPU path: it is the reference.
5. **Validation:** each pass against the exact result or the CPU reference in each GPU mode
   (`gpu/effect_pass_tests.rs`), `tests/FerroUI.RenderBackends.Comparison` scenes against Skia (the
   custom effects of #455 in `custom_effect_scenes.rs`, each backend and mode also against the CPU
   reference of each effect), and the allocation counter of the pool, which stays constant over
   steady frames.
