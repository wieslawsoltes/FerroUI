//! The scenes across the rendering modes of the Vello backend.
//!
//! Every scene of [`crate::scenes`] is drawn by each rendering mode that
//! the machine can run (the CPU mode always; the hybrid and the GPU mode
//! with a graphics adapter) and compared with the Skia backend and with
//! the CPU mode: the first tells how far a mode is from the reference
//! backend, the second how far the modes are from each other, which is
//! what an application sees when its window is drawn by one mode and its
//! bitmaps by another.
//!
//! The bound of a scene against Skia is the bound recorded for the scene
//! ([`Scene::bound`](crate::scenes::Scene)), which was measured in the CPU
//! mode, unless the mode has a bound of its own here; against the CPU mode
//! every scene of a mode has the bound of the mode, unless listed.
//!
//! A machine without a graphics adapter runs the CPU mode alone and says
//! so: `skipped: no adapter`.

use crate::scenes::{Scene, SCENE_SIZE};
use crate::{compare, render, Backend, Difference, Pixels};
use ferroui_vello::VelloRenderingMode;
use std::panic::{catch_unwind, AssertUnwindSafe};

/// The share of pixels (in percent) that may differ from the CPU mode by
/// more than the tolerance in a scene of a mode that has no bound of its
/// own: the hybrid mode shares the code that makes strips of paths with
/// the CPU mode, and the GPU mode is given the same lines.
const BOUND_AGAINST_CPU: f64 = 0.05;

/// The bounds of a scene in a mode that differ from the general ones, with
/// the reason: `(scene, mode, bound against Skia, bound against the CPU
/// mode)`. `None` leaves the general bound.
const BOUNDS: &[(&str, VelloRenderingMode, Option<f64>, Option<f64>)] = &[
    // The compute renderer has no edges without anti-aliasing: the ellipse
    // of the scene is drawn anti-aliased (measured: 0.555 % and 0.565 %).
    ("aliased_rectangle", VelloRenderingMode::Gpu, Some(0.89), Some(0.90)),
    // The same for the edges of a rectangle clip that is rotated (measured:
    // 0.750 % against both).
    ("transformed_clip", VelloRenderingMode::Gpu, Some(1.18), Some(1.18)),
    // And for text: the aliased glyphs of the scene are anti-aliased
    // (measured: 2.473 % against both).
    ("text_aliased", VelloRenderingMode::Gpu, Some(3.76), Some(3.76)),
];

/// What a mode does not draw natively in a scene and how the sink of the
/// mode draws it instead (design document, section 1.3 and the sinks).
const FALLBACKS: &[(&str, VelloRenderingMode, &str)] = &[
    ("aliased_rectangle", VelloRenderingMode::Gpu, "the rectangle snapped to pixels; the ellipse anti-aliased (no aliased edges)"),
    ("transformed_clip", VelloRenderingMode::Gpu, "the rotated clip anti-aliased (no aliased edges)"),
    ("nested_clips", VelloRenderingMode::Gpu, "the rectangle clip snapped to pixels"),
    ("text_aliased", VelloRenderingMode::Gpu, "the glyphs anti-aliased (no aliased edges)"),
    ("text_mixed_scripts_with_fallback", VelloRenderingMode::Hybrid, "the pictures of the colour font through the glyph atlas of the renderer"),
    ("text_simulations", VelloRenderingMode::Gpu, "the bold amount at the size of the font, the shear turned over (the renderer's own conventions)"),
    ("gradient_with_transform", VelloRenderingMode::Gpu, "the linear gradient given in the pixels of the target"),
    ("opacity_mask", VelloRenderingMode::Hybrid, "a layer composed DestIn (no mask layers), as in every mode"),
    ("opacity_mask", VelloRenderingMode::Gpu, "a layer composed DestIn, as in every mode"),
    ("tile_repeated", VelloRenderingMode::Hybrid, "the tile as a texture of the device"),
    ("tile_flipped", VelloRenderingMode::Hybrid, "the tile as a texture of the device"),
    ("tile_transformed", VelloRenderingMode::Hybrid, "the tile as a texture of the device"),
    ("tile_single_transformed", VelloRenderingMode::Hybrid, "the tile as a texture of the device"),
    ("bitmap", VelloRenderingMode::Hybrid, "the bitmap as a texture of the device"),
];

/// Scenes a mode cannot draw at all yet, with the stage that will: they are
/// listed in the table and do not fail the test. A scene that fails to draw
/// and is not listed here fails it.
const NOT_DRAWN: &[(&str, VelloRenderingMode, &str)] = &[];

/// The scenes of the table: the scenes of shapes and the scenes of text.
pub fn scenes() -> Vec<Scene> {
    crate::scenes::scenes().into_iter().chain(crate::text::text_scenes()).collect()
}

/// The modes of the Vello backend this machine runs, and for each of the
/// GPU modes it does not run, why not.
pub fn available_modes() -> (Vec<VelloRenderingMode>, Vec<String>) {
    let (mut available, mut unavailable) = (Vec::new(), Vec::new());

    for mode in [VelloRenderingMode::Cpu, VelloRenderingMode::Hybrid, VelloRenderingMode::Gpu] {
        match ferroui_vello::scene::try_create_scene_sink(mode, 1, 1) {
            Ok(_) => available.push(mode),
            Err(reason) => unavailable.push(format!("skipped: no adapter for the {mode:?} mode: {reason}")),
        }
    }

    (available, unavailable)
}

