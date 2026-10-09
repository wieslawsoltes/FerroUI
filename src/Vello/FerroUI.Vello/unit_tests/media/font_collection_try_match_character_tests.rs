//! Port of upstream's `Media/FontCollectionTryMatchCharacterTests.cs` of the
//! Skia unit tests.
//!
//! Exercises the tiered `FontCollectionBase.TryMatchCharacter` algorithm
//! (Tier A to Tier E) using a small set of embedded test fonts and a stub
//! subclass that intercepts the platform-fallback hook.

use crate::unit_tests::mock_platform_render_interface;
use crate::FontManagerImpl;
use ferroui_base::media::fonts::{
    FontCollectionBase, FontCollectionBaseImpl, FontCollectionKey, IFontCollection, IFontCollectionBase,
};
use ferroui_base::media::{FontSimulations, FontStretch, FontStyle, FontWeight, GlyphTypeface, Typeface};
use ferroui_base::platform::{IAssetLoader, IFontManagerImpl};
use ferroui_base::utilities::{CultureInfo, Uri, UriKind};
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_controls::testing::{UnitTestApplication, UnitTestApplicationScope};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

const ASSETS_NAMESPACE: &str = "FerroUI.Vello.UnitTests.Assets";

const ARABIC_ALEF: i32 = 0x0627; // 'ا' — Arabic
const HEBREW_ALEF: i32 = 0x05D0; // 'א' — Hebrew
const CJK_ICHI: i32 = 0x4E00; // '一' — not covered by any loaded test font

#[test]
fn tier_a_returns_requested_family_when_it_covers_the_codepoint() {
    let _app = start_app();

    let collection = build_collection(&["Inter-Regular.ttf", "NotoSans-Italic.ttf", "NotoSansArabic-Regular.ttf"]);

    let matched = collection
        .try_match_character(
            ARABIC_ALEF,
            FontStyle::Normal,
            FontWeight::Normal,
            FontStretch::Normal,
            Some("Noto Sans Arabic"),
            None,
        )
        .expect("a match");

    assert_eq!("Noto Sans Arabic", family_of(&matched));
}

#[test]
fn tier_a_falls_through_when_requested_family_does_not_cover_codepoint() {
    // "Noto Mono" sorts alphabetically before "Noto Sans Arabic" so a non-coverage-checked
    // implementation that simply returned the requested family would yield Noto Mono.
    let _app = start_app();

    let collection = build_collection(&["NotoMono-Regular.ttf", "NotoSansArabic-Regular.ttf"]);

    let matched = collection
        .try_match_character(
            ARABIC_ALEF,
            FontStyle::Normal,
            FontWeight::Normal,
            FontStretch::Normal,
            Some("Noto Mono"),
            None,
        )
        .expect("a match");

    assert_eq!("Noto Sans Arabic", family_of(&matched));
}

#[test]
fn tier_b_subsequent_calls_return_the_same_family() {
    let _app = start_app();

    let collection =
        build_collection(&["Inter-Regular.ttf", "NotoSansArabic-Regular.ttf", "NotoSansHebrew-Regular.ttf"]);

    let first = collection
        .try_match_character(
            ARABIC_ALEF,
            FontStyle::Normal,
            FontWeight::Normal,
            FontStretch::Normal,
            None,
            Some(&CultureInfo::get_culture_info("ar-SA")),
        )
        .expect("a match");

    for i in 0..25 {
        let subsequent = collection
            .try_match_character(
                ARABIC_ALEF,
                FontStyle::Normal,
                FontWeight::Normal,
                FontStretch::Normal,
                None,
                Some(&CultureInfo::get_culture_info("ar-SA")),
            )
            .expect("a match");

        assert_eq!(family_of(&first), family_of(&subsequent), "{i}");
    }
}

#[test]
fn tier_c_skips_families_that_do_not_cover_the_codepoint() {
    let _app = start_app();

    let collection = build_collection(&[
        "NotoMono-Regular.ttf",       // Latin only
        "Inter-Regular.ttf",          // Latin only
        "NotoSansArabic-Regular.ttf", // covers ا
    ]);

    let matched = collection
        .try_match_character(ARABIC_ALEF, FontStyle::Normal, FontWeight::Normal, FontStretch::Normal, None, None)
        .expect("a match");

    assert_eq!("Noto Sans Arabic", family_of(&matched));
}

