//! Port of `Media/Fonts/Tables/MaxpTableTests.cs`.
//!
//! The font is the embedded Inter of the glyph typeface tests, loaded the same
//! way (`load_inter`); the table loaders take the font memory of the typeface.

use crate::media::fonts::tables::MaxpTable;
use crate::media::glyph_typeface_tests::load_inter;

fn load_maxp() -> MaxpTable {
    let typeface = load_inter();

    MaxpTable::load(&**typeface.platform_typeface()).unwrap()
}

#[test]
fn should_load_maxp_table_from_inter_font() {
    let maxp_table = load_maxp();

    assert_ne!(MaxpTable::default(), maxp_table);
}

#[test]
fn maxp_table_should_have_valid_num_glyphs() {
    let maxp_table = load_maxp();

    assert_eq!(2547, maxp_table.num_glyphs);
}

#[test]
fn maxp_table_true_type_should_have_version_1_0() {
    let maxp_table = load_maxp();

    assert_eq!(1, maxp_table.version.major);
    assert_eq!(0, maxp_table.version.minor);
}

#[test]
fn maxp_table_version_1_0_should_have_valid_max_points() {
    let maxp_table = load_maxp();

    assert_eq!(148, maxp_table.max_points);
}

#[test]
fn maxp_table_version_1_0_should_have_valid_max_contours() {
    let maxp_table = load_maxp();

    assert_eq!(12, maxp_table.max_contours);
}

#[test]
fn maxp_table_version_1_0_should_have_valid_max_zones() {
    let maxp_table = load_maxp();

    assert_eq!(1, maxp_table.max_zones);
}

#[test]
fn maxp_table_should_have_valid_max_composite_points() {
    let maxp_table = load_maxp();

    assert_eq!(112, maxp_table.max_composite_points);
}

#[test]
fn maxp_table_should_have_valid_max_composite_contours() {
    let maxp_table = load_maxp();

    assert_eq!(7, maxp_table.max_composite_contours);
}

#[test]
fn maxp_table_should_have_valid_max_stack_elements() {
    let maxp_table = load_maxp();

    assert_eq!(0, maxp_table.max_stack_elements);
}

#[test]
fn maxp_table_should_have_valid_max_component_depth() {
    let maxp_table = load_maxp();

    assert_eq!(1, maxp_table.max_component_depth);
}

#[test]
fn maxp_table_num_glyphs_should_match_glyph_typeface_glyph_count() {
    let typeface = load_inter();

    let maxp_table = MaxpTable::load(&**typeface.platform_typeface()).unwrap();

    assert_eq!(maxp_table.num_glyphs as i32, typeface.glyph_count());
}