/// The bound of a scene in a mode against Skia.
pub fn bound_against_skia(scene: &Scene, mode: VelloRenderingMode) -> f64 {
    BOUNDS
        .iter()
        .find(|(name, bound_mode, _, _)| *name == scene.name && *bound_mode == mode)
        .and_then(|(_, _, against_skia, _)| *against_skia)
        .unwrap_or(scene.bound)
}

/// The bound of a scene in a mode against the CPU mode.
pub fn bound_against_cpu(scene: &Scene, mode: VelloRenderingMode) -> f64 {
    BOUNDS
        .iter()
        .find(|(name, bound_mode, _, _)| *name == scene.name && *bound_mode == mode)
        .and_then(|(_, _, _, against_cpu)| *against_cpu)
        .unwrap_or(BOUND_AGAINST_CPU)
}

/// How a mode draws what it does not draw natively in a scene, when there
/// is such a thing.
pub fn fallback(scene: &Scene, mode: VelloRenderingMode) -> Option<&'static str> {
    FALLBACKS.iter().find(|(name, fallback_mode, _)| *name == scene.name && *fallback_mode == mode).map(|(_, _, how)| *how)
}

/// A scene in a mode, measured.
pub struct ModeResult {
    pub scene: &'static str,
    pub mode: VelloRenderingMode,
    /// The difference from the Skia backend and from the CPU mode, or why
    /// the mode did not draw the scene.
    pub drawn: Result<(Difference, Difference), String>,
}

/// Draws a scene in a mode. A mode that panics on a scene (what it does
/// not draw fails with its stage) gives the message of the panic.
fn render_in_mode(mode: VelloRenderingMode, scene: &Scene) -> Result<Pixels, String> {
    let backend = Backend::vello(mode);

    let rendered = catch_unwind(AssertUnwindSafe(|| render(&backend, SCENE_SIZE, &scene.draw)));

    // For a look at what a mode drew: the pixels as they are (premultiplied
    // RGBA, 200 by 200), in the directory `FERROUI_VELLO_TEST_DUMP` names.
    if let (Ok(pixels), Ok(directory)) = (&rendered, std::env::var("FERROUI_VELLO_TEST_DUMP")) {
        let file = std::path::Path::new(&directory).join(format!("{}-{}.rgba", scene.name, mode_name(mode)));
        let _ = std::fs::write(file, &pixels.rgba);
    }

    rendered.map_err(|panic| {
        panic
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| panic.downcast_ref::<&str>().map(|message| message.to_string()))
            .unwrap_or_else(|| "a panic without a message".to_string())
    })
}

/// Measures every scene in the given modes.
pub fn measure(modes: &[VelloRenderingMode]) -> Vec<ModeResult> {
    let skia = Backend::skia();
    let mut results = Vec::new();

    for scene in scenes() {
        let reference = render(&skia, SCENE_SIZE, &scene.draw);
        let cpu = render_in_mode(VelloRenderingMode::Cpu, &scene);

        for mode in modes {
            let drawn = match (*mode, &cpu) {
                (VelloRenderingMode::Cpu, Ok(cpu)) => Ok((compare(&reference, cpu), compare(cpu, cpu))),
                (VelloRenderingMode::Cpu, Err(error)) => Err(error.clone()),
                (_, cpu) => render_in_mode(*mode, &scene).and_then(|pixels| match cpu {
                    Ok(cpu) => Ok((compare(&reference, &pixels), compare(cpu, &pixels))),
                    Err(error) => Err(format!("the CPU mode did not draw it: {error}")),
                }),
            };
            results.push(ModeResult { scene: scene.name, mode: *mode, drawn });
        }
    }

    results
}

fn mode_name(mode: VelloRenderingMode) -> &'static str {
    match mode {
        VelloRenderingMode::Cpu => "CPU",
        VelloRenderingMode::Hybrid => "hybrid",
        VelloRenderingMode::Gpu => "GPU",
    }
}

