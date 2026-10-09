use crate::scenes::SCENE_SIZE;
use crate::text::{
    base_glyph_run, decorations_layout, fallback_layout, glyph_run, glyph_typeface, platform_typeface, text_scenes,
    with_text_services, ContextSink, FALLBACK_TEXT, INTER, TEST_FONTS,
};
use crate::{compare, render, Backend, TOLERANCE};
use ferroui_base::media::fonts::OpenTypeTag;
use ferroui_base::media::text_formatting::{GlyphInfo, TextLayout, TextLayoutOptions};
use ferroui_base::media::{
    FontSimulations, FontStretch, FontStyle, FontWeight, GlyphTypeface, IPlatformTypeface, TextDecorations,
    TextWrapping,
};
use ferroui_base::platform::IFontManagerImpl;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{Point, Rect, Vector};
use ferroui_vello::VelloRenderingMode;
use std::rc::Rc;

fn backends() -> (Backend, Backend) {
    (Backend::skia(), Backend::vello(VelloRenderingMode::Cpu))
}

/// The text scenes drawn by the Skia backend and by every rendering mode of
/// the Vello backend that is built: the text table of the design document,
/// and the bound of every scene.
///
/// Run with `--nocapture` to see the table.
#[test]
fn text_scenes_stay_within_their_bounds() {
    let skia = Backend::skia();
    let mut failures = Vec::new();

    // The CPU mode; the other modes are measured in `modes.rs`, each with its own bounds.
    for mode in [VelloRenderingMode::Cpu] {
        let vello = Backend::vello(mode);
        let (mut share_sum, mut count) = (0.0, 0);

        println!("| Scene | Pixels beyond the tolerance of {TOLERANCE} ({} against {}) | Largest difference | Mean difference | Bound |", vello.name, skia.name);
        println!("|---|---|---|---|---|");

        for scene in text_scenes() {
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

        println!("| all {count} text scenes | {:.3} % | | | |", share_sum / count as f64);
        println!();
    }

    assert!(failures.is_empty(), "text scenes regressed:\n{}", failures.join("\n"));
}

/// The ink of a text scene: the share of pixels that are not white, in
/// percent. Both backends put ink on about as many pixels, and neither
/// draws nothing.
#[test]
fn both_backends_put_ink_where_text_is() {
    let (skia, vello) = backends();

    println!("| Scene | Pixels with ink, Skia | Pixels with ink, Vello | Mean coverage of the scene, Skia | Vello |");
    println!("|---|---|---|---|---|");

    for scene in text_scenes() {
        let ink = |backend: &Backend| {
            let pixels = render(backend, SCENE_SIZE, &scene.draw);
            let count = pixels.rgba.chunks_exact(4).filter(|pixel| pixel[..3].iter().any(|channel| *channel < 250)).count();
            let darkness: u64 = pixels
                .rgba
                .chunks_exact(4)
                .map(|pixel| pixel[..3].iter().map(|channel| 255 - *channel as u64).sum::<u64>())
                .sum();
            let pixel_count = (pixels.width * pixels.height) as f64;

            (100.0 * count as f64 / pixel_count, darkness as f64 / (pixel_count * 3.0 * 255.0) * 100.0)
        };

        let (skia_ink, skia_coverage) = ink(&skia);
        let (vello_ink, vello_coverage) = ink(&vello);

        println!(
            "| `{}` | {skia_ink:.2} % | {vello_ink:.2} % | {skia_coverage:.3} % | {vello_coverage:.3} % |",
            scene.name
        );

        assert!(skia_ink > 1.0 && vello_ink > 1.0, "{}: {skia_ink} {vello_ink}", scene.name);

        // The fallback scene is drawn in the fonts each font manager finds.
        if scene.name != "text_mixed_scripts_with_fallback" {
            let ratio = vello_coverage / skia_coverage;
            assert!((0.7..1.15).contains(&ratio), "{}: the coverage of Vello is {ratio:.3} of Skia's", scene.name);
        }
    }
}

/// A text scene drawn twice by the same backend gives the same pixels.
#[test]
fn a_backend_draws_text_the_same_way_twice() {
    for backend in [Backend::skia()].into_iter().chain(Backend::vello_modes().into_iter().map(Backend::vello)) {
        for scene in text_scenes() {
            let first = render(&backend, SCENE_SIZE, &scene.draw);
            let second = render(&backend, SCENE_SIZE, &scene.draw);
            if let Err(difference) = crate::drawn_the_same_way_twice(&backend, &first, &second) {
                panic!("{} does not draw {} the same way twice: {difference}", backend.name, scene.name);
            }
        }
    }
}

/// The tags of the tables of a font file, from its table directory.
fn table_tags(font: &[u8]) -> Vec<u32> {
    let count = u16::from_be_bytes([font[4], font[5]]) as usize;

    (0..count)
        .map(|index| {
            let record = &font[12 + index * 16..];
            u32::from_be_bytes([record[0], record[1], record[2], record[3]])
        })
        .collect()
}

fn tag_name(tag: u32) -> String {
    tag.to_be_bytes().iter().map(|byte| *byte as char).collect()
}

/// The platform typefaces of the two backends of every test font: the
/// identity each backend reads (family name, weight, style, stretch) and
/// the tables each serves, which the glyph typeface of the base library and
/// the shaper read.
#[test]
fn typefaces_agree() {
    let (skia, vello) = backends();
    let mut failures = Vec::new();

    println!("| Font | Family (Skia) | Family (Vello) | Weight, style, stretch (Skia) | (Vello) | Tables | Tables that differ |");
    println!("|---|---|---|---|---|---|---|");

    for (name, font) in TEST_FONTS {
        let skia_typeface = with_text_services(&skia, |fonts| platform_typeface(fonts, font, FontSimulations::None));
        let vello_typeface = with_text_services(&vello, |fonts| platform_typeface(fonts, font, FontSimulations::None));

        let (Some(skia_typeface), Some(vello_typeface)) = (&skia_typeface, &vello_typeface) else {
            println!(
                "| `{name}` | {} | {} | | | | |",
                if skia_typeface.is_some() { "read" } else { "not read" },
                if vello_typeface.is_some() { "read" } else { "not read" }
            );
            if skia_typeface.is_some() != vello_typeface.is_some() {
                failures.push(format!("{name}: one backend reads the font and the other does not"));
            }
            continue;
        };

        let identity = |typeface: &Rc<dyn IPlatformTypeface>| {
            format!("{}, {:?}, {:?}", typeface.weight().0, typeface.style(), typeface.stretch())
        };

        let tags = table_tags(font);
        let differing: Vec<String> = tags
            .iter()
            .filter(|tag| {
                let skia_table = skia_typeface.try_get_table(OpenTypeTag::new(**tag));
                let vello_table = vello_typeface.try_get_table(OpenTypeTag::new(**tag));

                skia_table.map(|table| table.to_vec()) != vello_table.map(|table| table.to_vec())
            })
            .map(|tag| tag_name(*tag))
            .collect();

        println!(
            "| `{name}` | {} | {} | {} | {} | {} | {} |",
            skia_typeface.family_name(),
            vello_typeface.family_name(),
            identity(skia_typeface),
            identity(vello_typeface),
            tags.len(),
            if differing.is_empty() { "none".to_owned() } else { differing.join(", ") }
        );

        if skia_typeface.family_name() != vello_typeface.family_name() {
            failures.push(format!("{name}: the family names differ"));
        }
        if identity(skia_typeface) != identity(vello_typeface) && !IDENTITY_DIFFERS.contains(name) {
            failures.push(format!("{name}: weight, style or stretch differ"));
        }
        if !differing.is_empty() {
            failures.push(format!("{name}: tables differ: {differing:?}"));
        }

        // A table the font does not have.
        assert!(skia_typeface.try_get_table(OpenTypeTag::parse("zzzz")).is_none());
        assert!(vello_typeface.try_get_table(OpenTypeTag::parse("zzzz")).is_none());
    }

    assert!(failures.is_empty(), "typefaces disagree:\n{}", failures.join("\n"));
}

/// Fonts of which the two backends read a different weight, style or
/// stretch, with the reason in the design document (section 8).
const IDENTITY_DIFFERS: &[&str] = &[];

/// The glyph typefaces over the typefaces of the two backends: the metrics
/// of the font and of every glyph. They are the base library's, read from
/// the tables each typeface serves, so they are the same numbers.
#[test]
fn glyph_metrics_agree() {
    let (skia, vello) = backends();

    println!("| Font | Em | Ascent | Descent | Line gap | Underline position, thickness | Strikethrough position, thickness | Glyphs | Advances that differ | Glyph metrics that differ |");
    println!("|---|---|---|---|---|---|---|---|---|---|");

    for (name, font) in TEST_FONTS {
        let load = |backend: &Backend| {
            with_text_services(backend, |fonts| {
                let typeface = platform_typeface(fonts, font, FontSimulations::None)?;
                GlyphTypeface::new(typeface, FontSimulations::None).ok()
            })
        };

        let (skia_typeface, vello_typeface) = (load(&skia), load(&vello));
        let (Some(skia_typeface), Some(vello_typeface)) = (&skia_typeface, &vello_typeface) else {
            assert_eq!(skia_typeface.is_some(), vello_typeface.is_some(), "{name}: a glyph typeface in both or in neither");
            println!("| `{name}` | no glyph typeface in either backend | | | | | | | | |");
            continue;
        };

        let (skia_metrics, vello_metrics) = (skia_typeface.metrics(), vello_typeface.metrics());
        assert_eq!(skia_metrics, vello_metrics, "{name}: the metrics of the font");
        assert_eq!(skia_typeface.glyph_count(), vello_typeface.glyph_count(), "{name}");
        assert_eq!(skia_typeface.family_name(), vello_typeface.family_name(), "{name}");
        assert_eq!(skia_typeface.weight(), vello_typeface.weight(), "{name}");
        assert_eq!(skia_typeface.style(), vello_typeface.style(), "{name}");
        assert_eq!(skia_typeface.stretch(), vello_typeface.stretch(), "{name}");

        let (mut advances, mut glyph_metrics) = (0, 0);
        for glyph in 0..skia_typeface.glyph_count().min(u16::MAX as i32) as u16 {
            if skia_typeface.try_get_horizontal_glyph_advance(glyph) != vello_typeface.try_get_horizontal_glyph_advance(glyph)
                || skia_typeface.try_get_vertical_glyph_advance(glyph) != vello_typeface.try_get_vertical_glyph_advance(glyph)
            {
                advances += 1;
            }
            if skia_typeface.try_get_glyph_metrics(glyph) != vello_typeface.try_get_glyph_metrics(glyph) {
                glyph_metrics += 1;
            }
        }

        println!(
            "| `{name}` | {} | {} | {} | {} | {}, {} | {}, {} | {} | {advances} | {glyph_metrics} |",
            skia_metrics.design_em_height,
            skia_metrics.ascent,
            skia_metrics.descent,
            skia_metrics.line_gap,
            skia_metrics.underline_position,
            skia_metrics.underline_thickness,
            skia_metrics.strikethrough_position,
            skia_metrics.strikethrough_thickness,
            skia_typeface.glyph_count()
        );

        assert_eq!((0, 0), (advances, glyph_metrics), "{name}");

        // The same characters map to the same glyphs.
        for codepoint in [0x20, 0x41, 0x67, 0x5D0, 0x627, 0x4E2D, 0x1F600] {
            assert_eq!(
                skia_typeface.character_to_glyph_map().try_get_glyph(codepoint),
                vello_typeface.character_to_glyph_map().try_get_glyph(codepoint),
                "{name}: U+{codepoint:04X}"
            );
        }
    }
}

/// The largest difference of an edge of two rectangles.
fn edge_difference(a: Rect, b: Rect) -> f64 {
    (a.x - b.x)
        .abs()
        .max((a.y - b.y).abs())
        .max((a.right() - b.right()).abs())
        .max((a.bottom() - b.bottom()).abs())
}

/// The bounds of glyph runs of the two backends: for every test font and
/// six em sizes, a run of each of its first glyphs alone.
#[test]
fn glyph_run_bounds_agree() {
    let (skia, vello) = backends();
    const SIZES: [f64; 6] = [9.0, 12.0, 16.0, 24.0, 40.0, 72.0];
    const GLYPHS: i32 = 300;

    println!("| Font | Runs compared | Runs with the same bounds | Largest difference of an edge | Mean difference | Skia's bounds that reach beyond Vello's |");
    println!("|---|---|---|---|---|---|");

    let mut failures = Vec::new();

    for (name, font) in TEST_FONTS {
        let bounds_of = |backend: &Backend| {
            with_text_services(backend, |fonts| {
                let typeface = platform_typeface(fonts, font, FontSimulations::None)?;
                let glyph_typeface = GlyphTypeface::new(typeface, FontSimulations::None).ok()?;
                let design_em_height = glyph_typeface.metrics().design_em_height as f64;
                let mut bounds = Vec::new();

                for em_size in SIZES {
                    for glyph in 0..glyph_typeface.glyph_count().min(GLYPHS) as u16 {
                        let advance = glyph_typeface.try_get_horizontal_glyph_advance(glyph).unwrap_or(0) as f64;
                        let glyph_infos = [GlyphInfo {
                            glyph_index: glyph,
                            glyph_cluster: 0,
                            glyph_advance: advance * em_size / design_em_height,
                            glyph_offset: Vector::new(0.0, 0.0),
                        }];
                        let run = backend.interface.create_glyph_run(&glyph_typeface, em_size, &glyph_infos, Point::new(10.0, 100.0));
                        assert_eq!(Point::new(10.0, 100.0), run.baseline_origin());
                        assert_eq!(em_size, run.font_rendering_em_size());
                        bounds.push(run.bounds());
                        run.dispose();
                    }
                }

                Some(bounds)
            })
        };

        let (Some(skia_bounds), Some(vello_bounds)) = (bounds_of(&skia), bounds_of(&vello)) else {
            println!("| `{name}` | no glyph typeface | | | | |");
            continue;
        };

        let (mut same, mut largest, mut sum, mut beyond) = (0, 0.0f64, 0.0, 0);
        for (skia_bounds, vello_bounds) in skia_bounds.iter().zip(&vello_bounds) {
            let difference = edge_difference(*skia_bounds, *vello_bounds);
            if difference == 0.0 {
                same += 1;
            }
            largest = largest.max(difference);
            sum += difference;
            if skia_bounds.width > 0.0 && !vello_bounds.contains_rect(*skia_bounds) {
                beyond += 1;
            }
        }

        let count = skia_bounds.len();
        println!(
            "| `{name}` | {count} | {same} | {largest:.2} | {:.3} | {beyond} |",
            sum / count.max(1) as f64
        );

        if largest > BOUNDS_TOLERANCE && !GLYPHS_WITHOUT_OUTLINES.contains(name) {
            failures.push(format!("{name}: the bounds of a run differ by {largest}"));
        }
    }

    assert!(failures.is_empty(), "the bounds of glyph runs disagree:\n{}", failures.join("\n"));
}

/// How far an edge of the bounds of a glyph run may be from the Skia
/// backend's, in pixels: both round the bounds of the outline out to whole
/// pixels and add one, and an edge that lies on a pixel boundary is rounded
/// either way.
const BOUNDS_TOLERANCE: f64 = 1.0;

/// Fonts of which Skia measures glyphs that are not outlines, which the
/// renderers of the Vello project do not draw: the bitmaps of a bitmap font
/// of Apple, and the SVG documents of a colour font (of which the Vello
/// backend draws, and measures, the plain outlines the font also has).
const GLYPHS_WITHOUT_OUTLINES: &[&str] = &["NISC18030.ttf", "TwitterColorEmoji-SVGinOT.ttf"];

/// The intersections of a run with a band, which a decoration leaves out:
/// the same glyphs are crossed at the same places.
#[test]
fn glyph_run_intersections_agree() {
    let (skia, vello) = backends();
    let text = "Hamburgefonstiv gjpqy Qg,;";

    println!("| Band (below the baseline) | Intervals, Skia | Intervals, Vello | Largest difference of an end |");
    println!("|---|---|---|---|");

    for (lower, upper) in [(1.0f32, 3.0f32), (2.5, 4.0), (-12.0, -10.0), (-2.0, 2.0), (30.0, 40.0), (-40.0, 40.0)] {
        let intersections = |backend: &Backend| {
            with_text_services(backend, |fonts| {
                let inter = glyph_typeface(fonts, INTER, FontSimulations::None);
                let run = glyph_run(backend, &inter, text, 24.0, 0, Point::new(10.0, 50.0));
                let intersections = run.get_intersections(lower, upper);
                run.dispose();
                intersections
            })
        };

        let (skia_intersections, vello_intersections) = (intersections(&skia), intersections(&vello));
        let largest = skia_intersections
            .iter()
            .zip(&vello_intersections)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);

        println!(
            "| {lower} to {upper} | {} | {} | {largest:.4} |",
            skia_intersections.len() / 2,
            vello_intersections.len() / 2
        );

        assert_eq!(skia_intersections.len(), vello_intersections.len(), "{lower} to {upper}: {skia_intersections:?} {vello_intersections:?}");
        assert!(
            largest < INTERSECTION_TOLERANCE,
            "{lower} to {upper}: {skia_intersections:?} {vello_intersections:?}"
        );
    }
}

