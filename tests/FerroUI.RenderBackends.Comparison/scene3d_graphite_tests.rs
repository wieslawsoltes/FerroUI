//! The 3D scenes drawn by the Skia backend on Graphite (Metal), with the Metal executor of step D5
//! (`docs/porting/media3d.md`, section 3.19): held to the CPU reference as the GPU modes of Vello
//! are (the faces within [`GPU_INTERIOR_LEVELS`], the edges of 4x MSAA within the mean of
//! `Tolerance::Moves`), with their golden properties, and with the steady state of the executor.
//! The render target is an offscreen layer of a Graphite context on the system's Metal device; a
//! frame ends with a flush of the context (the read back). A machine without a Metal device
//! checks nothing.

use super::{draw_with_animated_2d, reference, DeviceDifference, GPU_INTERIOR_LEVELS};
use crate::custom_effect_scenes::{MOVED_MEAN, SIDE};
use crate::scene3d_scenes::{
    draw_plain, lit_cube_scene, moving_brush_group, orbiting_cube, scenes, versioned_image_scene,
};
use crate::Pixels;
use ferroui_base::platform::{IDrawingContextImpl, IDrawingContextLayerImpl, IPlatformRenderInterfaceContext};
use ferroui_base::{Matrix, PixelSize, Vector};
use ferroui_metal::offscreen::OffscreenMetalDevice;
use ferroui_render3d::cpu::scenes_rasterized;
use ferroui_skia::gpu::metal::SkiaMetalGpu;
use ferroui_skia::{scene3d_counters, ImmutableBitmap, SkiaContext, SkiaScene3DCounters, SurfaceRenderTarget};
use std::rc::Rc;

/// A Graphite context on the system's Metal device.
struct Graphite {
    context: SkiaContext,
}

impl Graphite {
    fn try_new() -> Option<Graphite> {
        let device = OffscreenMetalDevice::try_new()?;
        let gpu = SkiaMetalGpu::new(device, None, None);
        Some(Graphite { context: SkiaContext::new(Some(gpu)) })
    }

    /// Draws `draw` at `scale` (the drawing scaled to it, as a render target bitmap of that DPI
    /// scales it) onto a new layer of [`SIDE`] logical pixels, and reads it back.
    fn render(&self, scale: i32, draw: &dyn Fn(&mut dyn IDrawingContextImpl)) -> Pixels {
        let side = SIDE * scale;
        let layer: Rc<dyn IDrawingContextLayerImpl> = self.context.create_offscreen_render_target(
            PixelSize::new(side, side),
            Vector::new(f64::from(scale), f64::from(scale)),
            true,
        );
        let mut context = layer.create_drawing_context();
        context.set_transform(Matrix::create_scale(f64::from(scale), f64::from(scale)));
        draw(&mut *context);
        context.dispose();
        let target = layer.as_any().downcast_ref::<SurfaceRenderTarget>().expect("a surface render target");
        let snapshot = target.create_non_affined_snapshot();
        let bitmap = snapshot.as_any().downcast_ref::<ImmutableBitmap>().expect("a raster snapshot");
        // The snapshot is a raster image of the pixels the context read back from the GPU (no
        // bitmap to lock): its pixels are read as premultiplied RGBA.
        let image = bitmap.image();
        let (width, height) = (image.width() as usize, image.height() as usize);
        let info = skia_safe::ImageInfo::new(
            (width as i32, height as i32),
            skia_safe::ColorType::RGBA8888,
            skia_safe::AlphaType::Premul,
            None,
        );
        let mut rgba = vec![0u8; width * height * 4];
        let read = image.read_pixels(&info, &mut rgba, width * 4, (0, 0), skia_safe::image::CachingHint::Allow);
        assert!(read, "the pixels of the Graphite snapshot are read");
        layer.dispose();
        Pixels { width, height, rgba }
    }
}

impl Drop for Graphite {
    fn drop(&mut self) {
        self.context.dispose();
    }
}

/// The counters of Graphite since `before`.
fn since(before: SkiaScene3DCounters) -> SkiaScene3DCounters {
    let now = scene3d_counters();
    let mut counters = now;
    counters.pipelines_created = now.pipelines_created - before.pipelines_created;
    counters.libraries_compiled = now.libraries_compiled - before.libraries_compiled;
    counters.mesh_uploads = now.mesh_uploads - before.mesh_uploads;
    counters.brush_textures_uploaded = now.brush_textures_uploaded - before.brush_textures_uploaded;
    counters.uniform_only_frames = now.uniform_only_frames - before.uniform_only_frames;
    counters.cached_output_frames = now.cached_output_frames - before.cached_output_frames;
    counters.passes = now.passes - before.passes;
    counters.textures_allocated = now.textures_allocated - before.textures_allocated;
    counters.buffers_created = now.buffers_created - before.buffers_created;
    counters.uniform_bytes = now.uniform_bytes - before.uniform_bytes;
    counters.scenes_drawn_on_gpu = now.scenes_drawn_on_gpu - before.scenes_drawn_on_gpu;
    counters
}

