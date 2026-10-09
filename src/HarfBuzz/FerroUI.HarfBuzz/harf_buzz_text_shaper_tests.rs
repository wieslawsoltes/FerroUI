//! Port of `Media/TextFormatting/HarfbuzzTextShaperTests.cs` of the base unit tests: the shaper is in this crate,
//! so its tests are too.
//!
//! The typeface is created from the Inter of the test assets of the base crate, the font upstream loads from the
//! assets of its test project, through the platform typeface of the font tests of the base crate (upstream's
//! `CustomPlatformTypeface` over the stream of the asset).

use crate::HarfBuzzTextShaper;
use ferroui_base::media::fonts::testing::TestPlatformTypeface;
use ferroui_base::media::text_formatting::TextShaperOptions;
use ferroui_base::media::{FontSimulations, GlyphTypeface};
use ferroui_base::platform::ITextShaperImpl;
use ferroui_base::utilities::ReadOnlyMemory;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use std::rc::Rc;

const INTER_FONT: &[u8] = include_bytes!("../../FerroUI.Base/test_assets/fonts/Inter-Regular.ttf");

/// The fixture of upstream's test class: the shaper, and an application whose text shaper it is.
fn start() -> (Rc<HarfBuzzTextShaper>, UnitTestApplicationScope) {
    let shaper = Rc::new(HarfBuzzTextShaper::new());
    let services = TestServices::mock_threading_interface().with_text_shaper_impl(shaper.clone());
    (shaper, UnitTestApplication::start(services))
}

fn create_text_shaper_options() -> TextShaperOptions {
    let bidi_level = 0;
    let letter_spacing = 0.0;
    let font_size = 16.0;

    let platform_typeface =
        TestPlatformTypeface::from_bytes(INTER_FONT.to_vec(), FontSimulations::None).expect("the font loads");
    let typeface = GlyphTypeface::new(platform_typeface, FontSimulations::None).expect("the font has its tables");

    TextShaperOptions::with_all(typeface, font_size, bidi_level, None, 0.0, letter_spacing, None)
}

fn utf16(text: &str) -> ReadOnlyMemory<u16> {
    ReadOnlyMemory::from_vec(text.encode_utf16().collect())
}

#[test]
fn shape_text_with_valid_input_returns_shaped_buffer() {
    let (shaper, _app) = start();
    let text = utf16("Hello World");
    let options = create_text_shaper_options();

    let result = shaper.shape_text(&text, &options);

    assert_eq!(text.len(), result.length());
}

#[test]
fn shape_text_with_empty_string_returns_empty_shaped_buffer() {
    let (shaper, _app) = start();
    let text = utf16("");
    let options = create_text_shaper_options();

    let result = shaper.shape_text(&text, &options);

    assert_eq!(0, result.length());
}

#[test]
fn shape_text_with_tab_character_replaces_with_space() {
    let (shaper, _app) = start();
    let text = utf16("Hello\tWorld");
    let options = create_text_shaper_options();

    let result = shaper.shape_text(&text, &options);

    assert!(result.length() == 11);
}

#[test]
fn shape_text_with_crlf_merges_break_pair() {
    let (shaper, _app) = start();
    let text = utf16("Line1\r\nLine2");
    let options = create_text_shaper_options();

    let result = shaper.shape_text(&text, &options);

    assert_ne!(0.0, result.glyph_infos()[5].glyph_advance);
}

#[test]
fn shape_text_end_with_crlf_merges_break_pair() {
    let (shaper, _app) = start();
    let text = utf16("Line1\r\n");
    let options = create_text_shaper_options();

    let result = shaper.shape_text(&text, &options);

    assert_eq!(0.0, result.glyph_infos()[5].glyph_advance);
}

#[test]
fn shape_text_with_sliced_memory_cluster_values_are_slice_relative() {
    let (shaper, _app) = start();
    let full_string = "A".repeat(1000) + "Hello" + &"B".repeat(1000);
    let sliced = utf16(&full_string).slice(1000, 5);

    let options = create_text_shaper_options();

    let result = shaper.shape_text(&sliced, &options);

    assert_eq!(5, result.length());

    for (i, glyph_info) in result.glyph_infos().iter().enumerate() {
        assert!(
            glyph_info.glyph_cluster >= 0 && glyph_info.glyph_cluster < 5,
            "Glyph cluster at index {i} was {}, expected a value in [0, 5).",
            glyph_info.glyph_cluster
        );
    }
}