/// How far an end of an intersection may be from the Skia backend's, in
/// pixels of a text of 24: Skia takes the control points of a curve that
/// lie in the band for points of the outline, so its interval of a round
/// glyph is a little wider than the outline is there (by 0.34 for the "y"
/// of the text).
const INTERSECTION_TOLERANCE: f32 = 0.5;

/// The geometry of a glyph run: the outlines of the glyphs without hinting.
#[test]
fn glyph_run_geometry_agrees() {
    let (skia, vello) = backends();

    println!("| Font, simulations | Difference of the bounds | Hit tests that differ (of 10000) |");
    println!("|---|---|---|");

    for (font_name, font_simulations) in [
        ("Inter-Regular.ttf", FontSimulations::None),
        ("Inter-Regular.ttf", FontSimulations::Oblique),
        ("Inter-Regular.ttf", FontSimulations::Bold),
        ("SourceSerif4_36pt-Italic.ttf", FontSimulations::None),
        ("NotoSansArabic-Regular.ttf", FontSimulations::None),
        ("CascadiaCode.ttf", FontSimulations::None),
    ] {
        let font = TEST_FONTS.iter().find(|(name, _)| *name == font_name).unwrap().1;
        let text = if font_name.contains("Arabic") { "\u{0633}\u{0644}\u{0627}\u{0645}" } else { "Ag&8" };

        let query = |backend: &Backend| {
            with_text_services(backend, |fonts| {
                let typeface = glyph_typeface(fonts, font, font_simulations);
                let run = base_glyph_run(&typeface, text, 64.0, Point::new(10.0, 80.0));
                let geometry = backend.interface.build_glyph_run_geometry(&run);
                let bounds = geometry.bounds();
                let mut hits = Vec::with_capacity(10000);
                for row in 0..100 {
                    for column in 0..100 {
                        hits.push(geometry.fill_contains(Point::new(column as f64 * 2.0 + 0.37, row as f64 * 1.2 + 0.41)));
                    }
                }
                (bounds, hits)
            })
        };

        let ((skia_bounds, skia_hits), (vello_bounds, vello_hits)) = (query(&skia), query(&vello));
        let difference = edge_difference(skia_bounds, vello_bounds);
        let hits = skia_hits.iter().zip(&vello_hits).filter(|(a, b)| a != b).count();
        let inside = skia_hits.iter().filter(|hit| **hit).count();

        println!("| `{font_name}`, {font_simulations:?} | {difference:.4} | {hits} ({inside} inside) |");

        assert!(inside > 300, "{font_name}: the grid covers the glyphs");
        let bold = font_simulations.contains(FontSimulations::Bold);
        assert!(difference < if bold { 0.6 } else { 0.01 }, "{font_name} {font_simulations:?}: {skia_bounds} {vello_bounds}");
        assert!(hits <= if bold { 150 } else { 5 }, "{font_name} {font_simulations:?}: {hits}");
    }
}

