//! The span operations a native accessibility bridge reads a text provider through.
//!
//! A bridge is linked into every application, so it holds none of this logic: it asks the provider
//! for [`ITextProvider::text_span_ops`], which only a provider whose ranges report their offsets
//! overrides (the code editor's). An application without one links none of the code below.

use super::{
    ITextProvider, ITextRangeProvider, SupportedTextSelection, TextAttribute, TextAttributeRun, TextAttributeValue,
    TextPatternRangeEndpoint, TextUnit,
};
use ferroui_base::media::{Color, FontWeight};
use ferroui_base::{Point, Rect};
use std::ops::Range;

/// The style of a run of text, its span relative to the requested text.
#[derive(Clone, Debug, PartialEq)]
pub struct TextStyleSpan {
    pub span: Range<usize>,
    pub font_name: Option<String>,
    /// The font size; 0 when the run does not state one.
    pub font_size: f64,
    pub font_weight: i32,
    /// 1 for superscript, -1 for subscript.
    pub script: i32,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub foreground: Option<Color>,
    pub background: Option<Color>,
}

/// The selected spans of a text and the one the caret is in.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextSelectionSpans {
    pub spans: Vec<Range<usize>>,
    pub primary: Option<Range<usize>>,
}

/// Offset-based reads and actions on a text provider.
pub trait TextSpanOps {
    /// Whether the ranges of the provider report their offsets.
    fn supports_spans(&self, text: &dyn ITextProvider) -> bool;
    fn can_select(&self, text: &dyn ITextProvider) -> bool;
    fn document_span(&self, text: &dyn ITextProvider) -> Option<Range<usize>>;
    fn selection(&self, text: &dyn ITextProvider, caret: Option<Range<usize>>) -> Option<TextSelectionSpans>;
    fn visible_span(&self, text: &dyn ITextProvider) -> Option<Range<usize>>;
    fn string(&self, text: &dyn ITextProvider, span: Range<usize>) -> Option<String>;
    fn bounds(&self, text: &dyn ITextProvider, span: Range<usize>) -> Option<Rect>;
    /// The span of `unit` around `offset`, when `offset` is inside it.
    fn unit_span(&self, text: &dyn ITextProvider, offset: usize, unit: TextUnit) -> Option<Range<usize>>;
    fn span_from_point(&self, text: &dyn ITextProvider, point: Point) -> Option<Range<usize>>;
    fn select(&self, text: &dyn ITextProvider, span: Range<usize>);
    fn has_logical_lines(&self, text: &dyn ITextProvider) -> bool;
    fn line_for_index(&self, text: &dyn ITextProvider, offset: usize) -> Option<usize>;
    fn range_for_line(&self, text: &dyn ITextProvider, index: usize) -> Option<Range<usize>>;
    fn has_styles(&self, text: &dyn ITextProvider) -> bool;
    /// The text of `span` with its style runs; `None` when a run leaves the span.
    fn styled_text(&self, text: &dyn ITextProvider, span: Range<usize>) -> Option<(String, Vec<TextStyleSpan>)>;
}

struct RangeTextSpanOps;

/// The operations for a provider whose ranges report their offsets.
pub fn text_span_ops() -> &'static dyn TextSpanOps {
    &RangeTextSpanOps
}

fn offsets_of(range: &dyn ITextRangeProvider) -> Option<Range<usize>> {
    range.text_offsets().ok()?
}

fn offsets_at_point(range: &dyn ITextRangeProvider, point: Point) -> Option<Range<usize>> {
    range
        .get_bounding_rectangles()
        .ok()?
        .iter()
        .any(|bounds| bounds.contains_exclusive(point))
        .then(|| offsets_of(range))
        .flatten()
}

fn style_span(run: TextAttributeRun, request: &Range<usize>) -> Option<TextStyleSpan> {
    use TextAttribute as A;
    use TextAttributeValue as V;

    if run.range.start < request.start || run.range.end > request.end || run.range.is_empty() {
        return None;
    }
    let mut style = TextStyleSpan {
        span: run.range.start - request.start..run.range.end - request.start,
        font_name: None,
        font_size: 0.0,
        font_weight: FontWeight::Normal.0,
        script: 0,
        italic: false,
        underline: false,
        strikethrough: false,
        foreground: None,
        background: None,
    };
    for (attribute, value) in run.attributes {
        match (attribute, value) {
            (A::FontName, V::String(value)) => style.font_name = Some(value),
            (A::FontSize, V::Number(value)) if value.is_finite() && value > 0.0 => style.font_size = value,
            (A::FontWeight, V::Integer(value)) => style.font_weight = value.clamp(1, 1000),
            (A::FontStyle, V::String(value)) => style.italic = matches!(value.as_str(), "italic" | "oblique"),
            (A::ForegroundColor, V::Color(value)) => style.foreground = Some(value),
            (A::BackgroundColor, V::Color(value)) => style.background = Some(value),
            (A::UnderlineStyle, V::Integer(value)) => style.underline = value != 0,
            (A::StrikethroughStyle, V::Integer(value)) => style.strikethrough = value != 0,
            (A::IsSuperscript, V::Bool(true)) => style.script = 1,
            (A::IsSubscript, V::Bool(true)) => style.script = -1,
            _ => {}
        }
    }
    Some(style)
}