#[test]
fn tier_c_picks_a_covering_family_even_when_no_culture_is_supplied() {
    let _app = start_app();

    let collection = build_collection(&["Inter-Regular.ttf", "NotoSans-Italic.ttf", "NotoSansHebrew-Regular.ttf"]);

    let matched = collection
        .try_match_character(HEBREW_ALEF, FontStyle::Normal, FontWeight::Normal, FontStretch::Normal, None, None)
        .expect("a match");

    assert_eq!("Noto Sans Hebrew", family_of(&matched));
}

#[test]
fn tier_c_prefers_culture_matched_family_over_alphabetical_order() {
    let _app = start_app();

    // Both Hebrew and Arabic fonts are loaded but neither covers the requested codepoint;
    // the only family that does is "Noto Sans Arabic". Even if the algorithm did not
    // filter by coverage, the requested ar-SA culture should still resolve to it.
    let collection =
        build_collection(&["NotoSansHebrew-Regular.ttf", "NotoSansArabic-Regular.ttf", "Inter-Regular.ttf"]);

    let matched = collection
        .try_match_character(
            ARABIC_ALEF,
            FontStyle::Normal,
            FontWeight::Normal,
            FontStretch::Normal,
            None,
            Some(&CultureInfo::get_culture_info("ar-SA")),
        )
        .expect("a match");

    assert_eq!("Noto Sans Arabic", family_of(&matched));
}

#[test]
fn tier_d_platform_fallback_is_invoked_when_cache_sweep_has_no_covering_family() {
    let _app = start_app();

    let collection = RecordingFontCollection::new(Uri::new("fonts:tierD-positive", UriKind::Absolute).unwrap());
    load_fonts(&*collection, &["Inter-Regular.ttf"]);

    // The fallback resolves to a known typeface that is *also* registered in the cache
    // (so BuildTypefaceWithSynthesis succeeds against the returned glyph typeface).
    let inter_gt = collection
        .try_get_glyph_typeface("Inter", FontStyle::Normal, FontWeight::Normal, FontStretch::Normal)
        .expect("Inter");

    collection.set_platform_fallback_result(Some(inter_gt));

    let matched = collection
        .try_match_character(
            CJK_ICHI,
            FontStyle::Normal,
            FontWeight::Normal,
            FontStretch::Normal,
            None,
            Some(&CultureInfo::get_culture_info("ja-JP")),
        )
        .expect("a match");

    assert_eq!("Inter", family_of(&matched));
    assert_eq!(1, collection.platform_call_count());
}

#[test]
fn tier_d_platform_fallback_is_invoked_at_most_once_per_script_culture_pair() {
    let _app = start_app();

    let collection = RecordingFontCollection::new(Uri::new("fonts:tierD-cache", UriKind::Absolute).unwrap());
    load_fonts(&*collection, &["Inter-Regular.ttf"]);
    collection.set_platform_fallback_result(None); // negative platform answer

    for _ in 0..5 {
        collection.try_match_character(
            CJK_ICHI,
            FontStyle::Normal,
            FontWeight::Normal,
            FontStretch::Normal,
            None,
            Some(&CultureInfo::get_culture_info("ja-JP")),
        );
    }

    assert_eq!(1, collection.platform_call_count());
}

#[test]
fn tier_d_negative_platform_result_is_cached_per_script_culture() {
    let _app = start_app();

    let collection = RecordingFontCollection::new(Uri::new("fonts:tierD-negcache", UriKind::Absolute).unwrap());
    load_fonts(&*collection, &["Inter-Regular.ttf"]);
    collection.set_platform_fallback_result(None);

    collection.try_match_character(
        CJK_ICHI,
        FontStyle::Normal,
        FontWeight::Normal,
        FontStretch::Normal,
        None,
        Some(&CultureInfo::get_culture_info("ja-JP")),
    );

    collection.try_match_character(
        CJK_ICHI,
        FontStyle::Normal,
        FontWeight::Normal,
        FontStretch::Normal,
        None,
        Some(&CultureInfo::get_culture_info("ko-KR")),
    );

    collection.try_match_character(
        CJK_ICHI,
        FontStyle::Normal,
        FontWeight::Normal,
        FontStretch::Normal,
        None,
        Some(&CultureInfo::get_culture_info("ja-JP")),
    );

    // ja-JP: 1 call + cached. ko-KR: 1 call. ja-JP again: cached. Total 2.
    assert_eq!(2, collection.platform_call_count());
}

