//! Port of upstream's run splitting / font fallback tests. Upstream curates
//! sets of real fonts with known coverage; here the same coverage is given to
//! synthetic fonts.
//!
//! Not ported: the tests of the script constrained fallback search
//! (`ShapingCapabilityFallbackTests`: a cmap-only primary is upgraded to a
//! shaping capable font). That search is the font manager's, whose port is a
//! separate piece.

use std::rc::Rc;

use crate::media::text_formatting::formatting_object_pool::FormattingObjectPool;
use crate::media::text_formatting::testing::{
    format_line, paragraph_properties, run_properties, run_properties_for, utf16, SingleBufferTextSource, TestFont,
    TextTestScope, GLYPH_ADVANCE,
};
use crate::media::text_formatting::unicode::Script;
use crate::media::text_formatting::{GenericTextRunProperties, TextCharacters, TextRun, TextRunProperties};
use crate::media::{BaselineAlignment, FontManager, FontStretch, FontStyle, FontWeight, TextWrapping, Typeface};
use crate::utilities::CultureInfo;

const PRIMARY: &str = "Test Mono";
const FALLBACK: &str = "Test Fallback";
const SC: &str = "Test Sans SC";
const JP: &str = "Test Sans JP";
const EMOJI: &str = "Test Emoji";

/// U+1F642 — covered by the emoji font only.
const EMOJI_CODEPOINT: i32 = 0x1F642;
/// U+4E2D — a CJK ideograph covered by neither the primary nor the broad fallback.
const NO_MATCH_CODEPOINT: i32 = 0x4E2D;
/// U+05D0 Hebrew aleph — covered by the fallback but not the primary.
const FALLBACK_CODEPOINT: i32 = 0x05D0;
/// A combining mark the fallback covers and the primary lacks.
const MARK: i32 = 0x0323;

/// ASCII only.
fn primary() -> TestFont {
    TestFont::new(PRIMARY).with_ranges(&[(0x20, 0x7E)])
}

/// Latin, combining marks, Hebrew and Arabic; no variation selectors.
fn fallback() -> TestFont {
    TestFont::new(FALLBACK).with_ranges(&[(0x20, 0x7E), (0x300, 0x36F), (0x590, 0x6FF), (0x200E, 0x200F)])
}

fn sans_sc() -> TestFont {
    TestFont::new(SC).with_ranges(&[(0x4E2D, 0x4E2D)]).with_language("zh")
}

fn sans_jp() -> TestFont {
    TestFont::new(JP).with_ranges(&[(0x3042, 0x3042), (0x4E2D, 0x4E2D)]).with_language("ja")
}

fn text_of(codepoints: &[i32]) -> String {
    codepoints.iter().map(|&codepoint| char::from_u32(codepoint as u32).unwrap()).collect()
}

fn match_character(codepoint: i32, culture: Option<&CultureInfo>) -> Option<Typeface> {
    FontManager::current().try_match_character(
        codepoint,
        FontStyle::Normal,
        FontWeight::Normal,
        FontStretch::Normal,
        Some(Typeface::default_typeface().font_family()),
        culture,
    )
}

fn family_of(run: &Rc<dyn TextRun>) -> String {
    run.properties().unwrap().cached_glyph_typeface().family_name().to_owned()
}

fn properties_with_culture(typeface: Typeface, culture: Option<CultureInfo>) -> Rc<dyn TextRunProperties> {
    Rc::new(GenericTextRunProperties::with_all(typeface, 12.0, None, None, None, BaselineAlignment::Baseline, culture, None))
}

/// Runs `get_shapeable_characters` over the whole text and returns the runs.
fn shapeable_characters(
    text: &str,
    default_properties: &Rc<dyn TextRunProperties>,
    mut previous_properties: Option<Rc<dyn TextRunProperties>>,
) -> Vec<Rc<dyn TextRun>> {
    let text = utf16(text);
    let text_characters = TextCharacters::new(text.clone(), default_properties.clone());

    let pool = FormattingObjectPool::instance();

    let mut results = pool.text_run_lists.rent();

    text_characters.get_shapeable_characters(text, 0, &FontManager::current(), &mut previous_properties, &mut results);

    let runs = results.clone();

    pool.text_run_lists.return_list(results);

    runs
}

