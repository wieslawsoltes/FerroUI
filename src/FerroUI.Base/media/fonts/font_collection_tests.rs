//! Tests of the font collections (the font collection base, the system and
//! the embedded font collection) on fonts built in code.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use crate::media::fonts::testing::{test_fonts, TestFontBuilder, TestFontManagerImpl, TestFontScope};
use crate::media::fonts::{
    EmbeddedFontCollection, FontCollectionBase, FontCollectionBaseImpl, FontCollectionKey, IFontCollection,
    IFontCollectionBase, SystemFontCollection,
};
use crate::media::{
    FontFallback, FontFamily, FontManager, FontSimulations, FontStretch, FontStyle, FontWeight, GlyphTypeface,
    Typeface, UnicodeRange,
};
use crate::platform::{IAssetLoader, IFontManagerImpl};
use crate::utilities::{CultureInfo, Uri};
use crate::{FerroLocator, LocatorExtensions};

const ARABIC_ALEF: i32 = 0x0627;
const HEBREW_ALEF: i32 = 0x05D0;
/// Not covered by any font of a test collection unless said otherwise.
const CJK_ICHI: i32 = 0x4E00;

fn uri(text: &str) -> Uri {
    Uri::absolute(text).unwrap()
}

fn culture(name: &str) -> CultureInfo {
    CultureInfo::get_culture_info(name)
}

// ── harness ──

#[test]
fn test_font_builder_builds_a_loadable_font() {
    let typeface = TestFontBuilder::new("Test Sans")
        .weight(FontWeight::Bold)
        .style(FontStyle::Italic)
        .stretch(FontStretch::Condensed)
        .codepoints(&[(0x41, 0x5A), (0x1F600, 0x1F60F)])
        .glyph_advance(0x42, 700)
        .typographic_family_name("Test Sans Family")
        .build();

    let glyph_typeface = GlyphTypeface::new(typeface, FontSimulations::None).unwrap();

    assert_eq!(glyph_typeface.family_name(), "Test Sans");
    assert_eq!(glyph_typeface.typographic_family_name(), "Test Sans Family");
    assert_eq!(glyph_typeface.weight(), FontWeight::Bold);
    assert_eq!(glyph_typeface.style(), FontStyle::Italic);
    assert_eq!(glyph_typeface.stretch(), FontStretch::Condensed);
    assert_eq!(glyph_typeface.glyph_count(), 1 + 26 + 16);
    assert_eq!(glyph_typeface.character_to_glyph_map().try_get_glyph(0x41), Some(1));
    assert_eq!(glyph_typeface.character_to_glyph_map().try_get_glyph(0x1F600), Some(27));
    assert_eq!(glyph_typeface.character_to_glyph_map().try_get_glyph(0x61), None);
    assert_eq!(glyph_typeface.try_get_horizontal_glyph_advance(1), Some(600));
    assert_eq!(glyph_typeface.try_get_horizontal_glyph_advance(2), Some(700));
    assert_eq!(glyph_typeface.metrics().design_em_height, 1000);
    assert_eq!(glyph_typeface.metrics().ascent, -800);
    assert_eq!(glyph_typeface.metrics().descent, 200);
    assert!(!glyph_typeface.is_last_resort());
}

#[test]
fn headless_scope_resolves_the_default_family() {
    let scope = TestFontScope::start();

    let font_manager = FontManager::current();

    assert_eq!(font_manager.default_font_family().name(), "Noto Mono");
    assert_eq!(font_manager.system_fonts().count(), 3);
    assert_eq!(scope.font_manager_impl().fonts().len(), 3);

    let glyph_typeface = Typeface::from_name("Noto Sans").glyph_typeface();

    assert_eq!(glyph_typeface.family_name(), "Noto Sans");
    assert_eq!(glyph_typeface.style(), FontStyle::Italic);
}

// ── test collections ──

/// A collection deriving from the font collection base without overrides.
struct CustomFontCollection {
    base: FontCollectionBase,
    key: Uri,
}

impl CustomFontCollection {
    fn new(key: &str) -> Rc<Self> {
        Rc::new(Self { base: FontCollectionBase::new(), key: uri(key) })
    }
}

impl FontCollectionBaseImpl for CustomFontCollection {
    fn base(this: &Self) -> &FontCollectionBase {
        &this.base
    }

    fn key(this: &Self) -> Uri {
        this.key.clone()
    }
}

/// Counts the platform fallback requests and answers them with a fixed result.
struct RecordingFontCollection {
    base: FontCollectionBase,
    key: Uri,
    platform_fallback_result: RefCell<Option<Rc<GlyphTypeface>>>,
    platform_call_count: Cell<i32>,
}

impl RecordingFontCollection {
    fn new(key: &str) -> Rc<Self> {
        Rc::new(Self {
            base: FontCollectionBase::new(),
            key: uri(key),
            platform_fallback_result: RefCell::new(None),
            platform_call_count: Cell::new(0),
        })
    }
}

impl FontCollectionBaseImpl for RecordingFontCollection {
    fn base(this: &Self) -> &FontCollectionBase {
        &this.base
    }

    fn key(this: &Self) -> Uri {
        this.key.clone()
    }

    fn try_match_character_from_platform(
        this: &Self,
        _codepoint: i32,
        _key: FontCollectionKey,
        _family_name: Option<&str>,
        _culture: Option<&CultureInfo>,
    ) -> Option<Rc<GlyphTypeface>> {
        this.platform_call_count.set(this.platform_call_count.get() + 1);
        this.platform_fallback_result.borrow().clone()
    }
}

/// A platform-backed collection holding one fallback family at several keys
/// (style/weight). Its single platform hook serves a face for an EXACT
/// requested key only, modelling the system font collection over a font
/// collection file, so the key-honouring fallback path (including the
/// exact-key upgrade, which reuses the platform hook) is exercised
/// deterministically.
struct KeyedFallbackCollection {
    base: FontCollectionBase,
    key: Uri,
    platform_faces: RefCell<HashMap<FontCollectionKey, Rc<GlyphTypeface>>>,
    covered_codepoint: i32,
}

impl KeyedFallbackCollection {
    fn new(key: &str, covered_codepoint: i32) -> Rc<Self> {
        Rc::new(Self {
            base: FontCollectionBase::new(),
            key: uri(key),
            platform_faces: RefCell::new(HashMap::new()),
            covered_codepoint,
        })
    }

    fn add_platform_face(&self, face: &Rc<GlyphTypeface>) {
        self.platform_faces.borrow_mut().insert(FontCollectionKey::from(&**face), face.clone());
    }
}

impl FontCollectionBaseImpl for KeyedFallbackCollection {
    fn base(this: &Self) -> &FontCollectionBase {
        &this.base
    }

    fn key(this: &Self) -> Uri {
        this.key.clone()
    }

    fn try_match_character_from_platform(
        this: &Self,
        codepoint: i32,
        key: FontCollectionKey,
        _family_name: Option<&str>,
        _culture: Option<&CultureInfo>,
    ) -> Option<Rc<GlyphTypeface>> {
        if codepoint != this.covered_codepoint {
            return None;
        }

        let face = this.platform_faces.borrow().get(&key).cloned()?;

        // Register the matched face, as the system font collection does, so later tiers can find it.
        FontCollectionBase::try_add_glyph_typeface_by_name(this, face.family_name(), key, Some(face.clone()));

        Some(face)
    }
}

/// A collection of embedded fonts overriding the character match (explicit
/// fallbacks) and the synthetic typeface creation (families to leave alone,
/// or no synthesis at all).
struct CustomizableFontCollection {
    base: FontCollectionBase,
    key: Uri,
    fallbacks: Vec<FontFallback>,
    ignorables: Vec<FontFamily>,
    create_synthetic_typefaces: bool,
}

