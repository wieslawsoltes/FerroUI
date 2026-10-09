//! Port of upstream's `Media/TextFormatting/ShapingCapabilityFallbackTests.cs`
//! of the Skia unit tests.
//!
//! Upstream returns the rented run list in a `finally` block; here it is
//! returned once the assertions have run.

use crate::unit_tests::media::CustomFontManagerImpl;
use crate::unit_tests::{mock_platform_render_interface, ASSEMBLY};
use crate::PlatformRenderInterface;
use ferroui_base::media::fonts::{FontCollectionBase, FontCollectionBaseImpl, IFontCollection};
use ferroui_base::media::text_formatting::{
    FormattingObjectPool, GenericTextRunProperties, TextCharacters, TextRunProperties,
};
use ferroui_base::media::{FontManager, Typeface};
use ferroui_base::platform::IFontManagerImpl;
use ferroui_base::utilities::{ReadOnlyMemory, Uri, UriKind};
use ferroui_base::FerroLocator;
use ferroui_controls::testing::{UnitTestApplication, UnitTestApplicationScope};
use std::rc::Rc;

const MONO_FONT: &str = "FerroUI.Vello.UnitTests.Assets.NotoMono-Regular.ttf";
const ARABIC_FONT: &str = "FerroUI.Vello.UnitTests.Assets.NotoSansArabic-Regular.ttf";

// Tiny Noto Sans Arabic subset with the layout tables stripped: it has the Arabic cmap glyphs
// but no GSUB/GPOS, so it cannot actually shape Arabic. Renamed so it doesn't collide with the
// full font.
const ARABIC_NO_LAYOUT_FONT: &str = "FerroUI.Vello.UnitTests.Fonts.NotoSansArabic-NoLayout.ttf";

// For a complex script, a primary that has the cmap glyphs but can't shape it (no GSUB/GPOS) is
// upgraded to a shaping-capable font. This is unconditional — there is no longer a mode toggle.
#[test]
fn cmap_only_complex_script_primary_is_upgraded_to_a_shaping_capable_font() {
    // A shaping-capable Arabic font is present, so the cmap-only primary is replaced by it.
    assert_eq!("Noto Sans Arabic", resolve_arabic_run_family(&[MONO_FONT, ARABIC_FONT, ARABIC_NO_LAYOUT_FONT]));
}

// When no shaping-capable font for the script exists, the cmap-only font is kept (the capability
// tier finds nothing, the cmap tier then accepts it) — we never reject more than before.
#[test]
fn cmap_only_complex_script_primary_is_kept_when_no_capable_font_exists() {
    assert_eq!("Noto Sans Arabic NoLayout", resolve_arabic_run_family(&[MONO_FONT, ARABIC_NO_LAYOUT_FONT]));
}

fn resolve_arabic_run_family(font_resource_names: &[&str]) -> String {
    let _scope = start(font_resource_names);

    let font_manager = FontManager::current();

    // Primary run typeface: the cmap-only (no-layout) Arabic font.
    let default_properties: Rc<dyn TextRunProperties> =
        Rc::new(GenericTextRunProperties::new(Typeface::from_name("fonts:SystemFonts#Noto Sans Arabic NoLayout")));

    let text = ReadOnlyMemory::from_str("\u{0627}"); // U+0627 ARABIC LETTER ALEF

    let text_characters = TextCharacters::new(text.clone(), default_properties);

    let mut results = FormattingObjectPool::instance().text_run_lists.rent();

    let mut previous_properties: Option<Rc<dyn TextRunProperties>> = None;

    text_characters.get_shapeable_characters(text, 0, &font_manager, &mut previous_properties, &mut results);

    assert_eq!(1, results.len());
    let run_glyph_typeface = font_manager.try_get_glyph_typeface(results[0].properties().unwrap().typeface());
    assert!(run_glyph_typeface.is_some());

    let family_name = run_glyph_typeface.unwrap().family_name().to_owned();

    FormattingObjectPool::instance().text_run_lists.return_list(results);

    family_name
}

fn start(font_resource_names: &[&str]) -> UnitTestApplicationScope {
    let disposable = UnitTestApplication::start(
        mock_platform_render_interface().with_render_interface(Rc::new(PlatformRenderInterface::default())),
    );

    let font_manager_impl: Rc<dyn IFontManagerImpl> = Rc::new(CustomFontManagerImpl::new());

    FerroLocator::current_mutable().bind::<dyn IFontManagerImpl>().to_constant(font_manager_impl.clone());

    let font_manager = FontManager::new(font_manager_impl);

    FerroLocator::current_mutable().bind::<FontManager>().to_constant(font_manager.clone());

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
