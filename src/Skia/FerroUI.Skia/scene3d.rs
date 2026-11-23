//! 3D scenes on the Skia backend (#608, `docs/porting/media3d.md`, steps D3 and D5).
//!
//! **On Graphite (Metal; section 3.19)** the scene of a `DrawScene3D` is drawn on the GPU by the
//! Metal executor of `ferroui-metal` (`scene3d`), on the Metal device and command queue of the
//! Graphite context, into a texture of the device that is wrapped once as a Graphite image
//! (`backend_textures::make_metal`, `graphite::images::wrap_texture`) and drawn over the bounds of
//! the scene like any image: under the transform, clip and opacity of the canvas, sampled
//! bilinearly. Nothing is read back and nothing is rasterized on the CPU.
//!
//! - **Kept outputs.** Each Graphite context keeps up to eight outputs, each a texture with
//!   its image and the uniform buffers of its viewport. A scene whose snapshot, generations, size,
//!   scale, edge mode and brush textures are those of a kept output draws that output again, with
//!   no 3D pass ([`SkiaScene3DCounters::cached_output_frames`]); otherwise the output of the same
//!   viewport (the same device position and size), of a scene that is gone, a new one or the least
//!   recently used is drawn again: only the uniforms that changed are written, only new meshes and
//!   brush textures uploaded, no pipeline the executor has is made.
//! - **Order.** The 3D pass is one command buffer committed to the queue the Graphite context
//!   submits to, when the scene is drawn: before the recording of the frame that draws its output,
//!   which the context submits at its next flush. An output drawn since the last flush is read by
//!   that recording, so a scene that has to be drawn again before the flush draws into another
//!   texture. A frame that draws a scene is one only the recorder of the context draws
//!   (`ISkiaGrContext::note_direct_only_draw`): the threads of a pipelined frame layer never read
//!   an output.
//! - **Failures.** An executor that cannot be made, or a render that fails, is reported once; the
//!   context then draws its 3D scenes with the CPU reference, each counted
//!   (`ferroui_render3d::scenes_drawn_on_cpu`). The outputs of a context are released with it.
//!
//! **On Ganesh GL (ANGLE, EGL, Android GLES, CGL, WebGL2; section 3.20)** the scene is drawn the
//! same way by the GL executor of `ferroui-opengl`, on the current context, into a texture that is
//! borrowed once as a Ganesh image. The executor changes the state of the context, so Ganesh is
//! told afterwards (`reset_context`).
//!
//! **Elsewhere** the scene is rasterized by the CPU reference of `ferroui-render3d` at the
//! resolution of the device and drawn as an image over its bounds. On a raster surface this is the
//! defined path (the surface has no GPU). On a GPU surface without an executor (Ganesh on Vulkan,
//! until step D9) it is the last resort: each such scene is counted and the first is logged. The
//! image of a scene is kept on the render thread while its snapshot, the size, the scale and the
//! edge mode are unchanged, so an unchanged viewport is neither rasterized nor uploaded again.

use crate::gpu::{ISkiaGpu, ISkiaGrContext};
use crate::sk_paint_cache::SkPaintCache;
use crate::skia_sharp_extensions::to_sk_rect;
use ferroui_base::rendering::scene3d::Scene3DFrame;
use ferroui_base::Matrix;
use ferroui_render3d::{report_scene_drawn_on_cpu, scene_pixel_size, CpuRenderOptions, Scene3DOutputCache};
use skia_safe as sk;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

ferroui_base::ferro_thread_local! {
    /// The images of the scenes drawn recently on this thread by the CPU reference.
    static OUTPUTS: RefCell<Scene3DOutputCache<sk::Image>> = RefCell::new(Scene3DOutputCache::new());
    static COUNTERS: Cell<FrameCounters> = const { Cell::new(FrameCounters::ZERO) };
}

/// What the drawing contexts of this thread did with the 3D scenes of Graphite, on top of what
/// the executors did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct FrameCounters {
    cached_output_frames: u64,
    uniform_only_frames: u64,
    output_textures_allocated: u64,
    scenes_drawn_on_gpu: u64,
}

impl FrameCounters {
    const ZERO: FrameCounters = FrameCounters {
        cached_output_frames: 0,
        uniform_only_frames: 0,
        output_textures_allocated: 0,
        scenes_drawn_on_gpu: 0,
    };
}

#[cfg_attr(not(all(target_vendor = "apple", not(ferro_skia_apple_ganesh))), allow(dead_code))]
fn count(change: impl FnOnce(&mut FrameCounters)) {
    COUNTERS.with(|counters| {
        let mut value = counters.get();
        change(&mut value);
        counters.set(value);
    });
}

/// What the Skia backend did with 3D scenes on the GPU on this thread since it started: what the
/// steady-state tests of `docs/porting/media3d.md` (section 3.19) count. All zero where the
/// backend has no GPU executor for 3D scenes (every target but Graphite on Apple's).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct SkiaScene3DCounters {
    /// Render pipelines made by the executors.
    pub pipelines_created: u64,
    /// Shader libraries compiled by the executors (one per device).
    pub libraries_compiled: u64,
    /// Meshes uploaded (a new `(id, version)`).
    pub mesh_uploads: u64,
    /// Textures of brushes uploaded (a new key).
    pub brush_textures_uploaded: u64,
    /// Scenes drawn again with nothing new but uniforms: no mesh, texture, pipeline or buffer.
    pub uniform_only_frames: u64,
    /// Scenes drawn from a kept output, without a 3D pass.
    pub cached_output_frames: u64,
    /// 3D passes committed.
    pub passes: u64,
    /// Textures made for 3D scenes: the outputs, and the multisampled colour, depth, resolved
    /// colour and brush textures of the executors.
    pub textures_allocated: u64,
    /// Buffers made by the executors (vertices, indices, uniforms).
    pub buffers_created: u64,
    /// The bytes of uniforms written.
    pub uniform_bytes: u64,
    /// Scenes drawn on the GPU (a 3D pass or a kept output).
    pub scenes_drawn_on_gpu: u64,
}

/// What the Skia backend did with 3D scenes on the GPU on this thread since it started.
pub fn scene3d_counters() -> SkiaScene3DCounters {
    let frames = COUNTERS.with(Cell::get);
    #[allow(unused_mut)]
    let mut counters = SkiaScene3DCounters {
        uniform_only_frames: frames.uniform_only_frames,
        cached_output_frames: frames.cached_output_frames,
        textures_allocated: frames.output_textures_allocated,
        scenes_drawn_on_gpu: frames.scenes_drawn_on_gpu,
        ..SkiaScene3DCounters::default()
    };
    #[cfg(all(target_vendor = "apple", not(ferro_skia_apple_ganesh)))]
    {
        let executors = ferroui_metal::scene3d::counters();
        counters.pipelines_created = executors.pipelines_created;
        counters.libraries_compiled = executors.libraries_compiled;
        counters.mesh_uploads = executors.mesh_uploads;
        counters.brush_textures_uploaded = executors.textures_uploaded;
        counters.passes = executors.passes;
        // The outputs are counted by the executor too, as they are its textures.
        counters.textures_allocated = executors.textures_created;
        counters.buffers_created = executors.buffers_created;
        counters.uniform_bytes = executors.uniform_bytes;
    }
    counters
}