impl CustomizableFontCollection {
    fn new(
        key: &str,
        source: &str,
        fallbacks: Vec<FontFallback>,
        ignorables: Vec<FontFamily>,
        create_synthetic_typefaces: bool,
    ) -> Rc<Self> {
        let collection = Rc::new(Self {
            base: FontCollectionBase::new(),
            key: uri(key),
            fallbacks,
            ignorables,
            create_synthetic_typefaces,
        });

        FontCollectionBase::try_add_font_source(&*collection, &uri(source));

        collection
    }
}

impl FontCollectionBaseImpl for CustomizableFontCollection {
    fn base(this: &Self) -> &FontCollectionBase {
        &this.base
    }

    fn key(this: &Self) -> Uri {
        this.key.clone()
    }

    fn try_match_character(
        this: &Self,
        codepoint: i32,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
        family_name: Option<&str>,
        culture: Option<&CultureInfo>,
    ) -> Option<Typeface> {
        for fallback in &this.fallbacks {
            if fallback.unicode_range.is_in_range(codepoint) {
                return Some(Typeface::with_style(fallback.font_family.clone(), style, weight, stretch));
            }
        }

        FontCollectionBase::try_match_character(this, codepoint, style, weight, stretch, family_name, culture)
    }

    fn try_create_synthetic_glyph_typeface(
        this: &Self,
        glyph_typeface: &Rc<GlyphTypeface>,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Option<Rc<GlyphTypeface>> {
        if !this.create_synthetic_typefaces {
            return None;
        }

        for ignorable in &this.ignorables {
            if glyph_typeface.family_name() == ignorable.name()
                || glyph_typeface.typographic_family_name() == ignorable.name()
            {
                return None;
            }
        }

        FontCollectionBase::try_create_synthetic_glyph_typeface(this, glyph_typeface, style, weight, stretch)
    }
}

fn load_fonts(collection: &dyn IFontCollectionBase, fonts: &[TestFontBuilder]) {
    for font in fonts {
        let bytes = font.build_bytes();

        assert!(FontCollectionBase::try_add_glyph_typeface_from_stream(collection, &mut bytes.as_slice()).is_some());
    }
}

fn build_collection(fonts: &[TestFontBuilder]) -> Rc<CustomFontCollection> {
    let collection = CustomFontCollection::new("fonts:test");

    load_fonts(&*collection, fonts);

    collection
}

/// Creates a glyph typeface through the font backend, at the requested
/// simulations. Bold and oblique report a bold weight and an italic style
/// while keeping the family name and character map, so the faces model one
/// family at several keys.
fn create_glyph_typeface(font: &TestFontBuilder, simulations: FontSimulations) -> Rc<GlyphTypeface> {
    let font_manager_impl = FerroLocator::current().get_required_service::<dyn IFontManagerImpl>();
    let bytes = font.build_bytes();

    let platform_typeface =
        font_manager_impl.try_create_glyph_typeface_from_stream(&mut bytes.as_slice(), simulations).unwrap();

    GlyphTypeface::try_create(platform_typeface, simulations).unwrap()
}

fn match_character(
    collection: &dyn IFontCollection,
    codepoint: i32,
    family_name: Option<&str>,
    culture: Option<&CultureInfo>,
) -> Option<Typeface> {
    collection.try_match_character(
        codepoint,
        FontStyle::Normal,
        FontWeight::Normal,
        FontStretch::Normal,
        family_name,
        culture,
    )
}

fn get_normal(collection: &dyn IFontCollection, family_name: &str) -> Option<Rc<GlyphTypeface>> {
    collection.try_get_glyph_typeface(family_name, FontStyle::Normal, FontWeight::Normal, FontStretch::Normal)
}

/// The family of a fallback typeface (its font family is `<collection key>#<family name>`).
fn family_of(typeface: &Typeface) -> &str {
    typeface.font_family().name()
}

// ── character matching ──

#[test]
fn tier_a_returns_requested_family_when_it_covers_the_codepoint() {
    let _scope = TestFontScope::start();

    let collection = build_collection(&[
        test_fonts::inter_regular(),
        test_fonts::noto_sans_italic(),
        test_fonts::noto_sans_arabic(),
    ]);

    let matched = match_character(&*collection, ARABIC_ALEF, Some("Noto Sans Arabic"), None).unwrap();

    assert_eq!(family_of(&matched), "Noto Sans Arabic");
    assert_eq!(matched.font_family().key().unwrap().source(), &uri("fonts:test"));
}

#[test]
fn tier_a_falls_through_when_requested_family_does_not_cover_codepoint() {
    // "Noto Mono" sorts alphabetically before "Noto Sans Arabic" so a non-coverage-checked
    // implementation that simply returned the requested family would yield Noto Mono.
    let _scope = TestFontScope::start();

    let collection = build_collection(&[test_fonts::noto_mono(), test_fonts::noto_sans_arabic()]);

    let matched = match_character(&*collection, ARABIC_ALEF, Some("Noto Mono"), None).unwrap();

    assert_eq!(family_of(&matched), "Noto Sans Arabic");
}

#[test]
fn tier_b_subsequent_calls_return_the_same_family() {
    let _scope = TestFontScope::start();

    let collection = build_collection(&[
        test_fonts::inter_regular(),
        test_fonts::noto_sans_arabic(),
        test_fonts::noto_sans_hebrew(),
    ]);

    let arabic = culture("ar-SA");

    let first = match_character(&*collection, ARABIC_ALEF, None, Some(&arabic)).unwrap();

    for _ in 0..25 {
        let subsequent = match_character(&*collection, ARABIC_ALEF, None, Some(&arabic)).unwrap();

        assert_eq!(family_of(&first), family_of(&subsequent));
    }
}

#[test]
fn tier_c_skips_families_that_do_not_cover_the_codepoint() {
    let _scope = TestFontScope::start();

    let collection = build_collection(&[
        test_fonts::noto_mono(),        // Latin only
        test_fonts::inter_regular(),    // Latin only
        test_fonts::noto_sans_arabic(), // covers the alef
    ]);

    let matched = match_character(&*collection, ARABIC_ALEF, None, None).unwrap();

    assert_eq!(family_of(&matched), "Noto Sans Arabic");
}

#[test]
fn tier_c_picks_a_covering_family_even_when_no_culture_is_supplied() {
    let _scope = TestFontScope::start();

    let collection = build_collection(&[
        test_fonts::inter_regular(),
        test_fonts::noto_sans_italic(),
        test_fonts::noto_sans_hebrew(),
    ]);

    let matched = match_character(&*collection, HEBREW_ALEF, None, None).unwrap();

    assert_eq!(family_of(&matched), "Noto Sans Hebrew");
}

#[test]
fn tier_c_prefers_culture_matched_family_over_alphabetical_order() {
    let _scope = TestFontScope::start();

    let collection = build_collection(&[
        test_fonts::noto_sans_hebrew(),
        test_fonts::noto_sans_arabic(),
        test_fonts::inter_regular(),
    ]);

    let matched = match_character(&*collection, ARABIC_ALEF, None, Some(&culture("ar-SA"))).unwrap();

    assert_eq!(family_of(&matched), "Noto Sans Arabic");
}

#[test]
fn tier_c_scores_the_declared_culture_above_the_alphabetical_order() {
    // Two families cover the ideograph. "Alpha CJK" sorts first, but "Beta CJK" declares
    // Japanese (code page bit and localized family name).
    let _scope = TestFontScope::start();

    let collection = build_collection(&[
        TestFontBuilder::new("Alpha CJK").codepoints(&[(0x4E00, 0x4E2D)]),
        TestFontBuilder::new("Beta CJK")
            .codepoints(&[(0x4E00, 0x4E2D)])
            .code_page_coverage(crate::media::fonts::FontCodePageCoverage::JapaneseJis)
            .localized_family_name(0x0411, "Beta CJK JP"),
    ]);

    assert_eq!(family_of(&match_character(&*collection, CJK_ICHI, None, None).unwrap()), "Alpha CJK");

    let japanese = match_character(&*collection, CJK_ICHI, None, Some(&culture("ja-JP"))).unwrap();

    assert_eq!(family_of(&japanese), "Beta CJK");

    // The requested family is rejected for the culture when it only advertises other
    // localized names, so the scored sweep decides.
    let requested = match_character(&*collection, CJK_ICHI, Some("Alpha CJK"), Some(&culture("ja-JP"))).unwrap();

    assert_eq!(family_of(&requested), "Beta CJK");
}

#[test]
fn tier_d_platform_fallback_is_invoked_when_cache_sweep_has_no_covering_family() {
    let _scope = TestFontScope::start();

    let collection = RecordingFontCollection::new("fonts:tierD-positive");

    load_fonts(&*collection, &[test_fonts::inter_regular()]);

    // The fallback resolves to a known typeface that is *also* registered in the cache.
    let inter = get_normal(&*collection, "Inter").unwrap();

    *collection.platform_fallback_result.borrow_mut() = Some(inter);

    let matched = match_character(&*collection, CJK_ICHI, None, Some(&culture("ja-JP"))).unwrap();

    assert_eq!(family_of(&matched), "Inter");
    assert_eq!(collection.platform_call_count.get(), 1);
}

#[test]
fn tier_d_platform_fallback_is_invoked_at_most_once_per_script_culture_pair() {
    let _scope = TestFontScope::start();

    let collection = RecordingFontCollection::new("fonts:tierD-cache");

    load_fonts(&*collection, &[test_fonts::inter_regular()]);

    // negative platform answer
    for _ in 0..5 {
        assert!(match_character(&*collection, CJK_ICHI, None, Some(&culture("ja-JP"))).is_none());
    }

    assert_eq!(collection.platform_call_count.get(), 1);
}

#[test]
fn tier_d_negative_platform_result_is_cached_per_script_culture() {
    let _scope = TestFontScope::start();

    let collection = RecordingFontCollection::new("fonts:tierD-negcache");

    load_fonts(&*collection, &[test_fonts::inter_regular()]);

    match_character(&*collection, CJK_ICHI, None, Some(&culture("ja-JP")));
    match_character(&*collection, CJK_ICHI, None, Some(&culture("ko-KR")));
    match_character(&*collection, CJK_ICHI, None, Some(&culture("ja-JP")));

    // ja-JP: 1 call + cached. ko-KR: 1 call. ja-JP again: cached. Total 2.
    assert_eq!(collection.platform_call_count.get(), 2);
}

#[test]
fn returns_none_when_no_family_covers_and_platform_has_no_fallback() {
    let _scope = TestFontScope::start();

    let collection = RecordingFontCollection::new("fonts:no-match");

    load_fonts(&*collection, &[test_fonts::inter_regular()]);

    assert!(match_character(&*collection, CJK_ICHI, None, None).is_none());
}

// A positive (script, culture) bucket entry records a preferred family, but it must not suppress
// the platform lookup for another same-script codepoint that family cannot cover. Here U+4E2D
// resolves to the "Noto Sans SC" subset, which lacks the Simplified-only U+534E; that one must
// then reach the platform (which can place it) rather than be denied.
#[test]
fn tier_d_positive_script_cache_does_not_block_platform_for_a_codepoint_the_bucket_font_lacks() {
    let _scope = TestFontScope::start();

    const ZHONG: i32 = 0x4E2D; // covered by the subset
    const HUA: i32 = 0x534E; // Simplified-only; absent from the subset

    let collection = RecordingFontCollection::new("fonts:bucket-coverage");

    // Latin primary (covers neither ideograph) plus the subset that covers the first but not the second.
    load_fonts(&*collection, &[test_fonts::noto_mono(), test_fonts::noto_sans_sc_subset()]);

    // The subset is Thin (weight 100). Resolve at that weight so the exact-key upgrade of the cache
    // sweep (covered by the dedicated weight test) does not fire here, leaving this test to isolate
    // the positive-bucket behaviour and its platform-call count.
    let weight = FontWeight::Thin;

    let sc = collection
        .try_get_glyph_typeface("Noto Sans SC", FontStyle::Normal, weight, FontStretch::Normal)
        .unwrap();

    assert!(sc.character_to_glyph_map().try_get_glyph(ZHONG).is_some());
    assert!(sc.character_to_glyph_map().try_get_glyph(HUA).is_none());

    // The platform can place the second ideograph. Its font is deliberately NOT in the
    // collection, so only the platform tier can supply it.
    let mi_sans = create_glyph_typeface(&test_fonts::mi_sans_normal(), FontSimulations::None);

    assert!(mi_sans.character_to_glyph_map().try_get_glyph(HUA).is_some());

    *collection.platform_fallback_result.borrow_mut() = Some(mi_sans);

    let zh = culture("zh-CN");

    // The first ideograph resolves from the loaded cache and writes the (Han, zh-CN) bucket.
    // The platform is not consulted for it.
    let zhong_match = collection
        .try_match_character(ZHONG, FontStyle::Normal, weight, FontStretch::Normal, None, Some(&zh))
        .unwrap();

    assert_eq!(family_of(&zhong_match), "Noto Sans SC");
    assert_eq!(collection.platform_call_count.get(), 0);

    // The second shares the bucket but the bucket font lacks it. It must reach the platform and
    // resolve to a font that covers it.
    let hua_match =
        collection.try_match_character(HUA, FontStyle::Normal, weight, FontStretch::Normal, None, Some(&zh)).unwrap();

    assert_eq!(family_of(&hua_match), "MiSans Normal");
    assert_eq!(collection.platform_call_count.get(), 1);
}

#[test]
fn match_typeface_carries_the_requested_style_weight_and_stretch() {
    let _scope = TestFontScope::start();

    let collection = build_collection(&[test_fonts::inter_regular(), test_fonts::noto_sans_arabic()]);

    let matched = collection
        .try_match_character(ARABIC_ALEF, FontStyle::Italic, FontWeight::Bold, FontStretch::Condensed, None, None)
        .unwrap();

    assert_eq!(matched.style(), FontStyle::Italic);
    assert_eq!(matched.weight(), FontWeight::Bold);
    assert_eq!(matched.stretch(), FontStretch::Condensed);

    // The matched face is pre-cached under the requested key: no synthesis happens later.
    let glyph_typeface = collection
        .try_get_glyph_typeface("Noto Sans Arabic", FontStyle::Italic, FontWeight::Bold, FontStretch::Condensed)
        .unwrap();

    assert_eq!(glyph_typeface.font_simulations(), FontSimulations::None);
}

// The (script, culture) bucket is key-agnostic, so once one face of a fallback family is resolved,
// a later request that differs in ANY axis (weight via bold, style via oblique) must still resolve
// its own face, not reuse the cached neighbour. The leak is order-dependent, so both orders are
// checked.
#[test]
fn fallback_resolves_the_requested_typeface_not_a_cached_neighbour() {
    for (first, second) in [
        (FontSimulations::Bold, FontSimulations::None),    // weight: bold then upright normal
        (FontSimulations::None, FontSimulations::Bold),    // weight: reverse
        (FontSimulations::Oblique, FontSimulations::None), // style: italic then upright
        (FontSimulations::None, FontSimulations::Oblique), // style: reverse
    ] {
        let _scope = TestFontScope::start();

        // One family exposed by the platform as several faces (regular plus a synthetic bold and
        // oblique). All cover the alef (same underlying font).
        let faces: Vec<(FontSimulations, Rc<GlyphTypeface>)> =
            [FontSimulations::None, FontSimulations::Bold, FontSimulations::Oblique]
                .into_iter()
                .map(|simulations| (simulations, create_glyph_typeface(&test_fonts::noto_sans_hebrew(), simulations)))
                .collect();

        let collection = KeyedFallbackCollection::new("fonts:key", HEBREW_ALEF);

        for (_, face) in &faces {
            collection.add_platform_face(face);
        }

        let face_of = |simulations: FontSimulations| {
            faces.iter().find(|(candidate, _)| *candidate == simulations).map(|(_, face)| face.clone()).unwrap()
        };

        let first_face = face_of(first);
        let second_face = face_of(second);

        let hebrew = culture("he-IL");

        // First face: resolved via the platform and pins the bucket to the family.
        assert!(collection
            .try_match_character(
                HEBREW_ALEF,
                first_face.style(),
                first_face.weight(),
                FontStretch::Normal,
                None,
                Some(&hebrew)
            )
            .is_some());

        // Second face: same family, same bucket, different key. It must come back as its own key.
        assert!(collection
            .try_match_character(
                HEBREW_ALEF,
                second_face.style(),
                second_face.weight(),
                FontStretch::Normal,
                None,
                Some(&hebrew)
            )
            .is_some());

        let second_result = collection
            .try_get_glyph_typeface(
                second_face.family_name(),
                second_face.style(),
                second_face.weight(),
                FontStretch::Normal,
            )
            .unwrap();

        assert_eq!(FontCollectionKey::from(&*second_result), FontCollectionKey::from(&*second_face));

        // The first face stays correct too.
        let first_result = collection
            .try_get_glyph_typeface(
                first_face.family_name(),
                first_face.style(),
                first_face.weight(),
                FontStretch::Normal,
            )
            .unwrap();

        assert_eq!(FontCollectionKey::from(&*first_result), FontCollectionKey::from(&*first_face));
    }
}

// A codepoint the bucket family cannot cover skips the bucket tier and lands in the cache sweep,
// where only a neighbouring-key face cached by an earlier run is available. The exact-key upgrade
// must run in the sweep too. Modelled with Tamil, which is not locale-sensitive, so resolving with
// no culture skips the bucket tier and drives the request into the sweep.
#[test]
fn tier_c_sweep_resolves_the_requested_weight_not_a_cached_neighbour() {
    const TAMIL_KA: i32 = 0x0B95;

    for (first, second) in
        [(FontSimulations::Bold, FontSimulations::None), (FontSimulations::None, FontSimulations::Bold)]
    {
        let _scope = TestFontScope::start();

        let regular = create_glyph_typeface(&test_fonts::noto_sans_tamil(), FontSimulations::None);
        let bold = create_glyph_typeface(&test_fonts::noto_sans_tamil(), FontSimulations::Bold);

        assert_eq!(regular.weight(), FontWeight::Normal);
        assert_eq!(bold.weight(), FontWeight::Bold);

        let collection = KeyedFallbackCollection::new("fonts:tierc", TAMIL_KA);

        collection.add_platform_face(&regular);
        collection.add_platform_face(&bold);

        let face_of = |simulations: FontSimulations| {
            if simulations == FontSimulations::Bold {
                bold.clone()
            } else {
                regular.clone()
            }
        };

        let first_face = face_of(first);
        let second_face = face_of(second);

        assert!(collection
            .try_match_character(TAMIL_KA, first_face.style(), first_face.weight(), FontStretch::Normal, None, None)
            .is_some());

        assert!(collection
            .try_match_character(TAMIL_KA, second_face.style(), second_face.weight(), FontStretch::Normal, None, None)
            .is_some());

        let second_result = collection
            .try_get_glyph_typeface(
                second_face.family_name(),
                second_face.style(),
                second_face.weight(),
                FontStretch::Normal,
            )
            .unwrap();

        assert_eq!(FontCollectionKey::from(&*second_result), FontCollectionKey::from(&*second_face));
    }
}

#[test]
fn shaping_constrained_match_skips_fonts_that_cannot_shape_the_script() {
    use crate::media::text_formatting::unicode::Script;

    let _scope = TestFontScope::start();

    // "Alpha Arabic" covers the alef but declares no Arabic shaping; it sorts first.
    let collection = RecordingFontCollection::new("fonts:shaping");

    load_fonts(
        &*collection,
        &[
            TestFontBuilder::new("Alpha Arabic").codepoints(&[(0x600, 0x6FF)]).shaping_scripts(&["latn"]),
            test_fonts::noto_sans_arabic(),
        ],
    );

    let unconstrained = match_character(&*collection, ARABIC_ALEF, None, None).unwrap();

    assert_eq!(family_of(&unconstrained), "Alpha Arabic");

    let constrained = FontCollectionBase::try_match_character_with_script(
        &*collection,
        ARABIC_ALEF,
        FontStyle::Normal,
        FontWeight::Normal,
        FontStretch::Normal,
        None,
        None,
        Script::Arabic,
    )
    .unwrap();

    assert_eq!(family_of(&constrained), "Noto Sans Arabic");

    // A constrained query never reaches the platform.
    let no_match = FontCollectionBase::try_match_character_with_script(
        &*collection,
        CJK_ICHI,
        FontStyle::Normal,
        FontWeight::Normal,
        FontStretch::Normal,
        None,
        None,
        Script::Arabic,
    );

    assert!(no_match.is_none());
    assert_eq!(collection.platform_call_count.get(), 0);
}

#[test]
fn last_resort_fonts_are_matched_last() {
    let _scope = TestFontScope::start();

    let collection = build_collection(&[test_fonts::adobe_blank(), test_fonts::noto_sans_hebrew()]);

    // Both cover the alef; the last resort font sorts first but is only used when nothing else covers.
    assert_eq!(family_of(&match_character(&*collection, HEBREW_ALEF, None, None).unwrap()), "Noto Sans Hebrew");
    assert_eq!(family_of(&match_character(&*collection, 0x2A736, None, None).unwrap()), "Adobe Blank 2 VF R");
}

// ── determinism ──

fn latin_fonts() -> Vec<TestFontBuilder> {
    vec![
        test_fonts::inter_regular(),
        test_fonts::inter_bold(),
        test_fonts::manrope_light(),
        test_fonts::noto_mono(),
        test_fonts::noto_sans_italic(),
        test_fonts::source_serif_italic(),
    ]
}

fn shuffle(source: &[TestFontBuilder], seed: u64) -> Vec<TestFontBuilder> {
    let mut copy = source.to_vec();
    let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;

    for i in (1..copy.len()).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;

        copy.swap(i, (state % (i as u64 + 1)) as usize);
    }

