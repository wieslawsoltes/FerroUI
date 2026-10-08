//! Port of upstream's `Media/TextFormatting/TextCharactersTests.cs` of the
//! Skia unit tests.
//!
//! Upstream returns the rented run lists in `finally` blocks; here they are
//! returned once the assertions have run.

use crate::unit_tests::media::text_formatting::SingleBufferTextSource;
use crate::unit_tests::media::CustomFontManagerImpl;
use crate::unit_tests::{mock_platform_render_interface, ASSEMBLY};
use crate::PlatformRenderInterface;
use ferroui_base::media::fonts::{FontCollectionBase, FontCollectionBaseImpl, IFontCollection};
use ferroui_base::media::text_formatting::{
    FormattingObjectPool, GenericTextParagraphProperties, GenericTextRunProperties, TextCharacters, TextFormatter,
    TextFormatterImpl, TextParagraphProperties, TextRunProperties,
};
use ferroui_base::media::{
    BaselineAlignment, FontFamily, FontManager, FontStretch, FontStyle, FontWeight, GlyphTypeface, Typeface,
};
use ferroui_base::platform::IFontManagerImpl;
use ferroui_base::utilities::{CultureInfo, ReadOnlyMemory, Uri, UriKind};
use ferroui_base::FerroLocator;
use ferroui_controls::testing::{UnitTestApplication, UnitTestApplicationScope};
use std::rc::Rc;

// Curated system fonts (see Start): "Noto Mono" is the primary; "DejaVu Sans" is a broad
// fallback that covers Hebrew and a wide range of combining marks. The broad-coverage faces
// bundled for other tests (AdobeBlank2VF, MiSans/NISC CJK) are excluded so that a CJK
// codepoint genuinely has no fallback.
const PRIMARY_FONT: &str = "FerroUI.Skia.UnitTests.Assets.NotoMono-Regular.ttf";
const FALLBACK_FONT: &str = "FerroUI.Skia.UnitTests.Fonts.DejaVuSans.ttf";

// A second broad fallback that covers Latin plus a range of combining marks but has no glyph
// for U+FE0F - unlike DejaVu Sans, which maps the variation selector and would therefore hide
// the default-ignorable bug.
const NO_VARIATION_SELECTOR_FALLBACK_FONT: &str = "FerroUI.Skia.UnitTests.Assets.NotoSans-Italic.ttf";

// Tiny zh/ja regional subsets (a few glyphs each) of the Google Fonts Noto Sans SC / JP, with
// distinct OS/2 codepage bits and localized family names so the culture-aware fallback scorer
// can tell them apart. Both cover U+4E2D (中); only the JP subset covers U+3042 (あ).
const NOTO_SANS_SC_FONT: &str = "FerroUI.Skia.UnitTests.Fonts.NotoSansSC-Subset.ttf";
const NOTO_SANS_JP_FONT: &str = "FerroUI.Skia.UnitTests.Fonts.NotoSansJP-Subset.ttf";

// A colour emoji font, of the kind every platform ships: it covers the emoji block and, like
// practically every font, U+0020 - at an advance of its own that is not the primary's.
const EMOJI_FONT: &str = "FerroUI.Skia.UnitTests.Assets.TwitterColorEmoji-SVGinOT.ttf";

// U+1F642 🙂 — covered by the emoji font only.
const EMOJI_CODEPOINT: i32 = 0x1F642;

// U+4E2D 中 — a CJK ideograph covered by neither curated font, and with no platform fallback,
// so it has no match at all.
const NO_MATCH_CODEPOINT: i32 = 0x4E2D;

// U+05D0 Hebrew aleph — covered by DejaVu Sans but not Noto Mono, so it resolves to a fallback.
const FALLBACK_CODEPOINT: i32 = 0x05D0;

