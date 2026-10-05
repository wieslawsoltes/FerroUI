//! Test harness of the text formatter: synthetic fonts built in code, a
//! deterministic text shaper, a glyph run factory and a locator scope that
//! binds all of it.
//!
//! Upstream's formatter tests run against real fonts and a real shaper. The
//! harness replaces both with fixed-advance fonts and a one-glyph-per-codepoint
//! shaper, so that every expected value follows from the constants below.

mod drawing;
mod fonts;
mod shaper;
mod sources;

use std::rc::Rc;

use crate::media::text_formatting::{
    FormattingObjectPool, GenericTextParagraphProperties, GenericTextRunProperties, ITextSource, TextFormatter,
    TextLine, TextLineBreak, TextParagraphProperties, TextRunProperties,
};
use crate::media::{FlowDirection, TextAlignment, TextWrapping, Typeface};
use crate::platform::{IFontManagerImpl, IPlatformRenderInterface, ITextShaperImpl};
use crate::rendering::testing::{DrawingLog, MockPlatformRenderInterface};
use crate::reactive::IDisposable;
use crate::FerroLocator;

pub use drawing::{DrawCall, RecordingDrawingSink};
#[allow(unused_imports)] // the typeface is exported for the tests built on top
pub use fonts::{TestFont, TestFontManagerImpl, TestPlatformTypeface};
pub use shaper::TestTextShaperImpl;
pub use sources::{
    CustomDrawableRun, EndOfLineTextSource, InvisibleRun, ListTextSource, MultiBufferTextSource,
    SingleBufferTextSource,
};

/// The design units per em of every test font.
pub const DESIGN_EM_HEIGHT: u16 = 1000;
/// The advance of every glyph of the default test fonts, in design units.
pub const GLYPH_ADVANCE: u16 = 500;
/// The ascender of every test font, in design units (the font metrics hold it negated).
pub const ASCENDER: i16 = 800;
/// The descender of every test font, in design units (the font metrics hold it negated).
pub const DESCENDER: i16 = -200;
/// The line gap of every test font, in design units.
pub const LINE_GAP: i16 = 100;

/// The family of the default test font: Latin, combining marks, general
/// punctuation, Hebrew and Arabic.
pub const DEFAULT_FAMILY: &str = "Test Sans";
/// A test font that only covers CJK (U+3000..U+30FF, U+4E00..U+9FFF).
pub const CJK_FAMILY: &str = "Test CJK";
/// A test font that only covers emoji (U+1F300..U+1FAFF) with a double width advance.
pub const EMOJI_FAMILY: &str = "Test Emoji";

/// The advance of one glyph of the default fonts at the given em size.
pub fn advance(font_rendering_em_size: f64) -> f64 {
    GLYPH_ADVANCE as f64 * font_rendering_em_size / DESIGN_EM_HEIGHT as f64
}

/// The natural line height of the test fonts at the given em size.
pub fn line_height(font_rendering_em_size: f64) -> f64 {
    (ASCENDER as f64 - DESCENDER as f64 + LINE_GAP as f64) * font_rendering_em_size / DESIGN_EM_HEIGHT as f64
}

/// A locator scope with the test font manager, text shaper and glyph run
/// factory bound. Dropping it leaves the scope and (unless the test is
/// already failing) verifies that every rented formatting list was returned.
pub struct TextTestScope {
    scope: Rc<dyn IDisposable>,
    shaper: Rc<TestTextShaperImpl>,
    #[allow(dead_code)] // see `font_manager()`
    font_manager: Rc<TestFontManagerImpl>,
}

#[allow(dead_code)] // the full harness API is kept for the tests built on top
impl TextTestScope {
    /// A scope with the default fonts: [`DEFAULT_FAMILY`] (the default
    /// family), [`CJK_FAMILY`] and [`EMOJI_FAMILY`].
    pub fn new() -> Self {
        Self::with_fonts(vec![
            TestFont::new(DEFAULT_FAMILY)
                .with_ranges(&[(0x20, 0x7E), (0xA0, 0x24F), (0x300, 0x36F), (0x590, 0x6FF), (0x2000, 0x206F)]),
            TestFont::new(CJK_FAMILY).with_ranges(&[(0x3000, 0x30FF), (0x4E00, 0x9FFF)]),
            TestFont::new(EMOJI_FAMILY).with_ranges(&[(0x1F300, 0x1FAFF)]).with_advance(GLYPH_ADVANCE * 2),
        ])
    }