    copy
}

#[test]
fn try_match_character_returns_same_family_regardless_of_add_order() {
    let _scope = TestFontScope::start();

    let fonts = latin_fonts();

    let mut orderings = vec![fonts.clone(), fonts.iter().rev().cloned().collect()];

    for seed in [1, 17, 42, 1337] {
        orderings.push(shuffle(&fonts, seed));
    }

    let mut expected: Option<String> = None;

    for ordering in &orderings {
        let collection = build_collection(ordering);

        let matched = match_character(&*collection, 'A' as i32, None, None).unwrap();
        let family_name = family_of(&matched).to_owned();

        match &expected {
            None => expected = Some(family_name),
            Some(expected) => assert_eq!(&family_name, expected),
        }
    }

    assert_eq!(expected.as_deref(), Some("Inter"));
}

#[test]
fn try_match_character_is_stable_across_repeated_invocations() {
    let _scope = TestFontScope::start();

    let collection = build_collection(&latin_fonts());

    let first_match = match_character(&*collection, 'A' as i32, None, None).unwrap();

    for _ in 0..50 {
        let matched = match_character(&*collection, 'A' as i32, None, None).unwrap();

        assert_eq!(family_of(&matched), family_of(&first_match));
    }
}

#[test]
fn try_match_character_result_is_independent_of_cache_population_order() {
    let _scope = TestFontScope::start();

    let warmup = |collection: &CustomFontCollection, family_names: &[&str]| {
        for name in family_names {
            get_normal(collection, name);
        }
    };

    let mut results_a = Vec::new();
    let mut results_b = Vec::new();

    for _ in 0..5 {
        let a = build_collection(&latin_fonts());
        let b = build_collection(&latin_fonts());

        // Different warm-up order on purpose.
        warmup(&a, &["Inter", "Manrope Light", "Noto Mono", "Noto Sans", "Source Serif 4 36pt"]);
        warmup(&b, &["Source Serif 4 36pt", "Noto Sans", "Noto Mono", "Manrope Light", "Inter"]);

        results_a.push(family_of(&match_character(&*a, 'A' as i32, None, None).unwrap()).to_owned());
        results_b.push(family_of(&match_character(&*b, 'A' as i32, None, None).unwrap()).to_owned());
    }

    // Every iteration produces the same family, for both warm-up orders.
    assert!(results_a.iter().all(|family| family == &results_a[0]));
    assert!(results_b.iter().all(|family| family == &results_a[0]));
}

