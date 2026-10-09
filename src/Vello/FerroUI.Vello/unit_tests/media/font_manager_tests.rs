//! Port of upstream's `Media/FontManagerTests.cs` of the Skia unit tests.
//!
//! Upstream's `using (FerroLocator.EnterScope())` is a [`LocatorScope`]
//! that disposes the scope when dropped. `Assert.Throws` is a caught panic:
//! the port's `Typeface::glyph_typeface` panics where upstream throws.
//! Upstream's `SKTypeface.Default` is the typeface Skia's default font
//! manager gives for no family name and the normal style: here the one the
//! font manager of the backend gives for it. Where upstream compares two
//! typefaces of Skia (`SKTypeface.Equals`), the fonts of the two typefaces
//! are compared: the font data and the index of the font in it.

use crate::unit_tests::media::CustomFontManagerImpl;
use crate::unit_tests::{mock_platform_render_interface, TestFontManager};
use crate::{FontManagerImpl, VelloTypeface};
use ferroui_base::logging::{LogArea, LogEventLevel};
use ferroui_base::media::fonts::{EmbeddedFontCollection, FontFamilyLoader};
use ferroui_base::media::text_formatting::unicode::Codepoint;
use ferroui_base::media::{
    FontFamily, FontManager, FontManagerOptions, FontSimulations, FontStretch, FontStyle, FontWeight,
    IPlatformTypeface, Typeface,
};
use ferroui_base::platform::{IAssetLoader, IFontManagerImpl};
use ferroui_base::reactive::IDisposable;
use ferroui_base::utilities::{CultureInfo, Uri, UriKind};
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_controls::testing::{TestLogSink, UnitTestApplication};
use ferroui_fonts_inter::InterFontCollection;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io::Read;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

const S_FONT_URI: &str = "resm:FerroUI.Vello.UnitTests.Assets?assembly=ferroui-vello#Noto Mono";

/// Upstream's `using (FerroLocator.EnterScope())`: the locator scope,
/// disposed when dropped.
struct LocatorScope(Rc<dyn IDisposable>);

impl LocatorScope {
    fn enter() -> Self {
        Self(FerroLocator::enter_scope())
    }
}

impl Drop for LocatorScope {
    fn drop(&mut self) {
        self.0.dispose();
    }
}

/// Upstream's `SKTypeface.Equals`: whether two typefaces are one font as it
/// is drawn.
fn same_font(first: &dyn IPlatformTypeface, second: &dyn IPlatformTypeface) -> bool {
    let first = VelloTypeface::try_get(first).expect("a typeface of this backend");
    let second = VelloTypeface::try_get(second).expect("a typeface of this backend");

    first.face().data() == second.face().data()
        && first.face().normalized_coords() == second.face().normalized_coords()
        && first.font_simulations() == second.font_simulations()
}

fn utf16(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

#[test]
fn should_create_typeface_from_fallback() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let _font_manager = FontManager::current();

    let glyph_typeface =
        Typeface::new(FontFamily::new(&format!("A, B, {}", FontFamily::DEFAULT_FONT_FAMILY_NAME))).glyph_typeface();

    let default_typeface = FontManagerImpl::new()
        .legacy_make_typeface(None, FontStyle::Normal, FontWeight::Normal, FontStretch::Normal)
        .expect("the default typeface of the system");

    assert_eq!(default_typeface.family_name(), glyph_typeface.family_name());
}

#[test]
fn should_create_typeface_from_fallback_bold() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let glyph_typeface = Typeface::with_style(
        FontFamily::new("A, B, Arial"),
        FontStyle::Normal,
        FontWeight::Bold,
        FontStretch::Normal,
    )
    .glyph_typeface();

    assert!(glyph_typeface.weight().value() >= 600);
}

#[test]
fn should_yield_default_glyph_typeface_for_invalid_family_name() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let glyph_typeface = Typeface::new(FontFamily::new("Unknown")).glyph_typeface();

    assert_eq!(FontManager::current().default_font_family().name(), glyph_typeface.family_name());
}

