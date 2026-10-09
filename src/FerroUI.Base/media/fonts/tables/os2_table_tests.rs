//! Port of `Media/Fonts/Tables/OS2TableTests.cs`.
//!
//! The font is the embedded Inter of the glyph typeface tests, loaded the same
//! way (`load_inter`); the table loaders take the font memory of the typeface.

use crate::media::fonts::tables::{FontSelectionFlags, OS2Table, PanoseFamilyKind};
use crate::media::glyph_typeface_tests::load_inter;

fn load_os2() -> OS2Table {
    let typeface = load_inter();

    let loaded = OS2Table::try_load(&**typeface.platform_typeface()).unwrap();

    assert!(loaded.is_some());
    loaded.unwrap()
}

#[test]
fn should_load_os2_table_from_inter_font() {
    let typeface = load_inter();

    let loaded = OS2Table::try_load(&**typeface.platform_typeface()).unwrap();

    assert!(loaded.is_some());
}

#[test]
fn os2_table_should_have_valid_weight_class() {
    let os2_table = load_os2();
    assert_eq!(400, os2_table.weight_class);
}

#[test]
fn os2_table_should_have_valid_width_class() {
    let os2_table = load_os2();
    assert_eq!(5, os2_table.width_class);
}

#[test]
fn os2_table_should_have_valid_typo_metrics() {
    let os2_table = load_os2();
    assert_eq!(2728, os2_table.typo_ascender);
    assert_eq!(-680, os2_table.typo_descender);
    assert!(os2_table.typo_ascender > os2_table.typo_descender);
}

#[test]
fn os2_table_should_have_valid_win_metrics() {
    let os2_table = load_os2();
    assert_eq!(2728, os2_table.win_ascent);
    assert_eq!(680, os2_table.win_descent);
}

#[test]
fn os2_table_should_have_valid_strikeout_metrics() {
    let os2_table = load_os2();
    assert_eq!(192, os2_table.strikeout_size);
}

#[test]
fn os2_table_inter_regular_should_be_regular() {
    let os2_table = load_os2();
    assert!(os2_table.selection.contains(FontSelectionFlags::REGULAR));
}

#[test]
fn os2_table_should_have_consistent_ascent_values() {
    let os2_table = load_os2();
    assert_eq!(2728, os2_table.typo_ascender);
    assert_eq!(2728, os2_table.win_ascent);
}

#[test]
fn os2_table_should_have_consistent_descent_values() {
    let os2_table = load_os2();
    assert_eq!(-680, os2_table.typo_descender);
    assert_eq!(680, os2_table.win_descent);
}

#[test]
fn os2_table_should_have_valid_panose() {
    let os2_table = load_os2();

    let panose = os2_table.panose;

    assert_eq!(PanoseFamilyKind::LatinText, panose.family_kind());
}