#[test]
fn try_match_character_cached_script_fallback_lookup_returns_same_family() {
    let _scope = TestFontScope::start();

    let collection = build_collection(&latin_fonts());

    // The first call populates the script/culture fallback cache; subsequent calls must hit
    // it and still return the same family.
    let english = culture("en-US");

    let first = match_character(&*collection, 'A' as i32, None, Some(&english)).unwrap();
    let second = match_character(&*collection, 'A' as i32, None, Some(&english)).unwrap();

    assert_eq!(family_of(&first), family_of(&second));
}

// ── overriding the virtual members ──

#[test]
fn should_use_fallback() {
    let _scope = TestFontScope::with_assets();

    let fallback = FontFallback { font_family: FontFamily::new("Arial"), unicode_range: UnicodeRange::new(65, 65) };

    let font_collection =
        CustomizableFontCollection::new(test_fonts::ASSETS, test_fonts::ASSETS, vec![fallback], Vec::new(), true);

    let matched = match_character(&*font_collection, 'A' as i32, None, None).unwrap();

    assert_eq!(matched.font_family().name(), "Arial");

    // Other codepoints go through the base implementation.
    assert_eq!(family_of(&match_character(&*font_collection, 'B' as i32, None, None).unwrap()), "Inter");
}

#[test]
fn should_ignore_font_family() {
    let _scope = TestFontScope::with_assets();

    let ignorable = FontFamily::new(&format!("{}#Noto Mono", test_fonts::ASSETS));

    let font_collection = CustomizableFontCollection::new(
        test_fonts::ASSETS,
        test_fonts::ASSETS,
        Vec::new(),
        vec![ignorable.clone()],
        true,
    );

    let glyph_typeface = Typeface::new(ignorable).glyph_typeface();

    assert!(font_collection
        .try_create_synthetic_glyph_typeface(&glyph_typeface, FontStyle::Italic, FontWeight::DemiBold, FontStretch::Normal)
        .is_none());

    // A family that is not ignored is synthesized by the base implementation.
    let inter = get_normal(&*font_collection, "Manrope").unwrap();

    let synthetic = font_collection
        .try_create_synthetic_glyph_typeface(&inter, FontStyle::Italic, FontWeight::DemiBold, FontStretch::Normal)
        .unwrap();

    assert_eq!(synthetic.font_simulations(), FontSimulations::Bold | FontSimulations::Oblique);
}