#[test]
fn returns_false_when_no_family_covers_and_platform_has_no_fallback() {
    let _app = start_app();

    let collection = RecordingFontCollection::new(Uri::new("fonts:no-match", UriKind::Absolute).unwrap());
    load_fonts(&*collection, &["Inter-Regular.ttf"]);
    collection.set_platform_fallback_result(None);

    assert!(collection
        .try_match_character(CJK_ICHI, FontStyle::Normal, FontWeight::Normal, FontStretch::Normal, None, None)
        .is_none());
}

// Regression for the "中华人民共和国 second codepoint is tofu" report. A positive (script,
// culture) bucket entry records a preferred family, but it must not suppress the Tier D
// platform lookup for another same-script codepoint that family cannot cover. Here 中 (U+4E2D)
// resolves to the Noto Sans SC subset, which lacks the Simplified-only 华 (U+534E); 华 must then
// reach the platform (which can place it) rather than be denied and rendered as tofu.
#[test]
fn tier_d_positive_script_cache_does_not_block_platform_for_a_codepoint_the_bucket_font_lacks() {
    let _app = start_app();

    const ZHONG: i32 = 0x4E2D; // 中 — covered by the Noto Sans SC subset
    const HUA: i32 = 0x534E; // 华 — Simplified-only; absent from the SC subset

    let collection = RecordingFontCollection::new(Uri::new("fonts:bucket-coverage", UriKind::Absolute).unwrap());

    // Latin primary (covers neither ideograph) plus the SC subset that covers 中 but not 华.
    load_fonts(&*collection, &["NotoMono-Regular.ttf"]);
    load_fonts_from_fonts_namespace(&*collection, &["NotoSansSC-Subset.ttf"]);

    // The SC subset is Thin (weight 100). Resolve at that weight so the Tier C exact-key upgrade
    // (covered by the dedicated weight test) does not fire here, leaving this test to isolate the
    // Tier D positive-bucket behaviour and its platform-call count.
    const WEIGHT: FontWeight = FontWeight::Thin;

    let sc_gt = collection
        .try_get_glyph_typeface("Noto Sans SC", FontStyle::Normal, WEIGHT, FontStretch::Normal)
        .expect("Noto Sans SC");
    assert!(sc_gt.character_to_glyph_map().try_get_glyph(ZHONG).is_some());
    assert!(sc_gt.character_to_glyph_map().try_get_glyph(HUA).is_none());

    // The platform can place 华 (MiSans covers it). It is deliberately NOT in the collection,
    // so only Tier D can supply it.
    let mi_sans = load_standalone_glyph_typeface("FerroUI.Vello.UnitTests.Assets.MiSans-Normal.ttf");
    assert!(mi_sans.character_to_glyph_map().try_get_glyph(HUA).is_some());
    collection.set_platform_fallback_result(Some(mi_sans));

    let zh = CultureInfo::get_culture_info("zh-CN");

    // 中 resolves from the loaded cache (Tier C) and writes the (Han, zh-CN) bucket. The platform
    // is not consulted for it.
    let zhong_match = collection
        .try_match_character(ZHONG, FontStyle::Normal, WEIGHT, FontStretch::Normal, None, Some(&zh))
        .expect("a match for 中");
    assert_eq!("Noto Sans SC", family_of(&zhong_match));
    assert_eq!(0, collection.platform_call_count());

    // 华 shares the bucket but the bucket font lacks it. Before the fix the positive entry made
    // Tier D treat the bucket as "platform already attempted", so 华 returned no match. It must
    // now reach the platform and resolve to a font that covers it.
    let hua_match = collection
        .try_match_character(HUA, FontStyle::Normal, WEIGHT, FontStretch::Normal, None, Some(&zh))
        .expect("a match for 华");
    assert_eq!("MiSans Normal", family_of(&hua_match));
    assert_eq!(1, collection.platform_call_count());
}

#[test]
fn match_typeface_carries_the_requested_style_weight_and_stretch() {
    let _app = start_app();

    let collection = build_collection(&["Inter-Regular.ttf", "NotoSansArabic-Regular.ttf"]);

    let matched = collection
        .try_match_character(ARABIC_ALEF, FontStyle::Italic, FontWeight::Bold, FontStretch::Condensed, None, None)
        .expect("a match");

    assert_eq!(FontStyle::Italic, matched.style());
    assert_eq!(FontWeight::Bold, matched.weight());
    assert_eq!(FontStretch::Condensed, matched.stretch());
}

