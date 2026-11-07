use crate::custom_effect_scenes::{reference_difference, render_at, Tolerance, MOVED_MEAN, SIDE};
use crate::scene3d_scenes::{draw_plain, lit_cube_scene, orbiting_cube, scenes, unshifted_matrix_camera_scene, Scene3DCase};
use crate::{compare, Backend, TOLERANCE};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{BoxShadows, Color};
use ferroui_base::platform::IDrawingContextImpl;
use ferroui_base::rendering::scene3d::Scene3DSnapshot;
use ferroui_base::{PixelSize, Rect, RoundedRect};
use ferroui_render3d::cpu::{render, scenes_rasterized};
use ferroui_render3d::CpuRenderOptions;
use ferroui_vello::gpu::{scene3d_counters, VelloScene3DCounters};
use ferroui_vello::VelloRenderingMode;
use std::sync::Arc;

/// The backends under test: Skia raster and every mode of the Vello backend that is built (the
/// hybrid and the GPU mode on the device of the machine, which draw 3D scenes with the wgpu
/// executor from step D4).
fn backends() -> Vec<Backend> {
    [Backend::skia()].into_iter().chain(Backend::vello_modes().into_iter().map(Backend::vello)).collect()
}

/// The modes of the Vello backend that draw 3D scenes on the device: the hybrid and the GPU mode,
/// where the machine has an adapter for them.
fn device_modes() -> Vec<VelloRenderingMode> {
    Backend::vello_modes().into_iter().filter(|mode| *mode != VelloRenderingMode::Cpu).collect()
}

/// Whether `backend` draws 3D scenes with a GPU executor (and so with 4x MSAA rather than the
/// ordered grid of the CPU reference).
fn draws_on_device(backend: &Backend) -> bool {
    backend.name == "Vello hybrid" || backend.name == "Vello GPU"
}

/// The CPU reference of a scene at `scale`: the image of [`SIDE`] by [`SIDE`] logical pixels.
fn reference(scene: &Scene3DSnapshot, scale: i32) -> Vec<u8> {
    let side = SIDE * scale;
    render(scene, PixelSize::new(side, side), f64::from(scale), CpuRenderOptions::default()).rgba
}

/// The levels by which a channel of a neighbour of a pixel of the CPU reference differs from it
/// beyond which the pixel is on an edge: a silhouette, the border of two faces, a clipped part.
/// The smooth falloff of a light changes by a level or two from one pixel to the next.
const EDGE_LEVELS: u8 = 8;

/// How far a GPU executor may be from the CPU reference in the interior of the faces, in levels:
/// the same lighting in `f32` on both sides, each sample stored as 8 bits before the resolve.
const GPU_INTERIOR_LEVELS: u8 = 3;

/// The share of the pixels (in percent) by which a GPU executor and Skia raster (the CPU
/// reference composed) may differ by more than the tolerance of the harness: the edges, where 4x
/// MSAA (rotated-grid samples) and the ordered grid of the CPU reference cover a pixel with a
/// sample more or less.
const GPU_EDGE_SHARE: f64 = 3.0;

/// Whether the pixel at `x`, `y` of the reference `rgba` (`side` pixels square) is on an edge:
/// a channel of a pixel of its 3x3 neighbourhood differs from it by more than [`EDGE_LEVELS`].
fn on_edge(rgba: &[u8], side: usize, x: usize, y: usize) -> bool {
    let pixel = |x: usize, y: usize| &rgba[(y * side + x) * 4..(y * side + x) * 4 + 4];
    let centre = pixel(x, y);
    for ny in y.saturating_sub(1)..=(y + 1).min(side - 1) {
        for nx in x.saturating_sub(1)..=(x + 1).min(side - 1) {
            if pixel(nx, ny).iter().zip(centre).any(|(a, b)| a.abs_diff(*b) > EDGE_LEVELS) {
                return true;
            }
        }
    }
    false
}

/// How a GPU executor's image differs from the CPU reference: inside the faces and overall.
#[derive(Clone, Copy, Debug)]
struct DeviceDifference {
    /// The largest difference of a channel of a pixel that is not on an edge.
    interior_largest: u8,
    /// The mean difference of the channels of all pixels.
    mean: f64,
    /// The share of the pixels (0 to 1) beyond [`GPU_INTERIOR_LEVELS`], all on edges.
    edge_outliers: f64,
}