// F2 — a cluster that has no home (NoMatchCodepoint) immediately followed by one that does
// (FallbackCodepoint). This used to make the .notdef recovery loop swallow the renderable
// cluster into the tofu run.
#[test]
fn get_shapeable_characters_does_not_swallow_fallbackable_cluster_after_unmatchable_one() {
    let _scope = start(&[PRIMARY_FONT, FALLBACK_FONT]);

    let font_manager = FontManager::current();

    let default_properties: Rc<dyn TextRunProperties> =
        Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let default_glyph_typeface = default_properties.cached_glyph_typeface();
    let default_font_family = default_properties.typeface().font_family().clone();

    // Preconditions: the primary covers neither codepoint, the first has no fallback, the
    // second does.
    assert!(!has_glyph(&default_glyph_typeface, NO_MATCH_CODEPOINT));
    assert!(!has_glyph(&default_glyph_typeface, FALLBACK_CODEPOINT));

    assert!(match_character(&font_manager, NO_MATCH_CODEPOINT, Some(&default_font_family), None).is_none());
    assert!(match_character(&font_manager, FALLBACK_CODEPOINT, Some(&default_font_family), None).is_some());

    let text = ReadOnlyMemory::from_str(&format!("{}{}", from_utf32(NO_MATCH_CODEPOINT), from_utf32(FALLBACK_CODEPOINT)));

    let text_characters = TextCharacters::new(text.clone(), default_properties.clone());

    let mut results = FormattingObjectPool::instance().text_run_lists.rent();

    let mut previous_properties: Option<Rc<dyn TextRunProperties>> = None;

    text_characters.get_shapeable_characters(text, 0, &font_manager, &mut previous_properties, &mut results);

    // Before the fix this was a SINGLE coalesced .notdef run spanning both codepoints
    // with the primary typeface — the Hebrew cluster was rendered as tofu even though a
    // fallback exists. The recovery loop now stops at the fallbackable cluster.
    assert_eq!(2, results.len());

    // First run: the genuinely unmatchable cluster, left with the primary (tofu) typeface.
    assert_eq!(1, results[0].length());
    assert_eq!(default_properties.typeface(), results[0].properties().unwrap().typeface());

    // Second run: the Hebrew cluster, handed to a fallback that actually covers it.
    assert_eq!(1, results[1].length());
    assert_ne!(default_properties.typeface(), results[1].properties().unwrap().typeface());
    assert!(has_glyph(&results[1].properties().unwrap().cached_glyph_typeface(), FALLBACK_CODEPOINT));

    FormattingObjectPool::instance().text_run_lists.return_list(results);
}

// F1 — a base+combining-mark cluster where the primary font has the base but not the mark, and
// a fallback covers the whole cluster. The whole cluster must be handed to that fallback rather
// than left on the primary (which would drop the mark).
#[test]
fn get_shapeable_characters_prefers_a_fallback_that_covers_the_whole_cluster_including_marks() {
    let _scope = start(&[PRIMARY_FONT, FALLBACK_FONT]);

    let font_manager = FontManager::current();

    let default_properties: Rc<dyn TextRunProperties> =
        Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let default_glyph_typeface = default_properties.cached_glyph_typeface();
    let default_font_family = default_properties.typeface().font_family().clone();

    const BASE_CODEPOINT: i32 = 'a' as i32;
    assert!(has_glyph(&default_glyph_typeface, BASE_CODEPOINT));

    let mark = find_mark_covered_by_fallback_only(&font_manager, &default_glyph_typeface, &default_font_family, BASE_CODEPOINT);

    let text = ReadOnlyMemory::from_str(&format!("a{}", from_utf32(mark)));

    let text_characters = TextCharacters::new(text.clone(), default_properties.clone());

    let mut results = FormattingObjectPool::instance().text_run_lists.rent();

    let mut previous_properties: Option<Rc<dyn TextRunProperties>> = None;

    text_characters.get_shapeable_characters(text.clone(), 0, &font_manager, &mut previous_properties, &mut results);

    // The base+mark cluster stays whole, on a font that covers the mark. Before the fix
    // it was left on the primary (which has the base but not the mark), dropping the mark.
    assert!(!results.is_empty());

    let first_run = &results[0];

    assert_eq!(text.len() as i32, first_run.length());
    assert_ne!(default_properties.typeface(), first_run.properties().unwrap().typeface());
    assert!(
        has_glyph(&first_run.properties().unwrap().cached_glyph_typeface(), mark),
        "The cluster's run uses a font that does not cover the combining mark."
    );

    FormattingObjectPool::instance().text_run_lists.return_list(results);
}

