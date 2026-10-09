//! Tests of the font manager on fonts built in code.

use std::any::Any;
use std::collections::HashMap;
use std::rc::Rc;

use crate::media::fonts::testing::{test_fonts, TestFontBuilder, TestFontManagerImpl, TestFontScope};
use crate::media::fonts::{
    EmbeddedFontCollection, FontCollectionBase, FontCollectionBaseImpl, IFontCollection, SystemFontCollection,
};
use crate::media::{
    FontFallback, FontFamily, FontManager, FontManagerOptions, FontSimulations, FontStretch, FontStyle, FontWeight,
    GlyphTypeface, Typeface, UnicodeRange,
};
use crate::utilities::{CultureInfo, Uri};
use crate::FerroLocator;

fn uri(text: &str) -> Uri {
    Uri::absolute(text).unwrap()
}

/// The embedded test fonts, family "Noto Mono".
fn font_uri() -> String {
    format!("{}#Noto Mono", test_fonts::ASSETS)
}

fn bind_options(options: FontManagerOptions) {
    FerroLocator::current_mutable().bind::<FontManagerOptions>().to_constant(Rc::new(options));
}

fn match_character(codepoint: i32, font_family: Option<&FontFamily>) -> Option<Typeface> {
    FontManager::current().try_match_character(
        codepoint,
        FontStyle::Normal,
        FontWeight::Normal,
        FontStretch::Normal,
        font_family,
        None,
    )
}

/// A collection of the "Inter" faces registered as `fonts:Inter`.
struct InterFontCollection {
    base: FontCollectionBase,
}

impl InterFontCollection {
    fn new() -> Rc<Self> {
        let collection = Rc::new(Self { base: FontCollectionBase::new() });

        for font in [
            test_fonts::inter_regular(),
            test_fonts::inter_bold(),
            TestFontBuilder::new("Inter").style(FontStyle::Italic),
            TestFontBuilder::new("Inter SemiBold").typographic_family_name("Inter").weight(FontWeight::SemiBold),
            TestFontBuilder::new("Inter Light").typographic_family_name("Inter").weight(FontWeight::Light),
        ] {
            let bytes = font.build_bytes();

            assert!(
                FontCollectionBase::try_add_glyph_typeface_from_stream(&*collection, &mut bytes.as_slice()).is_some()
            );
        }

        collection
    }
}

impl FontCollectionBaseImpl for InterFontCollection {
    fn base(this: &Self) -> &FontCollectionBase {
        &this.base
    }

    fn key(_this: &Self) -> Uri {
        uri("fonts:Inter")
    }
}

/// A collection implementing the interface directly.
struct StubFontCollection {
    key: Uri,
    is_disposed: std::cell::Cell<bool>,
}

impl StubFontCollection {
    fn new(key: &Uri) -> Rc<Self> {
        Rc::new(Self { key: key.clone(), is_disposed: std::cell::Cell::new(false) })
    }
}

impl IFontCollection for StubFontCollection {
    fn key(&self) -> Uri {
        self.key.clone()
    }

    fn count(&self) -> usize {
        0
    }

    fn get(&self, _index: usize) -> FontFamily {
        panic!("the collection is empty")
    }

    fn font_families(&self) -> Vec<FontFamily> {
        Vec::new()
    }

    fn try_get_glyph_typeface(
        &self,
        _family_name: &str,
        _style: FontStyle,
        _weight: FontWeight,
        _stretch: FontStretch,
    ) -> Option<Rc<GlyphTypeface>> {
        None
    }

    fn try_match_character(
        &self,
        _codepoint: i32,
        font_style: FontStyle,
        font_weight: FontWeight,
        font_stretch: FontStretch,
        _family_name: Option<&str>,
        _culture: Option<&CultureInfo>,
    ) -> Option<Typeface> {
        // Any character is matched to the default font family.
        Some(Typeface::with_style(FontFamily::new("Noto Sans"), font_style, font_weight, font_stretch))
    }

    fn try_get_family_typefaces(&self, _family_name: &str) -> Option<Vec<Typeface>> {
        None
    }

    fn try_create_synthetic_glyph_typeface(
        &self,
        _glyph_typeface: &Rc<GlyphTypeface>,
        _style: FontStyle,
        _weight: FontWeight,
        _stretch: FontStretch,
    ) -> Option<Rc<GlyphTypeface>> {
        None
    }

    fn try_get_nearest_match(
        &self,
        _family_name: &str,
        _style: FontStyle,
        _weight: FontWeight,
        _stretch: FontStretch,
    ) -> Option<Rc<GlyphTypeface>> {
        None
    }