#[test]
fn should_load_typeface_from_resource() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let glyph_typeface = Typeface::from_name(S_FONT_URI).glyph_typeface();

    assert_eq!("Noto Mono", glyph_typeface.family_name());
}

#[test]
fn should_load_nearest_matching_font() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let glyph_typeface =
        Typeface::from_name_with_style(S_FONT_URI, FontStyle::Italic, FontWeight::Black, FontStretch::Normal)
            .glyph_typeface();

    assert_eq!("Noto Mono", glyph_typeface.family_name());
}

#[test]
fn should_throw_for_invalid_custom_font() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let result = catch_unwind(AssertUnwindSafe(|| {
        Typeface::from_name("resm:FerroUI.Vello.UnitTests.Assets?assembly=ferroui-vello#Unknown").glyph_typeface()
    }));

    assert!(result.is_err());
}

#[test]
fn should_return_false_for_unregistered_font_collection_uri() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let result = FontManager::current().try_get_glyph_typeface(&Typeface::from_name("fonts:invalid#Something"));

    assert!(result.is_none());
}

#[test]
fn should_only_try_to_create_glyph_typeface_once() {
    let font_manager_impl = Rc::new(TestFontManager::new());

    let _app =
        UnitTestApplication::start(mock_platform_render_interface().with_font_manager_impl(font_manager_impl.clone()));

    assert!(FontManager::current().try_get_glyph_typeface(&Typeface::default_typeface()).is_some());

    let count_before = font_manager_impl.try_create_glyph_typeface_count();

    for _ in 0..10 {
        FontManager::current().try_get_glyph_typeface(&Typeface::from_name("Unknown"));
    }

    assert_eq!(count_before + 1, font_manager_impl.try_create_glyph_typeface_count());
}

#[test]
fn should_cache_match_character() {
    let font_manager_impl = Rc::new(CustomFontManagerImpl::new());

    let _app = UnitTestApplication::start(mock_platform_render_interface().with_font_manager_impl(font_manager_impl));

    let (emoji, _) = Codepoint::read_at(&utf16("😀"), 0);

    let first_match = FontManager::current()
        .try_match_character(
            emoji.value() as i32,
            FontStyle::Normal,
            FontWeight::Normal,
            FontStretch::Normal,
            None,
            None,
        )
        .expect("a first match");

    let first_glyph_typeface = first_match.glyph_typeface();

    let second_match = FontManager::current()
        .try_match_character(
            emoji.value() as i32,
            FontStyle::Normal,
            FontWeight::Normal,
            FontStretch::Normal,
            None,
            None,
        )
        .expect("a second match");

    let second_glyph_typeface = second_match.glyph_typeface();

    assert!(Rc::ptr_eq(&first_glyph_typeface, &second_glyph_typeface));
}

#[test]
fn should_load_embedded_default_font_family() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let _scope = LocatorScope::enter();

    FerroLocator::current_mutable().bind_to_self(Rc::new(FontManagerOptions {
        default_family_name: Some(S_FONT_URI.to_owned()),
        ..FontManagerOptions::default()
    }));

    let result = FontManager::current().try_get_glyph_typeface(&Typeface::default_typeface());

    assert!(result.is_some());
    let glyph_typeface = result.unwrap();
    assert_eq!("Noto Mono", glyph_typeface.family_name());
}

#[test]
fn should_return_false_for_invalid_default_font_family() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let _scope = LocatorScope::enter();

    FerroLocator::current_mutable().bind_to_self(Rc::new(FontManagerOptions {
        default_family_name: Some(
            "ferres://resm:FerroUI.Vello.UnitTests.Assets?assembly=ferroui-vello#Unknown".to_owned(),
        ),
        ..FontManagerOptions::default()
    }));

    let result = FontManager::current().try_get_glyph_typeface(&Typeface::default_typeface());

    assert!(result.is_none());
}