// A default ignorable codepoint inside a cluster must not be treated as content the font has to
// cover. U+FE0F is a nonspacing mark by general category, so it used to be demanded from every
// candidate font; no text font maps it, which made the whole base+VS16+mark cluster unmatchable
// and dropped it back onto the primary font (rendering the mark as .notdef).
#[test]
fn get_shapeable_characters_ignores_default_ignorable_codepoints_when_matching_cluster_coverage() {
    let _scope = start(&[PRIMARY_FONT, NO_VARIATION_SELECTOR_FALLBACK_FONT]);

    let font_manager = FontManager::current();

    let default_properties: Rc<dyn TextRunProperties> =
        Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let default_glyph_typeface = default_properties.cached_glyph_typeface();
    let default_font_family = default_properties.typeface().font_family().clone();

    const BASE_CODEPOINT: i32 = 'a' as i32;
    const VARIATION_SELECTOR16: char = '\u{FE0F}';

    assert!(has_glyph(&default_glyph_typeface, BASE_CODEPOINT));

    // The premise: no font here has a glyph for the variation selector, and none is expected
    // to - it is default ignorable.
    assert!(match_character(&font_manager, VARIATION_SELECTOR16 as i32, Some(&default_font_family), None).is_none());

    let mark = find_mark_covered_by_fallback_only(&font_manager, &default_glyph_typeface, &default_font_family, BASE_CODEPOINT);

    let text = ReadOnlyMemory::from_str(&format!("a{VARIATION_SELECTOR16}{}", from_utf32(mark)));

    let text_characters = TextCharacters::new(text.clone(), default_properties.clone());

    let mut results = FormattingObjectPool::instance().text_run_lists.rent();

    let mut previous_properties: Option<Rc<dyn TextRunProperties>> = None;

    text_characters.get_shapeable_characters(text.clone(), 0, &font_manager, &mut previous_properties, &mut results);

    assert!(!results.is_empty());

    let first_run = &results[0];

    assert_eq!(text.len() as i32, first_run.length());
    assert!(
        has_glyph(&first_run.properties().unwrap().cached_glyph_typeface(), mark),
        "The cluster's run uses a font that does not cover the combining mark."
    );

    FormattingObjectPool::instance().text_run_lists.return_list(results);
}

// Probes for a combining mark the primary font lacks but a fallback covers together with the
// base. Probing keeps the tests robust to the exact coverage of the embedded fonts.
fn find_mark_covered_by_fallback_only(
    font_manager: &FontManager,
    primary: &GlyphTypeface,
    primary_font_family: &FontFamily,
    base_codepoint: i32,
) -> i32 {
    for &candidate in COMBINING_MARK_CANDIDATES {
        if has_glyph(primary, candidate) {
            continue; // primary already covers it - not a useful probe
        }

        if let Some(mark_typeface) = match_character(font_manager, candidate, Some(primary_font_family), None) {
            if let Some(mark_glyph_typeface) = font_manager.try_get_glyph_typeface(&mark_typeface) {
                if has_glyph(&mark_glyph_typeface, base_codepoint) {
                    return candidate;
                }
            }
        }
    }

    panic!("No combining mark found that the primary font lacks but a fallback covers together with the base.");
}