/// Every plain 3D scene drawn by Skia on Graphite at 96 and 192 DPI is its CPU reference within
/// the bounds of the GPU modes of Vello, with nothing drawn by the CPU reference.
///
/// Run with `--nocapture` to see the table.
#[test]
fn graphite_draws_every_scene_3d_as_its_cpu_reference() {
    let Some(graphite) = Graphite::try_new() else { return };
    let mut failures = Vec::new();
    println!("| Scene | Scale | Largest difference inside the faces | Mean difference | Outliers on edges |");
    println!("|---|---|---|---|---|");
    for scale in [1, 2] {
        for case in scenes().into_iter().filter(|case| case.plain) {
            let scene = (case.scene)();
            let expected = reference(&scene, scale);
            let before = scene3d_counters();
            let rasterized = scenes_rasterized();
            let actual = graphite.render(scale, &|context| (case.draw)(context, &scene)).rgba;
            let drawn = since(before);
            let difference = DeviceDifference::of(&expected, &actual, (SIDE * scale) as usize);
            println!(
                "| `{}` | {scale}x | {} | {:.3} | {:.3} % |",
                case.name,
                difference.interior_largest,
                difference.mean,
                100.0 * difference.edge_outliers
            );
            if !difference.is_within(case.layers) {
                failures.push(format!(
                    "{} at {scale}x: {difference:?} (interior {GPU_INTERIOR_LEVELS}, mean {MOVED_MEAN})",
                    case.name
                ));
            }
            if case.name != "empty" && drawn.scenes_drawn_on_gpu != 1 {
                failures.push(format!("{} at {scale}x: not drawn on the GPU: {drawn:?}", case.name));
            }
            if rasterized != scenes_rasterized() {
                failures.push(format!("{} at {scale}x: drawn by the CPU reference", case.name));
            }
        }
    }
    assert!(failures.is_empty(), "3D scenes beyond their tolerances on Graphite:\n{}", failures.join("\n"));
}