/// The table of the design document: a row for each scene, and for each
/// mode the share of pixels beyond the tolerance against Skia and against
/// the CPU mode with the largest difference of each.
pub fn table(modes: &[VelloRenderingMode], results: &[ModeResult]) -> String {
    let mut text = String::from("| Scene |");
    for mode in modes {
        text += &format!(" {} against Skia |", mode_name(*mode));
        if *mode != VelloRenderingMode::Cpu {
            text += &format!(" {} against CPU |", mode_name(*mode));
        }
    }
    text += " Drawn another way |\n|---|";
    for mode in modes {
        text += if *mode == VelloRenderingMode::Cpu { "---|" } else { "---|---|" };
    }
    text += "---|\n";

    let scenes = scenes();
    let mut sums = vec![(0.0f64, 0.0f64, 0usize); modes.len()];

    for scene in &scenes {
        text += &format!("| `{}` |", scene.name);
        let mut notes = Vec::new();

        for (index, mode) in modes.iter().enumerate() {
            let result = results.iter().find(|result| result.scene == scene.name && result.mode == *mode);
            match result.map(|result| &result.drawn) {
                Some(Ok((against_skia, against_cpu))) => {
                    text += &format!(" {:.3} % ({}) |", against_skia.share, against_skia.largest);
                    if *mode != VelloRenderingMode::Cpu {
                        text += &format!(" {:.3} % ({}) |", against_cpu.share, against_cpu.largest);
                    }
                    sums[index] = (sums[index].0 + against_skia.share, sums[index].1 + against_cpu.share, sums[index].2 + 1);
                }
                Some(Err(_)) | None => {
                    text += if *mode == VelloRenderingMode::Cpu { " not drawn |" } else { " not drawn | not drawn |" };
                    if let Some((_, _, stage)) = NOT_DRAWN.iter().find(|(name, not_drawn_mode, _)| *name == scene.name && not_drawn_mode == mode) {
                        notes.push(format!("{}: not drawn ({stage})", mode_name(*mode)));
                    }
                }
            }
            if let Some(how) = fallback(scene, *mode) {
                notes.push(format!("{}: {how}", mode_name(*mode)));
            }
        }

        text += &format!(" {} |\n", notes.join("; "));
    }

    text += "| **mean of the scenes drawn** |";
    for (index, mode) in modes.iter().enumerate() {
        let (against_skia, against_cpu, count) = sums[index];
        let count = count.max(1) as f64;
        text += &format!(" **{:.3} %** |", against_skia / count);
        if *mode != VelloRenderingMode::Cpu {
            text += &format!(" **{:.3} %** |", against_cpu / count);
        }
    }
    text += " |\n";
    text
}

/// The scenes that are further from Skia or from the CPU mode than their
/// bound in a mode, and the scenes a mode did not draw without being
/// listed as not drawn.
pub fn failures(results: &[ModeResult]) -> Vec<String> {
    let scenes = scenes();
    let mut failures = Vec::new();

    for result in results {
        let Some(scene) = scenes.iter().find(|scene| scene.name == result.scene) else {
            continue;
        };
        let name = mode_name(result.mode);

        match &result.drawn {
            Ok((against_skia, against_cpu)) => {
                let bound = bound_against_skia(scene, result.mode);
                if against_skia.share > bound {
                    failures.push(format!(
                        "{}: the {name} mode differs from Skia in {:.3} % of the pixels, more than its bound of {bound:.2} %",
                        scene.name, against_skia.share
                    ));
                }
                let bound = bound_against_cpu(scene, result.mode);
                if against_cpu.share > bound {
                    failures.push(format!(
                        "{}: the {name} mode differs from the CPU mode in {:.3} % of the pixels, more than its bound of {bound:.2} %",
                        scene.name, against_cpu.share
                    ));
                }
            }
            Err(error) => {
                let listed = NOT_DRAWN.iter().any(|(scene_name, mode, _)| *scene_name == scene.name && *mode == result.mode);
                if !listed {
                    failures.push(format!("{}: the {name} mode did not draw the scene: {error}", scene.name));
                }
            }
        }
    }

    failures
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every scene in every mode this machine runs, against Skia and
    /// against the CPU mode: the table of the design document and the
    /// bounds.
    ///
    /// Run with `--nocapture` to see the table.
    #[test]
    fn scenes_stay_within_their_bounds_in_every_mode() {
        let (modes, unavailable) = available_modes();
        for reason in &unavailable {
            println!("{reason}");
        }
        assert!(modes.contains(&VelloRenderingMode::Cpu));

        let results = measure(&modes);
        println!("{}", table(&modes, &results));

        let failures = failures(&results);
        assert!(failures.is_empty(), "scenes regressed:\n{}", failures.join("\n"));
    }

    /// A machine that is meant to test the GPU modes sets
    /// `FERROUI_VELLO_REQUIRE_GPU`: without an adapter the table above has
    /// the CPU mode alone, and this is the test that then fails.
    #[test]
    fn the_gpu_modes_are_measured_when_demanded() {
        let demanded = std::env::var("FERROUI_VELLO_REQUIRE_GPU").is_ok_and(|value| !value.is_empty() && value != "0");
        let (modes, unavailable) = available_modes();

        println!("modes measured: {}", modes.iter().map(|mode| mode_name(*mode)).collect::<Vec<_>>().join(", "));
        for reason in &unavailable {
            println!("{reason}");
        }
        assert!(!demanded || unavailable.is_empty(), "FERROUI_VELLO_REQUIRE_GPU is set: {}", unavailable.join("; "));
    }

    /// The tables of this module name scenes that exist.
    #[test]
    fn the_tables_name_scenes_that_exist() {
        let scenes = scenes();
        let exists = |name: &str| scenes.iter().any(|scene| scene.name == name);

        assert!(BOUNDS.iter().all(|(name, ..)| exists(name)));
        assert!(FALLBACKS.iter().all(|(name, ..)| exists(name)));
        assert!(NOT_DRAWN.iter().all(|(name, ..)| exists(name)));
    }
}
