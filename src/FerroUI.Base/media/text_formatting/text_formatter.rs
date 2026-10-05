use std::rc::Rc;

use crate::media::text_formatting::text_formatter_impl::TextFormatterImpl;
use crate::media::text_formatting::{
    ITextSource, ShapedTextRun, TextLine, TextLineBreak, TextParagraphProperties, TextRun, TextRunCache, TextShaper,
    TextShaperOptions,
};
use crate::media::FlowDirection;
use crate::{FerroLocator, LocatorExtensions};

/// Represents a base class for text formatting.
pub trait TextFormatter: 'static {
    /// Formats a text line.
    ///
    /// * `text_source` — the text source.
    /// * `first_text_source_index` — the first character index to start the text line from.
    /// * `paragraph_width` — a value that specifies the width of the paragraph that the line fills.
    /// * `paragraph_properties` — properties that control paragraph features such as text
    ///   flow direction, text alignment, or indentation.
    /// * `previous_line_break` — the state of the previous line break.
    fn format_line(
        &self,
        text_source: &dyn ITextSource,
        first_text_source_index: i32,
        paragraph_width: f64,
        paragraph_properties: &Rc<dyn TextParagraphProperties>,
        previous_line_break: Option<&Rc<TextLineBreak>>,
    ) -> Option<Rc<dyn TextLine>>;

    /// Formats a text line, optionally using a [`TextRunCache`] to avoid redundant shaping.
    ///
    /// When a cache is provided, shaped runs from a previous call with the same
    /// `first_text_source_index` are reused, skipping shaping and bidi processing.
    /// This is useful when formatting the same text at different paragraph widths.
    ///
    /// * `text_source` — the text source.
    /// * `first_text_source_index` — the first character index to start the text line from.
    /// * `paragraph_width` — a value that specifies the width of the paragraph that the line fills.
    /// * `paragraph_properties` — properties that control paragraph features such as text
    ///   flow direction, text alignment, or indentation.
    /// * `previous_line_break` — the state of the previous line break.
    /// * `text_run_cache` — an optional cache for shaped text runs.
    fn format_line_with_cache(
        &self,
        text_source: &dyn ITextSource,
        first_text_source_index: i32,
        paragraph_width: f64,
        paragraph_properties: &Rc<dyn TextParagraphProperties>,
        previous_line_break: Option<&Rc<TextLineBreak>>,
        text_run_cache: Option<&TextRunCache>,
    ) -> Option<Rc<dyn TextLine>> {
        let _ = text_run_cache;

        self.format_line(text_source, first_text_source_index, paragraph_width, paragraph_properties, previous_line_break)
    }
}

impl dyn TextFormatter {
    /// Gets the current `TextFormatter` service.
    pub fn current() -> Rc<dyn TextFormatter> {
        if let Some(current) = FerroLocator::current().get_service::<dyn TextFormatter>() {
            return current;
        }

        let current: Rc<dyn TextFormatter> = Rc::new(TextFormatterImpl::new());

        FerroLocator::current_mutable().bind::<dyn TextFormatter>().to_constant(current.clone());

        current
    }

    /// Shapes the symbol run of collapsing properties (an ellipsis) in the
    /// given flow direction.
    ///
    /// Panics when the run has no properties.
    pub fn create_symbol(text_run: &dyn TextRun, flow_direction: FlowDirection) -> Rc<ShapedTextRun> {
        let text_shaper = TextShaper::current();

        let properties = text_run.properties().expect("a symbol run has properties");

        let glyph_typeface = properties.cached_glyph_typeface();

        let font_rendering_em_size = properties.font_rendering_em_size();

        let culture_info = properties.culture_info().cloned();

        let shaper_options = TextShaperOptions::with_all(
            glyph_typeface,
            font_rendering_em_size,
            flow_direction as i8,
            culture_info,
            0.0,
            0.0,
            properties.font_features().map(|features| Rc::new(features.to_vec())),
        );

        let shaped_buffer = text_shaper.shape_text(&text_run.text(), &shaper_options);

        Rc::new(ShapedTextRun::new(shaped_buffer, properties.clone()))
    }
}
