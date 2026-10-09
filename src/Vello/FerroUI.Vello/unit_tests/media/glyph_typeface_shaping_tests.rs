//! Port of upstream's `Media/GlyphTypefaceShapingTests.cs` of the Skia unit
//! tests.

use super::CustomFontManagerImpl;
use crate::unit_tests::{mock_platform_render_interface, ASSEMBLY};
use crate::PlatformRenderInterface;
use ferroui_base::media::fonts::{FontCollectionBase, FontCollectionBaseImpl};
use ferroui_base::media::text_formatting::unicode::Script;
use ferroui_base::media::{FontManager, Typeface};
use ferroui_base::platform::IFontManagerImpl;
use ferroui_base::utilities::Uri;
use ferroui_base::FerroLocator;
use ferroui_controls::testing::{UnitTestApplication, UnitTestApplicationScope};
use std::rc::Rc;

const MONO_FONT: &str = "FerroUI.Vello.UnitTests.Assets.NotoMono-Regular.ttf";
const ARABIC_FONT: &str = "FerroUI.Vello.UnitTests.Assets.NotoSansArabic-Regular.ttf";

// A tiny Noto Sans Arabic subset with the GSUB/GPOS layout tables stripped: it keeps Arabic
// cmap glyphs but cannot shape Arabic. Renamed so it doesn't collide with the full font.
const ARABIC_NO_LAYOUT_FONT: &str = "FerroUI.Vello.UnitTests.Fonts.NotoSansArabic-NoLayout.ttf";

// P0 — CanShapeScript gates complex scripts on GSUB/GPOS script coverage, not cmap. This is
// the primitive the F3 capability fallback (Strategy A) builds on.
#[test]
fn can_shape_script_gates_complex_scripts_on_layout_coverage_not_cmap() {
    let _app = start(&[MONO_FONT, ARABIC_FONT, ARABIC_NO_LAYOUT_FONT]);

    let font_manager = FontManager::current();

    let mono = font_manager.try_get_glyph_typeface(&Typeface::from_name("fonts:SystemFonts#Noto Mono"));
    assert!(mono.is_some());
    let mono = mono.unwrap();
    let arabic = font_manager.try_get_glyph_typeface(&Typeface::from_name("fonts:SystemFonts#Noto Sans Arabic"));
    assert!(arabic.is_some());
    let arabic = arabic.unwrap();
    let arabic_no_layout =
        font_manager.try_get_glyph_typeface(&Typeface::from_name("fonts:SystemFonts#Noto Sans Arabic NoLayout"));
    assert!(arabic_no_layout.is_some());
    let arabic_no_layout = arabic_no_layout.unwrap();

    // Simple scripts never require layout tables — always true, regardless of the font.
    assert!(mono.can_shape_script(Script::Latin));
    assert!(arabic.can_shape_script(Script::Latin));

    // The real Arabic font declares the 'arab' GSUB script, so it can shape Arabic.
    assert!(arabic.can_shape_script(Script::Arabic));

    // A Latin-only font has no Arabic layout coverage.
    assert!(!mono.can_shape_script(Script::Arabic));

    // The crux: the stripped font HAS Arabic cmap glyphs but no GSUB/GPOS, so it cannot
    // shape Arabic even though TryGetGlyph succeeds. cmap coverage is not shaping capability.
    assert!(arabic_no_layout.character_to_glyph_map().try_get_glyph(0x0627).is_some()); // ا is mapped
    assert!(!arabic_no_layout.can_shape_script(Script::Arabic));

    // A complex script the Arabic font does not declare is rejected too.
    assert!(!arabic.can_shape_script(Script::Devanagari));
}

fn start(font_resource_names: &[&str]) -> UnitTestApplicationScope {
    let disposable = UnitTestApplication::start(
        mock_platform_render_interface().with_render_interface(Rc::new(PlatformRenderInterface::default())),
    );

    let font_manager_impl: Rc<dyn IFontManagerImpl> = Rc::new(CustomFontManagerImpl::new());

    FerroLocator::current_mutable().bind::<dyn IFontManagerImpl>().to_constant(font_manager_impl.clone());

    let font_manager = FontManager::new(font_manager_impl);

    FerroLocator::current_mutable().bind::<FontManager>().to_constant(font_manager.clone());

    font_manager.add_font_collection(Rc::new(CuratedSystemFontCollection::new(font_resource_names)));

    disposable
}

struct CuratedSystemFontCollection {
    base: FontCollectionBase,
}

impl CuratedSystemFontCollection {
    fn new(font_resource_names: &[&str]) -> Self {
        let this = Self { base: FontCollectionBase::new() };

        for name in font_resource_names {
            FontCollectionBase::try_add_font_source(
                &this,
                &Uri::absolute(&format!("resm:{name}?assembly={ASSEMBLY}")).unwrap(),
            );
        }

        this
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
