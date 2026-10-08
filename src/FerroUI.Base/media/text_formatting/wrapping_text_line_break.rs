use std::rc::Rc;

use crate::media::text_formatting::{TextLineBreak, TextRun};
use crate::media::FlowDirection;

/// A line break of a wrapped line that carries the runs the next line starts
/// with.
///
/// Upstream models this as an internal subclass of `TextLineBreak`; here it is
/// a `TextLineBreak` with remaining runs, created and consumed through these
/// functions.
///
/// Internal upstream; public so that the Skia unit tests reach it.
pub struct WrappingTextLineBreak;

impl WrappingTextLineBreak {
    pub fn new(
        text_end_of_line: Option<Rc<dyn TextRun>>,
        flow_direction: FlowDirection,
        remaining_runs: Vec<Rc<dyn TextRun>>,
    ) -> TextLineBreak {
        debug_assert!(!remaining_runs.is_empty());

        let line_break = TextLineBreak::new(text_end_of_line, flow_direction, true);
        *line_break.remaining_runs.borrow_mut() = Some(remaining_runs);
        line_break.is_wrapping.set(true);
        line_break
    }

    /// Whether the line break is a wrapping line break (C# `is WrappingTextLineBreak`).
    pub fn is_wrapping(line_break: &TextLineBreak) -> bool {
        line_break.is_wrapping.get()
    }

    /// Takes the remaining runs; a second call returns `None`.
    pub fn acquire_remaining_runs(line_break: &TextLineBreak) -> Option<Vec<Rc<dyn TextRun>>> {
        line_break.remaining_runs.borrow_mut().take()
    }
}