/// Shaping and the layout of a paragraph are the same in both backends: the
/// shaper is the same, over the same tables, and the metrics are the base
/// library's.
#[test]
fn shaping_and_layout_agree() {
    let (skia, vello) = backends();

    for (name, font) in TEST_FONTS {
        let shape = |backend: &Backend| {
            with_text_services(backend, |fonts| {
                let typeface = platform_typeface(fonts, font, FontSimulations::None)?;
                let glyph_typeface = GlyphTypeface::new(typeface, FontSimulations::None).ok()?;

                Some(
                    ["Office affinity -> fi", "\u{0627}\u{0644}\u{0639}\u{0631}\u{0628}\u{064A}\u{0629}", "\u{05E9}\u{05DC}\u{05D5}\u{05DD}"]
                        .into_iter()
                        .enumerate()
                        .flat_map(|(index, text)| crate::text::shape(&glyph_typeface, text, 17.0, (index > 0) as i8))
                        .map(|glyph| (glyph.glyph_index, glyph.glyph_cluster, glyph.glyph_advance, glyph.glyph_offset))
                        .collect::<Vec<_>>(),
                )
            })
        };

        assert_eq!(shape(&skia), shape(&vello), "{name}: glyphs, clusters, advances and offsets");
    }

    let paragraph = "The quick brown fox jumps over the lazy dog, and then the dog sleeps. \
        Office affinity fluff; AV To WA. 1234567890";

    let measure = |backend: &Backend| {
        with_text_services(backend, |_| {
            let layout = TextLayout::new(
                paragraph,
                crate::text::embedded_typeface("Inter"),
                TextLayoutOptions {
                    font_size: 15.0,
                    text_wrapping: TextWrapping::Wrap,
                    max_width: 170.0,
                    ..TextLayoutOptions::default()
                },
            );

            let lines: Vec<(i32, f64, f64, f64)> = layout
                .text_lines()
                .iter()
                .map(|line| (line.length(), line.width(), line.height(), line.baseline()))
                .collect();
            let positions: Vec<Rect> = (0..paragraph.len() as i32).map(|index| layout.hit_test_text_position(index)).collect();
            let hits: Vec<(i32, bool)> = (0..40)
                .flat_map(|row| (0..40).map(move |column| Point::new(column as f64 * 4.5, row as f64 * 4.1)))
                .map(|point| {
                    let hit = layout.hit_test_point(point);
                    (hit.text_position(), hit.is_inside())
                })
                .collect();
            let result = (layout.width(), layout.height(), layout.baseline(), lines, positions, hits);
            layout.dispose();
            result
        })
    };

    let (skia_layout, vello_layout) = (measure(&skia), measure(&vello));
    println!(
        "A paragraph of {} characters at 15 in 170: {} lines, {:.3} by {:.3}, the same in both backends",
        paragraph.len(),
        skia_layout.3.len(),
        skia_layout.0,
        skia_layout.1
    );
    assert!(skia_layout.3.len() > 5);
    assert_eq!(skia_layout, vello_layout);
}

