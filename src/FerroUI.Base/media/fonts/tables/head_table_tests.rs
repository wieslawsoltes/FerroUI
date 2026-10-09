//! Port of `Media/Fonts/Tables/HeadTableTests.cs`.
//!
//! The font is the embedded Inter of the glyph typeface tests, loaded the same
//! way (`load_inter`); the table loaders take the font memory of the typeface.

use crate::media::fonts::tables::{FontDirectionHint, GlyphDataFormat, HeadFlags, HeadTable, IndexToLocFormat};
use crate::media::glyph_typeface_tests::load_inter;
use crate::utilities::DateTime;

fn load_head() -> HeadTable {
    let typeface = load_inter();

    let head_table = HeadTable::try_load(&**typeface.platform_typeface()).unwrap();
    assert!(head_table.is_some());
    head_table.unwrap()
}

#[test]
fn should_load_head_table_from_inter_font() {
    let typeface = load_inter();

    let head_table = HeadTable::try_load(&**typeface.platform_typeface()).unwrap();

    assert!(head_table.is_some());
}

#[test]
fn head_table_should_have_valid_version() {
    let head_table = load_head();
    assert_eq!(1u16, head_table.version.major);
    assert_eq!(0u16, head_table.version.minor);
}

#[test]
fn head_table_should_have_valid_magic_number() {
    let head_table = load_head();
    assert_eq!(0x5F0F3CF5u32, head_table.magic_number);
}

#[test]
fn head_table_should_have_valid_units_per_em() {
    let head_table = load_head();
    assert_eq!(2816, head_table.units_per_em);
}

#[test]
fn head_table_should_have_valid_bounding_box() {
    let head_table = load_head();
    assert_eq!(-2080, head_table.x_min);
    assert_eq!(7274, head_table.x_max);
    assert_eq!(-900, head_table.y_min);
    assert_eq!(3072, head_table.y_max);
}

#[test]
fn head_table_should_have_valid_index_to_loc_format() {
    let head_table = load_head();
    assert_eq!(IndexToLocFormat::Long, head_table.index_to_loc_format);
}

#[test]
fn head_table_should_have_valid_glyph_data_format() {
    let head_table = load_head();
    assert_eq!(GlyphDataFormat::Current, head_table.glyph_data_format);
}

#[test]
fn head_table_should_have_valid_lowest_rec_ppem() {
    let head_table = load_head();
    assert_eq!(6, head_table.lowest_rec_ppem);
}

#[test]
fn head_table_should_have_valid_font_revision() {
    let head_table = load_head();
    assert!(head_table.font_revision.to_float() > 0.0);
}

#[test]
fn head_table_should_have_valid_created_timestamp() {
    let head_table = load_head();
    assert!(head_table.created > DateTime::new(1904, 1, 1).ticks());
    assert!(head_table.created < DateTime::utc_now().ticks());
}

#[test]
fn head_table_should_have_valid_modified_timestamp() {
    let head_table = load_head();
    assert!(head_table.modified > DateTime::new(1904, 1, 1).ticks());
    assert!(head_table.modified < DateTime::utc_now().ticks());
}

#[test]
fn head_table_should_have_valid_flags() {
    let head_table = load_head();
    assert!(head_table.flags.contains(HeadFlags::BaselineAtY0));
}

#[test]
fn head_table_should_have_valid_font_direction_hint() {
    let head_table = load_head();
    assert_eq!(FontDirectionHint::LeftToRightWithNeutrals, head_table.font_direction_hint);
}
