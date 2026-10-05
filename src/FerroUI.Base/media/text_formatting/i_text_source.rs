use std::rc::Rc;

use crate::media::text_formatting::TextRun;

/// Produces `TextRun` objects that are used by the `TextFormatter`.
pub trait ITextSource {
    /// Gets a `TextRun` for specified text source index (UTF-16 code units).
    fn get_text_run(&self, text_source_index: i32) -> Option<Rc<dyn TextRun>>;
}
