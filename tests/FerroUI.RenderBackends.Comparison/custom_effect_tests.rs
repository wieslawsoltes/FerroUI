use crate::custom_effect_scenes::{content, draw_with_effect, reference_difference, render_at, scenes, SIDE};
use crate::{compare, Backend, TOLERANCE};

/// The backends under test: Skia raster and every mode of the Vello backend
/// that is built (the hybrid and the GPU mode on the device of the machine).
fn backends() -> Vec<Backend> {
    [Backend::skia()].into_iter().chain(Backend::vello_modes().into_iter().map(Backend::vello)).collect()
}

/// The pixels of `backend` drawing [`content`] plain at `scale`, with the
/// CPU reference of `effect` applied to them.
fn expected(backend: &Backend, scale: i32, effect: &ferroui_base::media::effects::ImmutableCustomEffect) -> (Vec<u8>, Vec<u8>) {
    let plain = render_at(backend, scale, &content).rgba;
    let mut expected = plain.clone();
    let side = (SIDE * scale) as usize;
    assert!(effect.cpu_reference(&mut expected, side, side, f64::from(scale)), "{effect}: a CPU reference");
    (plain, expected)
}

/// Every custom effect FerroUI ships, drawn by every backend and mode at 96
/// and 192 DPI, against its CPU reference applied to the content drawn plain
/// by the same backend and mode (#455, E9): the matrix of effect, backend
/// and mode, each held to the tolerance of the effect, and each changing the
/// content.
///
/// Run with `--nocapture` to see the table.
#[test]
fn every_custom_effect_matches_its_cpu_reference_in_every_backend_and_mode() {
    let mut failures = Vec::new();
    println!("| Effect | Backend | Scale | Largest difference | Mean difference | Outliers | Tolerance |");
    println!("|---|---|---|---|---|---|---|");
    for backend in backends() {
        for scale in [1, 2] {
            for scene in scenes() {
                let effect = (scene.effect)(&backend);
                let (plain, expected) = expected(&backend, scale, &effect);
                let actual = render_at(&backend, scale, &|context| draw_with_effect(context, &effect)).rgba;
                let difference = reference_difference(&expected, &actual, scene.tolerance.largest());
                println!(
                    "| `{}` | {} | {scale}x | {} | {:.3} | {:.3} % | {:?} |",
                    scene.name,
                    backend.name,
                    difference.largest,
                    difference.mean,
                    100.0 * difference.outliers,
                    scene.tolerance
                );
                if !difference.is_within(scene.tolerance) {
                    failures.push(format!("{} on {} at {scale}x: {difference:?}, beyond {:?}", scene.name, backend.name, scene.tolerance));
                }
                if plain == actual {
                    failures.push(format!("{} on {} at {scale}x: the content is unchanged", scene.name, backend.name));
                }
            }
        }
    }
    assert!(failures.is_empty(), "custom effects beyond their tolerances:\n{}", failures.join("\n"));
}

/// The comparison scenes of the custom effects: Skia raster against each
/// mode of the Vello backend, the same effect around the same content (the
/// table of the design document's comparisons). Each backend is held to the
/// CPU reference by the test above, so the two backends are within the sum
/// of their tolerances of each other wherever the references of their plain
/// renderings agree: the bound of the share of pixels beyond the tolerance of
/// the harness is the share where those references differ, plus the
/// outliers a program that moves the content may have in each backend; the
/// bound of the mean is the two tolerances of the mean plus the mean
/// difference of the references.
///
/// Run with `--nocapture` to see the table.
#[test]
fn the_backends_draw_every_custom_effect_alike() {
    let skia = Backend::skia();
    let mut failures = Vec::new();
    for mode in Backend::vello_modes() {
        let vello = Backend::vello(mode);
        println!(
            "| Effect | Pixels beyond the tolerance of {TOLERANCE} ({} against {}) | Largest difference | Mean difference | Bound of the share | Bound of the mean |",
            vello.name, skia.name
        );
        println!("|---|---|---|---|---|---|");
        for scene in scenes() {
            let skia_effect = (scene.effect)(&skia);
            let vello_effect = (scene.effect)(&vello);
            let (_, skia_reference) = expected(&skia, 1, &skia_effect);
            let (_, vello_reference) = expected(&vello, 1, &vello_effect);
            let skia_drawn = render_at(&skia, 1, &|context| draw_with_effect(context, &skia_effect));
            let vello_drawn = render_at(&vello, 1, &|context| draw_with_effect(context, &vello_effect));
            let difference = compare(&skia_drawn, &vello_drawn);

            // Where the references differ by more than what the tolerances
            // leave of the tolerance of the harness.
            let slack = TOLERANCE.saturating_sub(2 * scene.tolerance.largest());
            let references = reference_difference(&skia_reference, &vello_reference, slack);
            let share_bound = 100.0 * (references.outliers + 2.0 * scene.tolerance.outliers()) + 0.05;
            let mean_bound = 2.0 * scene.tolerance.mean() + references.mean + 0.05;
            println!(
                "| `{}` | {:.3} % | {} | {:.3} | {:.2} % | {:.2} |",
                scene.name, difference.share, difference.largest, difference.mean, share_bound, mean_bound
            );
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
        println!();
    }
    assert!(failures.is_empty(), "custom effects regressed:\n{}", failures.join("\n"));
}

/// A neutral value of each effect that has one draws the content unchanged
/// in every backend and mode: what a transition from no effect starts and
/// ends at.
#[test]
fn a_neutral_custom_effect_draws_the_content_unchanged_in_every_backend_and_mode() {
    for backend in backends() {
        let plain = render_at(&backend, 1, &content).rgba;
        for scene in scenes() {
            let Some(neutral) = (scene.effect)(&backend).neutral() else { continue };
            let drawn = render_at(&backend, 1, &|context| draw_with_effect(context, &neutral)).rgba;
            let difference = reference_difference(&plain, &drawn, 1);
            assert!(difference.largest <= 1, "{} on {}: {difference:?}", scene.name, backend.name);
        }
    }
}