// F5 — NUL characters are replaced with non-breaking WORD JOINER (U+2060), not ZERO WIDTH
// SPACE (U+200B), which would introduce a line-break opportunity NUL never had.
#[test]
fn get_shapeable_characters_replaces_null_characters_with_non_breaking_word_joiners() {
    let _scope = start(&[PRIMARY_FONT, FALLBACK_FONT]);

    let font_manager = FontManager::current();
    let default_properties: Rc<dyn TextRunProperties> =
        Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));

    let text = ReadOnlyMemory::from_str("\0\0");

    let text_characters = TextCharacters::new(text.clone(), default_properties);

    let mut results = FormattingObjectPool::instance().text_run_lists.rent();

    let mut previous_properties: Option<Rc<dyn TextRunProperties>> = None;

    text_characters.get_shapeable_characters(text.clone(), 0, &font_manager, &mut previous_properties, &mut results);

    assert_eq!(1, results.len());
    assert_eq!(text.len() as i32, results[0].length());

    for &c in results[0].text().span() {
        assert_eq!(0x2060u16, c);
    }

    FormattingObjectPool::instance().text_run_lists.return_list(results);
}

// F4 — the previous run's font is reused as an anti-thrashing bias, but for a locale-sensitive
// script (CJK Han unification) it must not be pinned across a culture change. A zh run's
// Simplified-Chinese font must not carry into a following ja run; the ja run resolves to the
// culture-appropriate Japanese font instead.
#[test]
fn get_shapeable_characters_does_not_pin_previous_region_font_across_a_culture_change() {
    let _scope = start(&[PRIMARY_FONT, NOTO_SANS_SC_FONT, NOTO_SANS_JP_FONT]);

    let font_manager = FontManager::current();
    let ja = CultureInfo::get_culture_info("ja-JP");
    let zh = CultureInfo::get_culture_info("zh-CN");

    // Previous run: the Simplified-Chinese font, resolved for a zh culture.
    let sc_typeface = Typeface::new(FontFamily::new("fonts:SystemFonts#Noto Sans SC"));
    let sc_glyph_typeface = font_manager.try_get_glyph_typeface(&sc_typeface);
    assert!(sc_glyph_typeface.is_some());
    let sc_glyph_typeface = sc_glyph_typeface.unwrap();

    // Current run: a Latin primary that lacks the ideograph, under a ja culture.
    let default_properties: Rc<dyn TextRunProperties> =
        Rc::new(run_properties_with_culture(Typeface::default_typeface(), Some(ja.clone())));

    const HAN: i32 = 0x4E2D; // 中 (a Han codepoint both regional fonts cover)

    // Preconditions: primary lacks 中; the zh font covers it; and the culture-aware fallback
    // for ja prefers the JP font over the SC font (distinct OS/2 codepage + localized names).
    assert!(!has_glyph(&default_properties.cached_glyph_typeface(), HAN));
    assert!(has_glyph(&sc_glyph_typeface, HAN));
    let ja_match = match_character(&font_manager, HAN, Some(default_properties.typeface().font_family()), Some(&ja));
    assert!(ja_match.is_some());
    let ja_match_glyph_typeface = font_manager.try_get_glyph_typeface(&ja_match.unwrap());
    assert!(ja_match_glyph_typeface.is_some());
    assert_eq!("Noto Sans JP", ja_match_glyph_typeface.unwrap().family_name());

    let text = ReadOnlyMemory::from_str(&from_utf32(HAN));
    let text_characters = TextCharacters::new(text.clone(), default_properties);

    let mut previous_properties: Option<Rc<dyn TextRunProperties>> =
        Some(Rc::new(run_properties_with_culture(sc_typeface, Some(zh))));

    let mut results = FormattingObjectPool::instance().text_run_lists.rent();

    text_characters.get_shapeable_characters(text, 0, &font_manager, &mut previous_properties, &mut results);

    assert_eq!(1, results.len());
    let run_glyph_typeface = font_manager.try_get_glyph_typeface(results[0].properties().unwrap().typeface());
    assert!(run_glyph_typeface.is_some());

    // With the fix, the zh→ja culture change on a locale-sensitive script skips reuse of
    // the previous (SC) font, so the run resolves to the ja-appropriate JP font. Before
    // the fix the SC font was pinned and this was "Noto Sans SC".
    assert_eq!("Noto Sans JP", run_glyph_typeface.unwrap().family_name());

    FormattingObjectPool::instance().text_run_lists.return_list(results);
}

