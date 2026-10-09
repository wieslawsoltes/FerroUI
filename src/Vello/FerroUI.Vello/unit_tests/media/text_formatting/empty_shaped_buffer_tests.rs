//! Port of upstream's `Media/TextFormatting/EmptyShapedBufferTests.cs` of the
//! Skia unit tests.
//!
//! A shaper hides the default ignorables it substitutes for line breaks behind the font's space
//! glyph. A font that has no space glyph leaves it no way to do that, so those glyphs are deleted
//! instead and a run holding nothing but a line break shapes to an empty glyph buffer. The run
//! still owns its characters and has to survive, otherwise the line covers no text at all.

use crate::unit_tests::media::text_formatting::SingleBufferTextSource;
use crate::unit_tests::media::CustomFontManagerImpl;
use crate::unit_tests::{mock_platform_render_interface, ASSEMBLY};
use crate::PlatformRenderInterface;
use ferroui_base::media::fonts::{FontCollectionBase, FontCollectionBaseImpl, IFontCollection};
use ferroui_base::media::text_formatting::{
    GenericTextParagraphProperties, GenericTextRunProperties, ShapedTextRun, TextFormatter, TextFormatterImpl,
    TextLayout, TextLayoutOptions, TextParagraphProperties, TextRun,
};
use ferroui_base::media::{Brushes, FontManager, TextAlignment, TextWrapping, Typeface};
use ferroui_base::platform::IFontManagerImpl;
use ferroui_base::utilities::{Uri, UriKind};
use ferroui_base::FerroLocator;
use ferroui_controls::testing::{UnitTestApplication, UnitTestApplicationScope};
use std::rc::Rc;

/// The headless platform's default font: four glyphs, no space, no coverage for anything else.
const GLYPHLESS_FONT: &str = "FerroUI.Vello.UnitTests.Fonts.BareMinimum.ttf";

fn typeface() -> Typeface {
    Typeface::from_name("fonts:SystemFonts#BareMinimum")
}

#[test]
fn line_break_that_shapes_to_no_glyphs_keeps_its_characters() {
    let _scope = start();

    let default_properties = Rc::new(GenericTextRunProperties::with_font_size(typeface(), 12.0));

    let formatter = TextFormatterImpl::new();

    let paragraph_properties: Rc<dyn TextParagraphProperties> = Rc::new(GenericTextParagraphProperties::with_options(
        default_properties.clone(),
        TextAlignment::Left,
        TextWrapping::Wrap,
        0.0,
        0.0,
    ));

    let text_line = formatter.format_line(
        &SingleBufferTextSource::new("\r\nfoo", default_properties, false),
        0,
        100.0,
        &paragraph_properties,
        None,
    );

    assert!(text_line.is_some());
    let text_line = text_line.unwrap();

    let text_runs = text_line.text_runs();
    assert_eq!(1, text_runs.len());
    let run = text_runs[0].downcast_ref::<ShapedTextRun>();
    assert!(run.is_some());
    let run = run.unwrap();

    assert_eq!(2, run.length());
    assert_eq!(2, text_line.length());

    // The premise of the test: shaping really did produce nothing for these characters.
    assert_eq!(0, run.shaped_buffer().length());
}

#[test]
fn should_wrap_text_that_starts_with_a_line_break() {
    let _scope = start();

    // MaxLines bounds the layout loop: a line that covers no text never advances the text
    // source, so without it a regression here hangs the test run instead of failing it.
    let layout = TextLayout::new(
        "\r\nPassword update failed",
        typeface(),
        TextLayoutOptions {
            font_size: 12.0,
            foreground: Some(Brushes::black()),
            text_wrapping: TextWrapping::Wrap,
            max_width: 290.0,
            max_lines: 5,
            ..TextLayoutOptions::default()
        },
    );

    assert_eq!(2, layout.text_lines().len());

    assert_eq!(2, layout.text_lines()[0].length());
    assert_eq!(22, layout.text_lines()[1].length());
}

fn start() -> UnitTestApplicationScope {
    let disposable = UnitTestApplication::start(
        mock_platform_render_interface().with_render_interface(Rc::new(PlatformRenderInterface::default())),
    );

    let font_manager_impl: Rc<dyn IFontManagerImpl> = Rc::new(CustomFontManagerImpl::new());

    FerroLocator::current_mutable().bind::<dyn IFontManagerImpl>().to_constant(font_manager_impl.clone());

    let font_manager = FontManager::new(font_manager_impl);

    FerroLocator::current_mutable().bind::<FontManager>().to_constant(font_manager.clone());

    font_manager.add_font_collection(Rc::new(GlyphlessSystemFontCollection::new()) as Rc<dyn IFontCollection>);

    disposable
}

struct GlyphlessSystemFontCollection {
    base: FontCollectionBase,
}

impl GlyphlessSystemFontCollection {
    fn new() -> Self {
        let collection = Self { base: FontCollectionBase::new() };

        FontCollectionBase::try_add_font_source(
            &collection,
            &Uri::new(&format!("resm:{GLYPHLESS_FONT}?assembly={ASSEMBLY}"), UriKind::Absolute).unwrap(),
        );

        collection
    }
}

impl FontCollectionBaseImpl for GlyphlessSystemFontCollection {
    fn base(this: &Self) -> &FontCollectionBase {
        &this.base
    }

    fn key(_this: &Self) -> Uri {
        FontManager::system_fonts_key()
    }
}