impl TextSpanOps for RangeTextSpanOps {
    fn supports_spans(&self, text: &dyn ITextProvider) -> bool {
        text.range_from_offsets(0..0)
            .ok()
            .flatten()
            .is_some_and(|range| offsets_of(range.as_ref()).is_some())
    }

    fn can_select(&self, text: &dyn ITextProvider) -> bool {
        text.supported_text_selection().is_ok_and(|mode| mode != SupportedTextSelection::None)
    }

    fn document_span(&self, text: &dyn ITextProvider) -> Option<Range<usize>> {
        offsets_of(text.document_range().ok()?.as_ref())
    }

    fn selection(&self, text: &dyn ITextProvider, caret: Option<Range<usize>>) -> Option<TextSelectionSpans> {
        let spans: Vec<_> =
            text.get_selection().ok()?.iter().filter_map(|range| offsets_of(range.as_ref())).collect();
        let primary = caret
            .as_ref()
            .and_then(|caret| spans.iter().find(|span| span.start <= caret.start && caret.start <= span.end).cloned())
            .or_else(|| spans.first().cloned())
            .or(caret);
        Some(TextSelectionSpans { spans, primary })
    }

    fn visible_span(&self, text: &dyn ITextProvider) -> Option<Range<usize>> {
        text.get_visible_ranges()
            .ok()?
            .iter()
            .filter_map(|range| offsets_of(range.as_ref()))
            .reduce(|first, second| first.start.min(second.start)..first.end.max(second.end))
    }

    fn string(&self, text: &dyn ITextProvider, span: Range<usize>) -> Option<String> {
        text.range_from_offsets(span).ok()??.get_text(None).ok()
    }

    fn bounds(&self, text: &dyn ITextProvider, span: Range<usize>) -> Option<Rect> {
        let range = text.range_from_offsets(span).ok()??;
        range.get_bounding_rectangles().ok()?.into_iter().reduce(|a, b| a.union(b))
    }

    fn unit_span(&self, text: &dyn ITextProvider, offset: usize, unit: TextUnit) -> Option<Range<usize>> {
        let range = text.range_from_offsets(offset..offset).ok()??;
        range.expand_to_enclosing_unit(unit).ok()?;
        offsets_of(range.as_ref()).filter(|span| span.start <= offset && offset < span.end)
    }

    fn span_from_point(&self, text: &dyn ITextProvider, point: Point) -> Option<Range<usize>> {
        let caret = text.range_from_point(point).ok()?;
        let character = caret.clone_range().ok()?;
        character.expand_to_enclosing_unit(TextUnit::Character).ok()?;
        // A hit on the trailing half of a glyph can resolve to its next caret.
        offsets_at_point(character.as_ref(), point).or_else(|| {
            let moved = caret
                .move_endpoint_by_unit(TextPatternRangeEndpoint::Start, TextUnit::Character, -1)
                .ok()?;
            if moved == -1 {
                offsets_at_point(caret.as_ref(), point)
            } else {
                None
            }
        })
    }

    fn select(&self, text: &dyn ITextProvider, span: Range<usize>) {
        if !self.can_select(text) {
            return;
        }
        if let Ok(Some(range)) = text.range_from_offsets(span) {
            // A provider that refuses the selection leaves it as it was.
            drop(range.select());
        }
    }

    fn has_logical_lines(&self, text: &dyn ITextProvider) -> bool {
        matches!(text.logical_line_range(0), Ok(Some(_)))
    }

    fn line_for_index(&self, text: &dyn ITextProvider, offset: usize) -> Option<usize> {
        text.logical_line_index(offset).ok()?
    }

    fn range_for_line(&self, text: &dyn ITextProvider, index: usize) -> Option<Range<usize>> {
        text.logical_line_range(index).ok()?
    }

    fn has_styles(&self, text: &dyn ITextProvider) -> bool {
        matches!(text.text_style_runs(0..0), Ok(Some(_)))
    }

    fn styled_text(&self, text: &dyn ITextProvider, span: Range<usize>) -> Option<(String, Vec<TextStyleSpan>)> {
        let runs = text
            .text_style_runs(span.clone())
            .ok()??
            .into_iter()
            .map(|run| style_span(run, &span))
            .collect::<Option<Vec<_>>>()?;
        let string = text.range_from_offsets(span).ok()??.get_text(None).ok()?;
        Some((string, runs))
    }
}