// A fallback run must end where the primary font regains coverage, whitespace included.
// Practically every font maps U+0020, so a run that is extended for as long as the fallback
// has glyphs swallows the space that follows the fallback text and shapes it with the
// fallback's space glyph - which is a full em in most emoji fonts.
#[test]
fn get_shapeable_characters_does_not_absorb_whitespace_into_a_fallback_run() {
    let _scope = start(&[PRIMARY_FONT, FALLBACK_FONT]);

    let font_manager = FontManager::current();

    let default_properties: Rc<dyn TextRunProperties> =
        Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let default_glyph_typeface = default_properties.cached_glyph_typeface();
    let default_font_family = default_properties.typeface().font_family().clone();

    // Preconditions: the primary lacks the Hebrew letter but covers both the space and the
    // letter after it, and the fallback that covers the Hebrew letter maps the space too -
    // which is what lets the fallback run reach past the letter today.
    assert!(!has_glyph(&default_glyph_typeface, FALLBACK_CODEPOINT));
    assert!(has_glyph(&default_glyph_typeface, ' ' as i32));
    assert!(has_glyph(&default_glyph_typeface, 'b' as i32));

    let fallback_typeface = match_character(&font_manager, FALLBACK_CODEPOINT, Some(&default_font_family), None);
    assert!(fallback_typeface.is_some());
    let fallback_typeface = fallback_typeface.unwrap();
    let fallback_glyph_typeface = font_manager.try_get_glyph_typeface(&fallback_typeface);
    assert!(fallback_glyph_typeface.is_some());
    assert!(has_glyph(&fallback_glyph_typeface.unwrap(), ' ' as i32));

    let text = ReadOnlyMemory::from_str(&format!("{} b", from_utf32(FALLBACK_CODEPOINT)));

    let text_characters = TextCharacters::new(text.clone(), default_properties.clone());

    let mut results = FormattingObjectPool::instance().text_run_lists.rent();

    let mut previous_properties: Option<Rc<dyn TextRunProperties>> = None;

    text_characters.get_shapeable_characters(text, 0, &font_manager, &mut previous_properties, &mut results);

    assert_eq!(2, results.len());

    // The fallback run covers the Hebrew letter only. Before the fix it was 2 characters
    // long: the space was pulled into the fallback run and rendered with its metrics.
    assert_eq!(1, results[0].length());
    assert_eq!(&fallback_typeface, results[0].properties().unwrap().typeface());

    // The space returns to the primary along with the rest of the text.
    assert_eq!(2, results[1].length());
    assert_eq!(default_properties.typeface(), results[1].properties().unwrap().typeface());

    FormattingObjectPool::instance().text_run_lists.return_list(results);
}