/// What a decoration draws: the layouts of the decorations scene draw the
/// same lines in both backends (the intersections of the glyphs with the
/// underline agree).
#[test]
fn decorations_draw_the_same_lines() {
    let (skia, vello) = backends();

    struct Lines(Vec<(Point, Point, f64)>);

    impl ferroui_base::media::text_formatting::ITextDrawingSink for Lines {
        fn draw_glyph_run(
            &mut self,
            _foreground: Option<&Rc<dyn ferroui_base::media::IBrush>>,
            _glyph_run: &Rc<ferroui_base::media::GlyphRun>,
        ) {
        }

        fn draw_rectangle(
            &mut self,
            _brush: Option<&Rc<dyn ferroui_base::media::IBrush>>,
            _pen: Option<&Rc<dyn ferroui_base::media::IPen>>,
            _rect: Rect,
        ) {
        }

        fn draw_line(&mut self, pen: &Rc<dyn ferroui_base::media::IPen>, p1: Point, p2: Point) {
            self.0.push((p1, p2, pen.thickness()));
        }

        fn push_transform(&mut self, _matrix: ferroui_base::Matrix) {}

        fn pop_transform(&mut self) {}
    }

    for (what, text_decorations) in [
        ("underline", TextDecorations::underline as fn() -> _),
        ("strikethrough", TextDecorations::strikethrough),
        ("overline", TextDecorations::overline),
    ] {
        let lines = |backend: &Backend| {
            with_text_services(backend, |_| {
                let layout = decorations_layout(text_decorations());
                let mut lines = Lines(Vec::new());
                layout.draw(&mut lines, Point::new(8.0, 20.0));
                layout.dispose();
                lines.0
            })
        };

        let (skia_lines, vello_lines) = (lines(&skia), lines(&vello));
        println!("The {what} of the decorations scene: {} lines in Skia, {} in Vello", skia_lines.len(), vello_lines.len());

        assert!(!skia_lines.is_empty(), "{what}");
        assert_eq!(skia_lines.len(), vello_lines.len(), "{what}: {skia_lines:?} {vello_lines:?}");
        for ((skia_p1, skia_p2, skia_thickness), (vello_p1, vello_p2, vello_thickness)) in skia_lines.iter().zip(&vello_lines) {
            assert_eq!(skia_thickness, vello_thickness, "{what}");
            assert!(
                (skia_p1.x - vello_p1.x).abs() < INTERSECTION_TOLERANCE as f64
                    && (skia_p2.x - vello_p2.x).abs() < INTERSECTION_TOLERANCE as f64,
                "{what}: {skia_lines:?} {vello_lines:?}"
            );
            assert_eq!((skia_p1.y, skia_p2.y), (vello_p1.y, vello_p2.y), "{what}");
        }
    }
}