/// Releases what the 3D scenes of the Graphite context `context_id` hold (its outputs and its
/// executor): called by the context before it goes.
#[cfg_attr(not(all(target_vendor = "apple", not(ferro_skia_apple_ganesh))), allow(dead_code))]
pub(crate) fn release_context(context_id: u64) {
    #[cfg(all(target_vendor = "apple", not(ferro_skia_apple_ganesh)))]
    graphite::release(context_id);
    #[cfg(not(all(target_vendor = "apple", not(ferro_skia_apple_ganesh))))]
    let _ = context_id;
}

/// Releases the outputs and the GL executor of the Ganesh context `context_id`. With
/// `context_current` their GL names are deleted; without it the context took them with it.
#[cfg(ferro_skia_ganesh_gl)]
pub(crate) fn release_gl_context(context_id: u64, context_current: bool) {
    ganesh_gl::release(context_id, context_current);
}

/// Without Ganesh GL there is nothing to release.
#[cfg(not(ferro_skia_ganesh_gl))]
#[allow(dead_code)]
pub(crate) fn release_gl_context(_context_id: u64, _context_current: bool) {}

/// Releases the outputs and the Vulkan executor of the Ganesh context `context_id`, after the
/// context has submitted what it recorded. Without `release_resources` the device is lost and its
/// handles go with it.
#[cfg(ferro_skia_vulkan)]
pub(crate) fn release_vulkan_context(context_id: u64, release_resources: bool) {
    ganesh_vk::release(context_id, release_resources);
}

/// Draws the scene of `frame` on `canvas`, whose transform to the device is `device` (the
/// transform of the context with its scale to the DPI), with `opacity` and antialiased edges or
/// not. Returns `false` when the image of the scene could not be made.
pub(crate) fn draw_scene_3d(
    canvas: &sk::Canvas,
    gr_context: Option<&Rc<dyn ISkiaGrContext>>,
    gpu: Option<&Rc<dyn ISkiaGpu>>,
    device: Matrix,
    opacity: f64,
    antialiased: bool,
    frame: &Scene3DFrame,
) -> bool {
    let bounds = frame.bounds;
    if bounds.width <= 0.0 || bounds.height <= 0.0 {
        return true;
    }
    let (size, scale) = scene_pixel_size(bounds, (device.m11, device.m12), (device.m21, device.m22));
    let mut paint = SkPaintCache::get();
    paint.set_color(sk::Color::from_argb((255.0 * opacity.clamp(0.0, 1.0)) as u8, 255, 255, 255));
    paint.set_anti_alias(antialiased);
    let sampling = sk::SamplingOptions::new(sk::FilterMode::Linear, sk::MipmapMode::None);

    #[cfg(all(target_vendor = "apple", not(ferro_skia_apple_ganesh)))]
    if let (Some(gr_context), Some(gpu)) = (gr_context, gpu) {
        // The device pixel of the top left of the viewport: what names a viewport whose snapshot
        // changed.
        let x = device.m11 * bounds.x + device.m21 * bounds.y + device.m31;
        let y = device.m12 * bounds.x + device.m22 * bounds.y + device.m32;
        let origin = (x.round() as i32, y.round() as i32);
        match graphite::scene_image(gr_context, gpu, &frame.scene, size, scale, antialiased, origin) {
            graphite::SceneImage::Drawn { image, size } => {
                // Only the recorder of the context draws the output (see the module).
                gr_context.note_direct_only_draw();
                let source = sk::Rect::from_wh(size.0 as f32, size.1 as f32);
                canvas.draw_image_rect_with_sampling_options(
                    &image,
                    Some((&source, sk::canvas::SrcRectConstraint::Strict)),
                    to_sk_rect(bounds),
                    sampling,
                    &paint,
                );
                SkPaintCache::return_reset(paint);
                return true;
            }
            // A prewarm of pipelines draws no 3D scene: its frame is not shown, and the executor
            // makes its own pipelines when a scene is first drawn.
            graphite::SceneImage::Skipped => {
                SkPaintCache::return_reset(paint);
                return true;
            }
            graphite::SceneImage::OnCpu => {}
        }
    }
    #[cfg(ferro_skia_ganesh_gl)]
    if let (Some(gr_context), Some(gpu)) = (gr_context, gpu) {
        let x = device.m11 * bounds.x + device.m21 * bounds.y + device.m31;
        let y = device.m12 * bounds.x + device.m22 * bounds.y + device.m32;
        let origin = (x.round() as i32, y.round() as i32);
        if let Some((image, size)) =
            ganesh_gl::scene_image(gr_context, gpu, &frame.scene, size, scale, antialiased, origin)
        {
            let source = sk::Rect::from_wh(size.0 as f32, size.1 as f32);
            canvas.draw_image_rect_with_sampling_options(
                &image,
                Some((&source, sk::canvas::SrcRectConstraint::Strict)),
                to_sk_rect(bounds),
                sampling,
                &paint,
            );
            SkPaintCache::return_reset(paint);
            return true;
        }
    }
    #[cfg(ferro_skia_vulkan)]
    if let (Some(gr_context), Some(gpu)) = (gr_context, gpu) {
        let x = device.m11 * bounds.x + device.m21 * bounds.y + device.m31;
        let y = device.m12 * bounds.x + device.m22 * bounds.y + device.m32;
        let origin = (x.round() as i32, y.round() as i32);
        if let Some((image, size)) =
            ganesh_vk::scene_image(gr_context, gpu, &frame.scene, size, scale, antialiased, origin)
        {
            let source = sk::Rect::from_wh(size.0 as f32, size.1 as f32);
            canvas.draw_image_rect_with_sampling_options(
                &image,
                Some((&source, sk::canvas::SrcRectConstraint::Strict)),
                to_sk_rect(bounds),
                sampling,
                &paint,
            );
            SkPaintCache::return_reset(paint);
            return true;
        }
    }
    #[cfg(not(any(
        ferro_skia_ganesh_gl,
        ferro_skia_vulkan,
        all(target_vendor = "apple", not(ferro_skia_apple_ganesh))
    )))]
    let _ = gpu;

    let options = CpuRenderOptions { antialiased };
    let image = OUTPUTS.with(|outputs| {
        outputs.borrow_mut().get_or_render(&frame.scene, size, scale, options, |image| {
            let info = sk::ImageInfo::new(
                (image.width as i32, image.height as i32),
                sk::ColorType::RGBA8888,
                sk::AlphaType::Premul,
                None,
            );
            // An image of pixels: uploaded to the GPU when drawn there, once while it is kept.
            crate::composition_effects::note_pixel_image();
            sk::images::raster_from_data(&info, sk::Data::new_copy(&image.rgba), image.width as usize * 4)
        })
    });
    let Some(image) = image else {
        SkPaintCache::return_reset(paint);
        return false;
    };
    if gr_context.is_some() {
        report_scene_drawn_on_cpu("Skia (GPU)");
    }
    let image = crate::gpu::drawable_image(gr_context.map(|context| &**context), image, false);
    canvas.draw_image_rect_with_sampling_options(&image, None, to_sk_rect(bounds), sampling, &paint);
    SkPaintCache::return_reset(paint);
    true
}

