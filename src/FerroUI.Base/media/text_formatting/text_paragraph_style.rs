use std::rc::Rc;

use crate::media::{FlowDirection, TextAlignment, TextDecorationCollection, TextLineBounds, TextWrapping};
use crate::utilities::ValueSpan;

use super::{TextParagraphProperties, TextRunProperties};

/// Overrides for one or more complete paragraphs in a [`super::TextLayout`].
/// Positions in [`TextParagraphStyles`] count UTF-16 code units. A style is
/// selected at the first character of a paragraph and remains in effect across
/// its wrapped lines. A zero-length span can style a final empty paragraph.
///
/// Unspecified values inherit the layout defaults. Spacing and indentation are
/// in device-independent units; line height is an absolute height, as in
/// [`super::TextLayoutOptions::line_height`]. Invalid non-finite/negative spacing
/// is treated as zero. Indentation applies only to the first line, measured from
/// the paragraph's logical start edge. Before/after spacing belongs to the
/// paragraph even at the beginning/end of the layout.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextParagraphStyle {
    pub text_alignment: Option<TextAlignment>,
    pub flow_direction: Option<FlowDirection>,
    pub line_height: Option<f64>,
    pub letter_spacing: Option<f64>,
    pub space_before: f64,
    pub space_after: f64,
    pub indent: f64,
}

/// Immutable paragraph formatting. Spans should be ordered complete paragraphs;
/// construction normalizes ordering, with the later span winning a tied start.
/// Replace the collection when styles change; text controls invalidate shaping
/// as well as layout for this property.
pub type TextParagraphStyles = Rc<[ValueSpan<TextParagraphStyle>]>;

impl TextParagraphStyle {
    pub(crate) fn normalize(styles: Option<TextParagraphStyles>) -> Option<TextParagraphStyles> {
        let styles = styles?;
        if styles.is_empty() { return None; }
        if styles.iter().all(|span| span.start() >= 0 && span.length() >= 0)
            && styles.windows(2).all(|pair| pair[0].start() <= pair[1].start()) {
            return Some(styles);
        }
        let mut values: Vec<_> = styles.iter().filter(|span| span.start() >= 0 && span.length() >= 0).cloned().collect();
        values.sort_by_key(ValueSpan::start);
        (!values.is_empty()).then(|| Rc::from(values))
    }

    pub(crate) fn nonnegative(value: f64) -> f64 {
        if value.is_finite() && value > 0.0 { value } else { 0.0 }
    }

    pub(crate) fn properties(
        &self,
        defaults: &Rc<dyn TextParagraphProperties>,
        first_line: bool,
    ) -> Rc<dyn TextParagraphProperties> {
        Rc::new(StyledParagraph { defaults: defaults.clone(), style: self.clone(), first_line })
    }
}

struct StyledParagraph {
    defaults: Rc<dyn TextParagraphProperties>,
    style: TextParagraphStyle,
    first_line: bool,
}

impl TextParagraphProperties for StyledParagraph {
    fn flow_direction(&self) -> FlowDirection {
        self.style.flow_direction.unwrap_or_else(|| self.defaults.flow_direction())
    }
    fn text_alignment(&self) -> TextAlignment {
        self.style.text_alignment.unwrap_or_else(|| self.defaults.text_alignment())
    }
    fn line_height(&self) -> f64 {
        self.style.line_height.filter(|v| v.is_finite() && *v > 0.0)
            .unwrap_or_else(|| self.defaults.line_height())
    }
    fn letter_spacing(&self) -> f64 {
        self.style.letter_spacing.filter(|v| v.is_finite())
            .unwrap_or_else(|| self.defaults.letter_spacing())
    }
    fn first_line_in_paragraph(&self) -> bool { self.first_line }
    fn line_spacing(&self) -> f64 { self.defaults.line_spacing() }
    fn text_line_bounds(&self) -> TextLineBounds { self.defaults.text_line_bounds() }
    fn always_collapsible(&self) -> bool { self.defaults.always_collapsible() }
    fn default_text_run_properties(&self) -> &Rc<dyn TextRunProperties> {
        self.defaults.default_text_run_properties()
    }
    fn text_decorations(&self) -> Option<&TextDecorationCollection> { self.defaults.text_decorations() }
    fn text_wrapping(&self) -> TextWrapping { self.defaults.text_wrapping() }
    fn indent(&self) -> f64 { Self::style_indent(&self.style) }
    fn paragraph_indent(&self) -> f64 { self.defaults.paragraph_indent() }
    fn default_incremental_tab(&self) -> f64 { self.defaults.default_incremental_tab() }
}

impl StyledParagraph {
    fn style_indent(style: &TextParagraphStyle) -> f64 { TextParagraphStyle::nonnegative(style.indent) }
}