#[test]
fn should_load_embedded_fallbacks() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let _scope = LocatorScope::enter();

    let font_family = FontFamily::parse(&format!("NotFound, {S_FONT_URI}")).expect("a valid font family");

    let typeface = Typeface::new(font_family);

    let glyph_typeface = typeface.glyph_typeface();

    assert_eq!("Noto Mono", glyph_typeface.family_name());
}

#[test]
fn should_match_chararcter_width_embedded_fallbacks() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let _scope = LocatorScope::enter();

    let font_family = FontFamily::parse(&format!("NotFound, {S_FONT_URI}")).expect("a valid font family");

    let typeface = FontManager::current()
        .try_match_character(
            'A' as i32,
            FontStyle::Normal,
            FontWeight::Normal,
            FontStretch::Normal,
            Some(&font_family),
            None,
        )
        .expect("a match");

    let glyph_typeface = typeface.glyph_typeface();

    assert_eq!("Noto Mono", glyph_typeface.family_name());
}

#[test]
fn should_match_chararcter_from_system_fonts() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let _scope = LocatorScope::enter();

    let typeface = FontManager::current()
        .try_match_character('A' as i32, FontStyle::Normal, FontWeight::Normal, FontStretch::Normal, None, None)
        .expect("a match");

    let glyph_typeface = typeface.glyph_typeface();

    assert_eq!(FontManager::current().default_font_family().name(), glyph_typeface.family_name());
}

#[test]
fn should_match_character_with_fallbacks() {
    for (family_name, base_uri) in [
        ("NotFound, Unknown", None),                     // system fonts
        ("/#NotFound, /#Unknown", Some("ferres://some/path")), // embedded fonts
    ] {
        let _app = UnitTestApplication::start(
            mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
        );

        let _scope = LocatorScope::enter();

        let base_uri = base_uri.map(|base_uri| Uri::new(base_uri, UriKind::Absolute).unwrap());

        let font_family =
            FontFamily::parse_with_base_uri(family_name, base_uri.as_ref()).expect("a valid font family");

        let typeface = FontManager::current()
            .try_match_character(
                'A' as i32,
                FontStyle::Normal,
                FontWeight::Normal,
                FontStretch::Normal,
                Some(&font_family),
                None,
            )
            .unwrap_or_else(|| panic!("a match for {family_name:?}"));

        let glyph_typeface = typeface.glyph_typeface();

        assert_eq!(
            FontManager::current().default_font_family().name(),
            glyph_typeface.family_name(),
            "{family_name:?}"
        );
    }
}

#[test]
fn should_use_custom_system_font() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let _scope = LocatorScope::enter();

    FontManager::current().add_font_collection(Rc::new(EmbeddedFontCollection::new(
        FontManager::system_fonts_key(),
        Uri::new(S_FONT_URI, UriKind::Absolute).unwrap(),
    )));

    let glyph_typeface =
        FontManager::current().try_get_glyph_typeface(&Typeface::from_name("Noto Mono")).expect("Noto Mono");

    assert_eq!("Noto Mono", glyph_typeface.family_name());
}

#[test]
fn should_get_nearest_match_for_custom_system_font() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let _scope = LocatorScope::enter();

    FontManager::current().add_font_collection(Rc::new(EmbeddedFontCollection::new(
        FontManager::system_fonts_key(),
        Uri::new(S_FONT_URI, UriKind::Absolute).unwrap(),
    )));

    let glyph_typeface = FontManager::current()
        .try_get_glyph_typeface(&Typeface::from_name_with_style(
            "Noto Mono",
            FontStyle::Italic,
            FontWeight::Normal,
            FontStretch::Normal,
        ))
        .expect("Noto Mono");

    assert_eq!("Noto Mono", glyph_typeface.family_name());
}

