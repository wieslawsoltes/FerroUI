//! Port of `Media/GlyphTypefaceTests.cs`.
//!
//! The fonts are embedded resources of the crate, loaded through the standard
//! asset loader as upstream loads the resources of its test assembly. Two of
//! upstream's fonts are replaced (see `NOTICE.md` of the crate and
//! `DEVIATIONS.md`, Tests and test support): `MiSans-Normal.ttf`, a CJK font
//! with vertical metrics, by `WenQuanYiMicroHei-Subset.ttf`, and
//! `NISC18030.ttf`, a font without a `head` table, by
//! `WenQuanYiMicroHei-NoHead.ttf`. Neither upstream file states a licence
//! that allows redistributing it.
//!
//! The constructor of `GlyphTypeface` returns a `Result` where upstream
//! throws; the tests unwrap it. A `KeyNotFoundException` is a panic.

use std::collections::HashSet;
use std::io::{Cursor, Read};
use std::rc::Rc;

use crate::media::fonts::tables::cmap::CharacterToGlyphMap;
use crate::media::fonts::tables::testing::SyntheticFont;
use crate::media::fonts::testing::{test_fonts, TestFontBuilder, TestPlatformTypeface};
use crate::media::fonts::{FontCodePageCoverage, OpenTypeTag, UnmanagedFontMemory};
use crate::media::text_formatting::unicode::Script;
use crate::media::{
    FontSimulations, FontStretch, FontStyle, FontWeight, GlyphMetrics, GlyphTypeface, IFontMemory, IPlatformTypeface,
};
use crate::platform::{register_manifest_resources, IAssetLoader, StandardAssetLoader};
use crate::utilities::{CultureInfo, ReadOnlyMemory, Uri};

const INTER_FONT_URI: &str = "resm:FerroUI.Base.UnitTests.Assets.Inter-Regular.ttf?assembly=ferroui-base";
const BLANK_FONT_URI: &str = "resm:FerroUI.Base.UnitTests.Assets.AdobeBlank2VF.ttf?assembly=ferroui-base";
// Deviation (DEVIATIONS.md, Tests and test support): upstream loads `NISC18030.ttf`.
const GB18030_FONT_URI: &str = "resm:FerroUI.Base.UnitTests.Assets.WenQuanYiMicroHei-NoHead.ttf?assembly=ferroui-base";
// Deviation (DEVIATIONS.md, Tests and test support): upstream loads `MiSans-Normal.ttf`.
const MI_SANS_FONT_URI: &str =
    "resm:FerroUI.Base.UnitTests.Assets.WenQuanYiMicroHei-Subset.ttf?assembly=ferroui-base";

/// The embedded resources of the test assembly: registers the fonts and
/// returns a standard asset loader (C# `new StandardAssetLoader()`).
fn standard_asset_loader() -> StandardAssetLoader {
    register_manifest_resources(
        "ferroui-base",
        &[
            ("FerroUI.Base.UnitTests.Assets.Inter-Regular.ttf", include_bytes!("../test_assets/fonts/Inter-Regular.ttf")),
            ("FerroUI.Base.UnitTests.Assets.AdobeBlank2VF.ttf", include_bytes!("../test_assets/fonts/AdobeBlank2VF.ttf")),
            (
                "FerroUI.Base.UnitTests.Assets.WenQuanYiMicroHei-NoHead.ttf",
                include_bytes!("../test_assets/fonts/WenQuanYiMicroHei-NoHead.ttf"),
            ),
            (
                "FerroUI.Base.UnitTests.Assets.WenQuanYiMicroHei-Subset.ttf",
                include_bytes!("../test_assets/fonts/WenQuanYiMicroHei-Subset.ttf"),
            ),
        ],
    );

    StandardAssetLoader::new(None)
}

fn open(asset_loader: &StandardAssetLoader, uri: &str) -> Box<dyn crate::platform::AssetStream> {
    asset_loader.open(&Uri::absolute(uri).unwrap(), None).unwrap()
}

/// C# `new GlyphTypeface(platformTypeface, fontSimulations)`.
fn glyph_typeface(platform_typeface: Rc<dyn IPlatformTypeface>, font_simulations: FontSimulations) -> Rc<GlyphTypeface> {
    GlyphTypeface::new(platform_typeface, font_simulations).unwrap()
}

