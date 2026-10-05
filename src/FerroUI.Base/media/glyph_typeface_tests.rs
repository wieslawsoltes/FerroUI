//! Tests of the glyph typeface on fonts built in code.

use std::rc::Rc;

use crate::media::fonts::tables::testing::SyntheticFont;
use crate::media::fonts::testing::{test_fonts, TestFontBuilder, TestPlatformTypeface};
use crate::media::fonts::FontCodePageCoverage;
use crate::media::text_formatting::unicode::Script;
use crate::media::{FontSimulations, FontStretch, FontStyle, FontWeight, GlyphTypeface};
use crate::utilities::CultureInfo;

fn inter() -> Rc<TestPlatformTypeface> {
    test_fonts::inter_regular().build()
}

fn create(simulations: FontSimulations) -> Rc<GlyphTypeface> {
    GlyphTypeface::new(inter(), simulations).unwrap()
}

fn try_create(font: &SyntheticFont) -> Option<Rc<GlyphTypeface>> {
    let platform_typeface = TestPlatformTypeface::from_bytes(font.to_bytes(), FontSimulations::None)?;

    GlyphTypeface::try_create(platform_typeface, FontSimulations::None)
}

#[test]
fn should_load_font() {
    let typeface = create(FontSimulations::None);

    assert_eq!(typeface.family_name(), "Inter");
}

#[test]
fn should_have_character_to_glyph_map_for_common_characters() {
    let typeface = create(FontSimulations::None);

    let map = typeface.character_to_glyph_map();

    for character in ['A', 'a', '0', ' '] {
        assert!(map.contains_glyph(character as i32), "{character}");
        assert_ne!(map.get_glyph(character as i32), 0, "{character}");
    }

    assert_ne!(map.get_glyph('A' as i32), map.get_glyph('B' as i32));
    assert!(!map.contains_glyph(0x4E2D));
}

#[test]
fn get_glyph_advance_should_return_advance_for_glyph_id() {
    let typeface = create(FontSimulations::None);

    let glyph = typeface.character_to_glyph_map().get_glyph('A' as i32);

    assert_eq!(typeface.try_get_horizontal_glyph_advance(glyph), Some(600));

    let mut advances = [0u16; 2];

    assert!(typeface.try_get_horizontal_glyph_advances(&[glyph, glyph + 1], &mut advances));
    assert_eq!(advances, [600, 600]);
}

#[test]
fn should_have_valid_font_metrics() {
    let typeface = create(FontSimulations::None);

    let metrics = typeface.metrics();

    assert!(metrics.design_em_height > 0);
    assert!(metrics.ascent < 0);
    assert!(metrics.descent > 0);
    assert_eq!(metrics.line_gap, 0);
    assert!(metrics.underline_thickness > 0);
    assert!(!metrics.is_fixed_pitch);
    assert!(GlyphTypeface::new(test_fonts::noto_mono().build(), FontSimulations::None).unwrap().metrics().is_fixed_pitch);
}

#[test]
fn should_have_positive_glyph_count() {
    assert!(create(FontSimulations::None).glyph_count() > 0);
}

#[test]
fn should_have_correct_font_properties() {
    let typeface = create(FontSimulations::None);

    assert_eq!(typeface.weight(), FontWeight::Normal);
    assert_eq!(typeface.style(), FontStyle::Normal);
    assert_eq!(typeface.stretch(), FontStretch::Normal);
    assert_eq!(typeface.font_simulations(), FontSimulations::None);
}

#[test]
fn should_read_weight_style_and_stretch_from_the_font() {
    let oblique = TestFontBuilder::new("Family")
        .weight(FontWeight(250))
        .style(FontStyle::Oblique)
        .stretch(FontStretch::ExtraExpanded)
        .build();

    let typeface = GlyphTypeface::new(oblique, FontSimulations::None).unwrap();

    assert_eq!(typeface.weight(), FontWeight(250));
    assert_eq!(typeface.style(), FontStyle::Oblique);
    assert_eq!(typeface.stretch(), FontStretch::ExtraExpanded);
}

#[test]
fn should_apply_bold_simulation() {
    let typeface = create(FontSimulations::Bold);

    assert_eq!(typeface.weight(), FontWeight::Bold);
    assert_eq!(typeface.font_simulations(), FontSimulations::Bold);
}

#[test]
fn should_apply_oblique_simulation() {
    let typeface = create(FontSimulations::Oblique);

    assert_eq!(typeface.style(), FontStyle::Italic);
    assert_eq!(typeface.font_simulations(), FontSimulations::Oblique);
}

