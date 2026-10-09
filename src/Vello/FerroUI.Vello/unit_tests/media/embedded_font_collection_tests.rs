//! Port of upstream's `Media/EmbeddedFontCollectionTests.cs` of the Skia
//! unit tests.
//!
//! Upstream's `TestEmbeddedFontCollection` derives from
//! `EmbeddedFontCollection`; here it holds one and forwards the base class
//! state to it, overriding `TryCreateSyntheticGlyphTypeface`.

use crate::unit_tests::media::CustomFontManagerImpl;
use crate::unit_tests::mock_platform_render_interface;
use crate::FontManagerImpl;
use ferroui_base::media::fonts::{
    EmbeddedFontCollection, FontCollectionBase, FontCollectionBaseImpl, GlyphTypefaceCache, IFontCollection,
};
use ferroui_base::media::{FontStretch, FontStyle, FontWeight, GlyphTypeface};
use ferroui_base::utilities::{Uri, UriKind};
use ferroui_controls::testing::UnitTestApplication;
use std::rc::Rc;

const S_FONT_ASSETS: &str = "resm:FerroUI.Vello.UnitTests.Assets?assembly=ferroui-vello";

#[test]
fn should_get_near_matching_typeface() {
    for (font_weight, font_style) in [
        (FontWeight::SemiLight, FontStyle::Normal),
        (FontWeight::Bold, FontStyle::Italic),
        (FontWeight::Heavy, FontStyle::Oblique),
    ] {
        let _app = UnitTestApplication::start(
            mock_platform_render_interface().with_font_manager_impl(Rc::new(CustomFontManagerImpl::new())),
        );

        let _key = Uri::new("fonts:testFonts", UriKind::Absolute).unwrap();
        let source = Uri::new(S_FONT_ASSETS, UriKind::Absolute).unwrap();

        let font_collection = TestEmbeddedFontCollection::new(source.clone(), source, false);

        let glyph_typeface = font_collection
            .try_get_glyph_typeface("Noto Mono", font_style, font_weight, FontStretch::Normal)
            .unwrap_or_else(|| panic!("({font_weight:?}, {font_style:?})"));

        let actual = glyph_typeface.family_name();

        assert_eq!("Noto Mono", actual, "({font_weight:?}, {font_style:?})");
    }
}

#[test]
fn should_not_get_typeface_for_invalid_family_name() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(CustomFontManagerImpl::new())),
    );

    let key = Uri::new("fonts:testFonts", UriKind::Absolute).unwrap();
    let source = Uri::new(S_FONT_ASSETS, UriKind::Absolute).unwrap();

    let font_collection = TestEmbeddedFontCollection::new(key, source, false);

    assert!(font_collection
        .try_get_glyph_typeface("ABC", FontStyle::Normal, FontWeight::Normal, FontStretch::Normal)
        .is_none());
}

#[test]
fn should_get_typeface_for_partial_family_name() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(CustomFontManagerImpl::new())),
    );

    let key = Uri::new("fonts:testFonts", UriKind::Absolute).unwrap();
    let source = Uri::new(S_FONT_ASSETS, UriKind::Absolute).unwrap();

    let font_collection = TestEmbeddedFontCollection::new(key, source, false);

    let glyph_typeface = font_collection
        .try_get_glyph_typeface("T", FontStyle::Normal, FontWeight::Normal, FontStretch::Normal)
        .expect("a typeface for the partial family name");

    assert_eq!("Twitter Color Emoji", glyph_typeface.family_name());
}

#[test]
fn should_get_typeface_for_typographic_family_name() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(CustomFontManagerImpl::new())),
    );

    let key = Uri::new("fonts:testFonts", UriKind::Absolute).unwrap();
    let source = Uri::new(S_FONT_ASSETS, UriKind::Absolute).unwrap();

    let font_collection = TestEmbeddedFontCollection::new(key, source, false);

    let glyph_typeface = font_collection
        .try_get_glyph_typeface("Manrope", FontStyle::Normal, FontWeight::Light, FontStretch::Normal)
        .expect("a typeface for the typographic family name");

    assert_eq!("Manrope Light", glyph_typeface.family_name());

    assert_eq!("Manrope", glyph_typeface.typographic_family_name());
}

#[test]
fn should_cache_synthetic_glyph_typeface() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(CustomFontManagerImpl::new())),
    );

    let key = Uri::new("fonts:testFonts", UriKind::Absolute).unwrap();
    let source = Uri::new(S_FONT_ASSETS, UriKind::Absolute).unwrap();

    let font_collection = TestEmbeddedFontCollection::new(key, source, true);

    let glyph_typeface = font_collection
        .try_get_glyph_typeface("Manrope", FontStyle::Normal, FontWeight::ExtraBlack, FontStretch::Normal)
        .expect("a synthetic typeface");

    let glyph_typefaces =
        font_collection.glyph_typeface_cache().try_get_value("Manrope").expect("Manrope is cached");

    assert_eq!(2, glyph_typefaces.borrow().len());

    let other_glyph_typeface = font_collection.try_get_glyph_typeface(
        "Manrope",
        FontStyle::Normal,
        FontWeight::ExtraBlack,
        FontStretch::Normal,
    );

    assert!(other_glyph_typeface.is_some_and(|other| Rc::ptr_eq(&glyph_typeface, &other)));
}

#[test]
fn should_cache_nearest_match_for_mi_sans() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let source = Uri::new(S_FONT_ASSETS, UriKind::Absolute).unwrap();

    let font_collection = TestEmbeddedFontCollection::new(source.clone(), source, false);

    // Font weight 304
    assert!(font_collection
        .try_get_glyph_typeface("MiSans", FontStyle::Normal, FontWeight::Normal, FontStretch::Normal)
        .is_some());

    // Font weight regular (400)
    assert!(font_collection
        .try_get_glyph_typeface("MiSans", FontStyle::Normal, FontWeight::Bold, FontStretch::Normal)
        .is_some());

    // Font weight 700
    let glyph_typefaces = font_collection.glyph_typeface_cache().try_get_value("MiSans").expect("MiSans is cached");

    assert_eq!(3, glyph_typefaces.borrow().len());
}

struct TestEmbeddedFontCollection {
    base: EmbeddedFontCollection,
    create_synthetic_typefaces: bool,
}

impl TestEmbeddedFontCollection {
    fn new(key: Uri, source: Uri, create_synthetic_typefaces: bool) -> Self {
        Self { base: EmbeddedFontCollection::new(key, source), create_synthetic_typefaces }
    }

    fn glyph_typeface_cache(&self) -> &GlyphTypefaceCache {
        FontCollectionBaseImpl::base(self).glyph_typeface_cache()
    }
}

impl FontCollectionBaseImpl for TestEmbeddedFontCollection {
    fn base(this: &Self) -> &FontCollectionBase {
        <EmbeddedFontCollection as FontCollectionBaseImpl>::base(&this.base)
    }

    fn key(this: &Self) -> Uri {
        <EmbeddedFontCollection as FontCollectionBaseImpl>::key(&this.base)
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

        FontCollectionBase::try_create_synthetic_glyph_typeface(this, glyph_typeface, style, weight, stretch)
    }
}
