//! Port of upstream's `Media/GlyphRunTests.cs` of the Skia unit tests.
//!
//! `GetCharacterHitFromDistance(distance, out _)` is
//! `get_character_hit_from_distance(distance).0`. xUnit's
//! `Assert.Equal(expected, actual, precision)` (both values rounded to
//! `precision` decimals) is [`assert_equal_precision`].

use super::CustomFontManagerImpl;
use crate::unit_tests::mock_platform_render_interface;
use crate::{GlyphRunImpl, PlatformRenderInterface};
use ferroui_base::media::text_formatting::unicode::Codepoint;
use ferroui_base::media::text_formatting::{ShapedBuffer, TextMetrics, TextShaper, TextShaperOptions};
use ferroui_base::media::{
    CharacterHit, FontManager, FontStretch, FontStyle, FontWeight, GlyphInfoList, GlyphRun, Typeface,
};
use ferroui_base::utilities::{CultureInfo, ReadOnlyMemory};
use ferroui_base::Rect;
use ferroui_controls::testing::{UnitTestApplication, UnitTestApplicationScope};
use std::rc::Rc;

/// The rows of the theories on caret navigation: the text and its bidi
/// level.
const CARET_ROWS: [(&str, i8); 2] = [
    ("ABC012345", 0),                                   //LeftToRight
    ("זה כיף סתם לשמוע איך תנצח קרפד עץ טוב בגן", 1), //RightToLeft
];

#[test]
fn should_get_next_character_hit() {
    for (text, direction) in CARET_ROWS {
        let _app = start();

        let options = TextShaperOptions::with_all(
            Typeface::default_typeface().glyph_typeface(),
            10.0,
            direction,
            Some(CultureInfo::current_culture()),
            0.0,
            0.0,
            None,
        );
        let shaped_buffer = TextShaper::current().shape_text(&ReadOnlyMemory::from_str(text), &options);

        let glyph_run = create_glyph_run(&shaped_buffer);

        let mut character_hit = CharacterHit::new(0);
        let rects = build_rects(&glyph_run);

        if glyph_run.is_left_to_right() {
            for rect in &rects {
                character_hit = glyph_run.get_next_caret_character_hit(character_hit);

                let distance = glyph_run.get_distance_from_character_hit(character_hit);

                assert_eq!(rect.right(), distance, "{text}");
            }
        } else {
            for rect in &rects {
                character_hit = glyph_run.get_next_caret_character_hit(character_hit);

                let distance = glyph_run.get_distance_from_character_hit(character_hit);

                assert_eq!(rect.left(), distance, "{text}");
            }
        }
    }
}

#[test]
fn should_get_previous_character_hit() {
    for (text, direction) in CARET_ROWS {
        let _app = start();

        let options = TextShaperOptions::with_all(
            Typeface::default_typeface().glyph_typeface(),
            10.0,
            direction,
            Some(CultureInfo::current_culture()),
            0.0,
            0.0,
            None,
        );
        let shaped_buffer = TextShaper::current().shape_text(&ReadOnlyMemory::from_str(text), &options);

        let glyph_run = create_glyph_run(&shaped_buffer);

        let mut character_hit = CharacterHit::new(utf16_length(text));
        let mut rects = build_rects(&glyph_run);

        rects.reverse();

        if glyph_run.is_left_to_right() {
            for rect in &rects {
                character_hit = glyph_run.get_previous_caret_character_hit(character_hit);

                let distance = glyph_run.get_distance_from_character_hit(character_hit);

                assert_eq!(rect.left(), distance, "{text}");
            }
        } else {
            for rect in &rects {
                character_hit = glyph_run.get_previous_caret_character_hit(character_hit);

                let distance = glyph_run.get_distance_from_character_hit(character_hit);

                assert_eq!(rect.right(), distance, "{text}");
            }
        }
    }
}