// Regression for the "Normal CJK fallback renders bold" report, generalized across the whole
// FontCollectionKey. The (script, culture) bucket is key-agnostic, so once one face of a fallback
// family is resolved, a later request that differs in ANY axis (weight via Bold, style via
// Oblique) must still resolve its own face, not reuse the cached neighbour. Stretch travels the
// same path. The leak is order-dependent, so both orders are checked.
#[test]
fn fallback_resolves_the_requested_typeface_not_a_cached_neighbour() {
    for (first, second) in [
        (FontSimulations::Bold, FontSimulations::None),    // weight: Bold then upright Normal (Sandbox order)
        (FontSimulations::None, FontSimulations::Bold),    // weight: reverse
        (FontSimulations::Oblique, FontSimulations::None), // style: Italic then upright
        (FontSimulations::None, FontSimulations::Oblique), // style: reverse
    ] {
        let row = format!("({first:?}, {second:?})");

        let _app = start_app();

        const ALEPH: i32 = 0x05D0; // Hebrew — absent from any Latin primary, so it needs fallback.

        // One family ("Noto Sans Hebrew") exposed by the platform as several faces (Regular plus a
        // synthetic Bold and Oblique), modelling a .ttc whose styles/weights are separate faces. All
        // cover aleph (same underlying font).
        let faces: HashMap<FontSimulations, Rc<GlyphTypeface>> = HashMap::from([
            (FontSimulations::None, create_glyph_typeface(FontSimulations::None)),
            (FontSimulations::Bold, create_glyph_typeface(FontSimulations::Bold)),
            (FontSimulations::Oblique, create_glyph_typeface(FontSimulations::Oblique)),
        ]);

        let collection = KeyedFallbackCollection::new(Uri::new("fonts:key", UriKind::Absolute).unwrap(), ALEPH);

        for face in faces.values() {
            collection.add_platform_face(face.clone());
        }

        let first_face = &faces[&first];
        let second_face = &faces[&second];
        let culture = CultureInfo::get_culture_info("he-IL");

        // First face: resolved via the platform (Tier D) and pins the bucket to the family.
        assert!(
            collection
                .try_match_character(
                    ALEPH,
                    first_face.style(),
                    first_face.weight(),
                    FontStretch::Normal,
                    None,
                    Some(&culture)
                )
                .is_some(),
            "{row}"
        );

        // Second face: same family, same bucket, different key. It must come back as its own key.
        assert!(
            collection
                .try_match_character(
                    ALEPH,
                    second_face.style(),
                    second_face.weight(),
                    FontStretch::Normal,
                    None,
                    Some(&culture)
                )
                .is_some(),
            "{row}"
        );

        let second_result = collection
            .try_get_glyph_typeface(
                second_face.family_name(),
                second_face.style(),
                second_face.weight(),
                FontStretch::Normal,
            )
            .unwrap_or_else(|| panic!("{row}"));
        assert_eq!(FontCollectionKey::from(&**second_face), FontCollectionKey::from(&*second_result), "{row}");

        // The first face stays correct too.
        let first_result = collection
            .try_get_glyph_typeface(first_face.family_name(), first_face.style(), first_face.weight(), FontStretch::Normal)
            .unwrap_or_else(|| panic!("{row}"));
        assert_eq!(FontCollectionKey::from(&**first_face), FontCollectionKey::from(&*first_result), "{row}");
    }
}