    fn dispose(&self) {
        self.is_disposed.set(true);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn same(a: &Rc<dyn IFontCollection>, b: &Rc<dyn IFontCollection>) -> bool {
    Rc::ptr_eq(a, b)
}

// ── the font manager ──

#[test]
fn should_create_single_instance_typeface() {
    let _scope = TestFontScope::start();

    let typeface = Typeface::new(FontFamily::new("MyFont"));

    let glyph_typeface = FontManager::current().try_get_glyph_typeface(&typeface).unwrap();

    let other = FontManager::current().try_get_glyph_typeface(&typeface).unwrap();

    assert!(Rc::ptr_eq(&glyph_typeface, &other));
}

#[test]
#[should_panic(expected = "Default font family name can't be null or empty.")]
fn should_throw_when_default_family_name_is_null_and_installed_font_family_names_is_empty() {
    let _scope = TestFontScope::with_font_manager(TestFontManagerImpl::new(""));

    FontManager::current();
}

#[test]
#[should_panic(expected = "is a placeholder and cannot be used as the default font family name")]
fn should_throw_when_default_family_name_is_the_placeholder() {
    let _scope = TestFontScope::with_font_manager(TestFontManagerImpl::new(FontFamily::DEFAULT_FONT_FAMILY_NAME));

    FontManager::current();
}

#[test]
fn should_use_font_manager_options_default_family_name() {
    let _scope = TestFontScope::start();

    bind_options(FontManagerOptions { default_family_name: Some("MyFont".to_owned()), ..Default::default() });

    assert_eq!(FontManager::current().default_font_family().name(), "MyFont");
}

#[test]
fn should_use_font_manager_options_font_fallback() {
    let _scope = TestFontScope::start();

    bind_options(FontManagerOptions {
        font_fallbacks: Some(vec![FontFallback {
            font_family: FontFamily::new("MyFont"),
            unicode_range: UnicodeRange::default_range(),
        }]),
        ..Default::default()
    });

    let typeface = match_character('A' as i32, Some(&FontFamily::default_family())).unwrap();

    assert_eq!(typeface.font_family().name(), "MyFont");
}

#[test]
fn font_fallbacks_that_do_not_cover_the_codepoint_are_skipped() {
    let _scope = TestFontScope::start();

    bind_options(FontManagerOptions {
        font_fallbacks: Some(vec![
            // In range, but the font (the default one) has no emoji.
            FontFallback { font_family: FontFamily::new("MyFont"), unicode_range: UnicodeRange::default_range() },
            // Not in range.
            FontFallback { font_family: FontFamily::new("Noto Sans"), unicode_range: UnicodeRange::new(0, 0x7F) },
        ]),
        ..Default::default()
    });

    let typeface = match_character(0x1F600, None).unwrap();

    assert_eq!(typeface.font_family().name(), "Twitter Color Emoji");
}

#[test]
fn should_return_first_installed_font_family_name_when_default_family_name_is_null() {
    let font_manager_impl = TestFontManagerImpl::new("");

    font_manager_impl.set_installed_font_family_names(Some(vec!["DejaVu".to_owned(), "Verdana".to_owned()]));

    let _scope = TestFontScope::with_font_manager(font_manager_impl);

    assert_eq!(FontManager::current().default_font_family().name(), "DejaVu");
}

#[test]
fn try_get_glyph_typeface_should_recreate_removed_embedded_collections() {
    let _scope = TestFontScope::with_assets();

    let font_manager = FontManager::current();

    let font_uri = font_uri();
    let collection_key = uri(test_fonts::ASSETS);

    // Warm up to validate the font URI is correct.
    assert!(font_manager.try_get_glyph_typeface(&Typeface::new(FontFamily::new(&font_uri))).is_some());

    for _ in 0..10 {
        font_manager.remove_font_collection(&collection_key);

        assert!(font_manager.try_get_glyph_typeface(&Typeface::new(FontFamily::new(&font_uri))).is_some());
        assert!(font_manager.try_get_glyph_typeface(&Typeface::new(FontFamily::new(&font_uri))).is_some());
    }
}

#[test]
fn current_is_registered_with_the_locator_and_dispose_releases_everything() {
    let scope = TestFontScope::start();

    let font_manager = FontManager::current();

    assert!(Rc::ptr_eq(&font_manager, &FontManager::current()));
    assert!(std::ptr::addr_eq(Rc::as_ptr(font_manager.platform_impl()), Rc::as_ptr(scope.font_manager_impl())));

    let key = uri("fonts:MyTest");
    let stub = StubFontCollection::new(&key);

    font_manager.add_font_collection(stub.clone());

    let glyph_typeface = Typeface::from_name("Noto Mono").glyph_typeface();

    font_manager.dispose();

    assert!(stub.is_disposed.get());
    assert!(scope.font_manager_impl().is_disposed());
    assert!(glyph_typeface.platform_typeface().try_get_stream().is_none());
    assert!(font_manager.try_get_font_collection(&key).is_none());
}

// ── try_get_font_collection ──

#[test]
fn try_get_font_collection_system_font_scheme_returns_true() {
    let _scope = TestFontScope::start();

    let source = uri(&format!("{}:Arial", FontManager::SYSTEM_FONT_SCHEME));

    assert!(FontManager::current().try_get_font_collection(&source).is_some());
}

#[test]
fn try_get_font_collection_system_fonts_key_returns_true() {
    let _scope = TestFontScope::start();

    assert!(FontManager::current().try_get_font_collection(&FontManager::system_fonts_key()).is_some());
}

#[test]
fn try_get_font_collection_absolute_resm_returns_true() {
    let _scope = TestFontScope::with_assets();

    let source = uri(test_fonts::ASSETS);

    assert!(FontManager::current().try_get_font_collection(&source).is_some());
}

#[test]
fn try_get_font_collection_ferres_returns_true() {
    let _scope = TestFontScope::start();

    let source = uri("ferres://FerroUI.Base.UnitTests/Assets");

    assert!(FontManager::current().try_get_font_collection(&source).is_some());
}

#[test]
fn try_get_font_collection_system_font_scheme_yields_system_font_collection() {
    let _scope = TestFontScope::start();

    let source = uri(&format!("{}:Arial", FontManager::SYSTEM_FONT_SCHEME));

    let collection = FontManager::current().try_get_font_collection(&source).unwrap();

    assert!(collection.as_any().is::<SystemFontCollection>());
}

#[test]
fn try_get_font_collection_system_font_scheme_returns_same_instance_on_subsequent_calls() {
    let _scope = TestFontScope::start();

    let source = uri(&format!("{}:Arial", FontManager::SYSTEM_FONT_SCHEME));
    let fm = FontManager::current();

    let first = fm.try_get_font_collection(&source).unwrap();
    let second = fm.try_get_font_collection(&source).unwrap();

    assert!(same(&first, &second));
}

#[test]
fn try_get_font_collection_system_fonts_key_yields_system_font_collection() {
    let _scope = TestFontScope::start();

    let collection = FontManager::current().try_get_font_collection(&FontManager::system_fonts_key()).unwrap();

    assert!(collection.as_any().is::<SystemFontCollection>());
}

#[test]
fn try_get_font_collection_system_font_scheme_and_system_fonts_key_return_same_instance() {
    let _scope = TestFontScope::start();

    let fm = FontManager::current();
    let scheme_source = uri(&format!("{}:Arial", FontManager::SYSTEM_FONT_SCHEME));

    let from_scheme = fm.try_get_font_collection(&scheme_source).unwrap();
    let from_key = fm.try_get_font_collection(&FontManager::system_fonts_key()).unwrap();

    assert!(same(&from_scheme, &from_key));
    assert!(same(&from_scheme, &fm.system_fonts()));
}

#[test]
fn try_get_font_collection_registered_fonts_collection_returns_true() {
    let _scope = TestFontScope::start();

    let key = uri("fonts:MyTest");
    let stub: Rc<dyn IFontCollection> = StubFontCollection::new(&key);
    let fm = FontManager::current();

    fm.add_font_collection(stub.clone());

    let collection = fm.try_get_font_collection(&key).unwrap();

    assert!(same(&stub, &collection));
}

#[test]
fn try_get_font_collection_unregistered_fonts_collection_returns_false() {
    let _scope = TestFontScope::start();

    let key = uri("fonts:DoesNotExist");

    assert!(FontManager::current().try_get_font_collection(&key).is_none());
}

#[test]
fn try_get_font_collection_unregistered_fonts_collection_does_not_cache_null() {
    let _scope = TestFontScope::start();

    let key = uri("fonts:DoesNotExist2");
    let fm = FontManager::current();

    // First call returns nothing
    assert!(fm.try_get_font_collection(&key).is_none());

    // Register after the first failed lookup
    let stub: Rc<dyn IFontCollection> = StubFontCollection::new(&key);

    fm.add_font_collection(stub.clone());

    // Now it should be found; if the miss had been cached this would still fail
    let collection = fm.try_get_font_collection(&key).unwrap();

    assert!(same(&stub, &collection));
}

#[test]
fn try_get_font_collection_absolute_resm_yields_embedded_font_collection() {
    let _scope = TestFontScope::with_assets();

    let source = uri(test_fonts::ASSETS);

    let collection = FontManager::current().try_get_font_collection(&source).unwrap();

    assert!(collection.as_any().is::<EmbeddedFontCollection>());
    assert!(collection.count() > 0);
}

#[test]
fn try_get_font_collection_absolute_resm_returns_same_instance_on_subsequent_calls() {
    let _scope = TestFontScope::with_assets();

    let source = uri(test_fonts::ASSETS);
    let fm = FontManager::current();

    let first = fm.try_get_font_collection(&source).unwrap();
    let second = fm.try_get_font_collection(&source).unwrap();

    assert!(same(&first, &second));
}

#[test]
fn try_get_font_collection_ferres_yields_embedded_font_collection() {
    let _scope = TestFontScope::start();

    let source = uri("ferres://FerroUI.Base.UnitTests/Assets");

    let collection = FontManager::current().try_get_font_collection(&source).unwrap();

    assert!(collection.as_any().is::<EmbeddedFontCollection>());
}

#[test]
fn try_get_font_collection_ferres_returns_same_instance_on_subsequent_calls() {
    let _scope = TestFontScope::start();

    let source = uri("ferres://FerroUI.Base.UnitTests/Assets");
    let fm = FontManager::current();

    let first = fm.try_get_font_collection(&source).unwrap();
    let second = fm.try_get_font_collection(&source).unwrap();

    assert!(same(&first, &second));
}

#[test]
fn try_get_font_collection_unknown_scheme_returns_false() {
    let _scope = TestFontScope::start();

    let source = uri("https://example.com/fonts");

    assert!(FontManager::current().try_get_font_collection(&source).is_none());
}

#[test]
fn try_get_font_collection_unknown_scheme_does_not_cache_null() {
    let _scope = TestFontScope::start();

    // Verify that repeated lookups for the same unknown-scheme URI
    // consistently return nothing rather than succeeding due to an
    // accidentally cached invalid entry.
    let source = uri("file:///some/path/fonts");
    let fm = FontManager::current();

    assert!(fm.try_get_font_collection(&source).is_none());
    assert!(fm.try_get_font_collection(&source).is_none());
}

#[test]
#[should_panic(expected = "Font collection Key should follow the fonts: scheme.")]
fn add_font_collection_rejects_keys_of_other_schemes() {
    let _scope = TestFontScope::start();

    FontManager::current().add_font_collection(StubFontCollection::new(&uri("resm:Not.Fonts")));
}

#[test]
fn add_font_collection_replaces_and_disposes_the_collection_of_the_same_key() {
    let _scope = TestFontScope::start();

    let key = uri("fonts:MyTest");
    let fm = FontManager::current();

    let first = StubFontCollection::new(&key);
    let second = StubFontCollection::new(&key);

    fm.add_font_collection(first.clone());
    fm.add_font_collection(second.clone());

    assert!(first.is_disposed.get());
    assert!(!second.is_disposed.get());

    fm.remove_font_collection(&key);

    assert!(second.is_disposed.get());
    assert!(fm.try_get_font_collection(&key).is_none());
}

// ── glyph typefaces ──

#[test]
fn should_create_typeface_from_fallback() {
    let _scope = TestFontScope::start();

    let glyph_typeface =
        Typeface::new(FontFamily::new(&format!("A, B, {}", FontFamily::DEFAULT_FONT_FAMILY_NAME))).glyph_typeface();

    assert_eq!(glyph_typeface.family_name(), "Noto Mono");
}

#[test]
fn should_yield_default_glyph_typeface_for_invalid_family_name() {
    let _scope = TestFontScope::start();

    let glyph_typeface = Typeface::new(FontFamily::new("Unknown")).glyph_typeface();

    assert_eq!(glyph_typeface.family_name(), FontManager::current().default_font_family().name());
}

#[test]
fn should_load_typeface_from_resource() {
    let _scope = TestFontScope::with_assets();

    let glyph_typeface = Typeface::from_name(&font_uri()).glyph_typeface();

    assert_eq!(glyph_typeface.family_name(), "Noto Mono");
}

#[test]
fn should_load_nearest_matching_font() {
    let _scope = TestFontScope::with_assets();

    let glyph_typeface =
        Typeface::from_name_with_style(&font_uri(), FontStyle::Italic, FontWeight::Black, FontStretch::Normal)
            .glyph_typeface();

    assert_eq!(glyph_typeface.family_name(), "Noto Mono");
}

#[test]
#[should_panic(expected = "Could not create glyphTypeface.")]
fn should_throw_for_invalid_custom_font() {
    let _scope = TestFontScope::with_assets();

    Typeface::from_name(&format!("{}#Unknown", test_fonts::ASSETS)).glyph_typeface();
}

#[test]
fn should_return_none_for_unregistered_font_collection_uri() {
    let _scope = TestFontScope::start();

    assert!(FontManager::current().try_get_glyph_typeface(&Typeface::from_name("fonts:invalid#Something")).is_none());
}

#[test]
fn should_only_try_to_create_glyph_typeface_once() {
    let scope = TestFontScope::start();

    assert!(FontManager::current().try_get_glyph_typeface(&Typeface::default()).is_some());

    let count_before = scope.font_manager_impl().create_typeface_calls();

    for _ in 0..10 {
        FontManager::current().try_get_glyph_typeface(&Typeface::from_name("Unknown"));
    }

    assert_eq!(scope.font_manager_impl().create_typeface_calls(), count_before + 1);
}

#[test]
fn should_cache_match_character() {
    let _scope = TestFontScope::start();

    let emoji = 0x1F600;

    let first_glyph_typeface = match_character(emoji, None).unwrap().glyph_typeface();
    let second_glyph_typeface = match_character(emoji, None).unwrap().glyph_typeface();

    assert!(Rc::ptr_eq(&first_glyph_typeface, &second_glyph_typeface));
    assert_eq!(first_glyph_typeface.family_name(), "Twitter Color Emoji");
}

#[test]
fn should_load_embedded_default_font_family() {
    let font_manager_impl = TestFontManagerImpl::new("Unused");

    let _scope = TestFontScope::with_services(font_manager_impl, test_fonts::asset_loader());

    bind_options(FontManagerOptions { default_family_name: Some(font_uri()), ..Default::default() });

    let glyph_typeface = FontManager::current().try_get_glyph_typeface(&Typeface::default()).unwrap();

    assert_eq!(glyph_typeface.family_name(), "Noto Mono");
}

#[test]
fn should_return_none_for_invalid_default_font_family() {
    let _scope = TestFontScope::with_assets();

    bind_options(FontManagerOptions {
        default_family_name: Some(format!("{}#Unknown", test_fonts::ASSETS)),
        ..Default::default()
    });

    assert!(FontManager::current().try_get_glyph_typeface(&Typeface::default()).is_none());
}

#[test]
fn should_load_embedded_fallbacks() {
    let _scope = TestFontScope::with_assets();

    let font_family = FontFamily::parse(&format!("NotFound, {}", font_uri())).unwrap();

    let glyph_typeface = Typeface::new(font_family).glyph_typeface();

    assert_eq!(glyph_typeface.family_name(), "Noto Mono");
}

#[test]
fn should_match_character_with_embedded_fallbacks() {
    let _scope = TestFontScope::with_assets();

    let font_family = FontFamily::parse(&format!("NotFound, {}", font_uri())).unwrap();

    let typeface = match_character('A' as i32, Some(&font_family)).unwrap();

    assert!(typeface.font_family().key().is_some());
    assert_eq!(typeface.glyph_typeface().family_name(), "Noto Mono");
}

#[test]
fn should_match_character_from_system_fonts() {
    let _scope = TestFontScope::start();

    let typeface = match_character('A' as i32, None).unwrap();

    assert_eq!(typeface.glyph_typeface().family_name(), FontManager::current().default_font_family().name());
}

#[test]
fn should_match_character_with_fallbacks() {
    for (family_name, base_uri) in [
        ("NotFound, Unknown", None),                            // system fonts
        ("/#NotFound, /#Unknown", Some("ferres://some/path")), // embedded fonts
    ] {
        let _scope = TestFontScope::with_assets();

        let base_uri = base_uri.map(uri);

        let font_family = FontFamily::parse_with_base_uri(family_name, base_uri.as_ref()).unwrap();

        let typeface = match_character('A' as i32, Some(&font_family)).unwrap();

        assert_eq!(typeface.glyph_typeface().family_name(), FontManager::current().default_font_family().name());
    }
}

fn add_embedded_system_fonts() {
    FontManager::current().add_font_collection(Rc::new(EmbeddedFontCollection::new(
        FontManager::system_fonts_key(),
        uri(test_fonts::ASSETS),
    )));
}

#[test]
fn should_use_custom_system_font() {
    let _scope = TestFontScope::with_services(TestFontManagerImpl::new("Noto Mono"), test_fonts::asset_loader());

    add_embedded_system_fonts();

    let glyph_typeface = FontManager::current().try_get_glyph_typeface(&Typeface::from_name("Noto Mono")).unwrap();

    assert_eq!(glyph_typeface.family_name(), "Noto Mono");
    assert!(FontManager::current().system_fonts().as_any().is::<EmbeddedFontCollection>());
}

#[test]
fn should_get_nearest_match_for_custom_system_font() {
    let _scope = TestFontScope::with_services(TestFontManagerImpl::new("Noto Mono"), test_fonts::asset_loader());

    add_embedded_system_fonts();

    let typeface =
        Typeface::from_name_with_style("Noto Mono", FontStyle::Italic, FontWeight::Normal, FontStretch::Normal);

    let glyph_typeface = FontManager::current().try_get_glyph_typeface(&typeface).unwrap();

    assert_eq!(glyph_typeface.family_name(), "Noto Mono");
}

#[test]
fn should_get_implicit_typeface() {
    let _scope = TestFontScope::with_services(TestFontManagerImpl::new("Noto Mono"), test_fonts::asset_loader());

    add_embedded_system_fonts();

    let glyph_typeface =
        FontManager::current().try_get_glyph_typeface(&Typeface::from_name("Noto Mono Italic")).unwrap();

    assert_eq!(glyph_typeface.family_name(), "Noto Mono");
    assert_eq!(glyph_typeface.style(), FontStyle::Italic);
}

#[test]
fn should_create_synthetic_typeface() {
    let _scope = TestFontScope::with_services(TestFontManagerImpl::new("Noto Mono"), test_fonts::asset_loader());

    add_embedded_system_fonts();

    let italic_bold_typeface = FontManager::current()
        .try_get_glyph_typeface(&Typeface::from_name_with_style(
            "Noto Mono",
            FontStyle::Italic,
            FontWeight::Bold,
            FontStretch::Normal,
        ))
        .unwrap();

    assert_eq!(italic_bold_typeface.family_name(), "Noto Mono");
    assert!(italic_bold_typeface.platform_typeface().font_simulations().contains(FontSimulations::Bold));
    assert!(italic_bold_typeface.platform_typeface().font_simulations().contains(FontSimulations::Oblique));

    let regular_typeface = FontManager::current().try_get_glyph_typeface(&Typeface::from_name("Noto Mono")).unwrap();

    assert!(!Rc::ptr_eq(regular_typeface.platform_typeface(), italic_bold_typeface.platform_typeface()));
    assert_eq!(regular_typeface.font_simulations(), FontSimulations::None);
}

#[test]
fn should_get_glyph_typeface_by_localized_family_name() {
    let font_manager_impl = TestFontManagerImpl::new("Test Gothic");

    let _scope = TestFontScope::with_font_manager(font_manager_impl);

    let font_collection = Rc::new(EmbeddedFontCollection::new(uri("fonts:Localized"), uri("fonts:Localized")));

    let bytes = TestFontBuilder::new("Test Gothic").localized_family_name(0x0411, "\u{30C6}\u{30B9}\u{30C8}").build_bytes();

    assert!(FontCollectionBase::try_add_glyph_typeface_from_stream(&*font_collection, &mut bytes.as_slice()).is_some());

    FontManager::current().add_font_collection(font_collection);

    let glyph_typeface = FontManager::current()
        .try_get_glyph_typeface(&Typeface::from_name("fonts:Localized#\u{30C6}\u{30B9}\u{30C8}"))
        .unwrap();

    assert_eq!(glyph_typeface.family_name(), "Test Gothic");
}

fn inter_mapping_options(default_family_name: String) -> FontManagerOptions {
    FontManagerOptions {
        default_family_name: Some(default_family_name),
        font_family_mappings: Some(HashMap::from([("Segoe UI".to_owned(), FontFamily::new("fonts:Inter#Inter"))])),
        ..Default::default()
    }
}

#[test]
fn should_map_font_family() {
    let _scope = TestFontScope::with_assets();

    bind_options(inter_mapping_options(font_uri()));

    FontManager::current().add_font_collection(InterFontCollection::new());

    let glyph_typeface = FontManager::current().try_get_glyph_typeface(&Typeface::from_name("Abc, Segoe UI")).unwrap();

    assert_eq!(glyph_typeface.family_name(), "Inter");
}

#[test]
fn should_map_font_family_regardless_of_casing() {
    let _scope = TestFontScope::with_assets();

    bind_options(inter_mapping_options(font_uri()));

    FontManager::current().add_font_collection(InterFontCollection::new());

    // The mapping table is configuration; its casing has nothing to do with the casing
    // a control asks in. Both the composite and the plain family path must still find
    // the mapping.
    let from_composite =
        FontManager::current().try_get_glyph_typeface(&Typeface::from_name("Abc, segoe ui")).unwrap();

    assert_eq!(from_composite.family_name(), "Inter");

    let from_plain_family = FontManager::current().try_get_glyph_typeface(&Typeface::from_name("SEGOE UI")).unwrap();

    assert_eq!(from_plain_family.family_name(), "Inter");
}

#[test]
fn should_map_font_family_to_a_system_font() {
    let _scope = TestFontScope::start();

    bind_options(FontManagerOptions {
        font_family_mappings: Some(HashMap::from([("Segoe UI".to_owned(), FontFamily::new("Noto Sans"))])),
        ..Default::default()
    });

    // A mapped family without a key is looked up in the system fonts, also inside a composite family.
    for name in ["Segoe UI", "Abc, Segoe UI"] {
        let glyph_typeface = FontManager::current().try_get_glyph_typeface(&Typeface::from_name(name)).unwrap();

        assert_eq!(glyph_typeface.family_name(), "Noto Sans", "{name}");
    }
}

#[test]
fn should_get_family_typefaces() {
    let _scope = TestFontScope::start();

    FontManager::current().add_font_collection(InterFontCollection::new());

    let family_typefaces = FontManager::current().get_family_typefaces(&FontFamily::new("fonts:Inter#Inter"));

    assert_eq!(family_typefaces.len(), 5);
    assert_eq!(FontFamily::new("fonts:Inter#Inter").family_typefaces().len(), 5);

    // System families ask the platform; unknown families have no typefaces.
    assert_eq!(FontManager::current().get_family_typefaces(&FontFamily::new("Noto Mono")).len(), 1);
    assert!(FontManager::current().get_family_typefaces(&FontFamily::new("Unknown")).is_empty());
    assert!(FontManager::current().get_family_typefaces(&FontFamily::new("fonts:Unknown#Inter")).is_empty());
}

fn add_my_collection() {
    FontManager::current().add_font_collection(Rc::new(EmbeddedFontCollection::new(
        uri("fonts:MyCollection"), // key
        uri(test_fonts::ASSETS),   // source
    )));
}

#[test]
fn should_use_font_collection_match_character() {
    let _scope = TestFontScope::with_assets();

    add_my_collection();

    let font_family = FontFamily::new("fonts:MyCollection#Noto Mono");

    let typeface = match_character(0x5D0, Some(&font_family)).unwrap();

    // The typeface should come from the font collection
    assert!(typeface.font_family().key().is_some());
    assert_eq!(typeface.glyph_typeface().family_name(), "Noto Sans Hebrew");
}

#[test]
fn should_use_last_resort_font_last_match_character() {
    let _scope = TestFontScope::with_assets();

    add_my_collection();

    let font_family = FontFamily::new("fonts:MyCollection#Noto Sans");

    // The typeface should come from the font collection - falling back to the Hebrew font
    let typeface1 = match_character(0x5D0, Some(&font_family)).unwrap();

    assert!(typeface1.font_family().key().is_some());
    assert_eq!(typeface1.glyph_typeface().family_name(), "Noto Sans Hebrew");

    // The typeface should come from the font collection - falling back to the last resort font
    let typeface2 = match_character(0x2A736, Some(&font_family)).unwrap();

    assert!(typeface2.font_family().key().is_some());
    assert_eq!(typeface2.glyph_typeface().family_name(), "Adobe Blank 2 VF R");
}

#[test]
fn should_use_a_foreign_font_collection_for_character_matching() {
    use crate::media::text_formatting::unicode::Script;

    let _scope = TestFontScope::start();

    let key = uri("fonts:Stub");

    FontManager::current().add_font_collection(StubFontCollection::new(&key));

    // A collection that does not derive from the font collection base gets the unconstrained match.
    let typeface = FontManager::current()
        .try_match_character_with_script(
            0x627,
            FontStyle::Normal,
            FontWeight::Normal,
            FontStretch::Normal,
            Some(&FontFamily::new("fonts:Stub#Anything")),
            None,
            Script::Arabic,
        )
        .unwrap();

    assert_eq!(typeface.font_family().name(), "Noto Sans");
}

#[test]
fn should_get_system_font_with_base_uri() {
    for name in ["Noto Sans", "#Noto Sans"] {
        let _scope = TestFontScope::start();

        let font_family = FontFamily::with_base_uri(Some(&uri("ferres://FerroUI.UnitTests/NotFound")), name);

        let glyph_typeface = Typeface::new(font_family).glyph_typeface();

        assert_eq!(glyph_typeface.family_name(), "Noto Sans");
    }
}

#[test]
fn should_fallback_when_font_family_is_empty() {
    let _scope = TestFontScope::start();

    let typeface = Typeface::from_name("");

    assert_eq!(typeface.glyph_typeface().family_name(), "Noto Mono");
}

#[test]
fn try_get_glyph_typeface_should_return_none_for_font_without_supported_cmap() {
    let scope = TestFontScope::start();

    // A font whose only character map is in a format that is not supported.
    let mut font = test_fonts::inter_regular().build_font();

    font.patch_uint16("cmap", 12, 6);

    let asset = "resm:FerroUI.UnitTests.Fonts.TestFontNoCmap.ttf?assembly=FerroUI.UnitTests";

    scope.asset_loader().add_asset(asset, font.to_bytes());

    assert!(FontManager::current().try_get_glyph_typeface(&Typeface::from_name(&format!("{asset}#Inter"))).is_none());
}

#[test]
fn try_match_character_should_return_correct_weight() {
    for (requested_weight, expected_family_name) in [
        (FontWeight::Normal, "Inter"),
        (FontWeight::Bold, "Inter"),
        (FontWeight::SemiBold, "Inter SemiBold"),
        (FontWeight::SemiLight, "Inter Light"),
    ] {
        let _scope = TestFontScope::start();

        FontManager::current().add_font_collection(InterFontCollection::new());

        let typeface = FontManager::current()
            .try_match_character(
                'A' as i32,
                FontStyle::Normal,
                requested_weight,
                FontStretch::Normal,
                Some(&FontFamily::new("fonts:Inter#Inter")),
                None,
            )
            .unwrap();

        assert_eq!(typeface.glyph_typeface().family_name(), expected_family_name);
        assert_eq!(typeface.weight(), requested_weight);
    }
}

#[test]
fn try_match_character_should_return_correct_stretch() {
    for requested_stretch in [
        FontStretch::Normal,
        FontStretch::Condensed,
        FontStretch::Expanded,
        FontStretch::SemiCondensed,
        FontStretch::SemiExpanded,
    ] {
        let _scope = TestFontScope::start();

        FontManager::current().add_font_collection(InterFontCollection::new());

        let typeface = FontManager::current()
            .try_match_character(
                'A' as i32,
                FontStyle::Normal,
                FontWeight::Normal,
                requested_stretch,
                Some(&FontFamily::new("fonts:Inter#Inter")),
                None,
            )
            .unwrap();

        assert_eq!(typeface.glyph_typeface().family_name(), "Inter");
        assert_eq!(typeface.stretch(), requested_stretch);
    }
}

#[test]
fn try_get_glyph_typeface_should_use_perfect_match_in_collection_before_nearest_match() {
    let font_manager_impl = TestFontManagerImpl::new("Inter");

    font_manager_impl.add_font(test_fonts::inter_bold().build());
    font_manager_impl.add_font(test_fonts::inter_regular().build());

    let _scope = TestFontScope::with_font_manager(font_manager_impl);

    let get = |weight: FontWeight| {
        FontManager::current()
            .try_get_glyph_typeface(&Typeface::from_name_with_style(
                "Inter",
                FontStyle::Normal,
                weight,
                FontStretch::Normal,
            ))
            .unwrap()
    };

    // Load the bold font first
    let bold_glyph_typeface = get(FontWeight::Bold);

    assert_eq!(bold_glyph_typeface.family_name(), "Inter");
    assert_eq!(bold_glyph_typeface.weight(), FontWeight::Bold);

    // The normal font should be loaded since it's a perfect match, instead of falling back
    let regular_glyph_typeface = get(FontWeight::Normal);

    assert!(!Rc::ptr_eq(&regular_glyph_typeface, &bold_glyph_typeface));
    assert_eq!(regular_glyph_typeface.family_name(), "Inter");
    assert_eq!(regular_glyph_typeface.weight(), FontWeight::Normal);

    // Nearest match should still work (650 falls back to 700 Bold)
    let nearest_match_typeface = get(FontWeight(650));

    assert!(Rc::ptr_eq(&bold_glyph_typeface, &nearest_match_typeface));
}

#[test]
fn try_get_glyph_typeface_should_cache_matched_glyph_typeface_under_requested_family_name() {
    let font_manager_impl = TestFontManagerImpl::new("Noto Mono");

    font_manager_impl.add_font(test_fonts::noto_mono().build());
    font_manager_impl.add_alias("NotInstalled", "Noto Mono");

    let scope = TestFontScope::with_font_manager(font_manager_impl);

    // "NotInstalled" is not installed, so the platform substitutes it with a different
    // family ("Noto Mono").
    let first = FontManager::current().try_get_glyph_typeface(&Typeface::from_name("NotInstalled")).unwrap();

    assert_eq!(first.family_name(), "Noto Mono");

    // The substitute should now be cached under the requested "NotInstalled" name, so a second
    // lookup must resolve from the cache instead of asking the platform again.
    let second = FontManager::current().try_get_glyph_typeface(&Typeface::from_name("NotInstalled")).unwrap();

    assert!(Rc::ptr_eq(&first, &second));
    assert_eq!(scope.font_manager_impl().create_typeface_calls_for("NotInstalled"), 1);
}