// A cluster that has no home (the ideograph) immediately followed by one that does (the Hebrew
// letter). The tofu recovery loop must not swallow the renderable cluster into the tofu run.
#[test]
fn get_shapeable_characters_does_not_swallow_fallbackable_cluster_after_unmatchable_one() {
    let _scope = TextTestScope::with_fonts(vec![primary(), fallback()]);

    let default_properties = run_properties(12.0);
    let default_glyph_typeface = default_properties.cached_glyph_typeface();

    // Preconditions: the primary covers neither codepoint, the first has no fallback, the second does.
    assert!(default_glyph_typeface.character_to_glyph_map().try_get_glyph(NO_MATCH_CODEPOINT).is_none());
    assert!(default_glyph_typeface.character_to_glyph_map().try_get_glyph(FALLBACK_CODEPOINT).is_none());
    assert!(match_character(NO_MATCH_CODEPOINT, None).is_none());
    assert!(match_character(FALLBACK_CODEPOINT, None).is_some());

    let results = shapeable_characters(&text_of(&[NO_MATCH_CODEPOINT, FALLBACK_CODEPOINT]), &default_properties, None);

    assert_eq!(results.len(), 2);

    // First run: the genuinely unmatchable cluster, left with the primary (tofu) typeface.
    assert_eq!(results[0].length(), 1);
    assert_eq!(results[0].properties().unwrap().typeface(), default_properties.typeface());

    // Second run: the Hebrew cluster, handed to a fallback that actually covers it.
    assert_eq!(results[1].length(), 1);
    assert_ne!(results[1].properties().unwrap().typeface(), default_properties.typeface());
    assert_eq!(family_of(&results[1]), FALLBACK);
}

// A base+combining-mark cluster where the primary font has the base but not the mark, and
// a fallback covers the whole cluster. The whole cluster must be handed to that fallback rather
// than left on the primary (which would drop the mark).
#[test]
fn get_shapeable_characters_prefers_a_fallback_that_covers_the_whole_cluster_including_marks() {
    let _scope = TextTestScope::with_fonts(vec![primary(), fallback()]);

    let default_properties = run_properties(12.0);

    let text = text_of(&['a' as i32, MARK]);

    let results = shapeable_characters(&text, &default_properties, None);

    assert!(!results.is_empty());

    let first_run = &results[0];

    assert_eq!(first_run.length(), 2);
    assert_ne!(first_run.properties().unwrap().typeface(), default_properties.typeface());
    assert!(
        first_run.properties().unwrap().cached_glyph_typeface().character_to_glyph_map().try_get_glyph(MARK).is_some(),
        "The cluster's run uses a font that does not cover the combining mark."
    );
}

// A default ignorable codepoint inside a cluster must not be treated as content the font has to
// cover.
#[test]
fn get_shapeable_characters_ignores_default_ignorable_codepoints_when_matching_cluster_coverage() {
    let _scope = TextTestScope::with_fonts(vec![primary(), fallback()]);

    let default_properties = run_properties(12.0);

    const VARIATION_SELECTOR_16: i32 = 0xFE0F;

    // The premise: no font here has a glyph for the variation selector.
    assert!(match_character(VARIATION_SELECTOR_16, None).is_none());

    let text = text_of(&['a' as i32, VARIATION_SELECTOR_16, MARK]);

    let results = shapeable_characters(&text, &default_properties, None);

    assert!(!results.is_empty());

    let first_run = &results[0];

    assert_eq!(first_run.length(), 3);
    assert_eq!(family_of(first_run), FALLBACK);
}

// NUL characters are replaced with non-breaking WORD JOINER (U+2060), not ZERO WIDTH
// SPACE (U+200B), which would introduce a line-break opportunity NUL never had.
#[test]
fn get_shapeable_characters_replaces_null_characters_with_non_breaking_word_joiners() {
    let _scope = TextTestScope::with_fonts(vec![primary(), fallback()]);

    let default_properties = run_properties(12.0);

    let results = shapeable_characters("\0\0", &default_properties, None);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].text_span(), [0x2060, 0x2060]);

    // Addition: more NULs than the cached joiner run holds, followed by text.
    let results = shapeable_characters("\0\0\0\0\0\0\0\0\0\0ab", &default_properties, None);

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].text_span(), [0x2060; 10]);
    assert_eq!(results[1].text().to_string_lossy(), "ab");
}