#[test]
fn should_apply_combined_simulations() {
    let typeface = create(FontSimulations::Bold | FontSimulations::Oblique);

    assert_eq!(typeface.weight(), FontWeight::Bold);
    assert_eq!(typeface.style(), FontStyle::Italic);
    assert_eq!(typeface.font_simulations(), FontSimulations::Bold | FontSimulations::Oblique);
}

#[test]
fn should_have_typographic_family_name() {
    assert_eq!(create(FontSimulations::None).typographic_family_name(), "");

    let manrope = GlyphTypeface::new(test_fonts::manrope_light().build(), FontSimulations::None).unwrap();

    assert_eq!(manrope.family_name(), "Manrope Light");
    assert_eq!(manrope.typographic_family_name(), "Manrope");
}

#[test]
fn should_have_family_names_dictionary() {
    let font = TestFontBuilder::new("Test Gothic").localized_family_name(0x0411, "Test Gothic JP").build();

    let typeface = GlyphTypeface::new(font, FontSimulations::None).unwrap();

    assert_eq!(typeface.family_names().len(), 2);
    assert_eq!(typeface.family_names()[&CultureInfo::get_culture_info("en-US")], "Test Gothic");
    assert_eq!(typeface.family_names()[&CultureInfo::get_culture_info("ja-JP")], "Test Gothic JP");
}

#[test]
fn should_have_face_names_dictionary() {
    let typeface = create(FontSimulations::None);

    assert_eq!(typeface.face_names()[&CultureInfo::get_culture_info("en-US")], "Regular");
}

#[test]
fn should_have_supported_features() {
    let typeface = create(FontSimulations::None);

    // The font has no layout tables: no features, and the (cached) list is stable.
    assert!(typeface.supported_features().is_empty());
    assert!(std::ptr::eq(typeface.supported_features(), typeface.supported_features()));
}

#[test]
fn try_get_glyph_advance_should_return_none_for_invalid_glyph_id() {
    assert_eq!(create(FontSimulations::None).try_get_horizontal_glyph_advance(u16::MAX), None);
}

#[test]
fn try_get_glyph_metrics_should_return_none_for_invalid_glyph_id() {
    assert_eq!(create(FontSimulations::None).try_get_glyph_metrics(u16::MAX), None);
}

#[test]
fn try_get_vertical_glyph_advance_returns_none_for_latin_font() {
    let typeface = create(FontSimulations::None);

    let glyph_index = typeface.character_to_glyph_map().get_glyph('A' as i32);

    // The font carries no vertical metrics.
    assert_eq!(typeface.try_get_vertical_glyph_advance(glyph_index), None);

    let mut advances = [7u16; 1];

    assert!(!typeface.try_get_vertical_glyph_advances(&[glyph_index], &mut advances));
}

#[test]
fn should_have_valid_platform_typeface() {
    let platform_typeface = inter();

    let typeface = GlyphTypeface::new(platform_typeface.clone(), FontSimulations::None).unwrap();

    assert!(std::ptr::addr_eq(Rc::as_ptr(typeface.platform_typeface()), Rc::as_ptr(&platform_typeface)));
    assert_eq!(typeface.platform_typeface().family_name(), "Inter");
}

#[test]
fn should_dispose_properly() {
    let platform_typeface = inter();

    let typeface = GlyphTypeface::new(platform_typeface.clone(), FontSimulations::None).unwrap();

    typeface.dispose();

    assert!(platform_typeface.is_disposed());

    // Should not fail on double dispose
    typeface.dispose();
}

#[test]
fn font_metrics_line_spacing_should_be_calculated_correctly() {
    let metrics = GlyphTypeface::new(
        TestFontBuilder::new("Family").vertical_metrics(900, -300, 50).build(),
        FontSimulations::None,
    )
    .unwrap()
    .metrics();

    assert_eq!(metrics.ascent, -900);
    assert_eq!(metrics.descent, 300);
    assert_eq!(metrics.line_gap, 50);
    assert_eq!(metrics.line_spacing(), metrics.descent - metrics.ascent + metrics.line_gap);
}

#[test]
fn as_read_only_dictionary_matches_the_underlying_map() {
    let typeface = create(FontSimulations::None);

    let map = typeface.character_to_glyph_map();
    let dictionary = map.as_read_only_dictionary();

    assert!(dictionary.count() > 0);
    assert_eq!(dictionary.count() as usize, dictionary.iter().count());

    for (code_point, glyph) in dictionary.iter() {
        assert_eq!(map.try_get_glyph(code_point), Some(glyph));
    }

    assert!(dictionary.contains_key('A' as i32));
    assert!(!dictionary.contains_key(0x4E2D));
}