/// The typeface of the font at `uri` without simulations.
fn load(uri: &str) -> Rc<GlyphTypeface> {
    let asset_loader = standard_asset_loader();

    let mut stream = open(&asset_loader, uri);

    glyph_typeface(CustomPlatformTypeface::new(&mut stream), FontSimulations::None)
}

#[test]
fn should_load_inter_font() {
    let asset_loader = standard_asset_loader();

    let mut stream = open(&asset_loader, INTER_FONT_URI);

    let typeface = glyph_typeface(CustomPlatformTypeface::new(&mut stream), FontSimulations::None);

    assert_eq!("Inter", typeface.family_name());
}

#[test]
fn should_have_character_to_glyph_map_for_common_characters() {
    let typeface = load(INTER_FONT_URI);

    let map = typeface.character_to_glyph_map();

    assert!(map.contains_glyph('A' as i32));
    assert!(map.get_glyph('A' as i32) != 0);

    assert!(map.contains_glyph('a' as i32));
    assert!(map.get_glyph('a' as i32) != 0);

    assert!(map.contains_glyph(' ' as i32));
    assert!(map.get_glyph(' ' as i32) != 0);
}

#[test]
fn get_glyph_advance_should_return_advance_for_glyph_id() {
    let typeface = load(INTER_FONT_URI);

    let map = typeface.character_to_glyph_map();

    assert!(map.contains_glyph('A' as i32));

    let glyph_index = map.get_glyph('A' as i32);

    // Ensure metrics are available for this glyph
    let metrics = typeface.try_get_glyph_metrics(glyph_index);
    assert!(metrics.is_some());

    // Ensure advance can be retrieved
    let advance = typeface.try_get_horizontal_glyph_advance(glyph_index);
    assert!(advance.is_some());

    // The advance lives on AdvanceWidth; Width is the ink bounding-box width.
    assert_eq!(metrics.unwrap().advance_width, advance.unwrap());
}

#[test]
fn should_have_valid_font_metrics() {
    for font_uri in [
        INTER_FONT_URI,
        GB18030_FONT_URI, // Font without head table
    ] {
        let typeface = load(font_uri);

        let metrics = typeface.metrics();

        assert!(metrics.design_em_height > 0, "{font_uri}");
        assert!(metrics.ascent != 0, "{font_uri}");
        assert!(metrics.descent != 0, "{font_uri}");
        assert!(metrics.line_spacing() > 0, "{font_uri}");
    }
}

#[test]
fn should_have_positive_glyph_count() {
    let typeface = load(INTER_FONT_URI);

    assert!(typeface.glyph_count() > 0);
}

#[test]
fn should_have_correct_font_properties() {
    let typeface = load(INTER_FONT_URI);

    assert_eq!(FontWeight::Normal, typeface.weight());
    assert_eq!(FontStyle::Normal, typeface.style());
    assert_eq!(FontStretch::Normal, typeface.stretch());
    assert_eq!(FontSimulations::None, typeface.font_simulations());
}

#[test]
fn should_apply_bold_simulation() {
    let asset_loader = standard_asset_loader();

    let mut stream = open(&asset_loader, INTER_FONT_URI);

    let typeface = glyph_typeface(CustomPlatformTypeface::new(&mut stream), FontSimulations::Bold);

    assert_eq!(FontWeight::Bold, typeface.weight());
    assert_eq!(FontSimulations::Bold, typeface.font_simulations());
}

#[test]
fn should_apply_oblique_simulation() {
    let asset_loader = standard_asset_loader();

    let mut stream = open(&asset_loader, INTER_FONT_URI);

    let typeface = glyph_typeface(CustomPlatformTypeface::new(&mut stream), FontSimulations::Oblique);

    assert_eq!(FontStyle::Italic, typeface.style());
    assert_eq!(FontSimulations::Oblique, typeface.font_simulations());
}

#[test]
fn should_apply_combined_simulations() {
    let asset_loader = standard_asset_loader();

    let mut stream = open(&asset_loader, INTER_FONT_URI);

    let typeface =
        glyph_typeface(CustomPlatformTypeface::new(&mut stream), FontSimulations::Bold | FontSimulations::Oblique);

    assert_eq!(FontWeight::Bold, typeface.weight());
    assert_eq!(FontStyle::Italic, typeface.style());
    assert_eq!(FontSimulations::Bold | FontSimulations::Oblique, typeface.font_simulations());
}

