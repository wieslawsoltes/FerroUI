use std::rc::Rc;

use crate::media::text_formatting::{TextLineMetricsMode, TextRunProperties, TextTabStop};
use crate::media::{FlowDirection, TextAlignment, TextDecorationCollection, TextLineBounds, TextWrapping};

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

    /// How the box of each line is bounded above and below (a FerroUI addition,
    /// docs/porting/DEVIATIONS.md, Text block): by the ascent and the descent of the font, or
    /// trimmed to its cap height and to the baseline.
    fn text_line_bounds(&self) -> TextLineBounds {
        TextLineBounds::Full
    }

    /// Chooses the font metrics used to allocate line boxes. Custom paragraph
    /// implementations retain the existing glyph-typeface behavior by default.
    fn line_metrics_mode(&self) -> TextLineMetricsMode {
        TextLineMetricsMode::GlyphTypeface
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

    /// Explicit paragraph tab stops. `Some(&[])` enables position-based default
    /// stops; `None` retains the existing fixed-width shaping convention for
    /// callers that have not opted into paragraph tab layout.
    fn tab_stops(&self) -> Option<&[TextTabStop]> {
        None
    }

    /// The current line's leading edge relative to the tab-stop origin, including
    /// paragraph and first-line indentation. Units follow `default_incremental_tab`.
    fn tab_origin(&self) -> f64 {
        0.0
    }

    /// Gets the letter spacing.
    fn letter_spacing(&self) -> f64 {
        0.0
    }
}