// Regression for "中华人民共和国: the first glyph is Normal but 华人民共和国 stay bold". A codepoint
// the bucket family cannot cover (the Simplified-only 华 when the bucket is a JP font) skips Tier B
// and lands in the Tier C sweep, where only a neighbouring-key face cached by an earlier run is
// available. The exact-key upgrade must run in Tier C too. Modelled with Tamil, which is not
// locale-sensitive, so resolving with no culture skips Tier B and drives the request into Tier C.
#[test]
fn tier_c_sweep_resolves_the_requested_weight_not_a_cached_neighbour() {
    for (first, second) in [
        (FontSimulations::Bold, FontSimulations::None), // Sandbox order: Bold block above Normal one
        (FontSimulations::None, FontSimulations::Bold), // reverse
    ] {
        let row = format!("({first:?}, {second:?})");

        let _app = start_app();

        const TAMIL_KA: i32 = 0x0B95; // க — Tamil; not locale-sensitive, so culture=null skips Tier B.
        const TAMIL: &str = "FerroUI.Vello.UnitTests.Assets.NotoSansTamil-Regular.ttf";

        let faces: HashMap<FontSimulations, Rc<GlyphTypeface>> = HashMap::from([
            (FontSimulations::None, create_glyph_typeface_from(TAMIL, FontSimulations::None)),
            (FontSimulations::Bold, create_glyph_typeface_from(TAMIL, FontSimulations::Bold)),
        ]);

        assert_eq!(FontWeight::Normal, faces[&FontSimulations::None].weight(), "{row}");
        assert_eq!(FontWeight::Bold, faces[&FontSimulations::Bold].weight(), "{row}");

        let collection = KeyedFallbackCollection::new(Uri::new("fonts:tierc", UriKind::Absolute).unwrap(), TAMIL_KA);

        for face in faces.values() {
            collection.add_platform_face(face.clone());
        }

        let first_face = &faces[&first];
        let second_face = &faces[&second];

        // culture=null on a non-locale-sensitive script skips Tier B, so the second request lands in
        // the Tier C sweep, where only the first (neighbouring-key) face is cached.
        assert!(
            collection
                .try_match_character(TAMIL_KA, first_face.style(), first_face.weight(), FontStretch::Normal, None, None)
                .is_some(),
            "{row}"
        );
        assert!(
            collection
                .try_match_character(
                    TAMIL_KA,
                    second_face.style(),
                    second_face.weight(),
                    FontStretch::Normal,
                    None,
                    None
                )
                .is_some(),
            "{row}"
        );

        let second_result = collection
            .try_get_glyph_typeface(
                second_face.family_name(),
                second_face.style(),
                second_face.weight(),
                FontStretch::Normal,
            )
            .unwrap_or_else(|| panic!("{row}"));
        assert_eq!(FontCollectionKey::from(&**second_face), FontCollectionKey::from(&*second_result), "{row}");
    }
}

fn start_app() -> UnitTestApplicationScope {
    UnitTestApplication::start(mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())))
}

fn build_collection(asset_file_names: &[&str]) -> Rc<TestFontCollection> {
    let collection = TestFontCollection::new(Uri::new("fonts:test", UriKind::Absolute).unwrap());
    load_fonts(&*collection, asset_file_names);
    collection
}

fn load_fonts(collection: &dyn IFontCollectionBase, asset_file_names: &[&str]) {
    let loader = FerroLocator::current().get_required_service::<dyn IAssetLoader>();

    for file_name in asset_file_names {
        let uri = Uri::new(
            &format!("resm:{ASSETS_NAMESPACE}.{file_name}?assembly=ferroui-vello"),
            UriKind::Absolute,
        )
        .unwrap();
        let mut stream = loader.open(&uri, None).expect("the font stream");
        assert!(FontCollectionBase::try_add_glyph_typeface_from_stream(collection, &mut *stream).is_some(), "{file_name}");
    }
}

// The subset test fonts live in the Fonts resource namespace rather than Assets.
fn load_fonts_from_fonts_namespace(collection: &dyn IFontCollectionBase, file_names: &[&str]) {
    let loader = FerroLocator::current().get_required_service::<dyn IAssetLoader>();

    for file_name in file_names {
        let uri = Uri::new(
            &format!("resm:FerroUI.Vello.UnitTests.Fonts.{file_name}?assembly=ferroui-vello"),
            UriKind::Absolute,
        )
        .unwrap();
        let mut stream = loader.open(&uri, None).expect("the font stream");
        assert!(FontCollectionBase::try_add_glyph_typeface_from_stream(collection, &mut *stream).is_some(), "{file_name}");
    }
}

// Loads a font into a throwaway collection and returns its glyph typeface, so the platform
// stub can return it without the font being present in the collection under test.
fn load_standalone_glyph_typeface(resource_name: &str) -> Rc<GlyphTypeface> {
    let loader = FerroLocator::current().get_required_service::<dyn IAssetLoader>();
    let sink = TestFontCollection::new(Uri::new("fonts:sink", UriKind::Absolute).unwrap());

    let mut stream = loader
        .open(&Uri::new(&format!("resm:{resource_name}?assembly=ferroui-vello"), UriKind::Absolute).unwrap(), None)
        .expect("the font stream");

    FontCollectionBase::try_add_glyph_typeface_from_stream(&*sink, &mut *stream).expect("the glyph typeface")
}