// ── the system font collection ──

/// A font backend whose alias family resolves through the platform but is
/// absent from the installed family list, the shape of a platform alias.
/// Such a family cannot be found again by the family-name search, so nothing
/// repairs a missing cache entry. The alias always resolves to the regular
/// face of the backing font, never to the requested weight.
fn alias_font_manager(alias: &str) -> Rc<TestFontManagerImpl> {
    let font_manager = TestFontManagerImpl::new("Noto Mono");

    font_manager.add_font(test_fonts::noto_mono().build());
    font_manager.add_alias(alias, "Noto Mono");
    font_manager.set_installed_font_family_names(Some(Vec::new()));

    font_manager
}

#[test]
fn should_cache_nearest_match() {
    let font_manager = TestFontManagerImpl::new("Arial");

    font_manager.add_font(TestFontBuilder::new("Arial").weight(FontWeight::Black).build());

    let _scope = TestFontScope::with_font_manager(font_manager.clone());

    let font_collection = SystemFontCollection::new(font_manager);

    let glyph_typeface = font_collection
        .try_get_glyph_typeface("Arial", FontStyle::Normal, FontWeight::ExtraBlack, FontStretch::Normal)
        .unwrap();

    let glyph_typefaces = font_collection.base().glyph_typeface_cache().try_get_value("Arial").unwrap();

    assert_eq!(glyph_typefaces.borrow().len(), 2);
    assert!(glyph_typefaces.borrow().contains_key(&FontCollectionKey::new(
        FontStyle::Normal,
        FontWeight::Black,
        FontStretch::Normal
    )));

    let other_glyph_typeface = font_collection
        .try_get_glyph_typeface("Arial", FontStyle::Normal, FontWeight::ExtraBlack, FontStretch::Normal)
        .unwrap();

    assert!(Rc::ptr_eq(&glyph_typeface, &other_glyph_typeface));
}

#[test]
fn should_cache_synthetic_match_under_requested_family_name() {
    let font_manager = alias_font_manager("MyAlias");

    let _scope = TestFontScope::with_font_manager(font_manager.clone());

    let font_collection = SystemFontCollection::new(font_manager.clone());

    let black_key = FontCollectionKey::new(FontStyle::Normal, FontWeight::Black, FontStretch::Normal);

    // Prime the cache with the bare family, as any control asking for the alias at a
    // normal weight would. This is what makes the next lookup take the nearest match.
    assert!(get_normal(&font_collection, "MyAlias").is_some());

    let first = font_collection
        .try_get_glyph_typeface("MyAlias", FontStyle::Normal, FontWeight::Black, FontStretch::Normal)
        .unwrap();

    // Guards the test itself: the first resolution must really be a synthesised bold.
    assert_eq!(first.font_simulations(), FontSimulations::Bold);

    let creations_after_first_call = font_manager.stream_typeface_creations();

    for _ in 0..10 {
        let next = font_collection
            .try_get_glyph_typeface("MyAlias", FontStyle::Normal, FontWeight::Black, FontStretch::Normal)
            .unwrap();

        assert!(Rc::ptr_eq(&first, &next));
    }

    // Each synthesis copies the entire font file through the platform typeface's stream,
    // so an uncached synthetic means one full font copy per call.
    assert_eq!(font_manager.stream_typeface_creations(), creations_after_first_call);

    let cached = font_collection.base().glyph_typeface_cache().try_get_value("MyAlias").unwrap();

    assert!(cached.borrow().contains_key(&black_key));
}

#[test]
fn should_ignore_family_name_casing_when_resolving_a_synthetic_match() {
    let font_manager = alias_font_manager("MyAlias");

    let _scope = TestFontScope::with_font_manager(font_manager.clone());

    let font_collection = SystemFontCollection::new(font_manager.clone());

    assert!(get_normal(&font_collection, "MyAlias").is_some());

    // Casing must not decide whether a request gets a synthesised bold.
    let upper_case = font_collection
        .try_get_glyph_typeface("MYALIAS", FontStyle::Normal, FontWeight::Black, FontStretch::Normal)
        .unwrap();

    assert_eq!(upper_case.font_simulations(), FontSimulations::Bold);

    let creations_after_first_call = font_manager.stream_typeface_creations();

    let mixed_case = font_collection
        .try_get_glyph_typeface("MyAlias", FontStyle::Normal, FontWeight::Black, FontStretch::Normal)
        .unwrap();

    // One shared cache entry, so the other casing neither re-synthesises nor gets a
    // second instance of the same face.
    assert!(Rc::ptr_eq(&upper_case, &mixed_case));
    assert_eq!(font_manager.stream_typeface_creations(), creations_after_first_call);
}

