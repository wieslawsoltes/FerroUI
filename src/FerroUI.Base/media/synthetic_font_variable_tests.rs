//! Port of `Variable_Font_Carries_The_Variation_Tables` of
//! `Media/Fonts/TestInfrastructure/SyntheticFontTests.cs` (base unit tests): the variable Inter of upstream's
//! assets (`test_assets/fonts/InterVariable.ttf`) read by the synthetic font. The other tests of the file are with
//! the synthetic font, in `media/fonts/tables/testing/synthetic_font.rs`.

use crate::media::fonts::tables::testing::SyntheticFont;

#[test]
fn variable_font_carries_the_variation_tables() {
    let font = SyntheticFont::from_bytes(include_bytes!("../test_assets/fonts/InterVariable.ttf")).unwrap();

    assert!(font.contains("fvar"));
    assert!(font.contains("HVAR"));
    assert!(font.contains("gvar"));
}
