use crate::effect_scenes::{scenes, SCENE_SIZE};
use crate::{compare, render, Backend, TOLERANCE};
use ferroui_vello::VelloRenderingMode;

/// The bounds of a scene in a rendering mode that differ from the bounds of
/// the scene (which were measured in the CPU mode), with the reason:
/// `(scene, mode, bound of the share, bound of the mean, reason)`. Each is
/// the measured value, half as much again and 0.05.
const MODE_BOUNDS: &[(&str, VelloRenderingMode, f64, f64, &str)] = &[
    // Measured: 2.495 % and 1.070.
    (
        "edge_mode_aliased",
        VelloRenderingMode::Gpu,
        3.80,
        1.66,
        "the compute renderer has no edges without anti-aliasing but for rectangles on the axes, which the scene has none of: its shapes, its stroke and its turned bitmap are anti-aliased",
    ),
    // Measured: 11.748 % and 13.486; 11.748 % and 13.272; 10.967 % and
    // 12.177; 11.630 % and 12.775.
    ("blend_source_in", VelloRenderingMode::Gpu, 17.68, 20.28, DESTRUCTIVE_LAYER),
    ("blend_destination_in", VelloRenderingMode::Gpu, 17.68, 19.96, DESTRUCTIVE_LAYER),
    ("blend_source_out", VelloRenderingMode::Gpu, 16.51, 18.32, DESTRUCTIVE_LAYER),
    ("blend_destination_atop", VelloRenderingMode::Gpu, 17.50, 19.22, DESTRUCTIVE_LAYER),
];

/// Why the four blending modes that also change what is under the
/// transparent pixels of a bitmap are off in the GPU mode: the compute
/// renderer composes a layer of such a mode over every tile (of 16 by 16
/// pixels) its clip touches, so the pixels of those tiles that lie outside
/// the rectangle of the bitmap are composed with an empty source and lose
/// what was below them: a band of up to 15 pixels around the bitmap. Open
/// (design document, section 4.1): the two modes that replace (`Copy`,
/// `Clear`) are drawn in two steps that are exact; these four need a
/// decomposition of their own.
const DESTRUCTIVE_LAYER: &str =
    "the renderer composes a destructive layer over every tile its clip touches: a band around the bitmap loses what was below it";

/// The two bounds of a scene in a mode.
fn bounds(scene: &crate::effect_scenes::EffectScene, mode: VelloRenderingMode) -> (f64, f64) {
    MODE_BOUNDS
        .iter()
        .find(|(name, bound_mode, _, _, _)| *name == scene.name && *bound_mode == mode)
        .map_or((scene.share_bound, scene.mean_bound), |(_, _, share, mean, _)| (*share, *mean))
}

/// The scenes of stage 6 drawn by the Skia backend and by every rendering
/// mode of the Vello backend that is built: the table of the design
/// document, and the two bounds of every scene in the mode.
///
/// Run with `--nocapture` to see the table.
#[test]
fn effect_scenes_stay_within_their_bounds() {
    let skia = Backend::skia();
    let mut failures = Vec::new();

    for mode in Backend::vello_modes() {
        let vello = Backend::vello(mode);
        let (mut share_sum, mut mean_sum, mut count) = (0.0, 0.0, 0);

        println!(
            "| Scene | Pixels beyond the tolerance of {TOLERANCE} ({} against {}) | Largest difference | Mean difference | Bound of the share | Bound of the mean |",
            vello.name, skia.name
        );
        println!("|---|---|---|---|---|---|");

        for scene in scenes() {
            let reference = render(&skia, SCENE_SIZE, &scene.draw);
            let tested = render(&vello, SCENE_SIZE, &scene.draw);
            let difference = compare(&reference, &tested);
            let (share_bound, mean_bound) = bounds(&scene, mode);

            println!(
                "| `{}` | {:.3} % | {} | {:.3} | {:.2} % | {:.2} |",
                scene.name, difference.share, difference.largest, difference.mean, share_bound, mean_bound
            );

            share_sum += difference.share;
            mean_sum += difference.mean;
            count += 1;

            if difference.share > share_bound {
                failures.push(format!(
                    "{}: {} differs from {} in {:.3} % of the pixels, more than its bound of {:.2} %",
                    scene.name, vello.name, skia.name, difference.share, share_bound
                ));
            }
            if difference.mean > mean_bound {
                failures.push(format!(
                    "{}: {} differs from {} by {:.3} of 255 in the mean, more than its bound of {:.2}",
                    scene.name, vello.name, skia.name, difference.mean, mean_bound
                ));
            }
        }

        println!("| all {count} scenes | {:.3} % | | {:.3} | | |", share_sum / count as f64, mean_sum / count as f64);
        println!();
    }
    for (name, mode, _, _, reason) in MODE_BOUNDS {
        println!("{name} in the {mode:?} mode has bounds of its own: {reason}");
    }

    assert!(failures.is_empty(), "scenes regressed:\n{}", failures.join("\n"));
}