/// The font managers: the default family, the installed families, a family
/// matched by its attributes and the font a character falls back to.
#[test]
fn font_managers_agree() {
    let skia: Rc<dyn IFontManagerImpl> = Rc::new(ferroui_skia::FontManagerImpl::new());
    let vello: Rc<dyn IFontManagerImpl> = Rc::new(ferroui_vello::FontManagerImpl::new());

    let (skia_default, vello_default) = (skia.get_default_font_family_name(), vello.get_default_font_family_name());
    println!("The default family: {skia_default} (Skia), {vello_default} (Vello)");

    let (skia_names, vello_names) = (skia.get_installed_font_family_names(false), vello.get_installed_font_family_names(false));
    let only_skia: Vec<&String> = skia_names.iter().filter(|name| !vello_names.contains(name)).collect();
    let only_vello: Vec<&String> = vello_names.iter().filter(|name| !skia_names.contains(name)).collect();
    println!(
        "Installed families: {} (Skia), {} (Vello); {} only in Skia's list {:?}, {} only in Vello's {:?}",
        skia_names.len(),
        vello_names.len(),
        only_skia.len(),
        only_skia.iter().take(12).collect::<Vec<_>>(),
        only_vello.len(),
        only_vello.iter().take(12).collect::<Vec<_>>()
    );
    assert!(vello_names.contains(&vello_default));
    assert!(skia_names.len() > 10 && vello_names.len() > 10);

    println!();
    println!("| Family, style, weight | Skia: family, weight, style, simulations | Vello: family, weight, style, simulations |");
    println!("|---|---|---|");

    let describe = |typeface: Option<Rc<dyn IPlatformTypeface>>| match typeface {
        Some(typeface) => format!(
            "{}, {}, {:?}, {:?}",
            typeface.family_name(),
            typeface.weight().0,
            typeface.style(),
            typeface.font_simulations()
        ),
        None => "none".to_owned(),
    };

    for family in [skia_default.as_str(), "Helvetica", "Times New Roman", "Courier New", "Menlo", "Arial", "No Such Family 4f1c"] {
        for (style, weight) in [
            (FontStyle::Normal, FontWeight::Normal),
            (FontStyle::Normal, FontWeight::Bold),
            (FontStyle::Italic, FontWeight::Normal),
            (FontStyle::Italic, FontWeight::Bold),
            (FontStyle::Normal, FontWeight::Light),
        ] {
            let matched =
                |fonts: &Rc<dyn IFontManagerImpl>| fonts.try_create_glyph_typeface(family, style, weight, FontStretch::Normal);
            let (skia_match, vello_match) = (matched(&skia), matched(&vello));
            assert_eq!(skia_match.is_some(), vello_match.is_some(), "{family}: installed for both or for neither");

            println!("| {family}, {style:?}, {} | {} | {} |", weight.0, describe(skia_match), describe(vello_match));
        }
    }

    println!();
    println!("| Character | Culture | Skia falls back to | Vello falls back to |");
    println!("|---|---|---|---|");

    let characters: [(u32, &str); 16] = [
        (0x41, "Latin"),
        (0x0416, "Cyrillic"),
        (0x03A9, "Greek"),
        (0x05D0, "Hebrew"),
        (0x0627, "Arabic"),
        (0x0915, "Devanagari"),
        (0x0E01, "Thai"),
        (0x4E2D, "Han"),
        (0x3042, "Hiragana"),
        (0xAC00, "Hangul"),
        (0x1F600, "emoji"),
        (0x2603, "symbol (snowman)"),
        (0x2192, "arrow"),
        (0x221E, "mathematics (infinity)"),
        (0x20AC, "currency (euro)"),
        (0x10400, "Deseret"),
    ];

    for (codepoint, what) in characters {
        for culture in ["en-US", "ja-JP", "zh-CN"] {
            // The culture decides for Han; the other characters are asked
            // once.
            if culture != "en-US" && codepoint != 0x4E2D {
                continue;
            }

            let culture_info = CultureInfo::get_culture_info(culture);
            let matched = |fonts: &Rc<dyn IFontManagerImpl>| {
                fonts.try_match_character(
                    codepoint as i32,
                    FontStyle::Normal,
                    FontWeight::Normal,
                    FontStretch::Normal,
                    None,
                    Some(&culture_info),
                )
            };
            let (skia_match, vello_match) = (matched(&skia), matched(&vello));

            // Whatever a backend falls back to has the character.
            for typeface in [&skia_match, &vello_match].into_iter().flatten() {
                let glyph_typeface = GlyphTypeface::new(typeface.clone(), FontSimulations::None).expect("the tables of the font");
                assert!(
                    glyph_typeface.character_to_glyph_map().try_get_glyph(codepoint as i32).is_some_and(|glyph| glyph != 0),
                    "{} has U+{codepoint:04X}",
                    typeface.family_name()
                );
            }

            println!(
                "| U+{codepoint:04X} {what} | {culture} | {} | {} |",
                skia_match.as_ref().map(|typeface| typeface.family_name()).unwrap_or_else(|| "none".into()),
                vello_match.as_ref().map(|typeface| typeface.family_name()).unwrap_or_else(|| "none".into())
            );

            assert!(skia_match.is_none() || vello_match.is_some(), "U+{codepoint:04X}: Vello finds a font where Skia does");
        }
    }

    // The fonts the fallback scene is drawn in.
    for backend in [Backend::skia(), Backend::vello(VelloRenderingMode::Cpu)] {
        let families = with_text_services(&backend, |_| {
            let bitmap = backend.interface.create_render_target_bitmap(SCENE_SIZE, Vector::new(96.0, 96.0));
            let mut context = bitmap.create_drawing_context();
            let layout = fallback_layout();
            let mut sink = ContextSink::new(&mut *context);
            layout.draw(&mut sink, Point::new(10.0, 20.0));
            let families = sink.families.clone();
            layout.dispose();
            context.dispose();
            bitmap.dispose();
            families
        });

        println!("The fallback scene ({FALLBACK_TEXT}) in {}: {families:?}", backend.name);
        assert!(families.len() >= 4, "{families:?}");
    }
}