/// The golden properties of every scene on Graphite at 96 and 192 DPI (the probes are inside the
/// faces, so the executor is held to the levels of the CPU reference).
#[test]
fn every_scene_3d_has_its_golden_properties_on_graphite() {
    let Some(graphite) = Graphite::try_new() else { return };
    let mut failures = Vec::new();
    for scale in [1, 2] {
        let side = (SIDE * scale) as usize;
        for case in scenes() {
            let scene = (case.scene)();
            let drawn = graphite.render(scale, &|context| (case.draw)(context, &scene));
            for probe in (case.probes)(side) {
                let actual = probe.read(&drawn);
                if !probe.holds(actual) {
                    failures.push(format!(
                        "{} at {scale}x, {} (pixel {}, {}): {actual:?}, expected {:?} within {}",
                        case.name, probe.what, probe.x, probe.y, probe.expected, probe.levels
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "3D scenes without their properties on Graphite:\n{}", failures.join("\n"));
}

/// The frames of a viewport whose camera moves, once the uniform slots of its viewport hold its
/// draws: each a 3D pass that writes the frame uniforms and nothing else (no mesh upload, no
/// pipeline, no texture, no buffer), at 96 and 192 DPI, the same for a viewport of brushes.
#[test]
fn graphite_steady_frames_of_a_moving_camera_upload_only_uniforms_and_allocate_no_texture() {
    const FRAMES: u32 = 8;
    let Some(graphite) = Graphite::try_new() else { return };
    let warm_up = ferroui_metal::scene3d::UNIFORM_SLOTS as u32 + 1;
    let frame_bytes = std::mem::size_of::<ferroui_render3d::uniforms::FrameUniforms>() as u64;
    let moving: [(&str, fn(u32) -> std::sync::Arc<ferroui_base::rendering::scene3d::Scene3DSnapshot>); 2] =
        [("orbiting cube", orbiting_cube), ("brush group", moving_brush_group)];
    for (name, scene_of) in moving {
        for scale in [1, 2] {
            for step in 0..warm_up {
                let scene = scene_of(step);
                graphite.render(scale, &|context| draw_plain(context, &scene));
            }
            let rasterized = scenes_rasterized();
            let before = scene3d_counters();
            for step in warm_up..warm_up + FRAMES {
                let scene = scene_of(step);
                graphite.render(scale, &|context| draw_plain(context, &scene));
            }
            let steady = since(before);
            let what = format!("{name} at {scale}x: {steady:?}");
            assert_eq!(u64::from(FRAMES), steady.passes, "{what}: one 3D pass a frame");
            assert_eq!(u64::from(FRAMES), steady.uniform_only_frames, "{what}: only uniforms");
            assert_eq!(0, steady.pipelines_created, "{what}: no pipeline");
            assert_eq!(0, steady.mesh_uploads, "{what}: no mesh upload");
            assert_eq!(0, steady.brush_textures_uploaded, "{what}: no brush texture upload");
            assert_eq!(0, steady.textures_allocated, "{what}: no texture");
            assert_eq!(0, steady.buffers_created, "{what}: no buffer");
            assert_eq!(u64::from(FRAMES) * frame_bytes, steady.uniform_bytes, "{what}: the frame uniforms alone");
            assert_eq!(rasterized, scenes_rasterized(), "{what}: nothing drawn by the CPU reference");
        }
    }
}

/// A viewport whose scene does not change, drawn in frames whose 2D content animates, issues no
/// 3D pass after its first frame: the kept output is drawn again.
#[test]
fn a_static_viewport_issues_no_3d_pass_on_graphite() {
    const FRAMES: u8 = 6;
    let Some(graphite) = Graphite::try_new() else { return };
    let scene = lit_cube_scene();
    let first = graphite.render(1, &|context| draw_with_animated_2d(context, &scene, 0));
    let before = scene3d_counters();
    let mut last = None;
    for frame in 1..=FRAMES {
        last = Some(graphite.render(1, &|context| draw_with_animated_2d(context, &scene, frame)));
    }
    let last = last.expect("the frames after the first");
    let steady = since(before);
    let what = format!("{steady:?}");
    assert_eq!(0, steady.passes, "{what}: no 3D pass");
    assert_eq!(u64::from(FRAMES), steady.cached_output_frames, "{what}: the kept output each frame");
    assert_eq!(0, steady.uniform_bytes, "{what}: no uniform written");
    let side = SIDE as usize;
    let differing = (0..side * side)
        .filter(|&pixel| {
            let (x, y) = (pixel % side, pixel / side);
            (x >= 12 || y >= 12) && first.rgba[pixel * 4..pixel * 4 + 4] != last.rgba[pixel * 4..pixel * 4 + 4]
        })
        .count();
    assert_eq!(0, differing, "{what}: the kept output draws the same scene");
}

/// Every scene drawn on Graphite at 96 and 192 DPI, twice: the first round compiles the library
/// once and makes the pipelines (at most one per key: two blends, two sample counts, the four
/// variants of the brush shader, and the resolve; culling and the depth test are state of the
/// encoder), the second makes none.
#[test]
fn pipelines_are_created_once_per_graphite_context() {
    const KEYS: u64 = 2 * 2 * 4 + 1;
    let Some(graphite) = Graphite::try_new() else { return };
    let draw_all = || {
        for scale in [1, 2] {
            for case in scenes() {
                let scene = (case.scene)();
                graphite.render(scale, &|context| (case.draw)(context, &scene));
            }
        }
    };
    let before = scene3d_counters();
    draw_all();
    let first = since(before);
    assert_eq!(1, first.libraries_compiled, "one library for the context: {first:?}");
    assert!(first.pipelines_created > 0, "the first round makes the pipelines: {first:?}");
    assert!(first.pipelines_created <= KEYS, "one pipeline per key: {first:?}");
    let before = scene3d_counters();
    draw_all();
    let again = since(before);
    assert_eq!(0, again.pipelines_created, "the context has its pipelines: {again:?}");
    assert_eq!(0, again.libraries_compiled, "{again:?}");
    assert!(again.scenes_drawn_on_gpu > 0, "{again:?}");
}

/// New pixels of the bitmap of an image brush: one 3D pass uploads them into the texture of the
/// bitmap, with no texture, mesh, pipeline or buffer made, and draws them.
#[test]
fn a_new_version_of_a_bitmap_uploads_only_its_texture_on_graphite() {
    let Some(graphite) = Graphite::try_new() else { return };
    let red = [255, 0, 0, 255];
    let blue = [0, 0, 255, 255];
    let (bitmap, scene) = versioned_image_scene(red);
    let centre = |pixels: &Pixels| {
        let index = (50 * pixels.width + 50) * 4;
        [pixels.rgba[index], pixels.rgba[index + 1], pixels.rgba[index + 2], pixels.rgba[index + 3]]
    };
    let first = graphite.render(1, &|context| draw_plain(context, &scene));
    assert_eq!(red, centre(&first));
    graphite.render(1, &|context| draw_plain(context, &scene));

    let before = scene3d_counters();
    bitmap.set_pixels(blue);
    let changed = graphite.render(1, &|context| draw_plain(context, &scene));
    assert_eq!(blue, centre(&changed), "the new pixels are drawn");
    let after = since(before);
    let what = format!("{after:?}");
    assert_eq!(1, after.passes, "{what}: drawn again");
    assert_eq!(0, after.cached_output_frames, "{what}: not the kept output");
    assert_eq!(1, after.brush_textures_uploaded, "{what}: the new pixels uploaded");
    assert_eq!(0, after.textures_allocated, "{what}: into the texture of the bitmap");
    assert_eq!(0, after.mesh_uploads, "{what}: no mesh");
    assert_eq!(0, after.pipelines_created, "{what}: no pipeline");
    assert_eq!(0, after.buffers_created, "{what}: no buffer");
}