impl DeviceDifference {
    fn of(expected: &[u8], actual: &[u8], side: usize) -> DeviceDifference {
        assert_eq!(expected.len(), actual.len(), "the renderings have the same size");
        let (mut interior_largest, mut outliers, mut total) = (0u8, 0usize, 0u64);
        for y in 0..side {
            for x in 0..side {
                let index = (y * side + x) * 4;
                let largest = (0..4).map(|i| expected[index + i].abs_diff(actual[index + i])).max().unwrap_or(0);
                total += (0..4).map(|i| u64::from(expected[index + i].abs_diff(actual[index + i]))).sum::<u64>();
                if on_edge(expected, side, x, y) {
                    if largest > GPU_INTERIOR_LEVELS {
                        outliers += 1;
                    }
                } else {
                    interior_largest = interior_largest.max(largest);
                }
            }
        }
        DeviceDifference {
            interior_largest,
            mean: total as f64 / expected.len().max(1) as f64,
            edge_outliers: outliers as f64 / (side * side).max(1) as f64,
        }
    }

    /// The faces within [`GPU_INTERIOR_LEVELS`], and the mean within that of
    /// [`Tolerance::Moves`]: the edges may differ, the image may not.
    fn is_within(&self) -> bool {
        self.interior_largest <= GPU_INTERIOR_LEVELS && self.mean <= MOVED_MEAN
    }
}

/// Every plain 3D scene, drawn by every backend and mode at 96 and 192 DPI, is its CPU reference.
/// Skia raster and the CPU mode of Vello draw the image of the reference one to one
/// (`Tolerance::Largest(2)`); the hybrid and the GPU mode draw the scene with the wgpu executor,
/// held to the reference within [`GPU_INTERIOR_LEVELS`] inside the faces and the mean of
/// `Tolerance::Moves` overall (their edges are 4x MSAA, the reference's an ordered grid).
///
/// Run with `--nocapture` to see the table.
#[test]
fn every_scene_3d_matches_its_cpu_reference_in_every_backend_and_mode() {
    let mut failures = Vec::new();
    println!("| Scene | Backend | Scale | Largest difference | Mean difference | Outliers | Tolerance |");
    println!("|---|---|---|---|---|---|---|");
    for backend in backends() {
        for scale in [1, 2] {
            for case in scenes().into_iter().filter(|case| case.plain) {
                let scene = (case.scene)();
                let expected = reference(&scene, scale);
                let actual = render_at(&backend, scale, &|context| (case.draw)(context, &scene)).rgba;
                if draws_on_device(&backend) {
                    let difference = DeviceDifference::of(&expected, &actual, (SIDE * scale) as usize);
                    println!(
                        "| `{}` | {} | {scale}x | {} inside the faces | {:.3} | {:.3} % on edges | GPU (interior {GPU_INTERIOR_LEVELS}, mean {MOVED_MEAN}) |",
                        case.name,
                        backend.name,
                        difference.interior_largest,
                        difference.mean,
                        100.0 * difference.edge_outliers,
                    );
                    if !difference.is_within() {
                        failures.push(format!("{} on {} at {scale}x: {difference:?}", case.name, backend.name));
                    }
                    continue;
                }
                let difference = reference_difference(&expected, &actual, case.tolerance.largest());
                println!(
                    "| `{}` | {} | {scale}x | {} | {:.3} | {:.3} % | {:?} |",
                    case.name,
                    backend.name,
                    difference.largest,
                    difference.mean,
                    100.0 * difference.outliers,
                    case.tolerance
                );
                if !difference.is_within(case.tolerance) {
                    failures.push(format!("{} on {} at {scale}x: {difference:?}, beyond {:?}", case.name, backend.name, case.tolerance));
                }
            }
        }
    }
    assert!(failures.is_empty(), "3D scenes beyond their tolerances:\n{}", failures.join("\n"));
}

