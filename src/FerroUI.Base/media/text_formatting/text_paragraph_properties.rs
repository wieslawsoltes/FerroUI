use std::rc::Rc;

use crate::media::text_formatting::TextRunProperties;
use crate::media::{FlowDirection, TextAlignment, TextDecorationCollection, TextWrapping};

/// Provides a set of properties that are used during the paragraph layout.
pub trait TextParagraphProperties: 'static {
    /// This property specifies whether the primary text advance direction
    /// shall be left-to-right, right-to-left.
    fn flow_direction(&self) -> FlowDirection;

    /// Gets the text alignment.
    fn text_alignment(&self) -> TextAlignment;

    /// Paragraph's line height.
    fn line_height(&self) -> f64;

    /// Paragraph's line spacing.
    fn line_spacing(&self) -> f64 {
        0.0
    }

    /// Indicates the first line of the paragraph.
    fn first_line_in_paragraph(&self) -> bool;

    /// If true, the formatted line may always be collapsed. If false, only
    /// lines that overflow the paragraph width are collapsed.
    fn always_collapsible(&self) -> bool {
        false
    }

    /// Gets the default text style.
    fn default_text_run_properties(&self) -> &Rc<dyn TextRunProperties>;

    /// If not null, text decorations to apply to all runs in the line. This
    /// is in addition to any text decorations specified by the text run
    /// properties for individual text runs.
    fn text_decorations(&self) -> Option<&TextDecorationCollection> {
        None
    }

    /// Gets the text wrapping.
    fn text_wrapping(&self) -> TextWrapping;

    /// Line indentation.
    fn indent(&self) -> f64;

    /// Get the paragraph indentation.
    fn paragraph_indent(&self) -> f64 {
        0.0
    }

    /// Gets the default incremental tab width.
    fn default_incremental_tab(&self) -> f64 {
        0.0
    }

    /// Gets the letter spacing.
    fn letter_spacing(&self) -> f64 {
        0.0
    }
}
