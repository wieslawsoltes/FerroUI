use crate::custom_effect_scenes::{reference_difference, render_at, SIDE};
use crate::scene3d_scenes::{draw_plain, scenes, unshifted_matrix_camera_scene};
use crate::{compare, Backend, TOLERANCE};
use ferroui_base::PixelSize;
use ferroui_render3d::cpu::{render, scenes_rasterized};
use ferroui_render3d::CpuRenderOptions;

/// The backends under test: Skia raster and every mode of the Vello backend that is built (the
/// hybrid and the GPU mode on the device of the machine, where the CPU reference is the counted
/// last resort until step D4).
fn backends() -> Vec<Backend> {
    [Backend::skia()].into_iter().chain(Backend::vello_modes().into_iter().map(Backend::vello)).collect()
}

/// The CPU reference of a scene at `scale`: the image of [`SIDE`] by [`SIDE`] logical pixels.
fn reference(scene: &ferroui_base::rendering::scene3d::Scene3DSnapshot, scale: i32) -> Vec<u8> {
    let side = SIDE * scale;
    render(scene, PixelSize::new(side, side), f64::from(scale), CpuRenderOptions::default()).rgba
}

/// Every plain 3D scene, drawn by every backend and mode at 96 and 192 DPI, is its CPU reference:
/// the backends draw the image of the reference one to one (and, from step D4, their GPU
/// executors are held to it here).
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
/// opacity.
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

/// The 3D scenes drawn by Skia raster and by each mode of the Vello backend: the same image of
/// the CPU reference, composed by each backend. A plain scene is drawn one to one by both, so
/// they agree to the last digit but for rounding; the rotated scene differs at the antialiased
/// edges of the rotated image and of its clip.
///
/// Run with `--nocapture` to see the table.
#[test]
fn the_backends_draw_every_scene_3d_alike() {
    let skia = Backend::skia();
    let mut failures = Vec::new();
    for mode in Backend::vello_modes() {
        let vello = Backend::vello(mode);
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
                let mean_bound = 2.0 * case.tolerance.mean() + 0.05;
                println!(
                    "| `{}` {scale}x | {:.3} % | {} | {:.3} | {:.2} % | {:.2} |",
                    case.name, difference.share, difference.largest, difference.mean, case.bound, mean_bound
                );
                if difference.share > case.bound {
                    failures.push(format!(
                        "{} at {scale}x: {} differs from {} in {:.3} % of the pixels, more than its bound of {:.2} %",
                        case.name, vello.name, skia.name, difference.share, case.bound
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
    let without_camera = std::sync::Arc::new(without_camera);
    for backend in backends() {
        for scene in [(empty.scene)(), without_camera.clone()] {
            let drawn = render_at(&backend, 1, &|context| draw_plain(context, &scene));
            assert!(drawn.rgba.iter().all(|&byte| byte == 0), "{} drew something", backend.name);
        }
    }
}

/// A viewport whose snapshot is unchanged is not rasterized again: the backend draws the image it
/// kept (the generations, the size and the scale are those it was made for); a new size or a new
/// snapshot is rasterized.
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
        assert_eq!(before + 1, scenes_rasterized(), "{}: drawn twice, rasterized once", backend.name);

        render_at(&backend, 2, &|context| draw_plain(context, &scene));
        assert_eq!(before + 2, scenes_rasterized(), "{}: a new size", backend.name);

        let changed = (scenes()[0].scene)();
        render_at(&backend, 2, &|context| draw_plain(context, &changed));
        assert_eq!(before + 3, scenes_rasterized(), "{}: a new snapshot", backend.name);
    }
}
