//! Port of `Media/GlyphRunTests.cs` (base unit tests): glyph runs built from glyph advances and clusters.
//!
//! The services of upstream's tests (a mocked render interface, or a styled window with the headless one) are the
//! text test scope here: its font manager supplies the default typeface the runs are created with.

use crate::media::text_formatting::testing::{utf16, TextTestScope};
use crate::media::text_formatting::GlyphInfo;
use crate::media::{CharacterHit, GlyphInfoList, GlyphRun, Typeface};
use crate::Rect;
use std::rc::Rc;

fn create_glyph_run(glyph_advances: &[f64], glyph_clusters: &[i32], bidi_level: i32) -> Rc<GlyphRun> {
    let count = glyph_advances.len();

    let glyph_infos: Vec<GlyphInfo> =
        (0..count).map(|i| GlyphInfo::new(0, glyph_clusters[i], glyph_advances[i])).collect();

    GlyphRun::new(
        Typeface::default_typeface().glyph_typeface(),
        10.0,
        utf16(&"a".repeat(count)),
        GlyphInfoList::from(glyph_infos),
        None,
        bidi_level,
    )
}

fn start() -> TextTestScope {
    TextTestScope::new()
}

#[test]
fn should_get_distance_from_character_hit() {
    let rows: [(&[f64], &[i32], i32, i32, f64); 5] = [
        (&[30.0, 0.0, 0.0], &[0, 0, 0], 0, 0, 0.0),
        (&[30.0, 0.0, 0.0], &[0, 0, 0], 0, 3, 30.0),
        (&[10.0, 10.0, 10.0], &[0, 1, 2], 1, 0, 10.0),
        (&[10.0, 10.0, 10.0], &[0, 1, 2], 2, 0, 20.0),
        (&[10.0, 10.0, 10.0], &[0, 1, 2], 2, 1, 30.0),
    ];
    for (advances, clusters, start_index, trailing_length, expected_distance) in rows {
        let _scope = start();
        let glyph_run = create_glyph_run(advances, clusters, 0);

        let character_hit = CharacterHit::with_trailing_length(start_index, trailing_length);

        let distance = glyph_run.get_distance_from_character_hit(character_hit);

        assert_eq!(expected_distance, distance);

        glyph_run.dispose();
    }
}

#[test]
fn should_get_character_hit_from_distance() {
    let rows: [(&[f64], &[i32], f64, i32, i32, bool); 4] = [
        (&[30.0, 0.0, 0.0], &[0, 0, 0], 26.0, 0, 3, true),
        (&[10.0, 10.0, 10.0], &[0, 1, 2], 20.0, 2, 0, true),
        (&[10.0, 10.0, 10.0], &[0, 1, 2], 26.0, 2, 1, true),
        (&[10.0, 10.0, 10.0], &[0, 1, 2], 35.0, 2, 1, false),
    ];
    for (advances, clusters, distance, start_index, trailing_length_expected, is_inside_expected) in rows {
        let _scope = start();
        let glyph_run = create_glyph_run(advances, clusters, 0);

        let (text_bounds, is_inside) = glyph_run.get_character_hit_from_distance(distance);

        assert_eq!(start_index, text_bounds.first_character_index());

        assert_eq!(trailing_length_expected, text_bounds.trailing_length());

        assert_eq!(is_inside_expected, is_inside);

        glyph_run.dispose();
    }
}

#[test]
fn should_find_nearest_character_hit() {
    let rows: [(&[f64], &[i32], i32, i32, i32, i32, f64); 7] = [
        (&[10.0, 10.0, 10.0], &[10, 11, 12], 0, -1, 10, 1, 10.0),
        (&[10.0, 10.0, 10.0], &[10, 11, 12], 0, 15, 12, 1, 10.0),
        (&[30.0, 0.0, 0.0], &[0, 0, 0], 0, 0, 0, 3, 30.0),
        (&[10.0, 10.0, 10.0], &[0, 1, 2], 0, 1, 1, 1, 10.0),
        (&[10.0, 20.0, 0.0, 10.0], &[0, 1, 1, 3], 0, 2, 1, 2, 20.0),
        (&[10.0, 20.0, 0.0, 10.0], &[0, 1, 1, 3], 0, 1, 1, 2, 20.0),
        (&[10.0, 0.0, 20.0, 10.0], &[3, 1, 1, 0], 1, 1, 1, 2, 20.0),
    ];
    for (advances, clusters, bidi_level, index, expected_index, expected_length, expected_width) in rows {
        let _scope = start();
        let glyph_run = create_glyph_run(advances, clusters, bidi_level);

        let (text_bounds, width) = glyph_run.find_nearest_character_hit(index);

        assert_eq!(expected_index, text_bounds.first_character_index());

        assert_eq!(expected_length, text_bounds.trailing_length());

        assert_eq!(format!("{expected_width:.2}"), format!("{width:.2}"));

        glyph_run.dispose();
    }
}