#[test]
fn should_not_cache_a_family_twice_when_the_platform_returns_another_casing() {
    // The platform reports the family as "Noto Mono"; the caller asks in lower case.
    let font_manager = alias_font_manager("Noto Mono");

    let _scope = TestFontScope::with_font_manager(font_manager.clone());

    let font_collection = SystemFontCollection::new(font_manager);

    assert!(get_normal(&font_collection, "noto mono").is_some());

    // A cache keyed ordinally would store the requested casing beside the platform's own, while
    // the font family list de-duplicates ignoring case and publishes only the first of the two.
    assert_eq!(font_collection.base().glyph_typeface_cache().count(), 1);
    assert_eq!(font_collection.base().glyph_typeface_cache().count(), font_collection.count());
}

#[test]
fn should_reuse_an_already_cached_synthetic_glyph_typeface() {
    let font_manager = alias_font_manager("MyAlias");

    let _scope = TestFontScope::with_font_manager(font_manager.clone());

    let font_collection = SystemFontCollection::new(font_manager.clone());

    let regular = get_normal(&font_collection, "MyAlias").unwrap();

    let first = font_collection
        .try_create_synthetic_glyph_typeface(&regular, FontStyle::Normal, FontWeight::Black, FontStretch::Normal)
        .unwrap();

    assert_eq!(first.font_simulations(), FontSimulations::Bold);

    let creations_after_first_call = font_manager.stream_typeface_creations();

    let second = font_collection
        .try_create_synthetic_glyph_typeface(&regular, FontStyle::Normal, FontWeight::Black, FontStretch::Normal)
        .unwrap();

    // A second synthesis would build a glyph typeface that loses the cache slot to the
    // first one, so it would be returned to the caller but never cached and never disposed.
    assert!(Rc::ptr_eq(&first, &second));
    assert_eq!(font_manager.stream_typeface_creations(), creations_after_first_call);
}

#[test]
fn system_font_collection_caches_a_missing_family() {
    let scope = TestFontScope::start();

    let font_collection = SystemFontCollection::new(scope.font_manager_impl().clone());

    assert_eq!(font_collection.key(), uri("fonts:SystemFonts"));
    assert_eq!(
        font_collection.font_families().iter().map(|family| family.name().to_owned()).collect::<Vec<_>>(),
        ["Noto Mono", "Noto Sans", "Twitter Color Emoji"]
    );

    assert!(get_normal(&font_collection, "Unknown").is_none());

    // The miss is remembered: `None` is cached for the key.
    let cached = font_collection.base().glyph_typeface_cache().try_get_value("Unknown").unwrap();

    assert_eq!(cached.borrow().values().filter(|value| value.is_none()).count(), 1);
    assert!(get_normal(&font_collection, "Unknown").is_none());

    // The family typefaces come from the platform.
    assert_eq!(font_collection.try_get_family_typefaces("Noto Mono").unwrap().len(), 1);
    assert!(font_collection.try_get_family_typefaces("Unknown").is_none());
}

#[test]
fn system_font_collection_matches_characters_through_the_platform_once() {
    let scope = TestFontScope::start();

    let font_collection = SystemFontCollection::new(scope.font_manager_impl().clone());

    const EMOJI: i32 = 0x1F600;

    let first = match_character(&font_collection, EMOJI, None, None).unwrap();

    assert_eq!(family_of(&first), "Twitter Color Emoji");
    assert_eq!(first.font_family().key().unwrap().source(), &uri("fonts:SystemFonts"));
    assert_eq!(scope.font_manager_impl().match_character_calls(), 1);

    // The platform match is registered, so the cache sweep serves it from now on.
    let second = match_character(&font_collection, EMOJI, None, None).unwrap();

    assert_eq!(family_of(&second), "Twitter Color Emoji");
    assert_eq!(scope.font_manager_impl().match_character_calls(), 1);
}

// ── custom collections ──

#[test]
fn should_add_glyph_typeface_by_stream() {
    let scope = TestFontScope::with_assets();

    let font_manager = FontManager::current();

    let font_collection = CustomFontCollection::new("fonts:custom");

    font_manager.add_font_collection(font_collection.clone());

    let infos = [
        ("AdobeBlank2VF.ttf", "Adobe Blank 2 VF R", FontWeight::Normal),
        ("Inter-Bold.ttf", "Inter", FontWeight::Bold),
        ("Inter-Regular.ttf", "Inter", FontWeight::Normal),
        ("Manrope-Light.ttf", "Manrope Light", FontWeight::Light),
        ("MiSans-Normal.ttf", "MiSans Normal", FontWeight(305)),
        ("NotoMono-Regular.ttf", "Noto Mono", FontWeight::Normal),
        ("NotoSans-Italic.ttf", "Noto Sans", FontWeight::Normal),
        ("NotoSansArabic-Regular.ttf", "Noto Sans Arabic", FontWeight::Normal),
        ("NotoSansHebrew-Regular.ttf", "Noto Sans Hebrew", FontWeight::Normal),
        ("NotoSansTamil-Regular.ttf", "Noto Sans Tamil", FontWeight::Normal),
        ("SourceSerif4_36pt-Italic.ttf", "Source Serif 4 36pt", FontWeight::Normal),
        ("TwitterColorEmoji-SVGinOT.ttf", "Twitter Color Emoji", FontWeight::Normal),
    ];

    let mut assets = scope.asset_loader().get_assets(&uri(test_fonts::ASSETS), None);

    assets.sort_by_key(|asset| asset.absolute_uri().to_ascii_lowercase());

    assert_eq!(assets.len(), infos.len());

    let mut glyph_typefaces = Vec::new();

    // Load fonts
    for ((file_name, family_name, weight), asset) in infos.iter().zip(&assets) {
        assert_eq!(asset.absolute_path(), format!("FerroUI.UnitTests.Assets.{file_name}"));

        let mut font_stream = scope.asset_loader().open(asset, None).unwrap();

        let glyph_typeface =
            FontCollectionBase::try_add_glyph_typeface_from_stream(&*font_collection, &mut *font_stream).unwrap();

        assert_eq!(glyph_typeface.family_name(), *family_name);
        assert_eq!(glyph_typeface.weight(), *weight);

        glyph_typefaces.push(glyph_typeface);
    }

    // Check against the custom collection
    for ((_, family_name, weight), glyph_typeface) in infos.iter().zip(&glyph_typefaces) {
        let typeface = Typeface::from_name_with_style(
            &format!("fonts:custom#{family_name}"),
            FontStyle::Normal,
            *weight,
            FontStretch::Normal,
        );

        let second_glyph_typeface = font_manager.try_get_glyph_typeface(&typeface).unwrap();

        assert!(Rc::ptr_eq(glyph_typeface, &second_glyph_typeface), "{family_name}");
    }
}

#[test]
fn should_enumerate_font_families() {
    let scope = TestFontScope::with_assets();

    let font_manager = FontManager::current();

    let font_collection = CustomFontCollection::new("fonts:custom");

    font_manager.add_font_collection(font_collection.clone());

    let assets = scope.asset_loader().get_assets(&uri(test_fonts::ASSETS), None);

    for asset in &assets {
        let mut stream = scope.asset_loader().open(asset, None).unwrap();

        FontCollectionBase::try_add_glyph_typeface_from_stream(&*font_collection, &mut *stream);
    }

    let families = font_collection.font_families();

    assert!(families.len() >= assets.len());

    let other = CustomFontCollection::new("fonts:other");

    for family in &families {
        for typeface in family.family_typefaces() {
            FontCollectionBase::try_add_glyph_typeface(&*other, &typeface.glyph_typeface());
        }
    }

    assert_eq!(other.count(), families.len());

    for (i, family) in families.iter().enumerate() {
        assert_eq!(other.get(i).name(), family.name());
    }
}

#[cfg(not(target_family = "wasm"))]
mod file_sources {
    use std::path::PathBuf;

    use super::*;

    /// A directory of font files that is removed when the value is dropped.
    struct FontDirectory(PathBuf);

    impl FontDirectory {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!("ferroui-font-tests-{}-{name}", std::process::id()));

            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();

