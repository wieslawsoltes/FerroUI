use crate::effect_scenes::{scenes, SCENE_SIZE};
use crate::{compare, render, Backend, TOLERANCE};

/// The scenes of stage 6 drawn by the Skia backend and by every rendering
/// mode of the Vello backend that is built: the table of the design
/// document, and the two bounds of every scene.
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

            println!(
                "| `{}` | {:.3} % | {} | {:.3} | {:.2} % | {:.2} |",
                scene.name, difference.share, difference.largest, difference.mean, scene.share_bound, scene.mean_bound
            );

            share_sum += difference.share;
            mean_sum += difference.mean;
            count += 1;

            if difference.share > scene.share_bound {
                failures.push(format!(
                    "{}: {} differs from {} in {:.3} % of the pixels, more than its bound of {:.2} %",
                    scene.name, vello.name, skia.name, difference.share, scene.share_bound
                ));
            }
            if difference.mean > scene.mean_bound {
                failures.push(format!(
                    "{}: {} differs from {} by {:.3} of 255 in the mean, more than its bound of {:.2}",
                    scene.name, vello.name, skia.name, difference.mean, scene.mean_bound
                ));
            }
        }

        println!("| all {count} scenes | {:.3} % | | {:.3} | | |", share_sum / count as f64, mean_sum / count as f64);
        println!();
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
            assert!(first == second, "{} draws {} the same way twice", backend.name, scene.name);
        }
    }
}