/// The golden properties of every scene in every backend and mode at 96 and 192 DPI: the probed
/// pixels have the colours of WPF's lighting at the probed points (the Lambert intensity of a
/// face, the attenuation and range of a point light, the cone of a spot light, the layers of a
/// material group), the background is transparent, back faces without a `BackMaterial` are culled,
/// what is before the near or beyond the far plane is clipped, transparency follows the tree
/// order, and a rotated, clipped viewport at half opacity is drawn rotated, clipped and at half
/// opacity. The probes are inside the faces, so the GPU executors are held to the same levels as
/// the CPU reference.
#[test]
fn every_scene_3d_has_its_golden_properties_in_every_backend_and_mode() {
    let mut failures = Vec::new();
    for backend in backends() {
        for scale in [1, 2] {
            let side = (SIDE * scale) as usize;
            for case in scenes() {
                let scene = (case.scene)();
                let drawn = render_at(&backend, scale, &|context| (case.draw)(context, &scene));
                for probe in (case.probes)(side) {
                    let actual = probe.read(&drawn);
                    if !probe.holds(actual) {
                        failures.push(format!(
                            "{} on {} at {scale}x, {} (pixel {}, {}): {actual:?}, expected {:?} within {}",
                            case.name, backend.name, probe.what, probe.x, probe.y, probe.expected, probe.levels
                        ));
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "3D scenes without their properties:\n{}", failures.join("\n"));
}

/// The bounds of the comparison of a case between Skia raster and a mode of Vello: those of the
/// case for a mode that composes the image of the CPU reference, and the edges of 4x MSAA
/// ([`GPU_EDGE_SHARE`], the mean of `Tolerance::Moves`) for a mode that draws on the device.
fn alike_bounds(case: &Scene3DCase, on_device: bool) -> (f64, f64) {
    if on_device {
        (case.bound.max(GPU_EDGE_SHARE), 2.0 * Tolerance::Moves.mean() + 0.05)
    } else {
        (case.bound, 2.0 * case.tolerance.mean() + 0.05)
    }
}

/// The 3D scenes drawn by Skia raster and by each mode of the Vello backend. The CPU mode
/// composes the same image of the CPU reference, so a plain scene agrees to the last digit but
/// for rounding; the rotated scene differs at the antialiased edges of the rotated image and of
/// its clip. The hybrid and the GPU mode draw the scene on the device: they differ at the edges of
/// the models, by a sample of 4x MSAA.
///
/// Run with `--nocapture` to see the table.
#[test]
fn the_backends_draw_every_scene_3d_alike() {
    let skia = Backend::skia();
    let mut failures = Vec::new();
    for mode in Backend::vello_modes() {
        let vello = Backend::vello(mode);
        let on_device = draws_on_device(&vello);
        println!(
            "| Scene | Pixels beyond the tolerance of {TOLERANCE} ({} against {}) | Largest difference | Mean difference | Bound of the share | Bound of the mean |",
            vello.name, skia.name
        );
        println!("|---|---|---|---|---|---|");
        for scale in [1, 2] {
            for case in scenes() {
                let scene = (case.scene)();
                let skia_drawn = render_at(&skia, scale, &|context| (case.draw)(context, &scene));
                let vello_drawn = render_at(&vello, scale, &|context| (case.draw)(context, &scene));
                let difference = compare(&skia_drawn, &vello_drawn);
                let (share_bound, mean_bound) = alike_bounds(&case, on_device);
                println!(
                    "| `{}` {scale}x | {:.3} % | {} | {:.3} | {:.2} % | {:.2} |",
                    case.name, difference.share, difference.largest, difference.mean, share_bound, mean_bound
                );
                if difference.share > share_bound {
                    failures.push(format!(
                        "{} at {scale}x: {} differs from {} in {:.3} % of the pixels, more than its bound of {:.2} %",
                        case.name, vello.name, skia.name, difference.share, share_bound
                    ));
                }
                if difference.mean > mean_bound {
                    failures.push(format!(
                        "{} at {scale}x: {} differs from {} by {:.3} of 255 in the mean, more than its bound of {:.2}",
                        case.name, vello.name, skia.name, difference.mean, mean_bound
                    ));
                }
            }
        }
        println!();
    }
    assert!(failures.is_empty(), "3D scenes drawn differently:\n{}", failures.join("\n"));
}

/// A `MatrixCamera` with the matrices of a perspective camera draws what that camera draws.
#[test]
fn a_matrix_camera_draws_as_the_camera_of_its_matrices() {
    let perspective = (scenes()[0].scene)();
    let matrix = unshifted_matrix_camera_scene();
    for scale in [1, 2] {
        assert_eq!(reference(&perspective, scale), reference(&matrix, scale), "at {scale}x");
    }
}

/// An empty scene, and a scene without a camera, draw nothing at all in every backend and mode.
#[test]
fn an_empty_scene_draws_nothing_in_every_backend_and_mode() {
    let empty = scenes().into_iter().find(|case| case.name == "empty").expect("the empty scene");
    let mut without_camera = (*(scenes()[0].scene)()).clone();
    without_camera.camera = None;
    let without_camera = Arc::new(without_camera);
    for backend in backends() {
        for scene in [(empty.scene)(), without_camera.clone()] {
            let drawn = render_at(&backend, 1, &|context| draw_plain(context, &scene));
            assert!(drawn.rgba.iter().all(|&byte| byte == 0), "{} drew something", backend.name);
        }
    }
}

/// A viewport whose snapshot is unchanged is not rasterized again by the backends that draw the
/// CPU reference (Skia raster, the CPU mode of Vello): the backend draws the image it kept (the
/// generations, the size and the scale are those it was made for); a new size or a new snapshot
/// is rasterized. The modes that draw on the device do not rasterize at all.
#[test]
fn an_unchanged_viewport_is_not_rasterized_again() {
    for backend in backends() {
        let scene = (scenes()[0].scene)();
        let before = scenes_rasterized();
        let first = render_at(&backend, 1, &|context| draw_plain(context, &scene));
        let second = render_at(&backend, 1, &|context| draw_plain(context, &scene));
        if let Err(difference) = crate::drawn_the_same_way_twice(&backend, &first, &second) {
            panic!("{} does not draw a kept scene the same way twice: {difference}", backend.name);
        }
        render_at(&backend, 2, &|context| draw_plain(context, &scene));
        let changed = (scenes()[0].scene)();
        render_at(&backend, 2, &|context| draw_plain(context, &changed));
        if draws_on_device(&backend) {
            assert_eq!(before, scenes_rasterized(), "{} draws on the device", backend.name);
        } else {
            assert_eq!(before + 3, scenes_rasterized(), "{}: drawn four times, rasterized three times", backend.name);
        }
    }
}

/// The counters of the GPU modes since `before`.
fn since(before: VelloScene3DCounters) -> VelloScene3DCounters {
    let now = scene3d_counters();
    VelloScene3DCounters {
        pipelines_created: now.pipelines_created - before.pipelines_created,
        mesh_uploads: now.mesh_uploads - before.mesh_uploads,
        uniform_only_frames: now.uniform_only_frames - before.uniform_only_frames,
        cached_output_frames: now.cached_output_frames - before.cached_output_frames,
        passes: now.passes - before.passes,
        textures_allocated: now.textures_allocated - before.textures_allocated,
        buffers_created: now.buffers_created - before.buffers_created,
        uniform_bytes: now.uniform_bytes - before.uniform_bytes,
        scenes_drawn_on_gpu: now.scenes_drawn_on_gpu - before.scenes_drawn_on_gpu,
    }
}

/// The frames of a viewport whose camera moves, after the first two: each a 3D pass that writes
/// the frame uniforms and nothing else (no mesh upload, no pipeline, no texture, no buffer), in
/// the hybrid and the GPU mode, at 96 and 192 DPI.
#[test]
fn steady_frames_of_a_moving_camera_upload_only_uniforms_and_allocate_no_texture() {
    const FRAMES: u32 = 8;
    let frame_bytes = std::mem::size_of::<ferroui_render3d::uniforms::FrameUniforms>() as u64;
    for mode in device_modes() {
        let backend = Backend::vello(mode);
        for scale in [1, 2] {
            // The first frames make the pipelines, the mesh buffers, the uniform buffers and the
            // targets.
            for step in 0..2 {
                let scene = orbiting_cube(step);
                render_at(&backend, scale, &|context| draw_plain(context, &scene));
            }
            let rasterized = scenes_rasterized();
            let before = scene3d_counters();
            for step in 2..2 + FRAMES {
                let scene = orbiting_cube(step);
                render_at(&backend, scale, &|context| draw_plain(context, &scene));
            }
            let steady = since(before);
            let what = format!("{} at {scale}x: {steady:?}", backend.name);
            assert_eq!(u64::from(FRAMES), steady.passes, "{what}: one 3D pass a frame");
            assert_eq!(u64::from(FRAMES), steady.uniform_only_frames, "{what}: only uniforms");
            assert_eq!(0, steady.pipelines_created, "{what}: no pipeline");
            assert_eq!(0, steady.mesh_uploads, "{what}: no mesh upload");
            assert_eq!(0, steady.textures_allocated, "{what}: no texture");
            assert_eq!(0, steady.buffers_created, "{what}: no buffer");
            assert_eq!(u64::from(FRAMES) * frame_bytes, steady.uniform_bytes, "{what}: the frame uniforms alone");
            assert_eq!(rasterized, scenes_rasterized(), "{what}: nothing drawn by the CPU reference");
        }
    }
}

/// Draws a rectangle of a colour that changes with `frame` beside the viewport, then the
/// viewport: 2D content that animates over a 3D scene that does not.
fn draw_with_animated_2d(context: &mut dyn IDrawingContextImpl, scene: &Arc<Scene3DSnapshot>, frame: u8) {
    let brush = ImmutableSolidColorBrush::new(Color::from_argb(255, frame.wrapping_mul(40), 64, 128));
    let corner = RoundedRect::from_rect(Rect::new(0.0, 0.0, 12.0, 12.0));
    context.draw_rectangle(Some(&brush), None, corner, &BoxShadows::default());
    draw_plain(context, scene);
}

/// A viewport whose scene does not change, drawn in frames whose 2D content animates, issues no
/// 3D pass after its first frame: the kept output is painted again, in the hybrid and the GPU mode.
#[test]
fn a_static_viewport_issues_no_3d_pass() {
    const FRAMES: u8 = 6;
    for mode in device_modes() {
        let backend = Backend::vello(mode);
        let scene = lit_cube_scene();
        let first = render_at(&backend, 1, &|context| draw_with_animated_2d(context, &scene, 0));
        let before = scene3d_counters();
        let mut last = None;
        for frame in 1..=FRAMES {
            last = Some(render_at(&backend, 1, &|context| draw_with_animated_2d(context, &scene, frame)));
        }
        let last = last.expect("the frames after the first");
        let steady = since(before);
        let what = format!("{}: {steady:?}", backend.name);
        assert_eq!(0, steady.passes, "{what}: no 3D pass");
        assert_eq!(u64::from(FRAMES), steady.cached_output_frames, "{what}: the kept output each frame");
        assert_eq!(0, steady.uniform_bytes, "{what}: no uniform written");
        // The scene is the same in the last frame as in the first (the rectangle aside).
        let side = SIDE as usize;
        let differing = (0..side * side)
            .filter(|&pixel| {
                let (x, y) = (pixel % side, pixel / side);
                (x >= 12 || y >= 12) && first.rgba[pixel * 4..pixel * 4 + 4] != last.rgba[pixel * 4..pixel * 4 + 4]
            })
            .count();
        assert_eq!(0, differing, "{what}: the kept output draws the same scene");
    }
}

/// Every scene drawn in the hybrid and the GPU mode at 96 and 192 DPI, twice: the first round
/// makes the pipelines of the device (at most one per key: two blends, two sides, depth written or
/// tested, one sample count, and the resolve), the second makes none, in the same mode or the
/// other one, which draws on the same device.
#[test]
fn pipelines_are_created_once_per_device() {
    const KEYS: u64 = 2 * 2 * 2 + 1;
    let modes = device_modes();
    let Some(first_mode) = modes.first().copied() else { return };
    let draw_all = |mode: VelloRenderingMode| {
        let backend = Backend::vello(mode);
        for scale in [1, 2] {
            for case in scenes() {
                let scene = (case.scene)();
                render_at(&backend, scale, &|context| (case.draw)(context, &scene));
            }
        }
    };
    let before = scene3d_counters();
    draw_all(first_mode);
    let first = since(before);
    assert!(first.pipelines_created > 0, "the first round makes the pipelines: {first:?}");
    assert!(first.pipelines_created <= KEYS, "one pipeline per key: {first:?}");

    let before = scene3d_counters();
    for mode in modes.iter().copied().chain([first_mode]) {
        draw_all(mode);
    }
    let again = since(before);
    assert_eq!(0, again.pipelines_created, "the device has its pipelines: {again:?}");
    assert!(again.scenes_drawn_on_gpu > 0, "{again:?}");
}