#[test]
fn should_get_character_hit_from_distance() {
    for (text, direction) in CARET_ROWS {
        let _app = start();

        let options = TextShaperOptions::with_all(
            Typeface::default_typeface().glyph_typeface(),
            10.0,
            direction,
            Some(CultureInfo::current_culture()),
            0.0,
            0.0,
            None,
        );
        let shaped_buffer = TextShaper::current().shape_text(&ReadOnlyMemory::from_str(text), &options);

        let glyph_run = create_glyph_run(&shaped_buffer);

        if glyph_run.is_left_to_right() {
            let (character_hit, _) = glyph_run.get_character_hit_from_distance(glyph_run.bounds().width);

            assert_eq!(
                glyph_run.characters().len() as i32,
                character_hit.first_character_index() + character_hit.trailing_length(),
                "{text}"
            );
        } else {
            let (character_hit, _) = glyph_run.get_character_hit_from_distance(0.0);

            assert_eq!(
                glyph_run.characters().len() as i32,
                character_hit.first_character_index() + character_hit.trailing_length(),
                "{text}"
            );
        }

        let mut rects = build_rects(&glyph_run);

        let mut last_cluster = -1;
        let mut index = 0;

        if !glyph_run.is_left_to_right() {
            rects.reverse();
        }

        let glyph_infos = glyph_run.glyph_infos();
        let glyph_infos = glyph_infos.borrow();

        for rect in &rects {
            let mut current_cluster = glyph_infos[index].glyph_cluster;

            while current_cluster == last_cluster && index + 1 < glyph_infos.len() {
                index += 1;
                current_cluster = glyph_infos[index].glyph_cluster;
            }

            //Non trailing edge
            let distance = if glyph_run.is_left_to_right() { rect.left() } else { rect.right() };

            let (character_hit, _) = glyph_run.get_character_hit_from_distance(distance);

            assert_eq!(
                current_cluster,
                character_hit.first_character_index() + character_hit.trailing_length(),
                "{text}"
            );

            last_cluster = current_cluster;

            index += 1;
        }
    }
}

// Upstream issue #12676.
#[test]
fn similar_runs_have_same_ink_bounds_after_blob_creation() {
    let _app = start();

    let typeface = Typeface::from_name("resm:FerroUI.Vello.UnitTests.Assets?assembly=ferroui-vello#Inter");
    let options = TextShaperOptions::with_all(typeface.glyph_typeface(), 14.0, 0, None, 0.0, 0.0, None);
    let shaped_buffer = TextShaper::current().shape_text(&ReadOnlyMemory::from_str("F"), &options);

    let glyph_run1 = create_glyph_run(&shaped_buffer);
    let bounds1 = glyph_run1.ink_bounds();
    // Upstream creates a text blob of the first run here, which once
    // changed the bounds of a later run. A glyph run of this backend has no
    // blob: what it makes when it is first asked are the outlines of its
    // glyphs, for the intersections.
    let platform_impl = glyph_run1.platform_impl();
    assert!(platform_impl.as_any().downcast_ref::<GlyphRunImpl>().is_some());
    platform_impl.get_intersections(0.0, 1.0);

    let bounds2 = create_glyph_run(&shaped_buffer).ink_bounds();

    assert_eq!(bounds1, bounds2);
}

#[test]
fn glyph_run_with_leading_space_has_correct_ink_bounds() {
    let _app = start();

    let typeface = Typeface::from_name("resm:FerroUI.Vello.UnitTests.Assets?assembly=ferroui-vello#Inter");
    let options = TextShaperOptions::with_all(typeface.glyph_typeface(), 14.0, 0, None, 0.0, 0.0, None);
    let shaped_buffer = TextShaper::current().shape_text(&ReadOnlyMemory::from_str(" I"), &options);

    let glyph_run1 = create_glyph_run(&shaped_buffer);
    let bounds = glyph_run1.ink_bounds();

    assert!(bounds.left() > 0.0, "{bounds}");
}

#[test]
fn should_get_distance_from_character_hit_non_trailing_right_to_left() {
    const TEXT: &str = "נִקּוּד";

    let _app = start();

    let typeface = Typeface::from_name("resm:FerroUI.Vello.UnitTests.Assets?assembly=ferroui-vello#Inter");
    let options = TextShaperOptions::with_all(typeface.glyph_typeface(), 14.0, 1, None, 0.0, 0.0, None);
    let shaped_buffer = TextShaper::current().shape_text(&ReadOnlyMemory::from_str(TEXT), &options);

    let glyph_run = create_glyph_run(&shaped_buffer);

    //Get the distance to the left side of the first glyph
    let mut actual_distance = glyph_run.get_distance_from_character_hit(CharacterHit::new(utf16_length(TEXT)));

    let mut expected_distance = 0.0;

    assert_equal_precision(expected_distance, actual_distance, 2);

    //Get the distance to the right side of the first glyph
    actual_distance = glyph_run.get_distance_from_character_hit(CharacterHit::new(utf16_length(TEXT) - 1));

    expected_distance = shaped_buffer.get(0).glyph_advance;

    assert_equal_precision(expected_distance, actual_distance, 2);
}