// The previous run's font is reused as an anti-thrashing bias, but for a locale-sensitive
// script (CJK Han unification) it must not be pinned across a culture change.
#[test]
fn get_shapeable_characters_does_not_pin_previous_region_font_across_a_culture_change() {
    let _scope = TextTestScope::with_fonts(vec![primary(), sans_sc(), sans_jp()]);

    let ja = CultureInfo::get_culture_info("ja-JP");
    let zh = CultureInfo::get_culture_info("zh-CN");

    // Previous run: the Simplified-Chinese font, resolved for a zh culture.
    let sc_typeface = Typeface::from_name(SC);

    // Current run: a Latin primary that lacks the ideograph, under a ja culture.
    let default_properties = properties_with_culture(Typeface::default_typeface(), Some(ja.clone()));

    const HAN: i32 = 0x4E2D;

    // Preconditions: the culture-aware fallback for ja prefers the JP font over the SC font.
    assert!(Script::Han == crate::media::text_formatting::unicode::Codepoint::new(HAN as u32).script());
    assert_eq!(match_character(HAN, Some(&ja)).unwrap().font_family().name(), JP);
    assert_eq!(match_character(HAN, Some(&zh)).unwrap().font_family().name(), SC);

    let previous_properties = properties_with_culture(sc_typeface.clone(), Some(zh));

    let results = shapeable_characters(&text_of(&[HAN]), &default_properties, Some(previous_properties));

    assert_eq!(results.len(), 1);

    // The zh→ja culture change on a locale-sensitive script skips reuse of
    // the previous (SC) font, so the run resolves to the ja-appropriate JP font.
    assert_eq!(family_of(&results[0]), JP);

    // Addition: without a culture change the previous font is reused.
    let previous_properties = properties_with_culture(sc_typeface, Some(ja));

    let results = shapeable_characters(&text_of(&[HAN]), &default_properties, Some(previous_properties));

    assert_eq!(family_of(&results[0]), SC);
}

// A fallback run must end where the primary font regains coverage, whitespace included.
#[test]
fn get_shapeable_characters_does_not_absorb_whitespace_into_a_fallback_run() {
    let _scope = TextTestScope::with_fonts(vec![primary(), fallback()]);

    let default_properties = run_properties(12.0);

    let fallback_typeface = match_character(FALLBACK_CODEPOINT, None).unwrap();

    // Precondition: the fallback that covers the Hebrew letter maps the space too.
    assert!(fallback_typeface.glyph_typeface().character_to_glyph_map().try_get_glyph(' ' as i32).is_some());

    let text = format!("{} b", text_of(&[FALLBACK_CODEPOINT]));

    let results = shapeable_characters(&text, &default_properties, None);

    assert_eq!(results.len(), 2);

    // The fallback run covers the Hebrew letter only.
    assert_eq!(results[0].length(), 1);
    assert_eq!(results[0].properties().unwrap().typeface(), &fallback_typeface);

    // The space returns to the primary along with the rest of the text.
    assert_eq!(results[1].length(), 2);
    assert_eq!(results[1].properties().unwrap().typeface(), default_properties.typeface());
}

// The user-visible half of the same defect: the absorbed space would be measured with the fallback font.
#[test]
fn format_line_keeps_a_space_after_a_fallback_run_at_the_primary_width() {
    // The emoji font maps the space too, at an advance of its own.
    let _scope = TextTestScope::with_fonts(vec![
        primary(),
        TestFont::new(EMOJI).with_ranges(&[(0x20, 0x20), (0x1F300, 0x1FAFF)]).with_advance(GLYPH_ADVANCE * 2),
    ]);

    let default_properties = run_properties(12.0);

    let width = |text: &str| -> f64 {
        format_line(
            &SingleBufferTextSource::new(text, default_properties.clone()),
            0,
            f64::INFINITY,
            &paragraph_properties(&default_properties, TextWrapping::NoWrap),
            None,
        )
        .unwrap()
        .width_including_trailing_whitespace()
    };

    let emoji = text_of(&[EMOJI_CODEPOINT]);

    // Isolate the space by differencing, so the surrounding glyphs' advances cancel out.
    let plain_space = width("a b") - width("ab");
    let space_after_fallback = width(&format!("{emoji} b")) - width(&format!("{emoji}b"));

    assert_eq!(plain_space, 6.0);
    assert_eq!(space_after_fallback, plain_space);
}

