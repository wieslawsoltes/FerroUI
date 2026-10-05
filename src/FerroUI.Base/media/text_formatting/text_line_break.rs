use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::media::text_formatting::TextRun;
use crate::media::FlowDirection;

/// Represents the state of a line break: how the line ended and what the
/// next line has to continue with.
pub struct TextLineBreak {
    text_end_of_line: Option<Rc<dyn TextRun>>,
    flow_direction: FlowDirection,
    is_split: bool,
    /// The runs left over by a wrapped line (upstream `WrappingTextLineBreak`).
    pub(crate) remaining_runs: RefCell<Option<Vec<Rc<dyn TextRun>>>>,
    /// Whether this is a wrapping line break (stays true after the runs were acquired).
    pub(crate) is_wrapping: Cell<bool>,
}

impl TextLineBreak {
    /// Creates a line break.
    ///
    /// `text_end_of_line` is the end of line run (a run whose
    /// `as_text_end_of_line` is `Some`) when the line ended with one.
    pub fn new(text_end_of_line: Option<Rc<dyn TextRun>>, flow_direction: FlowDirection, is_split: bool) -> Self {
        Self { text_end_of_line, flow_direction, is_split, remaining_runs: RefCell::new(None), is_wrapping: Cell::new(false) }
    }

    /// Get the end of line run.
    pub fn text_end_of_line(&self) -> Option<&Rc<dyn TextRun>> {
        self.text_end_of_line.as_ref()
    }

    /// Get the flow direction for remaining characters.
    pub fn flow_direction(&self) -> FlowDirection {
        self.flow_direction
    }

    /// Gets whether there were remaining runs after this line break, that
    /// were split up by the `TextFormatter` during the formatting process.
    pub fn is_split(&self) -> bool {
        self.is_split
    }
}