/// Where the difference of the text scenes comes from: the same lines of
/// text drawn as glyph runs and as the fill of the outlines of their glyphs
/// (the geometry of the run), by both backends, at six sizes.
///
/// The Vello backend draws a glyph as the fill of its outline, and so does
/// Skia when it fills the geometry of a run; the two agree as two fills of
/// a shape do. Skia draws a glyph run with the glyph rasterizer of the
/// platform, which on macOS is CoreText's and makes stems heavier than the
/// outline is: that is the difference the scenes measure.
#[test]
fn glyphs_are_the_fill_of_their_outlines() {
    use ferroui_base::media::{Colors, TextHintingMode, TextOptions};
    use ferroui_base::media::immutable::ImmutableSolidColorBrush;
    use ferroui_base::platform::IDrawingContextImpl;

    #[derive(Clone, Copy, PartialEq)]
    enum How {
        GlyphRun,
        GlyphRunWithoutHinting,
        Outline,
    }

    let (skia, vello) = backends();

    println!("| Em size | Skia: glyph run against outline | Vello glyph run against Skia glyph run | Vello glyph run against Skia outline | Vello glyph run without hinting against Skia outline | Vello outline against Skia outline | Coverage: Skia glyph run, Skia outline, Vello glyph run, Vello without hinting |");
    println!("|---|---|---|---|---|---|---|");

    let rows = [9.0, 12.0, 16.0, 24.0, 40.0, 72.0]
        .into_iter()
        .map(|em_size| (em_size, FontSimulations::None))
        .chain([FontSimulations::Bold, FontSimulations::Oblique].into_iter().map(|simulations| (28.0, simulations)));

    for (em_size, font_simulations) in rows {
        let draw = |backend: &Backend, how: How| {
            let scene = move |backend: &Backend, context: &mut dyn IDrawingContextImpl| {
                context.clear(Colors::WHITE);
                with_text_services(backend, |fonts| {
                    let inter = glyph_typeface(fonts, INTER, font_simulations);
                    let brush = ImmutableSolidColorBrush::new(Colors::BLACK);
                    // Baselines on rows of pixels: a glyph run is drawn
                    // with its baseline on one, a geometry where it is.
                    let line_height = (em_size * 1.25f64).ceil();
                    let mut baseline = em_size;
                    let mut line = 0;

                    while baseline < 200.0 {
                        let origin = Point::new(4.0 + (line as f64 * 0.37) % 1.0, baseline);
                        let text = ["Hamburgefonstiv", "quick brown fox", "0123456789 &@"][line % 3];

                        match how {
                            How::Outline => {
                                let run = base_glyph_run(&inter, text, em_size, origin);
                                let geometry = backend.interface.build_glyph_run_geometry(&run);
                                context.draw_geometry(Some(&brush), None, &*geometry);
                            }
                            How::GlyphRun | How::GlyphRunWithoutHinting => {
                                if how == How::GlyphRunWithoutHinting {
                                    context.push_text_options(TextOptions {
                                        text_hinting_mode: TextHintingMode::None,
                                        ..TextOptions::default()
                                    });
                                }
                                let run = glyph_run(backend, &inter, text, em_size, 0, origin);
                                context.draw_glyph_run(Some(&brush), &*run);
                                run.dispose();
                                if how == How::GlyphRunWithoutHinting {
                                    context.pop_text_options();
                                }
                            }
                        }

                        baseline += line_height;
                        line += 1;
                    }
                });
            };

            render(backend, SCENE_SIZE, &scene)
        };

        let coverage = |pixels: &crate::Pixels| {
            let darkness: u64 = pixels.rgba.chunks_exact(4).map(|pixel| 255 - pixel[0] as u64).sum();
            100.0 * darkness as f64 / (pixels.width * pixels.height * 255) as f64
        };

        let skia_run = draw(&skia, How::GlyphRun);
        let skia_outline = draw(&skia, How::Outline);
        let vello_run = draw(&vello, How::GlyphRun);
        let vello_unhinted = draw(&vello, How::GlyphRunWithoutHinting);
        let vello_outline = draw(&vello, How::Outline);

        let show = |a: &crate::Pixels, b: &crate::Pixels| {
            let difference = compare(a, b);
            format!("{:.3} % ({:.3})", difference.share, difference.mean)
        };

        let simulated = if font_simulations.is_empty() { String::new() } else { format!(", {font_simulations:?}") };
        println!(
            "| {em_size}{simulated} | {} | {} | {} | {} | {} | {:.2} %, {:.2} %, {:.2} %, {:.2} % |",
            show(&skia_run, &skia_outline),
            show(&vello_run, &skia_run),
            show(&vello_run, &skia_outline),
            show(&vello_unhinted, &skia_outline),
            show(&vello_outline, &skia_outline),
            coverage(&skia_run),
            coverage(&skia_outline),
            coverage(&vello_run),
            coverage(&vello_unhinted)
        );

        // The glyph runs of the Vello backend are the outlines: nearer to
        // the outlines as Skia fills them than Skia's own glyph runs are.
        let own = compare(&skia_run, &skia_outline).share;
        let unhinted = compare(&vello_unhinted, &skia_outline).share;
        assert!(unhinted < own, "{em_size}: {unhinted} {own}");
        let ratio = coverage(&vello_unhinted) / coverage(&skia_outline);
        assert!((0.97..1.03).contains(&ratio), "{em_size}: the coverage of Vello is {ratio:.3} of the outline's");
    }
}