#[test]
fn should_get_implicit_typeface() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let _scope = LocatorScope::enter();

    FontManager::current().add_font_collection(Rc::new(EmbeddedFontCollection::new(
        FontManager::system_fonts_key(),
        Uri::new(S_FONT_URI, UriKind::Absolute).unwrap(),
    )));

    let glyph_typeface = FontManager::current()
        .try_get_glyph_typeface(&Typeface::from_name("Noto Mono Italic"))
        .expect("Noto Mono Italic");

    assert_eq!("Noto Mono", glyph_typeface.family_name());

    assert_eq!(FontStyle::Italic, glyph_typeface.style());
}

#[test]
fn should_create_synthetic_typeface() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let _scope = LocatorScope::enter();

    FontManager::current().add_font_collection(Rc::new(EmbeddedFontCollection::new(
        FontManager::system_fonts_key(),
        Uri::new(S_FONT_URI, UriKind::Absolute).unwrap(),
    )));

    let italic_bold_typeface = FontManager::current()
        .try_get_glyph_typeface(&Typeface::from_name_with_style(
            "Noto Mono",
            FontStyle::Italic,
            FontWeight::Bold,
            FontStretch::Normal,
        ))
        .expect("an italic bold Noto Mono");

    assert_eq!("Noto Mono", italic_bold_typeface.family_name());

    assert!(italic_bold_typeface.platform_typeface().font_simulations().contains(FontSimulations::Bold));

    assert!(italic_bold_typeface.platform_typeface().font_simulations().contains(FontSimulations::Oblique));

    let regular_typeface = FontManager::current()
        .try_get_glyph_typeface(&Typeface::from_name_with_style(
            "Noto Mono",
            FontStyle::Normal,
            FontWeight::Normal,
            FontStretch::Normal,
        ))
        .expect("a regular Noto Mono");

    assert!(!same_font(&**regular_typeface.platform_typeface(), &**italic_bold_typeface.platform_typeface()));
}

#[test]
#[cfg_attr(not(windows), ignore = "Requires Windows Fonts")]
fn should_get_glyph_typeface_by_localized_family_name() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let _scope = LocatorScope::enter();

    let glyph_typeface =
        FontManager::current().try_get_glyph_typeface(&Typeface::from_name("微軟正黑體")).expect("微軟正黑體");

    assert_eq!("Microsoft JhengHei", glyph_typeface.family_name());
}

#[test]
fn should_get_font_features() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let _scope = LocatorScope::enter();

    FontManager::current().add_font_collection(Rc::new(InterFontCollection::new()));

    let glyph_typeface =
        FontManager::current().try_get_glyph_typeface(&Typeface::from_name("fonts:Inter#Inter")).expect("Inter");

    assert_eq!("Inter", glyph_typeface.family_name());

    let features = glyph_typeface.supported_features();

    assert!(!features.is_empty());
}

#[test]
fn should_map_font_family() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let _scope = LocatorScope::enter();

    FerroLocator::current_mutable().bind_to_self(Rc::new(FontManagerOptions {
        default_family_name: Some(S_FONT_URI.to_owned()),
        font_family_mappings: Some(HashMap::from([(
            "Segoe UI".to_owned(),
            FontFamily::new("fonts:Inter#Inter"),
        )])),
        ..FontManagerOptions::default()
    }));

    FontManager::current().add_font_collection(Rc::new(InterFontCollection::new()));

    let result = FontManager::current().try_get_glyph_typeface(&Typeface::from_name("Abc, Segoe UI"));

    assert!(result.is_some());
    let glyph_typeface = result.unwrap();
    assert_eq!("Inter", glyph_typeface.family_name());
}

#[test]
fn should_map_font_family_regardless_of_casing() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let _scope = LocatorScope::enter();

    FerroLocator::current_mutable().bind_to_self(Rc::new(FontManagerOptions {
        default_family_name: Some(S_FONT_URI.to_owned()),
        font_family_mappings: Some(HashMap::from([(
            "Segoe UI".to_owned(),
            FontFamily::new("fonts:Inter#Inter"),
        )])),
        ..FontManagerOptions::default()
    }));

    FontManager::current().add_font_collection(Rc::new(InterFontCollection::new()));

    // The mapping table is configuration; its casing has nothing to do with the casing
    // a control asks in. Both the composite and the plain family path must still find
    // the mapping.
    let from_composite = FontManager::current()
        .try_get_glyph_typeface(&Typeface::from_name("Abc, segoe ui"))
        .expect("the composite family");

    assert_eq!("Inter", from_composite.family_name());

    let from_plain_family =
        FontManager::current().try_get_glyph_typeface(&Typeface::from_name("SEGOE UI")).expect("the plain family");

    assert_eq!("Inter", from_plain_family.family_name());
}

