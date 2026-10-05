use crate::media::text_formatting::text_run::{text_run_any, TextRun};
use crate::media::text_formatting::TextEndOfLine;

/// A text run that supports end of paragraph.
///
/// It is an end of line too: [`TextRun::as_text_end_of_line`] returns its base.
pub struct TextEndOfParagraph {
    base: TextEndOfLine,
}

impl TextEndOfParagraph {
    /// An end of paragraph of the default text source length (1).
    pub fn new() -> Self {
        Self { base: TextEndOfLine::new() }
    }

    /// An end of paragraph covering `text_source_length` text source positions.
    pub fn with_length(text_source_length: i32) -> Self {
        Self { base: TextEndOfLine::with_length(text_source_length) }
    }
}

impl Default for TextEndOfParagraph {
    fn default() -> Self {
        Self::new()
    }
}

impl TextRun for TextEndOfParagraph {
    fn length(&self) -> i32 {
        self.base.length()
    }

    fn as_text_end_of_line(&self) -> Option<&TextEndOfLine> {
        Some(&self.base)
    }

    text_run_any!();
}
