use crate::media::text_formatting::TextLine;

/// Describes how a line is justified.
pub trait JustificationProperties {
    /// Gets the width in which the range is justified.
    fn width(&self) -> f64;

    /// Justifies given text line.
    fn justify(&self, text_line: &dyn TextLine);
}
