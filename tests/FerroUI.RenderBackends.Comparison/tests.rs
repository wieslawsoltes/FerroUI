use crate::geometries::{compare_shape, shapes, GRID_POINTS};
use crate::scenes::{scenes, SCENE_SIZE};
use crate::{compare, render, Backend, TOLERANCE};

/// The scenes drawn by the Skia backend and by the CPU mode of the Vello
/// backend: the table of the design document, and the bound of every
/// scene. The other modes are measured in `modes.rs`, against Skia and
/// against the CPU mode.
///
/// Run with `--nocapture` to see the table.
#[test]
fn scenes_stay_within_their_bounds() {
    let skia = Backend::skia();
    let mut failures = Vec::new();

    for mode in [ferroui_vello::VelloRenderingMode::Cpu] {
        let vello = Backend::vello(mode);
        let (mut share_sum, mut count) = (0.0, 0);

        println!("| Scene | Pixels beyond the tolerance of {TOLERANCE} ({} against {}) | Largest difference | Mean difference | Bound |", vello.name, skia.name);
        println!("|---|---|---|---|---|");

        for scene in scenes() {
            let reference = render(&skia, SCENE_SIZE, &scene.draw);
            let tested = render(&vello, SCENE_SIZE, &scene.draw);
            let difference = compare(&reference, &tested);

            println!(
                "| `{}` | {:.3} % | {} | {:.3} | {:.2} % |",
                scene.name, difference.share, difference.largest, difference.mean, scene.bound
            );

            share_sum += difference.share;
            count += 1;

            if difference.share > scene.bound {
                failures.push(format!(
                    "{}: {} differs from {} in {:.3} % of the pixels, more than its bound of {:.2} %",
                    scene.name, vello.name, skia.name, difference.share, scene.bound
                ));
            }
        }

        println!("| all {count} scenes | {:.3} % | | | |", share_sum / count as f64);
        println!();
    }

    assert!(failures.is_empty(), "scenes regressed:\n{}", failures.join("\n"));
}

/// A scene drawn twice by the same backend gives the same pixels: what the
/// comparison measures is the difference of the backends.
#[test]
fn a_backend_agrees_with_itself() {
    for backend in [Backend::skia()].into_iter().chain(Backend::vello_modes().into_iter().map(Backend::vello)) {
        for scene in scenes() {
            let first = render(&backend, SCENE_SIZE, &scene.draw);
            let second = render(&backend, SCENE_SIZE, &scene.draw);
            assert!(first == second, "{} draws {} the same way twice", backend.name, scene.name);
        }
    }
}

/// A scene that draws something else is found: the measure is not blind.
#[test]
fn a_different_scene_is_beyond_every_bound() {
    let skia = Backend::skia();
    let scenes = scenes();
    let rectangle = render(&skia, SCENE_SIZE, &scenes[0].draw);
    let ellipse = render(&skia, SCENE_SIZE, &scenes[4].draw);

    let difference = compare(&rectangle, &ellipse);
    assert!(difference.share > 10.0, "{difference:?}");
    assert_eq!(0.0, compare(&rectangle, &rectangle).share);
}

