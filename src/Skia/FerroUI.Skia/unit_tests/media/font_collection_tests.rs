//! Port of upstream's `Media/FontCollectionTests.cs` of the Skia unit tests.
//!
//! Upstream's test collections derive from `SystemFontCollection` and
//! `EmbeddedFontCollection`. `TestSystemFontCollection` only adds an
//! accessor of the glyph typeface cache, so here it wraps a
//! `SystemFontCollection` and dereferences to it; `CustomizableFontCollection`
//! overrides members, so it holds an `EmbeddedFontCollection` and forwards
//! the base class state to it.

use crate::unit_tests::media::CustomFontManagerImpl;
use crate::unit_tests::mock_platform_render_interface;
use crate::FontManagerImpl;
use ferroui_base::media::fonts::{
    EmbeddedFontCollection, FontCollectionBase, FontCollectionBaseImpl, FontCollectionKey, GlyphTypefaceCache,
    IFontCollection, SystemFontCollection,
};
use ferroui_base::media::{
    FontFallback, FontFamily, FontManager, FontSimulations, FontStretch, FontStyle, FontWeight, GlyphTypeface,
    IPlatformTypeface, Typeface, UnicodeRange,
};
use ferroui_base::platform::{IAssetLoader, IFontManagerImpl};
use ferroui_base::utilities::{CultureInfo, Uri, UriKind};
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_controls::testing::UnitTestApplication;
use std::cell::Cell;
use std::io::Read;
use std::ops::Deref;
use std::rc::Rc;

const NOTO_MONO: &str = "resm:FerroUI.Skia.UnitTests.Assets?assembly=ferroui-skia";

#[test]
#[cfg_attr(not(windows), ignore = "Relies on some installed font family")]
fn should_cache_nearest_match() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let font_collection = TestSystemFontCollection::new(FontManager::current().platform_impl().clone());

    let glyph_typeface = font_collection
        .try_get_glyph_typeface("Arial", FontStyle::Normal, FontWeight::ExtraBlack, FontStretch::Normal)
        .expect("Arial");

    let glyph_typefaces = font_collection.glyph_typeface_cache().try_get_value("Arial").expect("Arial is cached");

    assert_eq!(2, glyph_typefaces.borrow().len());

    assert!(glyph_typefaces.borrow().contains_key(&FontCollectionKey::new(
        FontStyle::Normal,
        FontWeight::Black,
        FontStretch::Normal
    )));

    let other_glyph_typeface = font_collection.try_get_glyph_typeface(
        "Arial",
        FontStyle::Normal,
        FontWeight::ExtraBlack,
        FontStretch::Normal,
    );

    assert!(other_glyph_typeface.is_some_and(|other| Rc::ptr_eq(&glyph_typeface, &other)));
}

struct TestSystemFontCollection {
    base: SystemFontCollection,
}

impl TestSystemFontCollection {
    fn new(platform_impl: Rc<dyn IFontManagerImpl>) -> Self {
        Self { base: SystemFontCollection::new(platform_impl) }
    }

    fn glyph_typeface_cache(&self) -> &GlyphTypefaceCache {
        SystemFontCollection::base(&self.base).glyph_typeface_cache()
    }
}

impl Deref for TestSystemFontCollection {
    type Target = SystemFontCollection;

    fn deref(&self) -> &SystemFontCollection {
        &self.base
    }
}

#[test]
fn should_use_fallback() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(CustomFontManagerImpl::new())),
    );

    let source = Uri::new(NOTO_MONO, UriKind::Absolute).unwrap();

    let fallback = FontFallback { font_family: FontFamily::new("Arial"), unicode_range: UnicodeRange::new('A' as i32, 'A' as i32) };

    let font_collection = CustomizableFontCollection::new(source.clone(), source, Some(vec![fallback]), None);

    let matched = font_collection
        .try_match_character('A' as i32, FontStyle::Normal, FontWeight::Normal, FontStretch::Normal, None, None)
        .expect("a match");

    assert_eq!("Arial", matched.font_family().name());
}

