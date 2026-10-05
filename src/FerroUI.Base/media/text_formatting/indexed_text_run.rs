use std::cell::Cell;
use std::rc::Rc;

use crate::media::text_formatting::TextRun;

/// A text run of a line together with its text source position and its
/// position in the visual run order.
pub(crate) struct IndexedTextRun {
    pub text_source_character_index: i32,
    pub run_index: Cell<i32>,
    pub next_run_index: Cell<i32>,
    pub text_run: std::cell::RefCell<Option<Rc<dyn TextRun>>>,
}