#[test]
fn should_have_typographic_family_name() {
    let typeface = load(INTER_FONT_URI);

    // C# `Assert.NotNull`: the name is a string, never null.
    let _name: &str = typeface.typographic_family_name();
}

#[test]
fn should_have_family_names_dictionary() {
    let typeface = load(INTER_FONT_URI);

    assert!(!typeface.family_names().is_empty());
}

#[test]
fn should_have_face_names_dictionary() {
    let typeface = load(INTER_FONT_URI);

    assert!(!typeface.face_names().is_empty());
}

#[test]
fn should_have_supported_features() {
    let typeface = load(INTER_FONT_URI);

    let features = typeface.supported_features();

    assert!(!features.is_empty());
}

#[test]
fn should_cache_supported_features() {
    let typeface = load(INTER_FONT_URI);

    let features1 = typeface.supported_features();
    let features2 = typeface.supported_features();

    assert!(std::ptr::eq(features1, features2));
}

#[test]
fn try_get_glyph_advance_should_return_false_for_invalid_glyph_id() {
    let typeface = load(INTER_FONT_URI);

    assert!(typeface.try_get_horizontal_glyph_advance(u16::MAX).is_none());
}

#[test]
fn try_get_glyph_metrics_should_return_false_for_invalid_glyph_id() {
    let typeface = load(INTER_FONT_URI);

    let result = typeface.try_get_glyph_metrics(u16::MAX);

    assert!(result.is_none());
    assert_eq!(GlyphMetrics::default(), result.unwrap_or_default());
}

#[test]
fn try_get_glyph_metrics_should_return_valid_metrics() {
    let typeface = load(INTER_FONT_URI);

    let map = typeface.character_to_glyph_map();
    assert!(map.contains_glyph('A' as i32));

    let glyph_index = map.get_glyph('A' as i32);
    let result = typeface.try_get_glyph_metrics(glyph_index);

    assert!(result.is_some());
    assert!(result.unwrap().width > 0);
}

#[test]
fn try_get_glyph_metrics_width_is_ink_box_not_advance() {
    let typeface = load(INTER_FONT_URI);

    let glyph_index = typeface.character_to_glyph_map().get_glyph('A' as i32);

    let metrics = typeface.try_get_glyph_metrics(glyph_index);
    assert!(metrics.is_some());
    let advance = typeface.try_get_horizontal_glyph_advance(glyph_index);
    assert!(advance.is_some());
    let (metrics, advance) = (metrics.unwrap(), advance.unwrap());

    // The advance belongs on AdvanceWidth...
    assert_eq!(advance, metrics.advance_width);

    // ...and Width is the ink bounding-box width, a distinct value.
    assert!(metrics.width > 0);
    assert_ne!(metrics.advance_width, metrics.width);
}

#[test]
fn try_get_glyph_metrics_empty_glyph_has_advance_but_no_ink() {
    let typeface = load(INTER_FONT_URI);

    let space_glyph = typeface.character_to_glyph_map().get_glyph(' ' as i32);

    let metrics = typeface.try_get_glyph_metrics(space_glyph);
    assert!(metrics.is_some());
    let metrics = metrics.unwrap();

    // The space glyph has a horizontal advance but no ink.
    assert!(metrics.advance_width > 0);
    assert_eq!(0u16, metrics.width);
    assert_eq!(0u16, metrics.height);
}

#[test]
fn try_get_glyph_metrics_batch_matches_single() {
    let typeface = load(INTER_FONT_URI);

    let map = typeface.character_to_glyph_map();
    let glyph_indices =
        [map.get_glyph('A' as i32), map.get_glyph('B' as i32), map.get_glyph('g' as i32), map.get_glyph(' ' as i32)];

    let mut batch = [GlyphMetrics::default(); 4];
    assert!(typeface.try_get_glyph_metrics_batch(&glyph_indices, &mut batch));

    for i in 0..glyph_indices.len() {
        let single = typeface.try_get_glyph_metrics(glyph_indices[i]);
        assert!(single.is_some());

        // GlyphMetrics is a record struct, so this is structural equality.
        assert_eq!(single.unwrap(), batch[i]);
    }
}