#[test]
fn should_ignore_font_family() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(CustomFontManagerImpl::new())),
    );

    let key = Uri::new(NOTO_MONO, UriKind::Absolute).unwrap();

    let ignorable = FontFamily::with_base_uri(Some(&Uri::new(NOTO_MONO, UriKind::Absolute).unwrap()), "Noto Mono");

    let font_collection = CustomizableFontCollection::new(key.clone(), key, None, Some(vec![ignorable.clone()]));

    let typeface = Typeface::new(ignorable);

    let _glyph_typeface = typeface.glyph_typeface();

    assert!(font_collection
        .try_create_synthetic_glyph_typeface(
            &typeface.glyph_typeface(),
            FontStyle::Italic,
            FontWeight::DemiBold,
            FontStretch::Normal,
        )
        .is_none());
}

struct CustomizableFontCollection {
    base: EmbeddedFontCollection,
    fallbacks: Option<Vec<FontFallback>>,
    ignorables: Option<Vec<FontFamily>>,
}

impl CustomizableFontCollection {
    fn new(
        key: Uri,
        source: Uri,
        fallbacks: Option<Vec<FontFallback>>,
        ignorables: Option<Vec<FontFamily>>,
    ) -> Self {
        Self { base: EmbeddedFontCollection::new(key, source), fallbacks, ignorables }
    }
}

impl FontCollectionBaseImpl for CustomizableFontCollection {
    fn base(this: &Self) -> &FontCollectionBase {
        <EmbeddedFontCollection as FontCollectionBaseImpl>::base(&this.base)
    }