// Builds a Noto Sans Hebrew glyph typeface at the requested simulations. None is the Regular
// face; Bold and Oblique report weight Bold / style Italic respectively while keeping the same
// family name and cmap, so the faces model one family at several keys.
fn create_glyph_typeface(simulations: FontSimulations) -> Rc<GlyphTypeface> {
    create_glyph_typeface_from(&format!("{ASSETS_NAMESPACE}.NotoSansHebrew-Regular.ttf"), simulations)
}

fn create_glyph_typeface_from(resource_name: &str, simulations: FontSimulations) -> Rc<GlyphTypeface> {
    let loader = FerroLocator::current().get_required_service::<dyn IAssetLoader>();
    let font_manager_impl = FerroLocator::current().get_required_service::<dyn IFontManagerImpl>();

    let mut stream = loader
        .open(&Uri::new(&format!("resm:{resource_name}?assembly=ferroui-vello"), UriKind::Absolute).unwrap(), None)
        .expect("the font stream");

    let platform_typeface = font_manager_impl
        .try_create_glyph_typeface_from_stream(&mut *stream, simulations)
        .expect("the platform typeface");

    GlyphTypeface::try_create(platform_typeface, simulations).expect("the glyph typeface")
}

fn family_of(typeface: &Typeface) -> &str {
    // Fallback typefaces are returned with FontFamily.Name == "<collectionKey>#<familyName>".
    let name = typeface.font_family().name();

    match name.rfind('#') {
        Some(hash_index) => &name[hash_index + 1..],
        None => name,
    }
}

struct TestFontCollection {
    base: FontCollectionBase,
    key: Uri,
}

impl TestFontCollection {
    fn new(key: Uri) -> Rc<Self> {
        Rc::new(Self { base: FontCollectionBase::new(), key })
    }
}

impl FontCollectionBaseImpl for TestFontCollection {
    fn base(this: &Self) -> &FontCollectionBase {
        &this.base
    }

    fn key(this: &Self) -> Uri {
        this.key.clone()
    }
}

struct RecordingFontCollection {
    base: FontCollectionBase,
    key: Uri,
    platform_call_count: Cell<i32>,
    platform_fallback_result: RefCell<Option<Rc<GlyphTypeface>>>,
}

impl RecordingFontCollection {
    fn new(key: Uri) -> Rc<Self> {
        Rc::new(Self {
            base: FontCollectionBase::new(),
            key,
            platform_call_count: Cell::new(0),
            platform_fallback_result: RefCell::new(None),
        })
    }

    fn set_platform_fallback_result(&self, value: Option<Rc<GlyphTypeface>>) {
        *self.platform_fallback_result.borrow_mut() = value;
    }

    fn platform_call_count(&self) -> i32 {
        self.platform_call_count.get()
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

// A platform-backed collection holding one fallback family at several keys (style/weight). Its
// single platform hook serves a face for an EXACT requested key only, modelling SystemFontCollection
// over a .ttc, so the key-honouring fallback path (including the exact-key upgrade, which reuses
// TryMatchCharacterFromPlatform) is exercised deterministically.
struct KeyedFallbackCollection {
    base: FontCollectionBase,
    key: Uri,
    platform_faces: RefCell<HashMap<FontCollectionKey, Rc<GlyphTypeface>>>,
    covered_codepoint: i32,
}

impl KeyedFallbackCollection {
    fn new(key: Uri, covered_codepoint: i32) -> Rc<Self> {
        Rc::new(Self {
            base: FontCollectionBase::new(),
            key,
            platform_faces: RefCell::new(HashMap::new()),
            covered_codepoint,
        })
    }

    fn add_platform_face(&self, face: Rc<GlyphTypeface>) {
        self.platform_faces.borrow_mut().insert(FontCollectionKey::from(&*face), face);
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

        // Register the matched face, as SystemFontCollection does, so later tiers can find it.
        FontCollectionBase::try_add_glyph_typeface_by_name(this, face.family_name(), key, Some(face.clone()));

        Some(face)
    }
}