/// The Ganesh GL route (section 3.20 of the design).
#[cfg(ferro_skia_ganesh_gl)]
mod ganesh_gl {
    use super::count;
    use crate::gpu::ganesh::GaneshGrContext;
    use crate::gpu::{ISkiaGpu, ISkiaGrContext};
    use crate::leased_gpu_bitmap::LeasedGpuBitmap;
    use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
    use ferroui_base::platform::{IOptionalFeatureProvider, SharedBitmapImpl};
    use ferroui_base::rendering::scene3d::Scene3DSnapshot;
    use ferroui_base::PixelSize;
    use ferroui_opengl::gl_consts::{GL_RGBA8, GL_TEXTURE_2D};
    use ferroui_opengl::scene3d::{
        counters, Scene3DGlDeviceTextures, Scene3DGlError, Scene3DGlExecutor, Scene3DGlTarget, Scene3DGlTexture,
        Scene3DGlViewport,
    };
    use ferroui_opengl::IGlContext;
    use ferroui_render3d::brush::{scene_texture_inputs, DeviceBitmap};
    use skia_safe as sk;
    use skia_safe::gpu::gl::TextureInfo;
    use skia_safe::gpu::{backend_textures, images, Mipmapped, SurfaceOrigin};
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Weak};

    /// The outputs a context keeps: the viewports of a window or two.
    const CAPACITY: usize = 8;

    /// A kept output: the texture a scene was drawn into, what it was drawn for, and the uniform
    /// buffers of its viewport.
    struct Slot {
        scene: Weak<Scene3DSnapshot>,
        generations: (u64, u64, u64),
        size: (u32, u32),
        scale_bits: u64,
        antialiased: bool,
        /// The textures of the image brushes of the scene.
        inputs: u64,
        /// The device pixel of the top left of the viewport.
        origin: (i32, i32),
        image: sk::Image,
        texture: Scene3DGlTexture,
        viewport: Scene3DGlViewport,
        last_used: u64,
        /// The flush count of the context when the image was last drawn: until a flush after it,
        /// a recording not yet submitted reads the texture.
        drawn_at_flush: Option<u64>,
    }

    impl Slot {
        fn is_pending(&self, flushes: u64) -> bool {
            self.drawn_at_flush.is_some_and(|drawn| drawn == flushes)
        }
    }

    /// The executor of a context and its kept outputs.
    struct ContextScenes {
        context_id: u64,
        /// The executor, or why the context draws its 3D scenes with the CPU reference.
        executor: Result<Scene3DGlExecutor, Scene3DGlError>,
        slots: Vec<Slot>,
        clock: u64,
    }

    ferroui_base::ferro_thread_local! {
        static CONTEXTS: RefCell<Vec<ContextScenes>> = const { RefCell::new(Vec::new()) };
    }

    static FAILURE_REPORTED: AtomicBool = AtomicBool::new(false);

    /// Reports once that a context draws its 3D scenes with the CPU reference, and why.
    fn report_failure(error: &Scene3DGlError) {
        if FAILURE_REPORTED.swap(true, Ordering::Relaxed) {
            return;
        }
        if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::VISUAL) {
            let reason = error.reason.as_str();
            logger.log_with_values(
                None,
                "The Skia backend draws 3D scenes (Viewport3D) with the CPU reference on this GL context: \
                 {Reason} (docs/porting/media3d.md, section 3.20).",
                &[&reason],
            );
        }
    }

    /// Releases the outputs and the executor of a context; see [`super::release_gl_context`].
    pub(super) fn release(context_id: u64, context_current: bool) {
        let released = CONTEXTS.try_with(|contexts| {
            let mut contexts = contexts.borrow_mut();
            let index = contexts.iter().position(|entry| entry.context_id == context_id);
            index.map(|index| contexts.swap_remove(index))
        });
        let Ok(Some(mut entry)) = released else { return };
        let slots = std::mem::take(&mut entry.slots);
        match (&mut entry.executor, context_current) {
            (Ok(executor), true) => {
                for mut slot in slots {
                    // The image borrows the texture: it goes first.
                    drop(slot.image);
                    slot.viewport.dispose(executor);
                    executor.delete_output_texture(slot.texture);
                }
                executor.dispose();
            }
            // Without a current context the names went, or go, with the context.
            _ => drop(slots),
        }
    }

    /// The frames of a video or a camera that a render samples from their textures. The storage
    /// of each is kept, as for a 2D draw, until the GPU has done what the context submits next.
    struct GlFrames<'a> {
        ganesh: &'a GaneshGrContext,
        context_id: u64,
        /// The bitmaps this render has kept the storage of.
        retained: RefCell<Vec<*const ()>>,
    }

    impl Scene3DGlDeviceTextures for GlFrames<'_> {
        fn sampling(&self, bitmap: &SharedBitmapImpl) -> DeviceBitmap {
            let Some(leased) = bitmap.as_any().downcast_ref::<LeasedGpuBitmap>() else {
                return DeviceBitmap::NotOnDevice;
            };
            if leased.gl_texture(self.context_id).is_none() {
                return DeviceBitmap::NotOnDevice;
            }
            let address = std::ptr::from_ref(leased).cast::<()>();
            let mut retained = self.retained.borrow_mut();
            if !retained.contains(&address) {
                if !leased.retain_for_render(self.ganesh) {
                    return DeviceBitmap::NotOnDevice;
                }
                retained.push(address);
            }
            DeviceBitmap::Stored
        }

        fn device_texture(&self, bitmap: &SharedBitmapImpl) -> Option<(i32, (u32, u32))> {
            bitmap.as_any().downcast_ref::<LeasedGpuBitmap>()?.gl_texture(self.context_id)
        }
    }

    /// The size of an output for a scene of `size` pixels: rounded up to 32 pixels, so that a
    /// viewport that is resized by a few pixels keeps its texture.
    fn pooled_size(size: (u32, u32)) -> (u32, u32) {
        (size.0.div_ceil(32) * 32, size.1.div_ceil(32) * 32)
    }

    /// A texture of `size`, borrowed once as an image of `ganesh`. Its first row is the top of the
    /// scene.
    fn output(
        executor: &Scene3DGlExecutor,
        ganesh: &GaneshGrContext,
        size: (u32, u32),
    ) -> Option<(Scene3DGlTexture, sk::Image)> {
        let texture = executor.create_output_texture(size.0, size.1)?;
        let info = TextureInfo {
            target: GL_TEXTURE_2D as u32,
            id: texture.id() as u32,
            format: GL_RGBA8 as u32,
            ..Default::default()
        };
        // SAFETY: the texture is a live `GL_TEXTURE_2D` of the current context, which the slot
        // keeps for as long as the image.
        let backend =
            unsafe { backend_textures::make_gl((size.0 as i32, size.1 as i32), Mipmapped::No, info, "") };
        let image = ganesh.with_context(|context| {
            images::borrow_texture_from(
                context,
                &backend,
                SurfaceOrigin::TopLeft,
                sk::ColorType::RGBA8888,
                sk::AlphaType::Premul,
                None,
            )
        });
        match image {
            Some(image) => Some((texture, image)),
            None => {
                executor.delete_output_texture(texture);
                None
            }
        }
    }

    fn new_slot(
        executor: &Scene3DGlExecutor,
        ganesh: &GaneshGrContext,
        size: (u32, u32),
        origin: (i32, i32),
        clock: u64,
    ) -> Option<Slot> {
        let (texture, image) = output(executor, ganesh, size)?;
        Some(Slot {
            scene: Weak::new(),
            generations: (0, 0, 0),
            size,
            scale_bits: 0,
            antialiased: false,
            inputs: 0,
            origin,
            image,
            texture,
            viewport: Scene3DGlViewport::new(),
            last_used: clock,
            drawn_at_flush: None,
        })
    }

    /// The image of `scene` drawn at `size` pixels on the current GL context of `gpu`, and the
    /// pixels of the scene at its top left. `None` when the context cannot draw it on the GPU:
    /// the caller draws the CPU reference.
    pub(super) fn scene_image(
        gr_context: &Rc<dyn ISkiaGrContext>,
        gpu: &Rc<dyn ISkiaGpu>,
        scene: &Arc<Scene3DSnapshot>,
        size: PixelSize,
        scale: f64,
        antialiased: bool,
        origin: (i32, i32),
    ) -> Option<(sk::Image, (u32, u32))> {
        let ganesh = gr_context.as_any().downcast_ref::<GaneshGrContext>()?;
        if ganesh.is_disposed() {
            return None;
        }
        let context_id = ganesh.frame_context_id()?;
        let flushes = ganesh.flush_count();
        let size = (size.width.max(1) as u32, size.height.max(1) as u32);
        let scale_bits = scale.to_bits();
        let generations = scene.generations();
        let inputs = scene_texture_inputs(scene);
        CONTEXTS.with(|contexts| {
            let mut contexts = contexts.borrow_mut();
            let index = match contexts.iter().position(|entry| entry.context_id == context_id) {
                Some(index) => index,
                None => {
                    // Ganesh on Vulkan has no GL context: no entry, and the caller goes on.
                    let platform = gpu.platform_graphics_context()?;
                    let features: &dyn IOptionalFeatureProvider = &*platform;
                    let gl_context = features.try_get::<dyn IGlContext>()?;
                    let executor = Scene3DGlExecutor::new(gl_context.gl_interface(), gl_context.version());
                    if let Err(error) = &executor {
                        report_failure(error);
                    }
                    contexts.push(ContextScenes { context_id, executor, slots: Vec::new(), clock: 0 });
                    contexts.len() - 1
                }
            };
            let entry = &mut contexts[index];
            let Ok(executor) = &mut entry.executor else { return None };
            entry.clock += 1;
            let clock = entry.clock;

            // 1. A kept output of this scene.
            let same_scene = |slot: &Slot| std::ptr::eq(slot.scene.as_ptr(), Arc::as_ptr(scene));
            if let Some(slot) = entry.slots.iter_mut().find(|slot| {
                let slot: &Slot = slot;
                same_scene(slot)
                    && slot.generations == generations
                    && slot.size == size
                    && slot.scale_bits == scale_bits
                    && slot.antialiased == antialiased
                    && slot.inputs == inputs
            }) {
                slot.last_used = clock;
                slot.drawn_at_flush = Some(flushes);
                count(|counters| {
                    counters.cached_output_frames += 1;
                    counters.scenes_drawn_on_gpu += 1;
                });
                return Some((slot.image.clone(), size));
            }

            // 2. The output to draw into: the same viewport, the output of a scene that is gone, a
            //    new one, or the least recently used; never one a recording not yet submitted reads.
            let gone = |slot: &Slot| slot.scene.strong_count() == 0;
            let same_viewport = |slot: &Slot| slot.origin == origin && slot.size == size;
            let free = |slot: &Slot| !slot.is_pending(flushes);
            let chosen = entry
                .slots
                .iter()
                .position(|slot| free(slot) && same_viewport(slot) && (gone(slot) || same_scene(slot)))
                .or_else(|| entry.slots.iter().position(|slot| free(slot) && same_viewport(slot)))
                .or_else(|| {
                    entry
                        .slots
                        .iter()
                        .enumerate()
                        .filter(|&(_, slot)| free(slot) && gone(slot))
                        .min_by_key(|(_, slot)| slot.last_used)
                        .map(|(index, _)| index)
                });
            let texture_size = pooled_size(size);
            let textures_before = counters().textures_created;
            let least_used = entry
                .slots
                .iter()
                .enumerate()
                .filter(|&(_, slot)| free(slot))
                .min_by_key(|(_, slot)| slot.last_used)
                .map(|(index, _)| index);
            let slot_index = match chosen {
                Some(index) => index,
                None => match least_used.filter(|_| entry.slots.len() >= CAPACITY) {
                    Some(index) => index,
                    // Room for one more, or every output is read by the frame being recorded.
                    None => {
                        let slot = new_slot(executor, ganesh, texture_size, origin, clock);
                        entry.slots.push(slot?);
                        entry.slots.len() - 1
                    }
                },
            };
            let slot = &mut entry.slots[slot_index];
            // A texture of another size: another one. The slot is free, so nothing reads the old.
            if (slot.texture.width(), slot.texture.height()) != texture_size {
                let (texture, image) = output(executor, ganesh, texture_size)?;
                slot.image = image;
                let old = std::mem::replace(&mut slot.texture, texture);
                executor.delete_output_texture(old);
            }
            let new_textures = counters().textures_created - textures_before;

            let target = Scene3DGlTarget { texture: &slot.texture, size, antialiased };
            let frames = GlFrames { ganesh, context_id, retained: RefCell::new(Vec::new()) };
            let rendered = executor.render_with_textures(&mut slot.viewport, scene, &target, &frames);
            // The executor changed the state of the context behind Ganesh.
            ganesh.reset_context();
            match rendered {
                Ok(report) => {
                    slot.scene = Arc::downgrade(scene);
                    slot.generations = generations;
                    slot.size = size;
                    slot.scale_bits = scale_bits;
                    slot.antialiased = antialiased;
                    slot.inputs = inputs;
                    slot.origin = origin;
                    slot.last_used = clock;
                    slot.drawn_at_flush = Some(flushes);
                    let uniform_only = report.is_uniform_only() && new_textures == 0;
                    count(|counters| {
                        counters.output_textures_allocated += new_textures;
                        counters.scenes_drawn_on_gpu += 1;
                        if uniform_only {
                            counters.uniform_only_frames += 1;
                        }
                    });
                    Some((slot.image.clone(), size))
                }
                Err(error) => {
                    // The context draws its 3D scenes with the CPU reference from now on.
                    report_failure(&error);
                    let slots = std::mem::take(&mut entry.slots);
                    for mut slot in slots {
                        drop(slot.image);
                        slot.viewport.dispose(executor);
                        executor.delete_output_texture(slot.texture);
                    }
                    executor.dispose();
                    entry.executor = Err(error);
                    None
                }
            }
        })
    }
}