#[test]
fn should_get_family_typefaces() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let _scope = LocatorScope::enter();

    FontManager::current().add_font_collection(Rc::new(InterFontCollection::new()));

    let family_typefaces = FontManager::current().get_family_typefaces(&FontFamily::new("fonts:Inter#Inter"));

    assert_eq!(6, family_typefaces.len());
}

#[test]
fn should_use_font_collection_match_character() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let _scope = LocatorScope::enter();

    FontManager::current().add_font_collection(Rc::new(EmbeddedFontCollection::new(
        Uri::absolute("fonts:MyCollection").unwrap(),                                      //key
        Uri::absolute("resm:FerroUI.Vello.UnitTests.Assets?assembly=ferroui-vello").unwrap(), //source
    )));

    let font_family = FontFamily::new("fonts:MyCollection#Noto Mono");

    let character = "א";

    let (codepoint, _) = Codepoint::read_at(&utf16(character), 0);

    let typeface = FontManager::current()
        .try_match_character(
            codepoint.value() as i32,
            FontStyle::Normal,
            FontWeight::Normal,
            FontStretch::Normal,
            Some(&font_family),
            None,
        )
        .expect("a match");

    //Typeface should come from the font collection
    assert!(typeface.font_family().key().is_some());

    assert_eq!("Noto Sans Hebrew", typeface.glyph_typeface().family_name());
}

#[test]
fn should_use_last_resort_font_last_match_character() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let _scope = LocatorScope::enter();

    FontManager::current().add_font_collection(Rc::new(EmbeddedFontCollection::new(
        Uri::absolute("fonts:MyCollection").unwrap(),                                      //key
        Uri::absolute("resm:FerroUI.Vello.UnitTests.Assets?assembly=ferroui-vello").unwrap(), //source
    )));

    let font_family = FontFamily::new("fonts:MyCollection#Noto Sans");

    let characters = utf16("א𪜶");

    let (codepoint1, _) = Codepoint::read_at(&characters, 0);
    assert_eq!(0x5D0, codepoint1.value()); // א

    // Typeface should come from the font collection - falling back to Noto Sans Hebrew
    let typeface1 = FontManager::current()
        .try_match_character(
            codepoint1.value() as i32,
            FontStyle::Normal,
            FontWeight::Normal,
            FontStretch::Normal,
            Some(&font_family),
            None,
        )
        .expect("a match for א");
    assert!(typeface1.font_family().key().is_some());
    assert_eq!("Noto Sans Hebrew", typeface1.glyph_typeface().family_name());

    let (codepoint2, _) = Codepoint::read_at(&characters, 1);
    assert_eq!(0x2A736, codepoint2.value()); // 𪜶

    // Typeface should come from the font collection - falling back to Adobe Blank 2 VF R as a last resort
    let typeface2 = FontManager::current()
        .try_match_character(
            codepoint2.value() as i32,
            FontStyle::Normal,
            FontWeight::Normal,
            FontStretch::Normal,
            Some(&font_family),
            None,
        )
        .expect("a match for 𪜶");
    assert!(typeface2.font_family().key().is_some());
    assert_eq!("Adobe Blank 2 VF R", typeface2.glyph_typeface().family_name());
}