// The user-visible half of the same defect: the absorbed space is measured with the fallback
// font, so a space typed after an emoji has a different advance than the same space elsewhere
// in the line - a full em with the platform emoji fonts, and a narrower space with the emoji
// font bundled here. Either way it is not the primary's.
// Upstream issue 14011.
#[test]
fn format_line_keeps_a_space_after_a_fallback_run_at_the_primary_width() {
    let _scope = start(&[PRIMARY_FONT, EMOJI_FONT]);

    let font_manager = FontManager::current();

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let default_glyph_typeface = default_properties.cached_glyph_typeface();

    assert!(!has_glyph(&default_glyph_typeface, EMOJI_CODEPOINT));

    let emoji_typeface =
        match_character(&font_manager, EMOJI_CODEPOINT, Some(default_properties.typeface().font_family()), None);
    assert!(emoji_typeface.is_some());
    let emoji_glyph_typeface = font_manager.try_get_glyph_typeface(&emoji_typeface.unwrap());
    assert!(emoji_glyph_typeface.is_some());
    let emoji_glyph_typeface = emoji_glyph_typeface.unwrap();

    // The whole point of the test: the two fonts disagree about how wide a space is, so
    // whichever font shapes it is directly observable in the line width.
    assert_not_equal_precision(
        space_advance_in_em(&default_glyph_typeface),
        space_advance_in_em(&emoji_glyph_typeface),
        3,
    );

    let formatter = TextFormatterImpl::new();

    let width = |text: &str| -> f64 {
        let paragraph_properties: Rc<dyn TextParagraphProperties> =
            Rc::new(GenericTextParagraphProperties::new(default_properties.clone()));

        let text_line = formatter.format_line(
            &SingleBufferTextSource::new(text, default_properties.clone(), false),
            0,
            f64::INFINITY,
            &paragraph_properties,
            None,
        );

        assert!(text_line.is_some());

        text_line.unwrap().width_including_trailing_whitespace()
    };

    let emoji = from_utf32(EMOJI_CODEPOINT);

    // Isolate the space by differencing, so the surrounding glyphs' advances cancel out.
    let plain_space = width("a b") - width("ab");
    let space_after_fallback = width(&format!("{emoji} b")) - width(&format!("{emoji}b"));

    assert_equal_precision(plain_space, space_after_fallback, 3);
}

fn space_advance_in_em(glyph_typeface: &GlyphTypeface) -> f64 {
    let glyph = glyph_typeface.character_to_glyph_map().try_get_glyph(' ' as i32);
    assert!(glyph.is_some());
    let advance = glyph_typeface.try_get_horizontal_glyph_advance(glyph.unwrap());
    assert!(advance.is_some());

    advance.unwrap() as f64 / glyph_typeface.metrics().design_em_height as f64
}

// The previous run's font is reused as an anti-thrashing bias. A space belongs to the primary
// font, so it forms a run of its own between two fallback words - and that run must not become
// the bias, or each word re-runs the fallback search and the two can land on different fonts.
#[test]
fn get_shapeable_characters_keeps_the_previous_fallback_across_a_space() {
    let _scope = start(&[PRIMARY_FONT, NOTO_SANS_SC_FONT, NOTO_SANS_JP_FONT]);

    let font_manager = FontManager::current();

    let default_properties: Rc<dyn TextRunProperties> =
        Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));

    // The previous run resolved to the Simplified-Chinese font.
    let sc_typeface = Typeface::new(FontFamily::new("fonts:SystemFonts#Noto Sans SC"));
    let sc_glyph_typeface = font_manager.try_get_glyph_typeface(&sc_typeface);
    assert!(sc_glyph_typeface.is_some());
    let sc_glyph_typeface = sc_glyph_typeface.unwrap();

    const HAN: i32 = 0x4E2D; // 中, covered by both regional fonts.

    // Preconditions: the primary covers the space but not the ideograph, the previous font
    // covers the ideograph, and a fresh search for it would pick the *other* font - so the
    // font of the second run tells us whether the bias survived the space.
    assert!(has_glyph(&default_properties.cached_glyph_typeface(), ' ' as i32));
    assert!(!has_glyph(&default_properties.cached_glyph_typeface(), HAN));
    assert!(has_glyph(&sc_glyph_typeface, HAN));

    let fresh_match = match_character(&font_manager, HAN, Some(default_properties.typeface().font_family()), None);
    assert!(fresh_match.is_some());
    let fresh_glyph_typeface = font_manager.try_get_glyph_typeface(&fresh_match.unwrap());
    assert!(fresh_glyph_typeface.is_some());
    assert_eq!("Noto Sans JP", fresh_glyph_typeface.unwrap().family_name());

    let text = ReadOnlyMemory::from_str(&format!(" {}", from_utf32(HAN)));

    let text_characters = TextCharacters::new(text.clone(), default_properties.clone());

    let mut results = FormattingObjectPool::instance().text_run_lists.rent();

    let mut previous_properties: Option<Rc<dyn TextRunProperties>> =
        Some(Rc::new(GenericTextRunProperties::new(sc_typeface)));

    text_characters.get_shapeable_characters(text, 0, &font_manager, &mut previous_properties, &mut results);

    assert_eq!(2, results.len());

    assert_eq!(1, results[0].length());
    assert_eq!(default_properties.typeface(), results[0].properties().unwrap().typeface());

    let run_glyph_typeface = font_manager.try_get_glyph_typeface(results[1].properties().unwrap().typeface());
    assert!(run_glyph_typeface.is_some());
    assert_eq!("Noto Sans SC", run_glyph_typeface.unwrap().family_name());

    FormattingObjectPool::instance().text_run_lists.return_list(results);
}