/// The geometry queries of the Vello backend against those of the Skia
/// backend, in numbers: the second table of the design document, and the
/// bounds of the differences.
#[test]
fn geometry_queries_agree() {
    let (skia, vello) = (Backend::skia(), Backend::vello(ferroui_vello::VelloRenderingMode::Cpu));
    let mut failures = Vec::new();
    let pen_count = crate::geometries::pens().len();

    let pen_names: Vec<&str> = crate::geometries::pens().iter().map(|(name, _)| *name).collect();
    println!("| Shape | Bounds | Render bounds of {pen_count} pens | Contour length | Point and tangent along the contour | Fill hit tests that differ (of {GRID_POINTS}) | Stroke hit tests that differ (of {GRID_POINTS} each: {}) |", pen_names.join(", "));
    println!("|---|---|---|---|---|---|---|");

    for shape in shapes() {
        let difference = compare_shape(&shape, &skia, &vello);
        let optional = |value: Option<f64>| value.map(|value| format!("{value:.5}")).unwrap_or_else(|| "not comparable".into());

        println!(
            "| `{}` | {:.5} | {:.5} | {} | {} | {} | {} |",
            shape.name,
            difference.bounds,
            difference.render_bounds,
            optional(difference.contour_length),
            optional(difference.point_at_distance),
            difference.fill_contains,
            difference.stroke_contains.map(|count| count.to_string()).join(", ")
        );

        let mut check = |what: &str, value: f64, bound: f64| {
            if !(value <= bound) {
                failures.push(format!("{}: {what} differs by {value}, more than {bound}", shape.name));
            }
        };

        let outline = shape.outline_of_a_stroke;
        check("the bounds", difference.bounds, if outline { RENDER_BOUNDS_TOLERANCE } else { BOUNDS_TOLERANCE });
        check(
            "the render bounds",
            difference.render_bounds,
            if outline { OUTLINE_TOLERANCE } else { RENDER_BOUNDS_TOLERANCE },
        );
        if let Some(contour_length) = difference.contour_length {
            check("the contour length", contour_length, CONTOUR_LENGTH_TOLERANCE);
        }
        if let Some(point_at_distance) = difference.point_at_distance {
            check("a point or tangent along the contour", point_at_distance, POINT_TOLERANCE);
        }
        check("fill hit tests", difference.fill_contains as f64, if outline { STROKE_HIT_TESTS } else { FILL_HIT_TESTS });
        for (index, count) in difference.stroke_contains.into_iter().enumerate() {
            // The dashed pen is the last one.
            let dashed = index == pen_count - 1;
            let bound = match (dashed, shape.same_contour) {
                (false, _) => STROKE_HIT_TESTS,
                (true, true) => DASHED_STROKE_HIT_TESTS,
                (true, false) => f64::INFINITY,
            };
            check("stroke hit tests", count as f64, bound);
        }
    }

    assert!(failures.is_empty(), "geometry queries disagree:\n{}", failures.join("\n"));
}

/// Which of two lengths that differ is the right one: the circumference
/// of the ellipse of the shapes, 160 by 110 units, is 427.759 (the second
/// approximation of Ramanujan, which is exact to more digits than that for
/// an ellipse this round).
#[test]
fn the_length_of_an_ellipse() {
    let ellipse = &shapes()[1];
    assert_eq!("ellipse", ellipse.name);
    let circumference = 427.759;

    let vello = (ellipse.create)(&Backend::vello(ferroui_vello::VelloRenderingMode::Cpu)).contour_length();
    let skia = (ellipse.create)(&Backend::skia()).contour_length();
    println!("The contour length of the ellipse: {circumference} exactly, {vello:.3} by Vello, {skia:.3} by Skia");

    assert!((vello - circumference).abs() < 0.01, "{vello}");
    assert!((skia - circumference).abs() < CONTOUR_LENGTH_TOLERANCE, "{skia}");
}

/// Skia computes in single precision: an edge of the bounds of a shape of
/// 200 units agrees to a thousandth.
const BOUNDS_TOLERANCE: f64 = 0.001;
/// The outline of a stroke is built from curves by both backends, each to
/// its own tolerance, and the dashes of a pen are laid along the contour as
/// each backend measures it: the bounds of a stroke agree to a third of a
/// unit, and those of a stroke of the outline of a stroke to one unit.
const RENDER_BOUNDS_TOLERANCE: f64 = 0.3;
const OUTLINE_TOLERANCE: f64 = 1.0;
/// Skia measures a contour along the chords of its curves (the ellipse of
/// 160 by 110 units, whose circumference is 427.8, is 0.7 shorter to it):
/// lengths and points along a contour agree to one unit.
const CONTOUR_LENGTH_TOLERANCE: f64 = 1.0;
const POINT_TOLERANCE: f64 = 1.0;
/// Points of the grid about which the hit tests of a fill may disagree:
/// none.
const FILL_HIT_TESTS: f64 = 0.0;
/// Points of the grid about which the hit tests of a stroke may disagree:
/// a third of a percent, the points that lie within a twentieth of a unit
/// of the edge of the stroke of a curve.
const STROKE_HIT_TESTS: f64 = 15.0;
/// The same for a dashed pen on a contour that both backends begin at the
/// same point. On the contour of a combined geometry, which the boolean
/// operations of each backend begin where they like, the dashes are
/// elsewhere and the hit tests are not compared.
const DASHED_STROKE_HIT_TESTS: f64 = 20.0;