#[test]
#[cfg_attr(not(windows), ignore = "Windows specific font")]
fn should_get_system_font_with_base_uri() {
    for name in ["Arial", "#Arial"] {
        let _app = UnitTestApplication::start(
            mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
        );

        let _scope = LocatorScope::enter();

        let font_family =
            FontFamily::with_base_uri(Some(&Uri::absolute("ferres://ferroui-vello/NotFound").unwrap()), name);

        let glyph_typeface = Typeface::new(font_family).glyph_typeface();

        assert_eq!("Arial", glyph_typeface.family_name(), "{name}");
    }
}

#[test]
#[cfg_attr(not(windows), ignore = "Windows specific font")]
fn should_get_regular_font_after_matching_italic_font() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let _scope = LocatorScope::enter();

    let italic_typeface = FontManager::current()
        .try_match_character('こ' as i32, FontStyle::Italic, FontWeight::Normal, FontStretch::Normal, None, None)
        .expect("an italic match");

    assert_eq!(FontSimulations::None, italic_typeface.glyph_typeface().font_simulations());

    assert_eq!("Yu Gothic UI", italic_typeface.glyph_typeface().family_name());

    assert_ne!(FontStyle::Normal, italic_typeface.style());

    let regular_typeface = FontManager::current()
        .try_match_character('こ' as i32, FontStyle::Normal, FontWeight::Normal, FontStretch::Normal, None, None)
        .expect("a regular match");

    assert_eq!("Yu Gothic UI", regular_typeface.glyph_typeface().family_name());

    assert_eq!(FontStyle::Normal, regular_typeface.style());

    assert!(!same_font(
        &**italic_typeface.glyph_typeface().platform_typeface(),
        &**regular_typeface.glyph_typeface().platform_typeface()
    ));
}

#[test]
fn should_fallback_when_font_family_is_empty() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let _scope = LocatorScope::enter();

    let typeface = Typeface::from_name("");

    // Upstream asserts that the font family is not null; a font family is a value here.
    let _font_family: &FontFamily = typeface.font_family();
}

#[test]
fn try_get_glyph_typeface_should_return_false_for_font_without_supported_cmap() {
    const FONT_URI: &str = "resm:FerroUI.Vello.UnitTests.Fonts.TestFontNoCmap412.ttf?assembly=ferroui-vello#TestFontNoCmap412";

    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );
    let _scope = LocatorScope::enter();

    // Skia can load the font
    assert_can_create_platform_typeface();

    // But FerroUI can't, because it has no supported cmap subtable
    assert_cannot_create_glyph_typeface();

    fn assert_can_create_platform_typeface() {
        let font_manager_impl = FerroLocator::current().get_required_service::<dyn IFontManagerImpl>();
        let asset_loader = FerroLocator::current().get_required_service::<dyn IAssetLoader>();

        let mut stream = asset_loader.open(&Uri::absolute(FONT_URI).unwrap(), None).expect("the font stream");

        let platform_typeface = font_manager_impl
            .try_create_glyph_typeface_from_stream(&mut *stream, FontSimulations::None)
            .expect("the platform typeface");
        assert_eq!("TestFontNoCmap412", platform_typeface.family_name());
    }

    fn assert_cannot_create_glyph_typeface() {
        let logged_template: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
        let logged_values: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));

        let log_sink_scope = {
            let logged_template = logged_template.clone();
            let logged_values = logged_values.clone();

            TestLogSink::start(move |level, area, _, template, values| {
                if level == LogEventLevel::Warning && area == LogArea::FONTS {
                    *logged_template.borrow_mut() = Some(template.to_owned());
                    *logged_values.borrow_mut() = values.iter().map(|value| value.to_string()).collect();
                }
            })
        };

        assert!(FontManager::current().try_get_glyph_typeface(&Typeface::from_name(FONT_URI)).is_none());

        log_sink_scope.dispose();

        assert_eq!(
            logged_template.borrow().as_deref(),
            Some(
                "Could not create glyph typeface from platform typeface named {FamilyName} with simulations {Simulations}: {Exception}"
            )
        );
        let logged_values = logged_values.borrow();
        assert_eq!(3, logged_values.len());
        assert_eq!("TestFontNoCmap412", logged_values[0]);
        assert_eq!("No suitable cmap subtable found.", logged_values[2]);
    }
}