/// The Ganesh Vulkan route (section 3.21 of the design).
#[cfg(ferro_skia_vulkan)]
mod ganesh_vk {
    use super::count;
    use crate::gpu::ganesh::GaneshGrContext;
    use crate::gpu::vulkan::sk_image_info;
    use crate::gpu::{ISkiaGpu, ISkiaGrContext};
    use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
    use ferroui_base::platform::IOptionalFeatureProvider;
    use ferroui_base::rendering::scene3d::Scene3DSnapshot;
    use ferroui_base::PixelSize;
    use ferroui_render3d::brush::scene_texture_inputs;
    use ferroui_vulkan::ash::vk::{self, Handle};
    use ferroui_vulkan::scene3d::{
        counters, Scene3DVkError, Scene3DVkExecutor, Scene3DVkTarget, Scene3DVkTexture, Scene3DVkViewport,
    };
    use ferroui_vulkan::{IVulkanPlatformGraphicsContext, VulkanImageInfo};
    use skia_safe as sk;
    use skia_safe::gpu::{backend_textures, images, SurfaceOrigin};
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Weak};

    /// The outputs a context keeps: the viewports of a window or two.
    const CAPACITY: usize = 8;

    /// A kept output: the texture a scene was drawn into, what it was drawn for, and the uniform
    /// buffers of its viewport.
    struct Slot {
        scene: Weak<Scene3DSnapshot>,
        generations: (u64, u64, u64),
        size: (u32, u32),
        scale_bits: u64,
        antialiased: bool,
        /// The textures of the image brushes of the scene.
        inputs: u64,
        /// The device pixel of the top left of the viewport.
        origin: (i32, i32),
        image: sk::Image,
        texture: Scene3DVkTexture,
        viewport: Scene3DVkViewport,
        last_used: u64,
        /// The flush count of the context when the image was last drawn: until a flush after it,
        /// a recording not yet submitted reads the texture.
        drawn_at_flush: Option<u64>,
    }

    impl Slot {
        fn is_pending(&self, flushes: u64) -> bool {
            self.drawn_at_flush.is_some_and(|drawn| drawn == flushes)
        }
    }

    /// The executor of a context and its kept outputs.
    struct ContextScenes {
        context_id: u64,
        /// The executor, or why the context draws its 3D scenes with the CPU reference.
        executor: Result<Scene3DVkExecutor, Scene3DVkError>,
        /// The queue family of the images.
        queue_family: u32,
        slots: Vec<Slot>,
        clock: u64,
    }

    ferroui_base::ferro_thread_local! {
        static CONTEXTS: RefCell<Vec<ContextScenes>> = const { RefCell::new(Vec::new()) };
    }

    static FAILURE_REPORTED: AtomicBool = AtomicBool::new(false);

    /// Reports once that a context draws its 3D scenes with the CPU reference, and why.
    fn report_failure(error: &Scene3DVkError) {
        if FAILURE_REPORTED.swap(true, Ordering::Relaxed) {
            return;
        }
        if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::VISUAL) {
            let reason = error.reason.as_str();
            logger.log_with_values(
                None,
                "The Skia backend draws 3D scenes (Viewport3D) with the CPU reference on this Vulkan device: \
                 {Reason} (docs/porting/media3d.md, section 3.21).",
                &[&reason],
            );
        }
    }

    /// Releases the outputs and the executor of a context; see [`super::release_vulkan_context`].
    pub(super) fn release(context_id: u64, release_resources: bool) {
        let released = CONTEXTS.try_with(|contexts| {
            let mut contexts = contexts.borrow_mut();
            let index = contexts.iter().position(|entry| entry.context_id == context_id);
            index.map(|index| contexts.swap_remove(index))
        });
        let Ok(Some(mut entry)) = released else { return };
        let slots = std::mem::take(&mut entry.slots);
        match (&mut entry.executor, release_resources) {
            (Ok(executor), true) => {
                for mut slot in slots {
                    // The image borrows the texture: it goes first.
                    drop(slot.image);
                    slot.viewport.dispose(executor);
                    executor.delete_output_texture(slot.texture);
                }
                executor.dispose();
            }
            // A lost device: its handles go with it.
            _ => drop(slots),
        }
    }

    /// The size of an output for a scene of `size` pixels: rounded up to 32 pixels, so that a
    /// viewport that is resized by a few pixels keeps its texture.
    fn pooled_size(size: (u32, u32)) -> (u32, u32) {
        (size.0.div_ceil(32) * 32, size.1.div_ceil(32) * 32)
    }

    /// An image of `size`, borrowed once as an image of `ganesh`. Its first row is the top of the
    /// scene, and a render leaves it in the layout Skia is told here.
    fn output(
        executor: &mut Scene3DVkExecutor,
        ganesh: &GaneshGrContext,
        queue_family: u32,
        size: (u32, u32),
    ) -> Option<(Scene3DVkTexture, sk::Image)> {
        let texture = executor.create_output_texture(size.0, size.1)?;
        let info = VulkanImageInfo {
            format: texture.format().as_raw() as u32,
            pixel_size: PixelSize::new(size.0 as i32, size.1 as i32),
            handle: texture.image().as_raw(),
            layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL.as_raw() as u32,
            tiling: vk::ImageTiling::OPTIMAL.as_raw() as u32,
            usage_flags: texture.usage().as_raw(),
            level_count: 1,
            sample_count: 1,
            memory_handle: texture.memory().as_raw(),
            view_handle: 0,
            memory_size: texture.allocation_size(),
            is_protected: false,
        };
        let image_info = sk_image_info(&info, queue_family);
        // SAFETY: the image and its memory are live objects of the device of the context, and the
        // slot keeps them for as long as the Skia image.
        let backend = unsafe { backend_textures::make_vk((size.0 as i32, size.1 as i32), &image_info, "") };
        let image = ganesh.with_context(|context| {
            images::borrow_texture_from(
                context,
                &backend,
                SurfaceOrigin::TopLeft,
                sk::ColorType::RGBA8888,
                sk::AlphaType::Premul,
                None,
            )
        });
        match image {
            Some(image) => Some((texture, image)),
            None => {
                executor.delete_output_texture(texture);
                None
            }
        }
    }

    fn new_slot(
        executor: &mut Scene3DVkExecutor,
        ganesh: &GaneshGrContext,
        queue_family: u32,
        size: (u32, u32),
        origin: (i32, i32),
        clock: u64,
    ) -> Option<Slot> {
        let (texture, image) = output(executor, ganesh, queue_family, size)?;
        Some(Slot {
            scene: Weak::new(),
            generations: (0, 0, 0),
            size,
            scale_bits: 0,
            antialiased: false,
            inputs: 0,
            origin,
            image,
            texture,
            viewport: Scene3DVkViewport::new(),
            last_used: clock,
            drawn_at_flush: None,
        })
    }

    /// The image of `scene` drawn at `size` pixels on the Vulkan device of `gpu`, and the
    /// pixels of the scene at its top left. `None` when the context cannot draw it on the GPU:
    /// the caller draws the CPU reference.
    pub(super) fn scene_image(
        gr_context: &Rc<dyn ISkiaGrContext>,
        gpu: &Rc<dyn ISkiaGpu>,
        scene: &Arc<Scene3DSnapshot>,
        size: PixelSize,
        scale: f64,
        antialiased: bool,
        origin: (i32, i32),
    ) -> Option<(sk::Image, (u32, u32))> {
        let ganesh = gr_context.as_any().downcast_ref::<GaneshGrContext>()?;
        if ganesh.is_disposed() {
            return None;
        }
        let context_id = ganesh.frame_context_id()?;
        let flushes = ganesh.flush_count();
        let size = (size.width.max(1) as u32, size.height.max(1) as u32);
        let scale_bits = scale.to_bits();
        let generations = scene.generations();
        let inputs = scene_texture_inputs(scene);
        CONTEXTS.with(|contexts| {
            let mut contexts = contexts.borrow_mut();
            let index = match contexts.iter().position(|entry| entry.context_id == context_id) {
                Some(index) => index,
                None => {
                    // Ganesh on GL has no Vulkan context: no entry, and the caller goes on.
                    let platform = gpu.platform_graphics_context()?;
                    let features: &dyn IOptionalFeatureProvider = &*platform;
                    let vulkan = features.try_get::<dyn IVulkanPlatformGraphicsContext>()?;
                    let queue_family = vulkan.graphics_queue_family_index();
                    let executor = Scene3DVkExecutor::new(vulkan.device());
                    if let Err(error) = &executor {
                        report_failure(error);
                    }
                    contexts.push(ContextScenes { context_id, executor, queue_family, slots: Vec::new(), clock: 0 });
                    contexts.len() - 1
                }
            };
            let entry = &mut contexts[index];
            let queue_family = entry.queue_family;
            let Ok(executor) = &mut entry.executor else { return None };
            entry.clock += 1;
            let clock = entry.clock;

            // 1. A kept output of this scene.
            let same_scene = |slot: &Slot| std::ptr::eq(slot.scene.as_ptr(), Arc::as_ptr(scene));
            if let Some(slot) = entry.slots.iter_mut().find(|slot| {
                let slot: &Slot = slot;
                same_scene(slot)
                    && slot.generations == generations
                    && slot.size == size
                    && slot.scale_bits == scale_bits
                    && slot.antialiased == antialiased
                    && slot.inputs == inputs
            }) {
                slot.last_used = clock;
                slot.drawn_at_flush = Some(flushes);
                count(|counters| {
                    counters.cached_output_frames += 1;
                    counters.scenes_drawn_on_gpu += 1;
                });
                return Some((slot.image.clone(), size));
            }

            // 2. The output to draw into: the same viewport, the output of a scene that is gone, a
            //    new one, or the least recently used; never one a recording not yet submitted reads.
            let gone = |slot: &Slot| slot.scene.strong_count() == 0;
            let same_viewport = |slot: &Slot| slot.origin == origin && slot.size == size;
            let free = |slot: &Slot| !slot.is_pending(flushes);
            let chosen = entry
                .slots
                .iter()
                .position(|slot| free(slot) && same_viewport(slot) && (gone(slot) || same_scene(slot)))
                .or_else(|| entry.slots.iter().position(|slot| free(slot) && same_viewport(slot)))
                .or_else(|| {
                    entry
                        .slots
                        .iter()
                        .enumerate()
                        .filter(|&(_, slot)| free(slot) && gone(slot))
                        .min_by_key(|(_, slot)| slot.last_used)
                        .map(|(index, _)| index)
                });
            let texture_size = pooled_size(size);
            let textures_before = counters().textures_created;
            let least_used = entry
                .slots
                .iter()
                .enumerate()
                .filter(|&(_, slot)| free(slot))
                .min_by_key(|(_, slot)| slot.last_used)
                .map(|(index, _)| index);
            let slot_index = match chosen {
                Some(index) => index,
                None => match least_used.filter(|_| entry.slots.len() >= CAPACITY) {
                    Some(index) => index,
                    // Room for one more, or every output is read by the frame being recorded.
                    None => {
                        let slot = new_slot(executor, ganesh, queue_family, texture_size, origin, clock);
                        entry.slots.push(slot?);
                        entry.slots.len() - 1
                    }
                },
            };
            let slot = &mut entry.slots[slot_index];
            // A texture of another size: another one. The slot is free, so nothing reads the old.
            if (slot.texture.width(), slot.texture.height()) != texture_size {
                let (texture, image) = output(executor, ganesh, queue_family, texture_size)?;
                slot.image = image;
                let old = std::mem::replace(&mut slot.texture, texture);
                executor.delete_output_texture(old);
            }
            let new_textures = counters().textures_created - textures_before;

            let target = Scene3DVkTarget { texture: &slot.texture, size, antialiased };
            // A frame of a video is drawn with the colour of its layer here (the gap of section 3.21).
            let rendered = executor.render(&mut slot.viewport, scene, &target);
            match rendered {
                Ok(report) => {
                    slot.scene = Arc::downgrade(scene);
                    slot.generations = generations;
                    slot.size = size;
                    slot.scale_bits = scale_bits;
                    slot.antialiased = antialiased;
                    slot.inputs = inputs;
                    slot.origin = origin;
                    slot.last_used = clock;
                    slot.drawn_at_flush = Some(flushes);
                    let uniform_only = report.is_uniform_only() && new_textures == 0;
                    count(|counters| {
                        counters.output_textures_allocated += new_textures;
                        counters.scenes_drawn_on_gpu += 1;
                        if uniform_only {
                            counters.uniform_only_frames += 1;
                        }
                    });
                    Some((slot.image.clone(), size))
                }
                Err(error) => {
                    // The context draws its 3D scenes with the CPU reference from now on.
                    report_failure(&error);
                    let slots = std::mem::take(&mut entry.slots);
                    for mut slot in slots {
                        drop(slot.image);
                        slot.viewport.dispose(executor);
                        executor.delete_output_texture(slot.texture);
                    }
                    executor.dispose();
                    entry.executor = Err(error);
                    None
                }
            }
        })
    }
}