/// A scene drawn twice by the same backend gives the same pixels.
#[test]
fn a_backend_draws_an_effect_scene_the_same_way_twice() {
    for backend in [Backend::skia()].into_iter().chain(Backend::vello_modes().into_iter().map(Backend::vello)) {
        for scene in scenes() {
            let first = render(&backend, SCENE_SIZE, &scene.draw);
            let second = render(&backend, SCENE_SIZE, &scene.draw);
            if let Err(difference) = crate::drawn_the_same_way_twice(&backend, &first, &second) {
                panic!("{} does not draw {} the same way twice: {difference}", backend.name, scene.name);
            }
        }
    }
}

/// Why a blurred scene has a bound on its mean difference: a blur that is
/// too wide is within the tolerance in every pixel, and the mean finds it.
#[test]
fn the_mean_difference_finds_a_blur_of_another_width() {
    use ferroui_base::media::effects::ImmutableBlurEffect;
    use ferroui_base::media::immutable::ImmutableSolidColorBrush;
    use ferroui_base::media::{BoxShadows, Color, Colors};
    use ferroui_base::platform::IDrawingContextImpl;
    use ferroui_base::{Rect, RoundedRect};

    fn blurred(context: &mut dyn IDrawingContextImpl, radius: f64) {
        context.clear(Colors::WHITE);
        let effects = context.as_drawing_context_impl_with_effects().expect("a drawing context with effects");
        effects.push_effect(None, &ImmutableBlurEffect::new(radius));
        effects.draw_rectangle(
            Some(&ImmutableSolidColorBrush::new(Color::from_argb(255, 20, 40, 120))),
            None,
            RoundedRect::from_rect(Rect::new(50.0, 60.0, 100.0, 80.0)),
            &BoxShadows::default(),
        );
        effects.pop_effect();
    }

    let skia = Backend::skia();
    let reference = render(&skia, SCENE_SIZE, &|_, context| blurred(context, 10.0));

    // The same blur by the other backend, and the bound of the mean the
    // scene would be given: the measured mean, half as much again and 0.05.
    let mut bound = 0.0f64;
    for mode in Backend::vello_modes() {
        let same = compare(&reference, &render(&Backend::vello(mode), SCENE_SIZE, &|_, context| blurred(context, 10.0)));
        println!("a blur of radius 10 by the other backend: {same:?}");
        assert!(same.share == 0.0 && same.mean < 0.5, "{same:?}");
        bound = bound.max(same.mean * 1.5 + 0.05);
    }

    // A blur that is 30 % wider, by the same backend: no pixel is beyond
    // the tolerance, so the share passes it; the mean is more than twice
    // the bound.
    let wider = compare(&reference, &render(&skia, SCENE_SIZE, &|_, context| blurred(context, 13.0)));
    println!("a blur of radius 13 against one of radius 10: {wider:?}");
    assert!(wider.share == 0.0, "{wider:?}");
    assert!(wider.mean > 2.0 * bound, "{wider:?} against a bound of {bound}");

    // A blur that is moved by two pixels.
    let moved = compare(
        &reference,
        &render(&skia, SCENE_SIZE, &|_, context| {
            context.set_transform(ferroui_base::Matrix::create_translation(2.0, 0.0));
            blurred(context, 10.0);
        }),
    );
    println!("a blur of radius 10 moved by two pixels: {moved:?}");
    assert!(moved.mean > 2.0 * bound, "{moved:?} against a bound of {bound}");
}