#[test]
fn try_match_character_should_return_correct_weight() {
    for (requested_weight, expected_family_name) in [
        (FontWeight::Normal, "Inter"),
        (FontWeight::Bold, "Inter"),
        (FontWeight::SemiBold, "Inter SemiBold"),
        (FontWeight::SemiLight, "Inter Light"),
    ] {
        let _app = UnitTestApplication::start(
            mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
        );
        let _scope = LocatorScope::enter();

        FontManager::current().add_font_collection(Rc::new(InterFontCollection::new()));

        let typeface = FontManager::current()
            .try_match_character(
                'A' as i32,
                FontStyle::Normal,
                requested_weight,
                FontStretch::Normal,
                Some(&FontFamily::new("fonts:Inter#Inter")),
                None,
            )
            .unwrap_or_else(|| panic!("a match for {requested_weight:?}"));

        assert_eq!(expected_family_name, typeface.glyph_typeface().family_name(), "{requested_weight:?}");
        assert_eq!(requested_weight, typeface.weight(), "{requested_weight:?}");
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
        let _app = UnitTestApplication::start(
            mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
        );
        let _scope = LocatorScope::enter();

        FontManager::current().add_font_collection(Rc::new(InterFontCollection::new()));

        let typeface = FontManager::current()
            .try_match_character(
                'A' as i32,
                FontStyle::Normal,
                FontWeight::Normal,
                requested_stretch,
                Some(&FontFamily::new("fonts:Inter#Inter")),
                None,
            )
            .unwrap_or_else(|| panic!("a match for {requested_stretch:?}"));

        assert_eq!("Inter", typeface.glyph_typeface().family_name(), "{requested_stretch:?}");
        assert_eq!(requested_stretch, typeface.stretch(), "{requested_stretch:?}");
    }
}

#[test]
fn try_get_glyph_typeface_should_use_perfect_match_in_collection_before_nearest_match() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(CustomFontManagerImpl::new())),
    );
    let _scope = LocatorScope::enter();

    // Load bold font (Inter-Bold.ttf) first
    let bold_glyph_typeface = FontManager::current()
        .try_get_glyph_typeface(&Typeface::from_name_with_style(
            "Inter",
            FontStyle::Normal,
            FontWeight::Bold,
            FontStretch::Normal,
        ))
        .expect("the bold Inter");
    assert_eq!("Inter", bold_glyph_typeface.family_name());
    assert_eq!(FontWeight::Bold, bold_glyph_typeface.weight());

    // Normal font (Inter-Regular.ttf) should be loaded since it's a perfect match, instead of falling back
    let regular_glyph_typeface = FontManager::current()
        .try_get_glyph_typeface(&Typeface::from_name_with_style(
            "Inter",
            FontStyle::Normal,
            FontWeight::Normal,
            FontStretch::Normal,
        ))
        .expect("the regular Inter");
    assert!(!Rc::ptr_eq(&regular_glyph_typeface, &bold_glyph_typeface));
    assert_eq!("Inter", regular_glyph_typeface.family_name());
    assert_eq!(FontWeight::Normal, regular_glyph_typeface.weight());

    // Nearest match should still work (650 falls back to 700 Bold)
    let nearest_match_typeface = FontManager::current()
        .try_get_glyph_typeface(&Typeface::from_name_with_style(
            "Inter",
            FontStyle::Normal,
            FontWeight(650),
            FontStretch::Normal,
        ))
        .expect("the nearest match");
    assert!(Rc::ptr_eq(&bold_glyph_typeface, &nearest_match_typeface));
}