/// The Graphite route (section 3.19 of the design).
#[cfg(all(target_vendor = "apple", not(ferro_skia_apple_ganesh)))]
mod graphite {
    use super::count;
    use crate::gpu::graphite::{GraphiteGrContext, GraphitePrewarmContext};
    use crate::gpu::{ISkiaGpu, ISkiaGrContext};
    use crate::leased_gpu_bitmap::LeasedGpuBitmap;
    use crate::metal::IMetalDevice;
    use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
    use ferroui_base::platform::{IOptionalFeatureProvider, SharedBitmapImpl};
    use ferroui_base::rendering::scene3d::Scene3DSnapshot;
    use ferroui_base::PixelSize;
    use ferroui_metal::scene3d::{
        counters, Scene3DMetalDeviceTextures, Scene3DMetalError, Scene3DMetalExecutor, Scene3DMetalTarget,
        Scene3DMetalTexture, Scene3DMetalViewport,
    };
    use ferroui_render3d::brush::{scene_texture_inputs, DeviceBitmap};
    use skia_safe as sk;
    use skia_safe::gpu::graphite::mtl::backend_textures;
    use std::cell::RefCell;
    use std::rc::{Rc, Weak as RcWeak};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Weak};

    /// A kept output: the texture a scene was drawn into with its image, what it was drawn for,
    /// and the uniform buffers of its viewport.
    struct Slot {
        scene: Weak<Scene3DSnapshot>,
        generations: (u64, u64, u64),
        size: (u32, u32),
        scale_bits: u64,
        antialiased: bool,
        /// The textures of the image brushes of the scene (new pixels of a bitmap change no
        /// generation).
        inputs: u64,
        /// The device pixel of the top left of the viewport.
        origin: (i32, i32),
        /// Declared before the texture: the image wraps it.
        image: sk::Image,
        texture: Scene3DMetalTexture,
        viewport: Scene3DMetalViewport,
        last_used: u64,
        /// The flush count of the context when the image was last drawn: until a flush after it,
        /// a recording not yet submitted reads the texture.
        drawn_at_flush: Option<u64>,
    }

    impl Slot {
        /// Whether a recording not yet submitted reads the texture.
        fn is_pending(&self, flushes: u64) -> bool {
            self.drawn_at_flush.is_some_and(|drawn| drawn == flushes)
        }
    }

    /// The executor of a Graphite context and its kept outputs.
    struct ContextScenes {
        context_id: u64,
        context: RcWeak<dyn ISkiaGrContext>,
        /// The executor, or why the context draws its 3D scenes with the CPU reference.
        executor: Result<Scene3DMetalExecutor, Scene3DMetalError>,
        slots: Vec<Slot>,
        clock: u64,
    }

    ferroui_base::ferro_thread_local! {
        static CONTEXTS: RefCell<Vec<ContextScenes>> = const { RefCell::new(Vec::new()) };
    }

    /// The outputs a Graphite context keeps: the viewports of a window or two.
    const CAPACITY: usize = 8;

    static FAILURE_REPORTED: AtomicBool = AtomicBool::new(false);

    /// Reports once that a Graphite context draws its 3D scenes with the CPU reference, and why.
    fn report_failure(error: &Scene3DMetalError) {
        if FAILURE_REPORTED.swap(true, Ordering::Relaxed) {
            return;
        }
        if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::VISUAL) {
            let reason = error.reason.as_str();
            logger.log_with_values(
                None,
                "The Skia backend draws 3D scenes (Viewport3D) with the CPU reference on this Metal device: {Reason} \
                 (docs/porting/media3d.md, section 3.19).",
                &[&reason],
            );
        }
    }

    /// The release of the outputs and the executor of a context.
    pub(super) fn release(context_id: u64) {
        // A drop of the outputs needs no context: taken out of the list first.
        let released = CONTEXTS.try_with(|contexts| {
            let mut contexts = contexts.borrow_mut();
            let index = contexts.iter().position(|entry| entry.context_id == context_id);
            index.map(|index| contexts.swap_remove(index))
        });
        drop(released);
    }

    /// What drawing a scene with Graphite gave.
    pub(super) enum SceneImage {
        /// The image of the output, and the pixels of the scene at its top left.
        Drawn { image: sk::Image, size: (u32, u32) },
        /// Nothing to draw: the context is a prewarm of pipelines.
        Skipped,
        /// The context cannot draw the scene on the GPU: the caller draws the CPU reference.
        OnCpu,
    }

    /// The frames of a video or a camera that a render samples from their textures (step D7).
    /// The storage of each is kept, as for a 2D draw, until the GPU has done what the context
    /// submits next: the 3D pass is committed to the same queue before that.
    struct GraphiteFrames<'a> {
        graphite: &'a GraphiteGrContext,
        context_id: u64,
        /// The bitmaps this render has kept the storage of.
        retained: RefCell<Vec<*const ()>>,
    }

    impl GraphiteFrames<'_> {
        fn leased<'b>(&self, bitmap: &'b SharedBitmapImpl) -> Option<&'b LeasedGpuBitmap> {
            bitmap.as_any().downcast_ref::<LeasedGpuBitmap>()
        }
    }

    // SAFETY: the texture is the one the image of the frame wraps, alive while the frame is held,
    // and `sampling` keeps the storage of the frame until the GPU has done the render.
    unsafe impl Scene3DMetalDeviceTextures for GraphiteFrames<'_> {
        fn sampling(&self, bitmap: &SharedBitmapImpl) -> DeviceBitmap {
            let Some(leased) = self.leased(bitmap) else { return DeviceBitmap::NotOnDevice };
            if leased.metal_texture(self.context_id).is_none() {
                return DeviceBitmap::NotOnDevice;
            }
            let address = std::ptr::from_ref(leased).cast::<()>();
            let mut retained = self.retained.borrow_mut();
            if !retained.contains(&address) {
                if !leased.retain_for_render(self.graphite) {
                    return DeviceBitmap::NotOnDevice;
                }
                retained.push(address);
            }
            DeviceBitmap::Stored
        }

        fn device_texture(&self, bitmap: &SharedBitmapImpl) -> Option<*mut std::ffi::c_void> {
            self.leased(bitmap)?.metal_texture(self.context_id)
        }
    }

    /// The image of `scene` drawn at `size` pixels (and `scale` device pixels per unit, with or
    /// without antialiased edges) on the device of `gr_context`, its top left at the device pixel
    /// `origin`: a kept output when nothing changed, otherwise a new render.
    pub(super) fn scene_image(
        gr_context: &Rc<dyn ISkiaGrContext>,
        gpu: &Rc<dyn ISkiaGpu>,
        scene: &Arc<Scene3DSnapshot>,
        size: PixelSize,
        scale: f64,
        antialiased: bool,
        origin: (i32, i32),
    ) -> SceneImage {
        let any = gr_context.as_any();
        if any.is::<GraphitePrewarmContext>() {
            return SceneImage::Skipped;
        }
        let Some(graphite) = any.downcast_ref::<GraphiteGrContext>() else { return SceneImage::OnCpu };
        let Some(context_id) = graphite.frame_context_id() else { return SceneImage::OnCpu };
        if graphite.is_disposed() {
            return SceneImage::OnCpu;
        }
        let flushes = graphite.flush_count();
        let size = (size.width.max(1) as u32, size.height.max(1) as u32);
        let scale_bits = scale.to_bits();
        let generations = scene.generations();
        let inputs = scene_texture_inputs(scene);
        CONTEXTS.with(|contexts| {
            let mut contexts = contexts.borrow_mut();
            // The entries of contexts that are gone go with them.
            contexts.retain(|entry| entry.context.strong_count() > 0);
            let index = match contexts.iter().position(|entry| entry.context_id == context_id) {
                Some(index) => index,
                None => {
                    let features: &dyn IOptionalFeatureProvider = &**gpu;
                    let executor = match features.try_get::<dyn IMetalDevice>() {
                        // SAFETY: the device contract hands out valid `id<MTLDevice>` and
                        // `id<MTLCommandQueue>` handles, the queue the Graphite context submits
                        // to; the executor takes references of its own.
                        Some(device) => unsafe { Scene3DMetalExecutor::new(device.device(), device.command_queue()) },
                        None => Err(Scene3DMetalError { reason: "the GPU has no Metal device".to_owned() }),
                    };
                    if let Err(error) = &executor {
                        report_failure(error);
                    }
                    contexts.push(ContextScenes {
                        context_id,
                        context: Rc::downgrade(gr_context),
                        executor,
                        slots: Vec::new(),
                        clock: 0,
                    });
                    contexts.len() - 1
                }
            };
            let entry = &mut contexts[index];
            let Ok(executor) = &mut entry.executor else { return SceneImage::OnCpu };
            entry.clock += 1;
            let clock = entry.clock;

            // 1. A kept output of this scene.
            let same_scene = |slot: &Slot| std::ptr::eq(slot.scene.as_ptr(), Arc::as_ptr(scene));
            if let Some(slot) = entry.slots.iter_mut().find(|slot| {
                let slot: &Slot = slot;
                same_scene(slot)
                    && slot.generations == generations
                    && slot.size == size
                    && slot.scale_bits == scale_bits
                    && slot.antialiased == antialiased
                    && slot.inputs == inputs
            }) {
                slot.last_used = clock;
                slot.drawn_at_flush = Some(flushes);
                count(|counters| {
                    counters.cached_output_frames += 1;
                    counters.scenes_drawn_on_gpu += 1;
                });
                return SceneImage::Drawn { image: slot.image.clone(), size };
            }

            // 2. The output to draw into: the same viewport (its position and size), the output
            //    of a scene that is gone, a new one, or the least recently used; never one a
            //    recording not yet submitted reads.
            let gone = |slot: &Slot| slot.scene.strong_count() == 0;
            let same_viewport = |slot: &Slot| slot.origin == origin && slot.size == size;
            let free = |slot: &Slot| !slot.is_pending(flushes);
            let chosen = entry
                .slots
                .iter()
                .position(|slot| free(slot) && same_viewport(slot) && (gone(slot) || same_scene(slot)))
                .or_else(|| entry.slots.iter().position(|slot| free(slot) && same_viewport(slot)))
                .or_else(|| {
                    entry
                        .slots
                        .iter()
                        .enumerate()
                        .filter(|&(_, slot)| free(slot) && gone(slot))
                        .min_by_key(|(_, slot)| slot.last_used)
                        .map(|(index, _)| index)
                });
            let (texture_width, texture_height) = pooled_size(size);
            let textures_before = counters().textures_created;
            let slot_index = match chosen {
                Some(index) => index,
                None if entry.slots.len() < CAPACITY => {
                    let Some(slot) = new_slot(executor, graphite, (texture_width, texture_height), origin, clock)
                    else {
                        return SceneImage::OnCpu;
                    };
                    entry.slots.push(slot);
                    entry.slots.len() - 1
                }
                None => {
                    let least = entry
                        .slots
                        .iter()
                        .enumerate()
                        .filter(|&(_, slot)| free(slot))
                        .min_by_key(|(_, slot)| slot.last_used)
                        .map(|(index, _)| index);
                    match least {
                        Some(index) => index,
                        // Every output is read by the frame being recorded: one more for now.
                        None => {
                            let Some(slot) =
                                new_slot(executor, graphite, (texture_width, texture_height), origin, clock)
                            else {
                                return SceneImage::OnCpu;
                            };
                            entry.slots.push(slot);
                            entry.slots.len() - 1
                        }
                    }
                }
            };
            let slot = &mut entry.slots[slot_index];
            // A texture of another size: another one.
            if slot.texture.width() != texture_width || slot.texture.height() != texture_height {
                let Some((texture, image)) = output(executor, graphite, (texture_width, texture_height)) else {
                    return SceneImage::OnCpu;
                };
                slot.image = image;
                slot.texture = texture;
            }
            let new_textures = counters().textures_created - textures_before;

            let target = Scene3DMetalTarget { texture: &slot.texture, size, antialiased };
            let frames = GraphiteFrames { graphite, context_id, retained: RefCell::new(Vec::new()) };
            let rendered = executor.render_with_textures(&mut slot.viewport, scene, &target, &frames);
            match rendered {
                Ok(report) => {
                    slot.scene = Arc::downgrade(scene);
                    slot.generations = generations;
                    slot.size = size;
                    slot.scale_bits = scale_bits;
                    slot.antialiased = antialiased;
                    slot.inputs = inputs;
                    slot.origin = origin;
                    slot.last_used = clock;
                    slot.drawn_at_flush = Some(flushes);
                    let uniform_only = report.is_uniform_only() && new_textures == 0;
                    count(|counters| {
                        counters.output_textures_allocated += new_textures;
                        counters.scenes_drawn_on_gpu += 1;
                        if uniform_only {
                            counters.uniform_only_frames += 1;
                        }
                    });
                    SceneImage::Drawn { image: slot.image.clone(), size }
                }
                Err(error) => {
                    // The context draws its 3D scenes with the CPU reference from now on.
                    report_failure(&error);
                    entry.slots.clear();
                    entry.executor = Err(error);
                    SceneImage::OnCpu
                }
            }
        })
    }

    /// The size of an output for a scene of `size` pixels: rounded up to 32 pixels, so that a
    /// viewport that is resized by a few pixels keeps its texture.
    fn pooled_size(size: (u32, u32)) -> (u32, u32) {
        (size.0.div_ceil(32) * 32, size.1.div_ceil(32) * 32)
    }

    /// A texture of `size` of the device, wrapped once as an image of the recorder of `graphite`.
    fn output(
        executor: &Scene3DMetalExecutor,
        graphite: &GraphiteGrContext,
        size: (u32, u32),
    ) -> Option<(Scene3DMetalTexture, sk::Image)> {
        let texture = executor.create_output_texture(size.0, size.1)?;
        // SAFETY: `texture` holds a reference to the `id<MTLTexture>` for the duration of the
        // call; the image Graphite wraps it in takes a reference of its own.
        let backend = unsafe { backend_textures::make_metal((size.0 as i32, size.1 as i32), texture.as_ptr()) };
        let image = graphite.with_recorder(|recorder| {
            sk::gpu::graphite::images::wrap_texture(
                recorder,
                &backend,
                sk::ColorType::RGBA8888,
                sk::AlphaType::Premul,
                None,
            )
        })?;
        Some((texture, image))
    }

    fn new_slot(
        executor: &Scene3DMetalExecutor,
        graphite: &GraphiteGrContext,
        size: (u32, u32),
        origin: (i32, i32),
        clock: u64,
    ) -> Option<Slot> {
        let (texture, image) = output(executor, graphite, size)?;
        Some(Slot {
            scene: Weak::new(),
            generations: (0, 0, 0),
            size,
            scale_bits: 0,
            antialiased: false,
            inputs: 0,
            origin,
            image,
            texture,
            viewport: Scene3DMetalViewport::new(),
            last_used: clock,
            drawn_at_flush: None,
        })
    }
}