#[test]
fn should_character_hit_from_distance_zero_width() {
    const DF7_FONT: &str = "resm:FerroUI.Vello.UnitTests.Fonts?assembly=ferroui-vello#DF7segHMI";
    const TEXT: &str = "3,47-=?:#";

    let _app = start();

    let typeface = Typeface::from_name(DF7_FONT);
    let options = TextShaperOptions::with_all(typeface.glyph_typeface(), 14.0, 0, None, 0.0, 0.0, None);
    let shaped_buffer = TextShaper::current().shape_text(&ReadOnlyMemory::from_str(TEXT), &options);

    assert!(shaped_buffer.length() > 0);

    let first_glyph_info = shaped_buffer.get(0);

    let glyph_run = create_glyph_run(&shaped_buffer);

    let (character_hit, _) = glyph_run.get_character_hit_from_distance(first_glyph_info.glyph_advance);

    assert_eq!(2, character_hit.first_character_index() + character_hit.trailing_length());
}

#[test]
fn should_get_distance_from_character_hit_zero_width() {
    const TEXT: &str = "נִקּוּד";

    let _app = start();

    let typeface = Typeface::from_name("resm:FerroUI.Vello.UnitTests.Assets?assembly=ferroui-vello#Inter");
    let options = TextShaperOptions::with_all(typeface.glyph_typeface(), 14.0, 1, None, 0.0, 0.0, None);
    let shaped_buffer = TextShaper::current().shape_text(&ReadOnlyMemory::from_str(TEXT), &options);

    let glyph_run = create_glyph_run(&shaped_buffer);

    // `GroupBy(x => x.GlyphCluster)`: the groups in order of first
    // appearance, each with the advances of its glyphs.
    let glyph_infos = glyph_run.glyph_infos();
    let mut clusters: Vec<(i32, Vec<f64>)> = Vec::new();
    for glyph_info in glyph_infos.borrow().iter() {
        match clusters.iter_mut().find(|(cluster, _)| *cluster == glyph_info.glyph_cluster) {
            Some((_, advances)) => advances.push(glyph_info.glyph_advance),
            None => clusters.push((glyph_info.glyph_cluster, vec![glyph_info.glyph_advance])),
        }
    }

    let mut right_side_distances = Vec::new();

    let mut left_side_distances = Vec::new();

    let mut current_x = 0.0;

    for (_, advances) in &clusters {
        left_side_distances.push(current_x);

        current_x += advances.iter().sum::<f64>();

        right_side_distances.push(current_x);
    }

    let character_indices: Vec<i32> = clusters.iter().map(|(cluster, _)| *cluster).collect();

    let mut character_hit = CharacterHit::new(utf16_length(TEXT));

    for i in 0..character_indices.len() {
        let character_index = character_indices[i];

        let left_side_distance = left_side_distances[i];

        let (left_side_character_hit, _) = glyph_run.get_character_hit_from_distance(left_side_distance);

        let mut distance = glyph_run.get_distance_from_character_hit(left_side_character_hit);

        assert_equal_precision(left_side_distance, distance, 2);

        let previous_character_hit = glyph_run.get_previous_caret_character_hit(character_hit);

        distance = glyph_run.get_distance_from_character_hit(CharacterHit::new(character_index));

        let right_side_distance = right_side_distances[i];

        assert_equal_precision(right_side_distance, distance, 2);

        character_hit = previous_character_hit;
    }
}

#[test]
fn should_get_distance_from_character_hit_within_cluster() {
    let text = "எடுத்துக்காட்டு வழி வினவல்";

    let _app = start();

    let units: Vec<u16> = text.encode_utf16().collect();
    let (cp, _) = Codepoint::read_at(&units, 0);

    let typeface = FontManager::current().try_match_character(
        cp.value() as i32,
        FontStyle::Normal,
        FontWeight::Normal,
        FontStretch::Normal,
        None,
        None,
    );
    assert!(typeface.is_some());
    let typeface = typeface.unwrap();

    let options = TextShaperOptions::with_all(typeface.glyph_typeface(), 12.0, 0, None, 0.0, 0.0, None);

    let shaped_buffer = TextShaper::current().shape_text(&ReadOnlyMemory::from_str(text), &options);

    let glyph_run = create_glyph_run(&shaped_buffer);

    let mut cluster_width = Vec::new();
    let mut distances = Vec::new();
    let mut clusters = Vec::new();
    let mut last_cluster = -1;
    let mut current_distance = 0.0;
    let mut current_advance = 0.0;

    for glyph_info in shaped_buffer.glyph_infos().iter() {
        if last_cluster != glyph_info.glyph_cluster {
            cluster_width.push(current_advance);
            distances.push(current_distance);
            clusters.push(glyph_info.glyph_cluster);

            current_advance = 0.0;
        }

        last_cluster = glyph_info.glyph_cluster;
        current_distance += glyph_info.glyph_advance;
        current_advance += glyph_info.glyph_advance;
    }

    cluster_width.remove(0);

    cluster_width.push(current_advance);

    let expected_left_hit = CharacterHit::new(11);

    let mut distance = glyph_run.get_distance_from_character_hit(expected_left_hit);

    let expected_left = distances[6];

    assert_eq!(expected_left, distance);

    let (left_hit, _) = glyph_run.get_character_hit_from_distance(expected_left);

    assert_eq!(11, left_hit.first_character_index() + left_hit.trailing_length());

    let expected_right = distances[7];

    distance = glyph_run.get_distance_from_character_hit(CharacterHit::new(12));

    assert_eq!(expected_right, distance);

    let expected_right_hit = CharacterHit::new(13);

    distance = glyph_run.get_distance_from_character_hit(expected_right_hit);

    assert_eq!(expected_right, distance);

    let (right_hit, _) = glyph_run.get_character_hit_from_distance(expected_right);

    assert_eq!(13, right_hit.first_character_index() + right_hit.trailing_length());
}