            for (file_name, font) in test_fonts::asset_fonts() {
                std::fs::write(path.join(file_name), font.build_bytes()).unwrap();
            }

            std::fs::write(path.join("readme.txt"), b"not a font").unwrap();

            Self(path)
        }

        fn uri(&self, file_name: &str) -> Uri {
            let path = self.0.join(file_name);
            let path = path.to_str().unwrap().replace('\\', "/").replace(' ', "%20");

            if path.starts_with('/') {
                uri(&format!("file://{path}"))
            } else {
                uri(&format!("file:///{path}"))
            }
        }
    }

    impl Drop for FontDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn should_add_font_source_from_file() {
        let _scope = TestFontScope::start();

        let directory = FontDirectory::new("file");

        let font_manager = FontManager::current();

        let font_collection = CustomFontCollection::new("fonts:custom");

        font_manager.add_font_collection(font_collection.clone());

        // Add the font file
        assert!(FontCollectionBase::try_add_font_source(&*font_collection, &directory.uri("Inter-Regular.ttf")));

        // Check if the font was loaded
        let glyph_typeface = get_normal(&*font_collection, "Inter").unwrap();

        assert_eq!(glyph_typeface.family_name(), "Inter");

        // Check if the font manager can find the font
        let glyph_typeface2 = font_manager.try_get_glyph_typeface(&Typeface::from_name("fonts:custom#Inter")).unwrap();

        assert!(Rc::ptr_eq(&glyph_typeface, &glyph_typeface2));

        // A missing file and a file that is not a font add nothing.
        assert!(!FontCollectionBase::try_add_font_source(&*font_collection, &directory.uri("Missing.ttf")));
        assert!(!FontCollectionBase::try_add_font_source(&*font_collection, &directory.uri("readme.txt")));
    }

    #[test]
    fn should_add_font_source_from_folder() {
        let _scope = TestFontScope::start();

        let directory = FontDirectory::new("folder");

        let font_manager = FontManager::current();

        let font_collection = CustomFontCollection::new("fonts:custom");

        font_manager.add_font_collection(font_collection.clone());

        // Add the fonts
        assert!(FontCollectionBase::try_add_font_source(&*font_collection, &directory.uri("")));

        // Check if the font was loaded
        let glyph_typeface = get_normal(&*font_collection, "Inter").unwrap();

        assert_eq!(glyph_typeface.family_name(), "Inter");
        assert!(get_normal(&*font_collection, "Noto Sans Hebrew").is_some());

        // Check if the font manager can find the font
        let glyph_typeface2 = font_manager.try_get_glyph_typeface(&Typeface::from_name("fonts:custom#Inter")).unwrap();

        assert!(Rc::ptr_eq(&glyph_typeface, &glyph_typeface2));

        // A missing directory adds nothing.
        assert!(!FontCollectionBase::try_add_font_source(&*font_collection, &directory.uri("missing/")));
    }
}

#[test]
fn should_add_font_source_from_resource() {
    let _scope = TestFontScope::with_assets();

    let font_manager = FontManager::current();

    let font_collection = CustomFontCollection::new("fonts:custom");

    font_manager.add_font_collection(font_collection.clone());

    // Add the font resource
    assert!(FontCollectionBase::try_add_font_source(&*font_collection, &uri(test_fonts::ASSETS)));

    // Get the loaded family names
    assert!(!font_collection.font_families().is_empty());

    // Try to get a glyph typeface
    let glyph_typeface = get_normal(&*font_collection, "Noto Mono").unwrap();

    assert_eq!(glyph_typeface.family_name(), "Noto Mono");

    // Check if the font manager can find the font
    let glyph_typeface2 = font_manager.try_get_glyph_typeface(&Typeface::from_name("fonts:custom#Noto Mono")).unwrap();

    assert!(Rc::ptr_eq(&glyph_typeface, &glyph_typeface2));
}

#[test]
fn unsupported_font_sources_add_nothing() {
    let _scope = TestFontScope::with_assets();

    let font_collection = CustomFontCollection::new("fonts:custom");

    assert!(!FontCollectionBase::try_add_font_source(&*font_collection, &uri("https://example.com/fonts")));
    assert!(!FontCollectionBase::try_add_font_source(
        &*font_collection,
        &Uri::new("/Assets/Fonts", crate::utilities::UriKind::Relative).unwrap()
    ));
    assert!(!FontCollectionBase::try_add_font_source(&*font_collection, &uri("resm:Other.Assets?assembly=Other")));
    assert_eq!(font_collection.count(), 0);

    // A stream that is not a font adds nothing either.
    assert!(FontCollectionBase::try_add_glyph_typeface_from_stream(&*font_collection, &mut [1u8, 2, 3].as_slice())
        .is_none());
}

#[test]
fn adding_a_glyph_typeface_twice_is_accepted_and_another_one_for_the_same_key_is_not() {
    let _scope = TestFontScope::start();

    let font_collection = CustomFontCollection::new("fonts:custom");

    let first = create_glyph_typeface(&test_fonts::inter_regular(), FontSimulations::None);
    let second = create_glyph_typeface(&test_fonts::inter_regular(), FontSimulations::None);

    assert!(FontCollectionBase::try_add_glyph_typeface(&*font_collection, &first));
    assert!(FontCollectionBase::try_add_glyph_typeface(&*font_collection, &first));
    assert!(!FontCollectionBase::try_add_glyph_typeface(&*font_collection, &second));
    assert!(!FontCollectionBase::try_add_glyph_typeface_by_name(
        &*font_collection,
        "",
        FontCollectionKey::from(&*first),
        Some(first.clone())
    ));

    assert_eq!(font_collection.count(), 1);
    assert_eq!(font_collection.get(0), FontFamily::new("fonts:custom#Inter"));
    assert!(Rc::ptr_eq(&get_normal(&*font_collection, "Inter").unwrap(), &first));

    let family_typefaces = font_collection.try_get_family_typefaces("inter").unwrap();

    assert_eq!(family_typefaces.len(), 1);
    assert_eq!(family_typefaces[0].font_family().key().unwrap().source(), &uri("fonts:custom"));
    assert!(font_collection.try_get_family_typefaces("Unknown").is_none());

    // Disposing the collection disposes its typefaces.
    font_collection.dispose();

    assert!(first.platform_typeface().try_get_stream().is_none());
}

// ── nearest matching ──

fn family_collection(faces: &[(FontStyle, i32, FontStretch)]) -> Rc<CustomFontCollection> {
    let fonts: Vec<TestFontBuilder> = faces
        .iter()
        .map(|(style, weight, stretch)| {
            TestFontBuilder::new("Family").style(*style).weight(FontWeight(*weight)).stretch(*stretch)
        })
        .collect();

    build_collection(&fonts)
}

#[test]
fn nearest_match_follows_the_weight_fallback_order() {
    let _scope = TestFontScope::start();

    let collection = family_collection(&[
        (FontStyle::Normal, 300, FontStretch::Normal),
        (FontStyle::Normal, 500, FontStretch::Normal),
        (FontStyle::Normal, 700, FontStretch::Normal),
    ]);

    let nearest = |weight: i32| {
        collection
            .try_get_nearest_match("Family", FontStyle::Normal, FontWeight(weight), FontStretch::Normal)
            .unwrap()
            .weight()
            .value()
    };

    // Between 400 and 500: up to 500 first, then lighter, then heavier.
    assert_eq!(nearest(400), 500);
    assert_eq!(nearest(450), 500);
    // Below 400: lighter first, then heavier.
    assert_eq!(nearest(350), 300);
    assert_eq!(nearest(100), 300);
    // Above 500: heavier first, then lighter.
    assert_eq!(nearest(600), 700);
    assert_eq!(nearest(900), 700);
    // Exact matches win.
    assert_eq!(nearest(300), 300);

    let lighter_only = family_collection(&[(FontStyle::Normal, 300, FontStretch::Normal)]);

    assert_eq!(
        lighter_only
            .try_get_nearest_match("Family", FontStyle::Normal, FontWeight::Normal, FontStretch::Normal)
            .unwrap()
            .weight(),
        FontWeight::Light
    );

    assert!(collection
        .try_get_nearest_match("Unknown", FontStyle::Normal, FontWeight::Normal, FontStretch::Normal)
        .is_none());
}