// A space belongs to the primary font, so it forms a run of its own between two fallback
// words - and that run must not become the bias for what follows.
#[test]
fn get_shapeable_characters_keeps_the_previous_fallback_across_a_space() {
    // JP is listed first: a fresh search for the ideograph picks it.
    let _scope = TextTestScope::with_fonts(vec![primary(), sans_jp(), sans_sc()]);

    let default_properties = run_properties(12.0);

    const HAN: i32 = 0x4E2D;

    assert_eq!(match_character(HAN, None).unwrap().font_family().name(), JP);

    // The previous run resolved to the Simplified-Chinese font.
    let previous_properties = run_properties_for(SC, 12.0);

    let text = format!(" {}", text_of(&[HAN]));

    let results = shapeable_characters(&text, &default_properties, Some(previous_properties));

    assert_eq!(results.len(), 2);

    assert_eq!(results[0].length(), 1);
    assert_eq!(results[0].properties().unwrap().typeface(), default_properties.typeface());

    assert_eq!(family_of(&results[1]), SC);
}

// Only spacing whitespace (Zs) returns to the default typeface. A default typeface that cannot
// shape the script must not pull a right-to-left mark out of the fallback run just because its
// cmap has it.
#[test]
fn try_get_shapeable_length_does_not_reclaim_a_bidi_control_as_whitespace() {
    let _scope = TextTestScope::with_fonts(vec![
        // The default: its cmap has the Arabic letter, the right-to-left mark and the space.
        fallback(),
        // The probed font: it has the letter and needs no glyph for the default-ignorable mark.
        TestFont::new("Test Arabic").with_ranges(&[(0x600, 0x6FF)]).with_script_tags(&["arab"]),
    ]);

    let default_glyph_typeface = Typeface::default_typeface().glyph_typeface();
    let probed_glyph_typeface = Typeface::from_name("Test Arabic").glyph_typeface();

    const ALEF: i32 = 0x0627;
    const RIGHT_TO_LEFT_MARK: i32 = 0x200F;

    assert!(probed_glyph_typeface.character_to_glyph_map().try_get_glyph(ALEF).is_some());
    assert!(default_glyph_typeface.character_to_glyph_map().try_get_glyph(RIGHT_TO_LEFT_MARK).is_some());
    assert!(default_glyph_typeface.character_to_glyph_map().try_get_glyph(' ' as i32).is_some());

    // Addition: the capability the tiers are built on.
    assert!(probed_glyph_typeface.can_shape_script(Script::Arabic));
    assert!(!default_glyph_typeface.can_shape_script(Script::Arabic));
    assert!(default_glyph_typeface.can_shape_script(Script::Latin));

    // Letter, mark, letter, then a space: the mark stays inside the fallback run, the
    // space still returns to the default.
    let text = utf16("\u{0627}\u{200F}\u{0627} z");

    let length = TextCharacters::try_get_shapeable_length(
        text.span(),
        &probed_glyph_typeface,
        Some(&default_glyph_typeface),
        false,
        true,
    );

    assert_eq!(length, Some(3));

    // Addition: a default that can shape the script takes the first cluster back at once.
    let length =
        TextCharacters::try_get_shapeable_length(text.span(), &probed_glyph_typeface, Some(&default_glyph_typeface), true, true);

    assert_eq!(length, None);
}

// When no shaping-capable font for the script exists, the cmap-only font is kept (the capability
// tier finds nothing, the cmap tier then accepts it).
#[test]
fn cmap_only_complex_script_primary_is_kept_when_no_capable_font_exists() {
    let _scope = TextTestScope::with_fonts(vec![primary(), fallback()]);

    let default_properties = run_properties_for(FALLBACK, 12.0);

    let results = shapeable_characters("\u{0627}", &default_properties, None);

    assert_eq!(results.len(), 1);
    assert_eq!(family_of(&results[0]), FALLBACK);
}

/// Addition: a run changes at a script change even when one font covers both.
#[test]
fn a_run_ends_where_the_script_changes() {
    let _scope = TextTestScope::with_fonts(vec![fallback()]);

    let default_properties = run_properties(12.0);

    let results = shapeable_characters("ab \u{05D0}\u{05D1} cd", &default_properties, None);

    let texts: Vec<String> = results.iter().map(|run| run.text().to_string_lossy()).collect();

    // Common characters (the spaces) stay with the run they follow.
    assert_eq!(texts, ["ab ", "\u{05D0}\u{05D1} ", "cd"]);
}

#[test]
#[should_panic(expected = "Invalid FontRenderingEmSize")]
fn text_characters_reject_a_non_positive_em_size() {
    let _scope = TextTestScope::with_fonts(vec![primary()]);

    TextCharacters::from_str("a", run_properties(0.0));
}