#[test]
fn try_get_vertical_glyph_advance_returns_false_for_latin_font() {
    let typeface = load(INTER_FONT_URI);

    let glyph_index = typeface.character_to_glyph_map().get_glyph('A' as i32);

    // Latin fonts typically carry no vmtx table — the call returns false and
    // leaves the advance at zero.
    let advance = typeface.try_get_vertical_glyph_advance(glyph_index);
    assert!(advance.is_none());
    assert_eq!(0u16, advance.unwrap_or_default());
}

#[test]
fn try_get_vertical_glyph_advances_batch_returns_false_for_latin_font() {
    let typeface = load(INTER_FONT_URI);

    let map = typeface.character_to_glyph_map();
    let glyph_indices = [map.get_glyph('A' as i32), map.get_glyph('B' as i32), map.get_glyph('g' as i32)];
    let mut advances = [0u16; 3];

    assert!(!typeface.try_get_vertical_glyph_advances(&glyph_indices, &mut advances));
}

#[test]
fn try_get_vertical_glyph_advance_returns_true_for_cjk_font() {
    let typeface = load(MI_SANS_FONT_URI);

    // CJK glyph: U+4E2D ("中"). MiSans is a CJK font with a vmtx table.
    let glyph_index = typeface.character_to_glyph_map().get_glyph('中' as i32);

    let advance = typeface.try_get_vertical_glyph_advance(glyph_index);
    assert!(advance.is_some());
    assert!(advance.unwrap() > 0, "Expected a positive vertical advance for a CJK glyph.");
}

#[test]
fn try_get_vertical_glyph_advances_batch_matches_single_for_cjk_font() {
    let typeface = load(MI_SANS_FONT_URI);

    let map = typeface.character_to_glyph_map();
    let glyph_indices = [
        map.get_glyph('中' as i32),
        map.get_glyph('文' as i32),
        map.get_glyph('字' as i32),
        map.get_glyph(' ' as i32),
    ];

    let mut batch = [0u16; 4];
    assert!(typeface.try_get_vertical_glyph_advances(&glyph_indices, &mut batch));

    for i in 0..glyph_indices.len() {
        let single = typeface.try_get_vertical_glyph_advance(glyph_indices[i]);
        assert!(single.is_some());
        assert_eq!(single.unwrap(), batch[i]);
    }
}

#[test]
fn should_have_valid_platform_typeface() {
    let asset_loader = standard_asset_loader();

    let mut stream = open(&asset_loader, INTER_FONT_URI);

    let platform_typeface = CustomPlatformTypeface::new(&mut stream);
    let typeface = glyph_typeface(platform_typeface.clone(), FontSimulations::None);

    assert!(std::ptr::addr_eq(Rc::as_ptr(typeface.platform_typeface()), Rc::as_ptr(&platform_typeface)));
}

#[test]
fn should_dispose_properly() {
    let typeface = load(INTER_FONT_URI);

    typeface.dispose();

    // Should not throw on double dispose
    typeface.dispose();
}

#[test]
fn character_to_glyph_map_should_have_different_glyphs_for_different_characters() {
    let typeface = load(INTER_FONT_URI);

    let map = typeface.character_to_glyph_map();

    assert!(map.contains_glyph('A' as i32));
    assert!(map.contains_glyph('B' as i32));

    let glyph_a = map.get_glyph('A' as i32);
    let glyph_b = map.get_glyph('B' as i32);

    assert_ne!(glyph_a, glyph_b);
}

#[test]
fn character_to_glyph_map_with_format13_should_have_same_glyph_for_different_characters() {
    let typeface = load(BLANK_FONT_URI);

    let map = typeface.character_to_glyph_map();

    assert!(map.contains_glyph('A' as i32));
    assert!(map.contains_glyph('B' as i32));

    let glyph_a = map.get_glyph('A' as i32);
    let glyph_b = map.get_glyph('B' as i32);

    assert_eq!(glyph_a, glyph_b);
}