// Only spacing whitespace (Zs) returns to the default typeface. Codepoint.IsWhiteSpace also
// covers control and format codepoints - including the default-ignorable bidi controls, which
// many fonts map. A default typeface that cannot shape the script must not pull a
// right-to-left mark out of the fallback run just because its cmap has it: the mark renders
// nothing either way, and splitting there cuts the run for no reason.
#[test]
fn try_get_shapeable_length_does_not_reclaim_a_bidi_control_as_whitespace() {
    let _scope = start(&[PRIMARY_FONT, FALLBACK_FONT]);

    // DejaVu Sans plays the default: its cmap has the Arabic letter, the right-to-left
    // mark and the space, but the test probes the tier where it cannot shape Arabic.
    // Cascadia Code plays the probed fallback; it has the letter and needs no glyph for
    // the default-ignorable mark.
    let default_glyph_typeface = Typeface::new(
        FontFamily::parse("resm:FerroUI.Skia.UnitTests.Fonts?assembly=ferroui-skia#DejaVu Sans").unwrap(),
    )
    .glyph_typeface();
    let probed_glyph_typeface = Typeface::new(
        FontFamily::parse("resm:FerroUI.Skia.UnitTests.Fonts?assembly=ferroui-skia#Cascadia Code").unwrap(),
    )
    .glyph_typeface();

    const ALEF: i32 = 0x0627;
    const RIGHT_TO_LEFT_MARK: i32 = 0x200F;

    assert!(has_glyph(&probed_glyph_typeface, ALEF));
    assert!(has_glyph(&default_glyph_typeface, RIGHT_TO_LEFT_MARK));
    assert!(has_glyph(&default_glyph_typeface, ' ' as i32));

    // Letter, mark, letter, then a space: the mark stays inside the fallback run, the
    // space still returns to the default.
    let text: Vec<u16> = "\u{0627}\u{200F}\u{0627} z".encode_utf16().collect();

    let length = TextCharacters::try_get_shapeable_length(
        &text,
        &probed_glyph_typeface,
        Some(&default_glyph_typeface),
        false,
        true,
    );

    assert!(length.is_some());

    assert_eq!(3, length.unwrap());
}

// A spread of combining marks (all grapheme-cluster Extend) likely present in a broad fallback
// font but absent from a minimal monospace primary. The F1 test picks the first workable one.
const COMBINING_MARK_CANDIDATES: &[i32] = &[
    0x0316, 0x0317, 0x031C, 0x0323, 0x032E, 0x0333, 0x0359, 0x035C, 0x0360, 0x0361, 0x0362, 0x0363, 0x036F, 0x0488,
    0x0489, 0x1DC0, 0x1DC1, 0x20DD, 0x20E0,
];