#[test]
fn nearest_match_follows_the_stretch_fallback_order_and_drops_the_style() {
    let _scope = TestFontScope::start();

    let collection = family_collection(&[
        (FontStyle::Normal, 400, FontStretch::Condensed),
        (FontStyle::Normal, 400, FontStretch::Expanded),
    ]);

    let nearest = |style: FontStyle, stretch: FontStretch| {
        collection.try_get_nearest_match("Family", style, FontWeight::Normal, stretch).unwrap().stretch()
    };

    // Narrower than normal: wider stretches are tried in ascending order.
    assert_eq!(nearest(FontStyle::Normal, FontStretch::ExtraCondensed), FontStretch::Condensed);
    assert_eq!(nearest(FontStyle::Normal, FontStretch::SemiCondensed), FontStretch::Expanded);
    // Normal or wider: narrower stretches are tried in descending order.
    assert_eq!(nearest(FontStyle::Normal, FontStretch::UltraExpanded), FontStretch::Expanded);
    assert_eq!(nearest(FontStyle::Normal, FontStretch::Normal), FontStretch::Condensed);
    // An italic request falls back to the normal style.
    assert_eq!(nearest(FontStyle::Italic, FontStretch::Expanded), FontStretch::Expanded);

    // The weight is dropped when no face of the stretch has it.
    assert_eq!(
        collection
            .try_get_nearest_match("Family", FontStyle::Normal, FontWeight::Bold, FontStretch::Expanded)
            .unwrap()
            .stretch(),
        FontStretch::Expanded
    );

    // Any face is better than none: an italic-only family serves a normal request.
    let italic_only = family_collection(&[(FontStyle::Italic, 400, FontStretch::Normal)]);

    assert_eq!(
        italic_only
            .try_get_nearest_match("Family", FontStyle::Normal, FontWeight::Bold, FontStretch::Condensed)
            .unwrap()
            .style(),
        FontStyle::Italic
    );
}

#[test]
fn glyph_typeface_lookup_matches_family_name_prefixes() {
    let _scope = TestFontScope::start();

    let collection = build_collection(&[
        test_fonts::noto_mono(),
        test_fonts::noto_sans_arabic(),
        test_fonts::noto_sans_hebrew(),
        test_fonts::twitter_color_emoji(),
    ]);

    // The first family (in name order) starting with the name is used.
    assert_eq!(get_normal(&*collection, "Noto Sans").unwrap().family_name(), "Noto Sans Arabic");
    assert_eq!(get_normal(&*collection, "noto sans h").unwrap().family_name(), "Noto Sans Hebrew");
    assert_eq!(get_normal(&*collection, "T").unwrap().family_name(), "Twitter Color Emoji");
    assert!(get_normal(&*collection, "Sans").is_none());
    assert!(get_normal(&*collection, "Noto Sans Hebrew Extra").is_none());
}

// ── the embedded font collection ──

fn embedded_collection(create_synthetic_typefaces: bool) -> Rc<CustomizableFontCollection> {
    CustomizableFontCollection::new(
        "fonts:testFonts",
        test_fonts::ASSETS,
        Vec::new(),
        Vec::new(),
        create_synthetic_typefaces,
    )
}

#[test]
fn should_get_near_matching_typeface() {
    for (font_weight, font_style) in [
        (FontWeight::SemiLight, FontStyle::Normal),
        (FontWeight::Bold, FontStyle::Italic),
        (FontWeight::Heavy, FontStyle::Oblique),
    ] {
        let _scope = TestFontScope::with_assets();

        let font_collection = embedded_collection(false);

        let glyph_typeface =
            font_collection.try_get_glyph_typeface("Noto Mono", font_style, font_weight, FontStretch::Normal).unwrap();

        assert_eq!(glyph_typeface.family_name(), "Noto Mono");
    }
}

#[test]
fn should_not_get_typeface_for_invalid_family_name() {
    let _scope = TestFontScope::with_assets();

    let font_collection = EmbeddedFontCollection::new(uri("fonts:testFonts"), uri(test_fonts::ASSETS));

    assert!(get_normal(&font_collection, "ABC").is_none());
}

#[test]
fn should_get_typeface_for_partial_family_name() {
    let _scope = TestFontScope::with_assets();

    let font_collection = EmbeddedFontCollection::new(uri("fonts:testFonts"), uri(test_fonts::ASSETS));

    let glyph_typeface = get_normal(&font_collection, "T").unwrap();

    assert_eq!(glyph_typeface.family_name(), "Twitter Color Emoji");
}

#[test]
fn should_get_typeface_for_typographic_family_name() {
    let _scope = TestFontScope::with_assets();

    let font_collection = EmbeddedFontCollection::new(uri("fonts:testFonts"), uri(test_fonts::ASSETS));

    let glyph_typeface = font_collection
        .try_get_glyph_typeface("Manrope", FontStyle::Normal, FontWeight::Light, FontStretch::Normal)
        .unwrap();

    assert_eq!(glyph_typeface.family_name(), "Manrope Light");
    assert_eq!(glyph_typeface.typographic_family_name(), "Manrope");
}

#[test]
fn should_cache_synthetic_glyph_typeface() {
    let _scope = TestFontScope::with_assets();

    let font_collection = embedded_collection(true);

    let glyph_typeface = font_collection
        .try_get_glyph_typeface("Manrope", FontStyle::Normal, FontWeight::ExtraBlack, FontStretch::Normal)
        .unwrap();

    assert_eq!(glyph_typeface.font_simulations(), FontSimulations::Bold);

    let glyph_typefaces = font_collection.base().glyph_typeface_cache().try_get_value("Manrope").unwrap();

    assert_eq!(glyph_typefaces.borrow().len(), 2);

    let other_glyph_typeface = font_collection
        .try_get_glyph_typeface("Manrope", FontStyle::Normal, FontWeight::ExtraBlack, FontStretch::Normal)
        .unwrap();

    assert!(Rc::ptr_eq(&glyph_typeface, &other_glyph_typeface));
}

#[test]
fn should_cache_nearest_match_for_mi_sans() {
    let _scope = TestFontScope::with_assets();

    let font_collection = embedded_collection(true);

    // Font weight 305
    assert!(get_normal(&*font_collection, "MiSans").is_some());

    // Font weight regular (400)
    assert!(font_collection
        .try_get_glyph_typeface("MiSans", FontStyle::Normal, FontWeight::Bold, FontStretch::Normal)
        .is_some());

    // Font weight 700
    let glyph_typefaces = font_collection.base().glyph_typeface_cache().try_get_value("MiSans").unwrap();

    assert_eq!(glyph_typefaces.borrow().len(), 3);
}

#[test]
fn embedded_font_collection_loads_a_single_font_file_and_a_file_pattern() {
    let _scope = TestFontScope::with_assets();

    let single = EmbeddedFontCollection::new(uri("fonts:single"), uri(&test_fonts::asset_uri("Inter-Regular.ttf")));

    assert_eq!(single.key(), uri("fonts:single"));
    assert_eq!(single.count(), 1);
    assert_eq!(single.get(0).name(), "Inter");
    assert!(single.as_font_collection_base().is_some());
    assert!(single.as_any().is::<EmbeddedFontCollection>());

    let pattern = EmbeddedFontCollection::new(uri("fonts:pattern"), uri(&test_fonts::asset_uri("NotoSans*.ttf")));

    assert_eq!(
        pattern.font_families().iter().map(|family| family.name().to_owned()).collect::<Vec<_>>(),
        ["Noto Sans", "Noto Sans Arabic", "Noto Sans Hebrew", "Noto Sans Tamil"]
    );
}