#[test]
fn supported_unicode_range_follows_the_character_map() {
    let typeface = create(FontSimulations::None);

    let range = typeface.supported_unicode_range();

    assert!(range.is_in_range('A' as i32));
    assert!(range.is_in_range(0xE9));
    assert!(!range.is_in_range(0x4E2D));
}

#[test]
fn declared_coverage_and_scripts_are_read_from_the_font() {
    let japanese = GlyphTypeface::new(
        TestFontBuilder::new("Test Gothic")
            .codepoints(&[(0x3040, 0x309F)])
            .code_page_coverage(FontCodePageCoverage::JapaneseJis)
            .design_languages("ja")
            .unicode_range_bit(59)
            .build(),
        FontSimulations::None,
    )
    .unwrap();

    assert_eq!(japanese.code_page_coverage(), FontCodePageCoverage::JapaneseJis);
    assert_eq!(japanese.design_languages(), ["ja"]);
    assert!(japanese.declares_language_coverage(Some(&CultureInfo::get_culture_info("ja-JP"))));
    assert!(!japanese.declares_language_coverage(Some(&CultureInfo::get_culture_info("ko-KR"))));
    assert!(!japanese.declares_language_coverage(None));
    // Declared by the Unicode range bit, and proven by the probe codepoint.
    assert!(japanese.supports_script(Script::Han));
    assert!(japanese.supports_script(Script::Hiragana));
    assert!(!japanese.supports_script(Script::Hangul));

    let arabic = GlyphTypeface::new(test_fonts::noto_sans_arabic().build(), FontSimulations::None).unwrap();

    assert!(arabic.can_shape_script(Script::Arabic));
    assert!(!arabic.can_shape_script(Script::Devanagari));
    // Simple scripts need no shaping tables.
    assert!(arabic.can_shape_script(Script::Latin));
}

#[test]
fn last_resort_fonts_are_flagged() {
    let typeface = GlyphTypeface::new(test_fonts::adobe_blank().build(), FontSimulations::None).unwrap();

    assert!(typeface.is_last_resort());
}

// ── malformed and missing tables ──
//
// `name` and `post` are optional/cosmetic: an absent table is tolerated, and so is a
// present-but-malformed (truncated) one: the loader degrades to the same fallback as an
// absent table.

#[test]
fn truncated_post_table_does_not_deny_the_font() {
    // Keep only the 4-byte version field; reading the rest of the header over-runs.
    let mut font = test_fonts::inter_regular().build_font();

    font.truncate("post", 4);

    // A malformed 'post' (underline / italic-angle / fixed-pitch hints only) degrades to
    // defaults; the rest of the font - including the intact 'name' - still loads.
    assert_eq!(try_create(&font).unwrap().family_name(), "Inter");
}

#[test]
fn truncated_name_table_falls_back_to_unknown_family() {
    // Keep only format + count; reading stringOffset and the record array over-runs.
    let mut font = test_fonts::inter_regular().build_font();

    font.truncate("name", 4);

    // A malformed 'name' degrades to no name table, so the family name falls back to "unknown"
    // instead of denying a renderable font.
    assert_eq!(try_create(&font).unwrap().family_name(), "unknown");
}

#[test]
fn missing_post_table_still_loads_the_font() {
    let mut font = test_fonts::inter_regular().build_font();

    font.remove("post");

    assert_eq!(try_create(&font).unwrap().family_name(), "Inter");
}

#[test]
fn missing_name_table_falls_back_to_unknown_family() {
    let mut font = test_fonts::inter_regular().build_font();

    font.remove("name");

    let typeface = try_create(&font).unwrap();

    assert_eq!(typeface.family_name(), "unknown");
    assert_eq!(typeface.family_names()[&CultureInfo::invariant_culture()], "unknown");
    assert_eq!(typeface.face_names()[&CultureInfo::invariant_culture()], FontWeight::Normal.to_string());
}

#[test]
fn missing_required_tables_deny_the_font() {
    let mut font = test_fonts::inter_regular().build_font();

    font.remove("maxp");

    assert!(try_create(&font).is_none());

    let mut font = test_fonts::inter_regular().build_font();

    font.truncate("OS/2", 10);

    assert!(try_create(&font).is_none());
}
