use crate::media::text_formatting::text_run::{text_run_any, TextRun, DEFAULT_TEXT_SOURCE_LENGTH};

/// A text run that supports end of line.
pub struct TextEndOfLine {
    length: i32,
}

impl TextEndOfLine {
    /// An end of line of the default text source length (1).
    pub fn new() -> Self {
        Self { length: DEFAULT_TEXT_SOURCE_LENGTH }
    }

    /// An end of line covering `text_source_length` text source positions.
    pub fn with_length(text_source_length: i32) -> Self {
        Self { length: text_source_length }
    }
}

impl Default for TextEndOfLine {
    fn default() -> Self {
        Self::new()
    }
}

impl TextRun for TextEndOfLine {
    fn length(&self) -> i32 {
        self.length
    }

    fn as_text_end_of_line(&self) -> Option<&TextEndOfLine> {
        Some(self)
    }

    text_run_any!();
}