#[test]
fn should_get_next_character_hit() {
    let rows: [(&[f64], &[i32], i32, i32, i32, i32, i32); 6] = [
        (&[30.0, 0.0, 0.0], &[0, 0, 0], 0, 0, 0, 3, 0),
        (&[0.0, 0.0, 30.0], &[0, 0, 0], 0, 0, 0, 3, 1),
        (&[30.0, 0.0, 0.0, 10.0], &[0, 0, 0, 3], 3, 0, 3, 1, 0),
        (&[10.0, 0.0, 0.0, 30.0], &[3, 0, 0, 0], 3, 0, 3, 1, 1),
        (&[10.0, 30.0, 0.0, 0.0, 10.0], &[0, 1, 1, 1, 4], 1, 0, 4, 0, 0),
        (&[10.0, 0.0, 0.0, 30.0, 10.0], &[4, 1, 1, 1, 0], 1, 0, 4, 0, 1),
    ];
    for (advances, clusters, first_character_index, trailing_length, next_index, next_length, bidi_level) in rows {
        let _scope = start();
        let glyph_run = create_glyph_run(advances, clusters, bidi_level);

        let character_hit = glyph_run
            .get_next_caret_character_hit(CharacterHit::with_trailing_length(first_character_index, trailing_length));

        assert_eq!(next_index, character_hit.first_character_index());

        assert_eq!(next_length, character_hit.trailing_length());

        glyph_run.dispose();
    }
}

#[test]
fn should_get_previous_character_hit() {
    let rows: [(&[f64], &[i32], i32, i32, i32, i32, i32); 6] = [
        (&[30.0, 0.0, 0.0], &[0, 0, 0], 0, 0, 0, 0, 0),
        (&[0.0, 0.0, 30.0], &[0, 0, 0], 0, 0, 0, 0, 1),
        (&[30.0, 0.0, 0.0, 10.0], &[0, 0, 0, 3], 3, 1, 3, 0, 0),
        (&[0.0, 0.0, 30.0, 10.0], &[3, 0, 0, 0], 3, 1, 3, 0, 1),
        (&[10.0, 30.0, 0.0, 0.0, 10.0], &[0, 1, 1, 1, 4], 4, 1, 4, 0, 0),
        (&[10.0, 0.0, 0.0, 30.0, 10.0], &[4, 1, 1, 1, 0], 4, 1, 4, 0, 1),
    ];
    for (advances, clusters, current_index, current_length, previous_index, previous_length, bidi_level) in rows {
        let _scope = start();
        let glyph_run = create_glyph_run(advances, clusters, bidi_level);

        let character_hit =
            glyph_run.get_previous_caret_character_hit(CharacterHit::new(current_index + current_length));

        assert_eq!(previous_index, character_hit.first_character_index());

        assert_eq!(previous_length, character_hit.trailing_length());

        glyph_run.dispose();
    }
}

#[test]
fn should_find_glyph_index() {
    let rows: [(&[f64], &[i32], i32); 6] = [
        (&[30.0, 0.0, 0.0], &[0, 0, 0], 0),
        (&[0.0, 0.0, 30.0], &[0, 0, 0], 1),
        (&[10.0, 10.0, 10.0, 10.0], &[0, 0, 0, 3], 0),
        (&[10.0, 10.0, 10.0, 10.0], &[3, 0, 0, 0], 1),
        (&[10.0, 10.0, 10.0, 10.0, 10.0], &[0, 1, 1, 1, 4], 0),
        (&[10.0, 10.0, 10.0, 10.0, 10.0], &[4, 1, 1, 1, 0], 1),
    ];
    for (advances, clusters, bidi_level) in rows {
        let _scope = start();
        let glyph_run = create_glyph_run(advances, clusters, bidi_level);

        if glyph_run.is_left_to_right() {
            for i in 0..clusters.len() {
                let cluster = clusters[i];

                let found = glyph_run.find_glyph_index(cluster);

                let mut expected = i;

                while expected >= 1 && clusters[expected - 1] == cluster {
                    expected -= 1;
                }

                assert_eq!(expected, found);
            }
        } else {
            for i in (1..clusters.len()).rev() {
                let cluster = clusters[i];

                let found = glyph_run.find_glyph_index(cluster);

                let mut expected = i;

                while expected + 1 < clusters.len() && clusters[expected + 1] == cluster {
                    expected += 1;
                }

                assert_eq!(expected, found);
            }
        }

        glyph_run.dispose();
    }
}

#[test]
fn should_hit_test_run_without_glyphs() {
    let _scope = start();
    let glyph_run = GlyphRun::new(
        Typeface::default_typeface().glyph_typeface(),
        10.0,
        utf16("\r\n"),
        GlyphInfoList::empty(),
        None,
        0,
    );

    assert_eq!(0.0, glyph_run.bounds().width);

    // Answered without building a platform glyph run for something that marks nothing.
    assert_eq!(Rect::default(), glyph_run.ink_bounds());

    assert_eq!(0.0, glyph_run.get_distance_from_character_hit(CharacterHit::new(0)));
    assert_eq!(0.0, glyph_run.get_distance_from_character_hit(CharacterHit::with_trailing_length(0, 2)));

    assert_eq!(0, glyph_run.find_glyph_index(0));

    let (nearest_hit, width) = glyph_run.find_nearest_character_hit(0);

    assert_eq!(0, nearest_hit.first_character_index());
    assert_eq!(2, nearest_hit.trailing_length());
    assert_eq!(0.0, width);

    let (hit_from_distance, is_inside) = glyph_run.get_character_hit_from_distance(0.0);

    assert!(!is_inside);
    assert_eq!(0, hit_from_distance.first_character_index());

    glyph_run.dispose();
}