#[test]
fn try_get_glyph_typeface_should_cache_matched_glyph_typeface_under_requested_family_name() {
    let font_manager_impl = Rc::new(FamilyRemappingFontManagerImpl::new("NotInstalled", "Noto Mono"));
    let _app =
        UnitTestApplication::start(mock_platform_render_interface().with_font_manager_impl(font_manager_impl.clone()));

    // "NotInstalled" is not installed, so the platform substitutes it with a different
    // family ("Noto Mono"), much like requesting "Arial" yields "Liberation Sans" on some Linux distributions.
    let first =
        FontManager::current().try_get_glyph_typeface(&Typeface::from_name("NotInstalled")).expect("a substitute");
    assert_eq!("Noto Mono", first.family_name());

    // The substitute should now be cached under the requested "NotInstalled" name, so a second
    // lookup must resolve from the cache instead of asking the platform again.
    let second =
        FontManager::current().try_get_glyph_typeface(&Typeface::from_name("NotInstalled")).expect("a substitute");
    assert!(Rc::ptr_eq(&first, &second));
    assert_eq!(1, font_manager_impl.requested_family_create_count());
}

/// A font manager whose every by-name lookup resolves to a single matched font whose family name
/// differs from the requested one.
struct FamilyRemappingFontManagerImpl {
    requested_family_name: String,
    matched_family_name: String,
    requested_family_create_count: Cell<i32>,
}

impl FamilyRemappingFontManagerImpl {
    fn new(requested_family_name: &str, matched_family_name: &str) -> Self {
        Self {
            requested_family_name: requested_family_name.to_owned(),
            matched_family_name: matched_family_name.to_owned(),
            requested_family_create_count: Cell::new(0),
        }
    }

    fn requested_family_create_count(&self) -> i32 {
        self.requested_family_create_count.get()
    }

    fn create_matched_typeface(&self) -> Rc<VelloTypeface> {
        let asset_loader = FerroLocator::current().get_required_service::<dyn IAssetLoader>();

        // LoadFontAssets ignores the family fragment and returns every embedded font asset,
        // so pick the one whose family name matches the substitute we want to return.
        for font_asset in FontFamilyLoader::load_font_assets(&Uri::absolute(S_FONT_URI).unwrap()) {
            let typeface = asset_loader.open(&font_asset, None).ok().and_then(|mut stream| {
                let mut bytes = Vec::new();
                stream.read_to_end(&mut bytes).ok()?;
                VelloTypeface::from_bytes(bytes, FontSimulations::None)
            });

            if let Some(typeface) = typeface {
                if typeface.family_name().eq_ignore_ascii_case(&self.matched_family_name) {
                    return typeface;
                }
            }
        }

        panic!("Could not load the '{}' font asset.", self.matched_family_name);
    }
}

impl IFontManagerImpl for FamilyRemappingFontManagerImpl {
    fn get_default_font_family_name(&self) -> String {
        self.matched_family_name.clone()
    }

    fn get_installed_font_family_names(&self, _check_for_updates: bool) -> Vec<String> {
        vec![self.matched_family_name.clone()]
    }

    fn try_create_glyph_typeface(
        &self,
        family_name: &str,
        _style: FontStyle,
        _weight: FontWeight,
        _stretch: FontStretch,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        if family_name.eq_ignore_ascii_case(&self.requested_family_name) {
            self.requested_family_create_count.set(self.requested_family_create_count.get() + 1);
        }

        Some(self.create_matched_typeface() as Rc<dyn IPlatformTypeface>)
    }

    fn try_create_glyph_typeface_from_stream(
        &self,
        stream: &mut dyn Read,
        font_simulations: FontSimulations,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes).ok()?;

        Some(VelloTypeface::from_bytes(bytes, font_simulations)? as Rc<dyn IPlatformTypeface>)
    }

    fn try_match_character(
        &self,
        _codepoint: i32,
        _font_style: FontStyle,
        _font_weight: FontWeight,
        _font_stretch: FontStretch,
        _family_name: Option<&str>,
        _culture: Option<&CultureInfo>,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        None
    }

    fn try_get_family_typefaces(&self, _family_name: &str) -> Option<Vec<Typeface>> {
        None
    }

    fn dispose(&self) {}
}