#[test]
fn should_add_half_line_gap_to_baseline() {
    let _app = start();

    let typeface = Typeface::from_name("resm:FerroUI.Vello.UnitTests.Fonts?assembly=ferroui-vello#Inter");
    let options = TextShaperOptions::with_all(typeface.glyph_typeface(), 14.0, 0, None, 0.0, 0.0, None);
    let shaped_buffer = TextShaper::current().shape_text(&ReadOnlyMemory::from_str("F"), &options);

    let text_metrics = TextMetrics::new(shaped_buffer.glyph_typeface(), 14.0);

    let glyph_run = create_glyph_run(&shaped_buffer);

    let expected_baseline = -text_metrics.ascent + text_metrics.line_gap / 2.0;

    assert_eq!(expected_baseline, glyph_run.metrics().baseline);
}

fn build_rects(glyph_run: &GlyphRun) -> Vec<Rect> {
    let height = glyph_run.bounds().height;

    let mut current_x = if glyph_run.is_left_to_right() { 0.0 } else { glyph_run.bounds().width };

    let glyph_infos = glyph_run.glyph_infos();
    let glyph_infos = glyph_infos.borrow();

    let mut rects: Vec<Rect> = Vec::with_capacity(glyph_infos.len());

    let mut last_cluster = -1;

    for index in 0..glyph_infos.len() {
        let current_cluster = glyph_infos[index].glyph_cluster;

        let advance = glyph_infos[index].glyph_advance;

        if last_cluster != current_cluster {
            if glyph_run.is_left_to_right() {
                rects.push(Rect::new(current_x, 0.0, advance, height));
            } else {
                rects.push(Rect::new(current_x - advance, 0.0, advance, height));
            }
        } else {
            let mut rect = rects[index - 1];

            // `List.Remove`: the first equal rectangle.
            if let Some(position) = rects.iter().position(|r| *r == rect) {
                rects.remove(position);
            }

            rect = if glyph_run.is_left_to_right() {
                rect.with_width(rect.width + advance)
            } else {
                Rect::new(rect.x - advance, 0.0, rect.width + advance, height)
            };

            rects.push(rect);
        }

        if glyph_run.is_left_to_right() {
            current_x += advance;
        } else {
            current_x -= advance;
        }

        last_cluster = current_cluster;
    }

    rects
}

fn create_glyph_run(shaped_buffer: &Rc<ShapedBuffer>) -> Rc<GlyphRun> {
    GlyphRun::new(
        shaped_buffer.glyph_typeface().clone(),
        shaped_buffer.font_rendering_em_size(),
        shaped_buffer.text().clone(),
        GlyphInfoList::from(shaped_buffer.clone()),
        None,
        shaped_buffer.bidi_level() as i32,
    )
}

fn start() -> UnitTestApplicationScope {
    UnitTestApplication::start(
        mock_platform_render_interface()
            .with_render_interface(Rc::new(PlatformRenderInterface::default()))
            .with_font_manager_impl(Rc::new(CustomFontManagerImpl::new())),
    )
}

/// The length of `text` in UTF-16 code units (C# `string.Length`).
fn utf16_length(text: &str) -> i32 {
    text.encode_utf16().count() as i32
}

/// xUnit's `Assert.Equal(double expected, double actual, int precision)`.
fn assert_equal_precision(expected: f64, actual: f64, precision: i32) {
    let scale = 10f64.powi(precision);
    let round = |value: f64| (value * scale).round_ties_even() / scale;
    assert_eq!(round(expected), round(actual), "expected {expected}, actual {actual} (precision {precision})");
}