    /// A scope whose default font has no line gap (the shape of the monospaced
    /// font upstream uses where a run has to reproduce the metrics of a line).
    pub fn without_line_gap() -> Self {
        Self::with_fonts(vec![TestFont::new(DEFAULT_FAMILY).with_ranges(&[(0x20, 0x7E)]).with_line_gap(0)])
    }

    /// A scope with the given fonts; the first one is the default family.
    pub fn with_fonts(fonts: Vec<TestFont>) -> Self {
        let scope = FerroLocator::enter_scope();

        let font_manager = Rc::new(TestFontManagerImpl::new(fonts));
        let shaper = Rc::new(TestTextShaperImpl::new());

        let locator = FerroLocator::current_mutable();

        locator.bind::<dyn IFontManagerImpl>().to_constant(font_manager.clone());
        locator.bind::<dyn ITextShaperImpl>().to_constant(shaper.clone());
        locator.bind::<dyn IPlatformRenderInterface>().to_constant(MockPlatformRenderInterface::new(DrawingLog::new()));

        // Load every font into the system font collection, as an installed
        // font set is: the character fallback first considers the fonts the
        // collection already knows before it asks the platform.
        let manager = crate::media::FontManager::current();
        for family in font_manager.get_installed_font_family_names(false) {
            let _ = manager.try_get_glyph_typeface(&Typeface::from_name(&family));
        }

        Self { scope, shaper, font_manager }
    }

    /// The text shaper of the scope (to configure ligatures).
    pub fn shaper(&self) -> &TestTextShaperImpl {
        &self.shaper
    }

    /// The font manager backend of the scope.
    pub fn font_manager(&self) -> &TestFontManagerImpl {
        &self.font_manager
    }
}

impl Drop for TextTestScope {
    fn drop(&mut self) {
        self.scope.dispose();

        if !std::thread::panicking() {
            FormattingObjectPool::instance().verify_all_returned();
        }
    }
}

/// Run properties with the default typeface at the given em size.
pub fn run_properties(font_rendering_em_size: f64) -> Rc<dyn TextRunProperties> {
    Rc::new(GenericTextRunProperties::with_font_size(Typeface::default_typeface(), font_rendering_em_size))
}

/// Run properties with the typeface of the given family at the given em size.
pub fn run_properties_for(family: &str, font_rendering_em_size: f64) -> Rc<dyn TextRunProperties> {
    Rc::new(GenericTextRunProperties::with_font_size(Typeface::from_name(family), font_rendering_em_size))
}

/// Left aligned, left to right paragraph properties.
pub fn paragraph_properties(
    default_properties: &Rc<dyn TextRunProperties>,
    text_wrapping: TextWrapping,
) -> Rc<dyn TextParagraphProperties> {
    paragraph_properties_with(default_properties, text_wrapping, TextAlignment::Left, FlowDirection::LeftToRight)
}

/// Paragraph properties with alignment and flow direction.
pub fn paragraph_properties_with(
    default_properties: &Rc<dyn TextRunProperties>,
    text_wrapping: TextWrapping,
    text_alignment: TextAlignment,
    flow_direction: FlowDirection,
) -> Rc<dyn TextParagraphProperties> {
    Rc::new(GenericTextParagraphProperties::with_all(
        flow_direction,
        text_alignment,
        true,
        true,
        default_properties.clone(),
        text_wrapping,
        0.0,
        0.0,
        0.0,
    ))
}

/// Formats one line with the current formatter.
pub fn format_line(
    text_source: &dyn ITextSource,
    first_text_source_index: i32,
    paragraph_width: f64,
    paragraph_properties: &Rc<dyn TextParagraphProperties>,
    previous_line_break: Option<&Rc<TextLineBreak>>,
) -> Option<Rc<dyn TextLine>> {
    <dyn TextFormatter>::current().format_line(
        text_source,
        first_text_source_index,
        paragraph_width,
        paragraph_properties,
        previous_line_break,
    )
}

