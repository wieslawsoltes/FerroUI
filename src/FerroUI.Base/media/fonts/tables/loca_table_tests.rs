//! Port of `Media/Fonts/Tables/LocaTableTests.cs`: the tests on the embedded
//! Inter of the glyph typeface tests, loaded the same way (`load_inter`).
//!
//! `TryGetOffsets_Returns_False_For_Out_Of_Range` and
//! `Glyph_Count_Is_Clamped_To_What_The_Table_Covers` are with the table
//! (`loca_table.rs`).

use std::rc::Rc;

use crate::media::fonts::tables::{HeadTable, LocaTable, MaxpTable};
use crate::media::glyph_typeface_tests::load_inter;
use crate::media::GlyphTypeface;

fn load_loca(typeface: &Rc<GlyphTypeface>) -> LocaTable {
    let font = &**typeface.platform_typeface();
    let head = HeadTable::try_load(font).unwrap();
    assert!(head.is_some());
    let maxp = MaxpTable::load(font).unwrap();

    let loca = LocaTable::load(font, &head.unwrap(), &maxp);

    assert!(loca.is_some());
    loca.unwrap()
}

#[test]
fn load_returns_table_with_glyphs_for_inter() {
    let loca = load_loca(&load_inter());

    assert!(loca.glyph_count() > 0);
}

#[test]
fn glyph_count_matches_maxp() {
    let typeface = load_inter();
    let maxp = MaxpTable::load(&**typeface.platform_typeface()).unwrap();

    let loca = load_loca(&typeface);

    assert_eq!(maxp.num_glyphs as i32, loca.glyph_count());
}

#[test]
fn try_get_offsets_returns_false_at_glyph_count() {
    let loca = load_loca(&load_inter());

    // GlyphCount is one past the last valid glyph index.
    assert!(loca.try_get_offsets(loca.glyph_count()).is_none());
}

#[test]
fn try_get_offsets_yields_ascending_ranges_for_every_glyph() {
    let loca = load_loca(&load_inter());

    for i in 0..loca.glyph_count() {
        let offsets = loca.try_get_offsets(i);
        assert!(offsets.is_some());
        let (start, end) = offsets.unwrap();

        // A glyph's data range must be non-negative in length; equal start/end
        // marks an empty glyph.
        assert!(end >= start, "Glyph {i}: end {end} < start {start}");
    }
}

#[test]
fn empty_glyph_has_equal_offsets() {
    let typeface = load_inter();
    let map = typeface.character_to_glyph_map();

    assert!(map.contains_glyph(' ' as i32));
    let space_glyph = map.get_glyph(' ' as i32);

    let loca = load_loca(&typeface);

    let offsets = loca.try_get_offsets(space_glyph as i32);
    assert!(offsets.is_some());
    let (start, end) = offsets.unwrap();

    // The space glyph carries no outline, so loca gives it a zero-length range.
    assert_eq!(start, end);
}

#[test]
fn letter_glyph_has_non_empty_range() {
    let typeface = load_inter();
    let map = typeface.character_to_glyph_map();

    assert!(map.contains_glyph('A' as i32));
    let letter_glyph = map.get_glyph('A' as i32);

    let loca = load_loca(&typeface);

    let offsets = loca.try_get_offsets(letter_glyph as i32);
    assert!(offsets.is_some());
    let (start, end) = offsets.unwrap();
    assert!(end > start, "'A' should have a non-empty glyf range.");
}