#[test]
fn font_metrics_line_spacing_should_be_calculated_correctly() {
    let typeface = load(INTER_FONT_URI);

    let metrics = typeface.metrics();

    let expected_line_spacing = metrics.descent - metrics.ascent + metrics.line_gap;

    assert_eq!(expected_line_spacing, metrics.line_spacing());
}

#[test]
fn should_support_multiple_characters_in_character_to_glyph_map() {
    let typeface = load(INTER_FONT_URI);

    let map = typeface.character_to_glyph_map();

    let test_characters = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];

    for ch in test_characters {
        assert!(map.contains_glyph(ch as i32), "Character '{ch}' not found in glyph map");
    }
}

#[test]
fn as_read_only_dictionary_returns_non_null_dictionary() {
    // C# `Assert.NotNull`: the dictionary is a value, never null.
    let _dict = load_inter_character_to_glyph_map().as_read_only_dictionary();
}

#[test]
fn as_read_only_dictionary_contains_key_matches_underlying_map() {
    let map = load_inter_character_to_glyph_map();
    let dict = map.as_read_only_dictionary();

    // 'A' is in Inter.
    assert!(map.contains_glyph('A' as i32));
    assert!(dict.contains_key('A' as i32));

    // U+10FFFD is the last code point of the supplementary private-use
    // Plane 16 — Inter does not map it and a Format 4 cmap cannot.
    assert!(!map.contains_glyph(0x10FFFD));
    assert!(!dict.contains_key(0x10FFFD));
}

#[test]
fn as_read_only_dictionary_indexer_returns_same_glyph_id_as_map() {
    for code_point in ['A' as i32, 'z' as i32, '0' as i32, ' ' as i32] {
        let map = load_inter_character_to_glyph_map();
        let dict = map.as_read_only_dictionary();

        assert_eq!(map.get_glyph(code_point), dict.get(code_point));
    }
}

#[test]
#[should_panic]
fn as_read_only_dictionary_indexer_throws_for_unmapped_code_point() {
    let dict = load_inter_character_to_glyph_map().as_read_only_dictionary();

    let _ = dict.get(0x10FFFD);
}

#[test]
fn as_read_only_dictionary_try_get_value_returns_true_with_glyph_id_for_known_code_point() {
    let map = load_inter_character_to_glyph_map();
    let dict = map.as_read_only_dictionary();

    let glyph_index = dict.try_get_value('A' as i32);
    assert!(glyph_index.is_some());
    let glyph_index = glyph_index.unwrap();
    assert_eq!(map.get_glyph('A' as i32), glyph_index);
    assert_ne!(0, glyph_index);
}

#[test]
fn as_read_only_dictionary_try_get_value_returns_false_for_unmapped_code_point() {
    let dict = load_inter_character_to_glyph_map().as_read_only_dictionary();

    let glyph_index = dict.try_get_value(0x10FFFD);
    assert!(glyph_index.is_none());
    assert_eq!(0u16, glyph_index.unwrap_or_default());
}

#[test]
fn as_read_only_dictionary_count_is_positive_and_matches_enumeration() {
    let dict = load_inter_character_to_glyph_map().as_read_only_dictionary();

    assert!(dict.count() > 0);

    let mut enumerated = 0;
    for _ in dict.iter() {
        enumerated += 1;
    }

    assert_eq!(dict.count(), enumerated);
}

#[test]
fn as_read_only_dictionary_enumeration_yields_pairs_that_round_trip_through_the_map() {
    let map = load_inter_character_to_glyph_map();
    let dict = map.as_read_only_dictionary();

    let mut checked_pairs = 0;
    for (key, value) in dict.iter() {
        // Every (key, value) the dictionary yields must agree with the
        // underlying map. The dictionary is a view, not a snapshot.
        assert_eq!(map.get_glyph(key), value);
        assert!(map.contains_glyph(key));

        checked_pairs += 1;
        if checked_pairs >= 500 {
            // Inter has thousands of mappings; sampling the first 500
            // is enough to exercise the enumerator without making the
            // test prohibitively slow.
            break;
        }
    }

    assert!(checked_pairs > 0);
}