fn start(font_resource_names: &[&str]) -> UnitTestApplicationScope {
    let disposable = UnitTestApplication::start(
        mock_platform_render_interface().with_render_interface(Rc::new(PlatformRenderInterface::default())),
    );

    let font_manager_impl: Rc<dyn IFontManagerImpl> = Rc::new(CustomFontManagerImpl::new());

    FerroLocator::current_mutable().bind::<dyn IFontManagerImpl>().to_constant(font_manager_impl.clone());

    let font_manager = FontManager::new(font_manager_impl);

    FerroLocator::current_mutable().bind::<FontManager>().to_constant(font_manager.clone());

    // Register a curated system collection holding only the fonts each test needs. This excludes
    // the broad-coverage fonts bundled for other tests, so coverage is exactly the requested set.
    font_manager
        .add_font_collection(Rc::new(CuratedSystemFontCollection::new(font_resource_names)) as Rc<dyn IFontCollection>);

    disposable
}

struct CuratedSystemFontCollection {
    base: FontCollectionBase,
}

impl CuratedSystemFontCollection {
    fn new(font_resource_names: &[&str]) -> Self {
        let collection = Self { base: FontCollectionBase::new() };

        for name in font_resource_names {
            FontCollectionBase::try_add_font_source(
                &collection,
                &Uri::new(&format!("resm:{name}?assembly={ASSEMBLY}"), UriKind::Absolute).unwrap(),
            );
        }

        collection
    }
}

impl FontCollectionBaseImpl for CuratedSystemFontCollection {
    fn base(this: &Self) -> &FontCollectionBase {
        &this.base
    }

    fn key(_this: &Self) -> Uri {
        FontManager::system_fonts_key()
    }
}

/// `glyphTypeface.CharacterToGlyphMap.TryGetGlyph(codepoint, out _)`.
fn has_glyph(glyph_typeface: &GlyphTypeface, codepoint: i32) -> bool {
    glyph_typeface.character_to_glyph_map().try_get_glyph(codepoint).is_some()
}

/// `fontManager.TryMatchCharacter(codepoint, Normal, Normal, Normal, fontFamily, culture, out var typeface)`.
fn match_character(
    font_manager: &FontManager,
    codepoint: i32,
    font_family: Option<&FontFamily>,
    culture: Option<&CultureInfo>,
) -> Option<Typeface> {
    font_manager.try_match_character(
        codepoint,
        FontStyle::Normal,
        FontWeight::Normal,
        FontStretch::Normal,
        font_family,
        culture,
    )
}

/// `char.ConvertFromUtf32(codepoint)`.
fn from_utf32(codepoint: i32) -> String {
    char::from_u32(codepoint as u32).unwrap().to_string()
}

/// `new GenericTextRunProperties(typeface, cultureInfo: culture)`.
fn run_properties_with_culture(typeface: Typeface, culture: Option<CultureInfo>) -> GenericTextRunProperties {
    GenericTextRunProperties::with_all(
        typeface,
        GenericTextRunProperties::DEFAULT_FONT_RENDERING_EM_SIZE,
        None,
        None,
        None,
        BaselineAlignment::Baseline,
        culture,
        None,
    )
}

/// xUnit's `Assert.Equal(double, double, int precision)`.
fn assert_equal_precision(expected: f64, actual: f64, precision: i32) {
    let factor = 10f64.powi(precision);

    assert_eq!(
        (expected * factor).round_ties_even() / factor,
        (actual * factor).round_ties_even() / factor,
        "expected {expected}, actual {actual} (precision {precision})"
    );
}

/// xUnit's `Assert.NotEqual(double, double, int precision)`.
fn assert_not_equal_precision(expected: f64, actual: f64, precision: i32) {
    let factor = 10f64.powi(precision);

    assert_ne!(
        (expected * factor).round_ties_even() / factor,
        (actual * factor).round_ties_even() / factor,
        "expected {expected} and {actual} to differ (precision {precision})"
    );
}