    fn key(this: &Self) -> Uri {
        <EmbeddedFontCollection as FontCollectionBaseImpl>::key(&this.base)
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
        if let Some(fallbacks) = &this.fallbacks {
            for fallback in fallbacks {
                if fallback.unicode_range.is_in_range(codepoint) {
                    return Some(Typeface::with_style(fallback.font_family.clone(), style, weight, stretch));
                }
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
        if let Some(ignorables) = &this.ignorables {
            for ignorable in ignorables {
                if glyph_typeface.family_name() == ignorable.name()
                    || glyph_typeface.typographic_family_name() == ignorable.name()
                {
                    return None;
                }
            }
        }

        FontCollectionBase::try_create_synthetic_glyph_typeface(this, glyph_typeface, style, weight, stretch)
    }
}

#[test]
fn should_cache_synthetic_match_under_requested_family_name() {
    let font_manager = Rc::new(AliasFontManagerImpl::new("MyAlias"));

    let _app = UnitTestApplication::start(mock_platform_render_interface().with_font_manager_impl(font_manager.clone()));

    let font_collection = TestSystemFontCollection::new(font_manager.clone());
    let black_key = FontCollectionKey::new(FontStyle::Normal, FontWeight::Black, FontStretch::Normal);

    // Prime the cache with the bare family, as any control asking for the alias at a
    // normal weight would. This is what makes the next lookup take the nearest match.
    assert!(font_collection
        .try_get_glyph_typeface("MyAlias", FontStyle::Normal, FontWeight::Normal, FontStretch::Normal)
        .is_some());

    let first = font_collection
        .try_get_glyph_typeface("MyAlias", FontStyle::Normal, FontWeight::Black, FontStretch::Normal)
        .expect("the alias at black weight");

    // Guards the test itself: the first resolution must really be a synthesised bold.
    // If the backing font could not be emboldened, TryCreateSyntheticGlyphTypeface would
    // fail and the old else branch would cache the nearest match, making everything below
    // pass against unfixed code.
    assert_eq!(FontSimulations::Bold, first.font_simulations());

    let creations_after_first_call = font_manager.stream_typeface_creations();

    for i in 0..10 {
        let next = font_collection
            .try_get_glyph_typeface("MyAlias", FontStyle::Normal, FontWeight::Black, FontStretch::Normal)
            .expect("the alias at black weight");

        assert!(Rc::ptr_eq(&first, &next), "{i}");
    }

    // Each synthesis copies the entire font file through IPlatformTypeface.TryGetStream,
    // so an uncached synthetic means one full font copy per call.
    assert_eq!(creations_after_first_call, font_manager.stream_typeface_creations());

    let cached = font_collection.glyph_typeface_cache().try_get_value("MyAlias").expect("the alias is cached");
    assert!(cached.borrow().contains_key(&black_key));
}

#[test]
fn should_ignore_family_name_casing_when_resolving_a_synthetic_match() {
    let font_manager = Rc::new(AliasFontManagerImpl::new("MyAlias"));

    let _app = UnitTestApplication::start(mock_platform_render_interface().with_font_manager_impl(font_manager.clone()));

    let font_collection = TestSystemFontCollection::new(font_manager.clone());

    assert!(font_collection
        .try_get_glyph_typeface("MyAlias", FontStyle::Normal, FontWeight::Normal, FontStretch::Normal)
        .is_some());

    // Casing must not decide whether a request gets a synthesised bold. A cache keyed
    // ordinally sends this lookup past the synthesis branch and down the family-name
    // search, which returns the nearest match raw - so the very same family renders
    // faux-bold under one casing and regular weight under another.
    let upper_case = font_collection
        .try_get_glyph_typeface("MYALIAS", FontStyle::Normal, FontWeight::Black, FontStretch::Normal)
        .expect("the alias in upper case");

    assert_eq!(FontSimulations::Bold, upper_case.font_simulations());

    let creations_after_first_call = font_manager.stream_typeface_creations();

    let mixed_case = font_collection
        .try_get_glyph_typeface("MyAlias", FontStyle::Normal, FontWeight::Black, FontStretch::Normal)
        .expect("the alias in mixed case");

    // One shared cache entry, so the other casing neither re-synthesises nor gets a
    // second instance of the same face.
    assert!(Rc::ptr_eq(&upper_case, &mixed_case));
    assert_eq!(creations_after_first_call, font_manager.stream_typeface_creations());
}

#[test]
fn should_not_cache_a_family_twice_when_the_platform_returns_another_casing() {
    // The platform reports the family as "Noto Mono"; the caller asks in lower case, as any
    // XAML author may.
    let font_manager = Rc::new(AliasFontManagerImpl::new("Noto Mono"));

    let _app = UnitTestApplication::start(mock_platform_render_interface().with_font_manager_impl(font_manager.clone()));

    let font_collection = TestSystemFontCollection::new(font_manager);

    assert!(font_collection
        .try_get_glyph_typeface("noto mono", FontStyle::Normal, FontWeight::Normal, FontStretch::Normal)
        .is_some());

    // A cache keyed ordinally stores the requested casing beside the platform's own, but
    // AddFontFamily de-duplicates case-insensitively and publishes only the first of the
    // two, leaving the second bucket unreachable from every family-name search.
    assert_eq!(1, font_collection.glyph_typeface_cache().count());
    assert_eq!(font_collection.glyph_typeface_cache().count(), font_collection.count());
}

#[test]
fn should_reuse_an_already_cached_synthetic_glyph_typeface() {
    let font_manager = Rc::new(AliasFontManagerImpl::new("MyAlias"));

    let _app = UnitTestApplication::start(mock_platform_render_interface().with_font_manager_impl(font_manager.clone()));

    let font_collection = TestSystemFontCollection::new(font_manager.clone());

    let regular = font_collection
        .try_get_glyph_typeface("MyAlias", FontStyle::Normal, FontWeight::Normal, FontStretch::Normal)
        .expect("the alias");

    let first = font_collection
        .try_create_synthetic_glyph_typeface(&regular, FontStyle::Normal, FontWeight::Black, FontStretch::Normal)
        .expect("a synthetic typeface");

    assert_eq!(FontSimulations::Bold, first.font_simulations());

    let creations_after_first_call = font_manager.stream_typeface_creations();

    let second = font_collection
        .try_create_synthetic_glyph_typeface(&regular, FontStyle::Normal, FontWeight::Black, FontStretch::Normal)
        .expect("a synthetic typeface");

    // A second synthesis builds a GlyphTypeface that then loses the cache slot to the
    // first one, so it is returned to the caller but never cached and never disposed -
    // and GlyphTypeface has no finalizer, so its native typeface is retained until the
    // process exits.
    assert!(Rc::ptr_eq(&first, &second));
    assert_eq!(creations_after_first_call, font_manager.stream_typeface_creations());
}

/// Font manager whose `MyAlias` family resolves through the platform but is absent from
/// the installed family list, the shape of a platform alias (for instance Android's
/// `<alias name="arial" to="sans-serif"/>` in `/system/etc/fonts.xml`).
/// Such a family cannot be found again by the family-name search, so nothing repairs a
/// missing cache entry.
///
/// The alias is backed by an embedded test font rather than an installed one, so the test
/// runs identically on every platform.
struct AliasFontManagerImpl {
    inner: FontManagerImpl,
    alias: String,
    stream_typeface_creations: Cell<i32>,
}

impl AliasFontManagerImpl {
    /// Named explicitly rather than enumerated: the backing font must be a real
    /// text face, since a font that cannot be emboldened would make the test pass against
    /// unfixed code (the old else branch cached the nearest match).
    const BACKING_FONT_URI: &'static str =
        "resm:FerroUI.Skia.UnitTests.Assets.NotoMono-Regular.ttf?assembly=ferroui-skia";

    fn new(alias: &str) -> Self {
        Self { inner: FontManagerImpl::new(), alias: alias.to_owned(), stream_typeface_creations: Cell::new(0) }
    }

    /// Number of typefaces created from a stream: both the alias resolution and every
    /// synthetic emboldening go through this overload, so the counter also proves that a cached
    /// result short-circuits the platform call.
    fn stream_typeface_creations(&self) -> i32 {
        self.stream_typeface_creations.get()
    }

    fn open_backing_font() -> Box<dyn Read> {
        let asset_loader = FerroLocator::current().get_required_service::<dyn IAssetLoader>();

        Box::new(
            asset_loader
                .open(&Uri::new(Self::BACKING_FONT_URI, UriKind::Absolute).unwrap(), None)
                .expect("the backing font"),
        )
    }
}

impl IFontManagerImpl for AliasFontManagerImpl {
    fn get_default_font_family_name(&self) -> String {
        self.inner.get_default_font_family_name()
    }

    fn get_installed_font_family_names(&self, _check_for_updates: bool) -> Vec<String> {
        Vec::new()
    }

    fn try_create_glyph_typeface(
        &self,
        family_name: &str,
        _style: FontStyle,
        _weight: FontWeight,
        _stretch: FontStretch,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        // The alias always resolves to the regular face of the backing font, never to the
        // requested weight, exactly what a platform alias does.
        if family_name.eq_ignore_ascii_case(&self.alias) {
            let mut stream = Self::open_backing_font();

            return self.inner.try_create_glyph_typeface_from_stream(&mut stream, FontSimulations::None);
        }

        None
    }

    fn try_create_glyph_typeface_from_stream(
        &self,
        stream: &mut dyn Read,
        font_simulations: FontSimulations,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        self.stream_typeface_creations.set(self.stream_typeface_creations.get() + 1);

        self.inner.try_create_glyph_typeface_from_stream(stream, font_simulations)
    }

    fn try_get_family_typefaces(&self, family_name: &str) -> Option<Vec<Typeface>> {
        self.inner.try_get_family_typefaces(family_name)
    }

    fn try_match_character(
        &self,
        codepoint: i32,
        font_style: FontStyle,
        font_weight: FontWeight,
        font_stretch: FontStretch,
        family_name: Option<&str>,
        culture: Option<&CultureInfo>,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        self.inner.try_match_character(codepoint, font_style, font_weight, font_stretch, family_name, culture)
    }
}