#[test]
fn as_read_only_dictionary_keys_match_dictionary_enumeration_keys() {
    let dict = load_inter_character_to_glyph_map().as_read_only_dictionary();

    let mut keys_from_enumeration = HashSet::new();
    let mut pairs_keys_from_enumeration = HashSet::new();

    for key in dict.keys() {
        keys_from_enumeration.insert(key);
        if keys_from_enumeration.len() >= 500 {
            break;
        }
    }

    for (key, _) in dict.iter() {
        pairs_keys_from_enumeration.insert(key);
        if pairs_keys_from_enumeration.len() >= 500 {
            break;
        }
    }

    assert!(!keys_from_enumeration.is_empty());
    assert_eq!(pairs_keys_from_enumeration, keys_from_enumeration);
}

#[test]
fn as_read_only_dictionary_returns_a_fresh_view_that_is_functionally_equivalent() {
    let map = load_inter_character_to_glyph_map();

    let first = map.as_read_only_dictionary();
    let second = map.as_read_only_dictionary();

    // The dictionary is a lightweight wrapper that may or may not be
    // the same instance; what matters is that two views of the same
    // map agree on lookups.
    assert!(first.contains_key('A' as i32));
    assert!(second.contains_key('A' as i32));
    assert_eq!(first.get('A' as i32), second.get('A' as i32));
}

fn load_inter_character_to_glyph_map() -> CharacterToGlyphMap {
    let typeface = load(INTER_FONT_URI);
    typeface.character_to_glyph_map().clone()
}

#[test]
fn family_names_should_contain_invariant_culture_entry() {
    let typeface = load(INTER_FONT_URI);

    assert!(
        typeface.family_names().contains_key(&CultureInfo::invariant_culture()) || !typeface.family_names().is_empty()
    );
}

#[test]
fn face_names_should_contain_invariant_culture_entry() {
    let typeface = load(INTER_FONT_URI);

    assert!(typeface.face_names().contains_key(&CultureInfo::invariant_culture()) || !typeface.face_names().is_empty());
}

/// C# `private class CustomPlatformTypeface : IPlatformTypeface`: a platform
/// typeface over the bytes of a stream, family "Custom", without
/// simulations.
struct CustomPlatformTypeface {
    font_memory: UnmanagedFontMemory,
    family_name: String,
}

impl CustomPlatformTypeface {
    fn new(stream: &mut dyn Read) -> Rc<dyn IPlatformTypeface> {
        Self::with_family(stream, "Custom")
    }

    fn with_family(stream: &mut dyn Read, font_family: &str) -> Rc<dyn IPlatformTypeface> {
        Rc::new(CustomPlatformTypeface {
            font_memory: UnmanagedFontMemory::load_from_stream(stream).unwrap(),
            family_name: font_family.to_string(),
        })
    }
}

impl IFontMemory for CustomPlatformTypeface {
    fn try_get_table(&self, tag: OpenTypeTag) -> Option<ReadOnlyMemory<u8>> {
        self.font_memory.try_get_table(tag)
    }

    fn dispose(&self) {
        self.font_memory.dispose();
    }
}

impl IPlatformTypeface for CustomPlatformTypeface {
    fn family_name(&self) -> String {
        self.family_name.clone()
    }

    fn weight(&self) -> FontWeight {
        FontWeight::Normal
    }

    fn style(&self) -> FontStyle {
        FontStyle::Normal
    }

    fn stretch(&self) -> FontStretch {
        FontStretch::Normal
    }

    fn font_simulations(&self) -> FontSimulations {
        FontSimulations::None
    }

    fn try_get_stream(&self) -> Option<Box<dyn Read>> {
        // Upstream pins the font memory and hands out a stream over it; here
        // the stream reads a copy of the bytes.
        Some(Box::new(Cursor::new(self.font_memory.memory().span().to_vec())))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// ── Not from upstream ──
//
// Tests of the glyph typeface on fonts built in code: font identity, coverage
// and scripts, and malformed and missing tables, which the upstream suite does
// not cover.

fn create(simulations: FontSimulations) -> Rc<GlyphTypeface> {
    GlyphTypeface::new(test_fonts::inter_regular().build(), simulations).unwrap()
}

fn try_create(font: &SyntheticFont) -> Option<Rc<GlyphTypeface>> {
    let platform_typeface = TestPlatformTypeface::from_bytes(font.to_bytes(), FontSimulations::None)?;

    GlyphTypeface::try_create(platform_typeface, FontSimulations::None)
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