/// Formats every line of a text source (until the formatter returns no line
/// or the text length is reached).
pub fn format_lines(
    text_source: &dyn ITextSource,
    text_length: i32,
    paragraph_width: f64,
    paragraph_properties: &Rc<dyn TextParagraphProperties>,
) -> Vec<Rc<dyn TextLine>> {
    let formatter = <dyn TextFormatter>::current();

    let mut lines: Vec<Rc<dyn TextLine>> = Vec::new();
    let mut current_position = 0;
    let mut previous_line_break: Option<Rc<TextLineBreak>> = None;

    while current_position < text_length {
        let Some(line) = formatter.format_line(
            text_source,
            current_position,
            paragraph_width,
            paragraph_properties,
            previous_line_break.as_ref(),
        ) else {
            break;
        };

        assert!(line.length() > 0, "the formatter produced an empty line at {current_position}");

        current_position += line.length();
        previous_line_break = line.text_line_break();
        lines.push(line);
    }

    lines
}

/// The text of a line: the text of its runs in the order of the run list.
pub fn line_text(line: &dyn TextLine) -> String {
    let mut units: Vec<u16> = Vec::new();

    for run in line.text_runs().iter() {
        units.extend_from_slice(run.text_span());
    }

    String::from_utf16_lossy(&units)
}

/// UTF-16 text of a string.
pub fn utf16(text: &str) -> crate::utilities::ReadOnlyMemory<u16> {
    crate::utilities::ReadOnlyMemory::<u16>::from_str(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::text_formatting::ShapedTextRun;
    use crate::media::FontManager;

    #[test]
    fn harness_serves_fonts_and_shapes_one_glyph_per_codepoint() {
        let _scope = TextTestScope::new();

        let glyph_typeface = Typeface::default_typeface().glyph_typeface();

        assert_eq!(glyph_typeface.family_name(), DEFAULT_FAMILY);
        assert_eq!(glyph_typeface.metrics().design_em_height, DESIGN_EM_HEIGHT);
        assert_eq!(glyph_typeface.metrics().ascent, -(ASCENDER as i32));
        assert_eq!(glyph_typeface.metrics().descent, -(DESCENDER as i32));
        assert_eq!(glyph_typeface.metrics().line_gap, LINE_GAP as i32);
        assert_eq!(glyph_typeface.try_get_horizontal_glyph_advance(b'a' as u16), Some(GLYPH_ADVANCE));
        assert_eq!(glyph_typeface.character_to_glyph_map().try_get_glyph('a' as i32), Some('a' as u16));
        assert_eq!(glyph_typeface.character_to_glyph_map().try_get_glyph(0x4E00), None);

        let cjk = FontManager::current()
            .try_match_character(
                0x4E00,
                crate::media::FontStyle::Normal,
                crate::media::FontWeight::Normal,
                crate::media::FontStretch::Normal,
                None,
                None,
            )
            .unwrap();

        assert_eq!(cjk.font_family().name(), CJK_FAMILY);

        let properties = run_properties(10.0);
        let source = SingleBufferTextSource::new("ab \u{05D0}\u{05D1}", properties.clone());
        let line = format_line(&source, 0, f64::INFINITY, &paragraph_properties(&properties, TextWrapping::NoWrap), None)
            .unwrap();

        assert_eq!(line.length(), 5);
        assert_eq!(line.width_including_trailing_whitespace(), 5.0 * advance(10.0));
        assert_eq!(line.height(), line_height(10.0));

        let runs = line.text_runs();

        assert_eq!(runs.len(), 2);

        let rtl = runs[1].downcast_ref::<ShapedTextRun>().unwrap();

        assert_eq!(rtl.bidi_level(), 1);
        assert_eq!(
            rtl.shaped_buffer().glyph_infos().iter().map(|glyph| glyph.glyph_cluster).collect::<Vec<_>>(),
            [1, 0]
        );
    }
}
